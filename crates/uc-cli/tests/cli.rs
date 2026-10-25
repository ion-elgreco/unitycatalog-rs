//! `uc` against a live server: upstream's `bin/uc` commands, options and
//! output.

use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::authn::security::{ADMIN, SecurityContext};
use uc_server::{AppState, build_app};

const API: &str = "/api/2.1/unity-catalog";

/// The server signing key, made once per test binary.
fn security() -> Arc<SecurityContext> {
    static SECURITY: OnceLock<Arc<SecurityContext>> = OnceLock::new();
    SECURITY
        .get_or_init(|| Arc::new(SecurityContext::generate().unwrap()))
        .clone()
}

struct Server {
    url: String,
    dir: TempDir,
    /// A token of the bootstrap `admin`, as `uc-server token` prints it.
    admin: String,
}

impl Server {
    async fn start() -> Self {
        let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
            .await
            .expect("open the test database");
        let app = build_app(AppState::new(metastore).with_security(security()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        Self {
            url: format!("http://{addr}"),
            dir: tempfile::tempdir().unwrap(),
            admin: security()
                .create_access_token(ADMIN, std::time::Duration::from_secs(3600))
                .unwrap(),
        }
    }

    fn path(&self, name: &str) -> String {
        self.dir.path().join(name).to_string_lossy().into_owned()
    }

    /// An empty settings file: none from this machine counts.
    fn no_settings(&self) -> std::path::PathBuf {
        let path = self.dir.path().join("empty.toml");
        std::fs::write(&path, "").unwrap();
        path
    }

    /// `uc` against the server, in its directory, with no sign-ins from this
    /// machine.
    fn command(&self, args: &[&str]) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_uc"));
        command
            .arg("--server")
            .arg(&self.url)
            .args(args)
            .env_remove("UC_CLI_SERVER")
            .env_remove("UC_CLI_TOKEN")
            .env("UC_CONFIG", self.no_settings())
            .env("XDG_STATE_HOME", self.dir.path().join("state"))
            .env("UC_CLI_OUTPUT_WIDTH", "120")
            .current_dir(self.dir.path());
        command
    }

    /// Run `uc` with `stdin` as its standard input; exit code, stdout,
    /// stderr.
    async fn uc_with_input(&self, args: &[&str], stdin: &str) -> (i32, String, String) {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        input.write_all(stdin.as_bytes()).await.unwrap();
        drop(input);
        let output = child.wait_with_output().await.unwrap();
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// Run `uc`; exit code, stdout, stderr.
    async fn uc(&self, args: &[&str]) -> (i32, String, String) {
        let output = self.command(args).output().await.unwrap();
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// Run `uc … --output json` and parse what it prints.
    async fn json(&self, args: &[&str]) -> Value {
        let mut all = args.to_vec();
        all.extend(["--output", "json"]);
        let (code, out, err) = self.uc(&all).await;
        assert_eq!(code, 0, "uc {args:?} failed: {err}{out}");
        serde_json::from_str(out.trim()).unwrap_or_else(|e| panic!("uc {args:?}: {e}: {out}"))
    }

    /// [`Self::json`], signed in as the admin.
    async fn admin_json(&self, args: &[&str]) -> Value {
        let mut all = args.to_vec();
        all.extend(["--auth_token", &self.admin]);
        self.json(&all).await
    }

    async fn ok(&self, args: &[&str]) -> String {
        let (code, out, err) = self.uc(args).await;
        assert_eq!(code, 0, "uc {args:?} failed: {err}{out}");
        out
    }

    /// [`Self::ok`], signed in as the admin.
    async fn admin_ok(&self, args: &[&str]) -> String {
        let mut all = args.to_vec();
        all.extend(["--auth_token", &self.admin]);
        self.ok(&all).await
    }

    /// The call fails with exit code 1; its error.
    async fn fails(&self, args: &[&str]) -> String {
        let (code, out, err) = self.uc(args).await;
        assert_eq!(code, 1, "uc {args:?} did not fail: {out}");
        err
    }

    /// POST `body` to the API at `path`.
    async fn post(&self, path: &str, body: Value) -> Value {
        let response = reqwest::Client::new()
            .post(format!("{}{API}{path}", self.url))
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = response.status();
        let text = response.text().await.unwrap();
        assert!(status.is_success(), "POST {path}: {status} {text}");
        serde_json::from_str(&text).unwrap()
    }
}

fn len(value: &Value) -> usize {
    value.as_array().unwrap().len()
}

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn catalogs_and_schemas_as_upstream() {
    let s = Server::start().await;
    let root = format!("file://{}", s.path("root"));
    let catalog = s
        .json(&[
            "catalog",
            "create",
            "--name",
            "cli",
            "--comment",
            "CLI catalog",
            "--properties",
            r#"{"k": "v"}"#,
            "--storage_root",
            &root,
        ])
        .await;
    assert_eq!(
        keys(&catalog),
        [
            "name",
            "comment",
            "properties",
            "owner",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "id",
            "storage_root",
            "storage_location"
        ]
    );
    assert_eq!(catalog["properties"]["k"], "v");
    let listed = s.json(&["catalog", "list", "--max_results", "10"]).await;
    assert!(
        listed
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "cli")
    );

    // The default output is a table.
    let table = s.ok(&["catalog", "get", "--name", "cli"]).await;
    assert!(table.contains("│ NAME "), "{table}");
    assert!(table.contains("┆ cli "), "{table}");
    assert!(table.starts_with("┌─"), "{table}");

    // jsonPretty is Jackson's printer.
    let pretty = s
        .ok(&["catalog", "get", "--name", "cli", "--output", "jsonPretty"])
        .await;
    assert!(pretty.starts_with("{\n  \"name\" : \"cli\",\n"), "{pretty}");

    let updated = s
        .json(&["catalog", "update", "--name", "cli", "--comment", "new"])
        .await;
    assert_eq!(updated["comment"], "new");

    let schema = s
        .json(&["schema", "create", "--catalog", "cli", "--name", "s"])
        .await;
    assert_eq!(schema["full_name"], "cli.s");
    assert_eq!(keys(&schema)[0], "name");
    let schemas = s.json(&["schema", "list", "--catalog", "cli"]).await;
    assert_eq!(schemas.as_array().unwrap().len(), 1);
    let got = s.json(&["schema", "get", "--full_name", "cli.s"]).await;
    assert_eq!(got["schema_id"], schema["schema_id"]);
    let renamed = s
        .json(&[
            "schema",
            "update",
            "--full_name",
            "cli.s",
            "--new_name",
            "s2",
        ])
        .await;
    assert_eq!(renamed["full_name"], "cli.s2");
    assert_eq!(
        s.ok(&[
            "schema",
            "delete",
            "--full_name",
            "cli.s2",
            "--output",
            "json"
        ])
        .await,
        "\n"
    );

    // Errors are upstream's ApiException message, with exit code 1.
    let (code, _, err) = s.uc(&["catalog", "get", "--name", "missing"]).await;
    assert_eq!(code, 1);
    assert!(err.contains("getCatalog call failed with: 404 - "), "{err}");

    let deleted = s
        .ok(&[
            "catalog", "delete", "--name", "cli", "--force", "true", "--output", "json",
        ])
        .await;
    assert_eq!(deleted, "{}\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_errors_and_help() {
    let s = Server::start().await;
    // clap's usage errors: on stderr, with exit code 2.
    let (code, _, err) = s.uc(&["catalog"]).await;
    assert_eq!(code, 2);
    assert!(
        err.contains("Usage: uc catalog [OPTIONS] <COMMAND>"),
        "{err}"
    );
    let (code, _, err) = s.uc(&["catalog", "get"]).await;
    assert_eq!(code, 2);
    assert!(
        err.contains("the following required arguments were not provided:\n  --name <NAME>"),
        "{err}"
    );
    let (code, _, err) = s.uc(&["catalog", "list", "--bogus", "x"]).await;
    assert_eq!(code, 2);
    assert!(err.contains("unexpected argument '--bogus' found"), "{err}");
    // An update needs a change.
    let (code, _, err) = s.uc(&["catalog", "update", "--name", "c"]).await;
    assert_eq!(code, 2);
    assert!(
        err.contains("<--new_name <NEW_NAME>|--comment <COMMENT>|--properties <PROPERTIES>>"),
        "{err}"
    );

    let out = s.ok(&["table", "--help"]).await;
    for operation in ["create", "list", "get", "delete", "read", "write"] {
        assert!(out.contains(&format!("\n  {operation} ")), "{out}");
    }
    let out = s.ok(&["--version"]).await;
    assert_eq!(out.trim(), concat!("uc ", env!("CARGO_PKG_VERSION")));
}

#[tokio::test(flavor = "multi_thread")]
async fn delta_tables_create_write_and_read() {
    let s = Server::start().await;
    let root = format!("file://{}", s.path("root"));
    s.ok(&["catalog", "create", "--name", "c", "--storage_root", &root])
        .await;
    s.ok(&["schema", "create", "--catalog", "c", "--name", "s"])
        .await;

    // EXTERNAL: a Delta table at the location, then registered.
    let location = s.path("ext");
    let table = s
        .json(&[
            "table",
            "create",
            "--full_name",
            "c.s.ext",
            "--columns",
            "id INT, name STRING, score DOUBLE",
            "--storage_location",
            &location,
        ])
        .await;
    assert_eq!(table["table_type"], "EXTERNAL");
    assert_eq!(
        table["columns"][0]["type_json"],
        r#"{"name":"id","type":"integer","nullable":true,"metadata":{}}"#
    );
    assert_eq!(table["columns"][0]["nullable"], true);
    assert!(
        std::path::Path::new(&location)
            .join("_delta_log/00000000000000000000.json")
            .exists()
    );
    let out = s.ok(&["table", "write", "--full_name", "c.s.ext"]).await;
    assert!(
        out.starts_with("Table written to successfully at: file://"),
        "{out}"
    );
    let read = s
        .ok(&[
            "table",
            "read",
            "--full_name",
            "c.s.ext",
            "--max_results",
            "4",
        ])
        .await;
    let lines: Vec<&str> = read.lines().collect();
    assert!(lines[1].starts_with("| id(integer) "), "{read}");
    assert!(lines[1].contains("| name(string) "), "{read}");
    // Arrow's table: a rule, the header, a rule, four rows, a rule.
    assert_eq!(lines.len(), 3 + 4 + 1, "{read}");
    assert!(lines[3].starts_with("| 1 "), "{read}");

    // MANAGED: a catalog-managed Delta table in a staging location.
    let managed = s
        .json(&[
            "table",
            "create",
            "--full_name",
            "c.s.man",
            "--columns",
            "id INT",
            "--table_type",
            "MANAGED",
        ])
        .await;
    assert_eq!(managed["table_type"], "MANAGED");
    assert_eq!(
        managed["properties"]["delta.feature.catalogManaged"],
        "supported"
    );
    assert_eq!(
        managed["properties"]["io.unitycatalog.tableId"],
        managed["table_id"]
    );
    // Writes are ratified by the server's commit coordinator.
    s.ok(&["table", "write", "--full_name", "c.s.man"]).await;
    s.ok(&["table", "write", "--full_name", "c.s.man"]).await;
    let read = s.ok(&["table", "read", "--full_name", "c.s.man"]).await;
    let rows = read
        .lines()
        .filter(|l| l.starts_with("| ") && !l.contains("(integer)"))
        .count();
    assert_eq!(rows, 30, "{read}");

    let tables = s
        .json(&["table", "list", "--catalog", "c", "--schema", "s"])
        .await;
    assert_eq!(tables.as_array().unwrap().len(), 2);
    let got = s.json(&["table", "get", "--full_name", "c.s.ext"]).await;
    assert_eq!(got["table_id"], table["table_id"]);
    assert_eq!(keys(&got)[0], "name");
    assert_eq!(got["columns"][2]["name"], "score");
    s.ok(&["table", "delete", "--full_name", "c.s.ext"]).await;
    let err = s.fails(&["table", "get", "--full_name", "c.s.ext"]).await;
    assert!(err.contains("getTable call failed with: 404 - "), "{err}");
}

/// As upstream, an EXTERNAL table may go over a Delta table that is already
/// at its location, such as one that a failed attempt left behind; but
/// only one with the same columns.
#[tokio::test(flavor = "multi_thread")]
async fn external_tables_over_a_delta_table() {
    let s = Server::start().await;
    s.ok(&["catalog", "create", "--name", "c"]).await;
    s.ok(&["schema", "create", "--catalog", "c", "--name", "s"])
        .await;
    let location = s.path("t");
    let columns = "id INT, price DECIMAL(10,2)";
    let create = [
        "table",
        "create",
        "--full_name",
        "c.s.t",
        "--columns",
        columns,
        "--storage_location",
        &location,
    ];
    s.ok(&create).await;
    s.ok(&["table", "write", "--full_name", "c.s.t"]).await;
    // DECIMAL columns get values too: from 0 to 999.99.
    let read = s.ok(&["table", "read", "--full_name", "c.s.t"]).await;
    let prices: Vec<&str> = read
        .lines()
        .filter(|l| l.starts_with("| ") && !l.contains("(integer)"))
        .map(|l| l.split('|').nth(2).unwrap().trim())
        .collect();
    assert_eq!(prices.len(), 15, "{read}");
    assert!(
        prices
            .iter()
            .all(|p| p.parse::<f64>().is_ok_and(|v| (0.0..1000.0).contains(&v))),
        "{read}"
    );

    // An EXTERNAL table's data stays when it is deleted: the same columns
    // go over it again.
    s.ok(&["table", "delete", "--full_name", "c.s.t"]).await;
    s.ok(&create).await;
    let read = s.ok(&["table", "read", "--full_name", "c.s.t"]).await;
    assert_eq!(read.lines().filter(|l| l.starts_with("| ")).count(), 16);
    // Other columns, other types or another order do not.
    for other in [
        "id INT",
        "id LONG, price DECIMAL(10,2)",
        "price DECIMAL(10,2), id INT",
    ] {
        let mut args = create;
        (args[3], args[5]) = ("c.s.u", other);
        let err = s.fails(&args).await;
        assert!(
            err.contains(
                "the Delta table there has other columns: id integer, price decimal(10,2)"
            ),
            "{err}"
        );
    }
    let err = s.fails(&["table", "get", "--full_name", "c.s.u"]).await;
    assert!(err.contains("404"), "{err}");
}

/// Delta Kernel writes no Iceberg or Hudi metadata, so `uc` makes no
/// UniForm tables.
#[tokio::test(flavor = "multi_thread")]
async fn no_uniform_tables() {
    let s = Server::start().await;
    s.ok(&["catalog", "create", "--name", "c"]).await;
    s.ok(&["schema", "create", "--catalog", "c", "--name", "s"])
        .await;
    let location = s.path("t");
    let err = s
        .fails(&[
            "table",
            "create",
            "--full_name",
            "c.s.t",
            "--columns",
            "id INT",
            "--storage_location",
            &location,
            "--properties",
            r#"{"delta.universalFormat.enabledFormats": "iceberg"}"#,
        ])
        .await;
    assert!(err.contains("uc cannot create UniForm tables yet"), "{err}");
    assert!(!std::path::Path::new(&location).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn volumes_functions_and_models() {
    let s = Server::start().await;
    let root = format!("file://{}", s.path("root"));
    s.ok(&["catalog", "create", "--name", "c", "--storage_root", &root])
        .await;
    s.ok(&["schema", "create", "--catalog", "c", "--name", "s"])
        .await;

    let location = s.path("vol");
    std::fs::create_dir_all(&location).unwrap();
    let volume = s
        .json(&[
            "volume",
            "create",
            "--full_name",
            "c.s.v",
            "--storage_location",
            &location,
        ])
        .await;
    assert_eq!(volume["volume_type"], "EXTERNAL");
    let out = s.ok(&["volume", "write", "--full_name", "c.s.v"]).await;
    let written = out
        .lines()
        .next()
        .unwrap()
        .strip_prefix("File written to: ")
        .unwrap();
    let file_name = written.rsplit('/').next().unwrap().to_string();
    let listing = s.ok(&["volume", "read", "--full_name", "c.s.v"]).await;
    assert_eq!(listing.trim(), format!("{file_name} [file]"));
    let content = s
        .ok(&[
            "volume",
            "read",
            "--full_name",
            "c.s.v",
            "--path",
            &file_name,
        ])
        .await;
    assert_eq!(content.trim(), "This is a test file with random content.");
    let (code, _, err) = s
        .uc(&["volume", "read", "--full_name", "c.s.v", "--path", "../x"])
        .await;
    assert_eq!(code, 1);
    assert!(
        err.contains("Resolved URI is outside the base URI path"),
        "{err}"
    );

    // `--source` uploads a local file, to `--path` or its own name.
    let source = s.path("data.csv");
    std::fs::write(&source, "id,name\n1,a\n").unwrap();
    let out = s
        .ok(&[
            "volume",
            "write",
            "--full_name",
            "c.s.v",
            "--source",
            &source,
        ])
        .await;
    assert!(out.trim().ends_with("/data.csv"), "{out}");
    s.ok(&[
        "volume",
        "write",
        "--full_name",
        "c.s.v",
        "--source",
        &source,
        "--path",
        "raw/data.csv",
    ])
    .await;
    // The file's text, without the new line `uc` prints after it.
    let read = async |path: &str| {
        let out = s
            .ok(&["volume", "read", "--full_name", "c.s.v", "--path", path])
            .await;
        out.strip_suffix('\n').unwrap_or(&out).to_string()
    };
    assert_eq!(read("data.csv").await, "id,name\n1,a\n");
    assert_eq!(read("raw/data.csv").await, "id,name\n1,a\n");
    // `-` is standard input, which needs `--path`.
    let (code, _, _) = s
        .uc_with_input(
            &[
                "volume",
                "write",
                "--full_name",
                "c.s.v",
                "--source",
                "-",
                "--path",
                "in.txt",
            ],
            "from stdin",
        )
        .await;
    assert_eq!(code, 0);
    assert_eq!(read("in.txt").await, "from stdin");
    let (code, _, err) = s
        .uc(&["volume", "write", "--full_name", "c.s.v", "--source", "-"])
        .await;
    assert_eq!(code, 1);
    assert!(err.contains("--path is required"), "{err}");
    let (code, _, err) = s
        .uc(&[
            "volume",
            "write",
            "--full_name",
            "c.s.v",
            "--source",
            &source,
            "--path",
            "../x",
        ])
        .await;
    assert_eq!(code, 1);
    assert!(err.contains("Write Failed"), "{err}");

    let volumes = s
        .json(&["volume", "list", "--catalog", "c", "--schema", "s"])
        .await;
    assert_eq!(len(&volumes), 1);
    assert_eq!(volumes[0]["full_name"], "c.s.v");
    let got = s.json(&["volume", "get", "--full_name", "c.s.v"]).await;
    assert_eq!(got["volume_id"], volume["volume_id"]);
    let updated = s
        .json(&[
            "volume",
            "update",
            "--full_name",
            "c.s.v",
            "--comment",
            "files",
        ])
        .await;
    assert_eq!(updated["comment"], "files");
    let renamed = s
        .json(&[
            "volume",
            "update",
            "--full_name",
            "c.s.v",
            "--new_name",
            "v2",
        ])
        .await;
    assert_eq!(renamed["full_name"], "c.s.v2");
    s.ok(&["volume", "delete", "--full_name", "c.s.v2"]).await;
    let volumes = s
        .json(&["volume", "list", "--catalog", "c", "--schema", "s"])
        .await;
    assert_eq!(len(&volumes), 0);

    let function = s
        .json(&[
            "function",
            "create",
            "--full_name",
            "c.s.add",
            "--input_params",
            "a INT, b INT",
            "--data_type",
            "INT",
            "--def",
            r"c = a + b\nreturn c",
        ])
        .await;
    assert_eq!(function["routine_body"], "EXTERNAL");
    assert_eq!(function["external_language"], "python");
    assert_eq!(function["input_params"]["parameters"][1]["name"], "b");
    // `function call` runs the function with python3, which the tests need.
    let result = s
        .ok(&[
            "function",
            "call",
            "--full_name",
            "c.s.add",
            "--input_params",
            "1,2",
        ])
        .await;
    assert_eq!(result, "3\n\n");
    let functions = s
        .json(&["function", "list", "--catalog", "c", "--schema", "s"])
        .await;
    assert_eq!(len(&functions), 1);
    let got = s.json(&["function", "get", "--full_name", "c.s.add"]).await;
    assert_eq!(got["routine_definition"], r"c = a + b\nreturn c");
    s.ok(&["function", "delete", "--full_name", "c.s.add"])
        .await;
    let err = s
        .fails(&["function", "get", "--full_name", "c.s.add"])
        .await;
    assert!(
        err.contains("getFunction call failed with: 404 - "),
        "{err}"
    );

    let model = s
        .json(&[
            "registered_model",
            "create",
            "--catalog",
            "c",
            "--schema",
            "s",
            "--name",
            "m",
        ])
        .await;
    assert_eq!(model["full_name"], "c.s.m");
    let version = s
        .json(&[
            "model_version",
            "create",
            "--catalog",
            "c",
            "--schema",
            "s",
            "--name",
            "m",
            "--source",
            "file:///tmp/model",
        ])
        .await;
    assert_eq!(version["version"], 1);
    assert_eq!(keys(&version)[0], "model_name");
    let finalized = s
        .json(&[
            "model_version",
            "finalize",
            "--full_name",
            "c.s.m",
            "--version",
            "1",
        ])
        .await;
    assert_eq!(finalized["status"], "READY");
    let updated = s
        .json(&[
            "model_version",
            "update",
            "--full_name",
            "c.s.m",
            "--version",
            "1",
            "--comment",
            "v1",
        ])
        .await;
    assert_eq!(updated["comment"], "v1");
    let versions = s
        .json(&["model_version", "list", "--full_name", "c.s.m"])
        .await;
    assert_eq!(versions.as_array().unwrap().len(), 1);
    let got = s
        .json(&[
            "model_version",
            "get",
            "--full_name",
            "c.s.m",
            "--version",
            "1",
        ])
        .await;
    assert_eq!(got["comment"], "v1");
    assert_eq!(got["source"], "file:///tmp/model");
    let got = s
        .json(&["registered_model", "get", "--full_name", "c.s.m"])
        .await;
    assert_eq!(got["full_name"], "c.s.m");
    assert_eq!(keys(&got)[0], "name");
    let renamed = s
        .json(&[
            "registered_model",
            "update",
            "--full_name",
            "c.s.m",
            "--new_name",
            "m2",
        ])
        .await;
    assert_eq!(renamed["full_name"], "c.s.m2");
    let models = s
        .json(&[
            "registered_model",
            "list",
            "--catalog",
            "c",
            "--schema",
            "s",
        ])
        .await;
    assert_eq!(models.as_array().unwrap().len(), 1);

    s.ok(&[
        "model_version",
        "delete",
        "--full_name",
        "c.s.m2",
        "--version",
        "1",
    ])
    .await;
    let versions = s
        .json(&["model_version", "list", "--full_name", "c.s.m2"])
        .await;
    assert_eq!(len(&versions), 0);
    s.ok(&["registered_model", "delete", "--full_name", "c.s.m2"])
        .await;
    let models = s.json(&["registered_model", "list"]).await;
    assert_eq!(len(&models), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn users_permissions_credentials_and_locations() {
    let s = Server::start().await;
    let user = s
        .json(&[
            "user",
            "create",
            "--name",
            "Ada",
            "--email",
            "ada@example.com",
        ])
        .await;
    assert_eq!(
        keys(&user),
        [
            "id",
            "name",
            "email",
            "external_id",
            "state",
            "picture_url",
            "created_at",
            "updated_at"
        ]
    );
    assert_eq!(user["state"], "ENABLED");
    assert_eq!(user["email"], "ada@example.com");
    let id = user["id"].as_str().unwrap().to_string();
    let renamed = s
        .json(&["user", "update", "--id", &id, "--name", "Ada L"])
        .await;
    assert_eq!(renamed["name"], "Ada L");
    // The email names the user in grants: it cannot change (upstream takes
    // the option, and its server keeps the email).
    let err = s
        .fails(&["user", "update", "--id", &id, "--email", "a@example.com"])
        .await;
    assert!(err.contains("A user's email cannot change"), "{err}");
    let users = s
        .json(&[
            "user",
            "list",
            "--filter",
            "userName eq \"ada@example.com\"",
        ])
        .await;
    assert_eq!(users.as_array().unwrap().len(), 1);
    let got = s.json(&["user", "get", "--id", &id]).await;
    assert_eq!(got["name"], "Ada L");
    assert_eq!(keys(&got), keys(&user));

    s.ok(&["catalog", "create", "--name", "c"]).await;
    let granted = s
        .json(&[
            "permission",
            "create",
            "--securable_type",
            "catalog",
            "--name",
            "c",
            "--principal",
            "ada@example.com",
            "--privilege",
            "USE CATALOG",
        ])
        .await;
    assert_eq!(granted[0]["principal"], "ada@example.com");
    assert_eq!(granted[0]["privileges"][0], "USE CATALOG");
    let listed = s
        .json(&[
            "permission",
            "get",
            "--securable_type",
            "catalog",
            "--name",
            "c",
            "--principal",
            "ada@example.com",
        ])
        .await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    let revoked = s
        .json(&[
            "permission",
            "delete",
            "--securable_type",
            "catalog",
            "--name",
            "c",
            "--principal",
            "ada@example.com",
            "--privilege",
            "USE CATALOG",
        ])
        .await;
    assert!(revoked.as_array().unwrap().is_empty());

    let summary = s.json(&["metastore", "get"]).await;
    assert!(summary["metastore_id"].is_string());

    let credential = s
        .json(&[
            "credential",
            "create",
            "--name",
            "cred",
            "--comment",
            "c1",
            "--aws_iam_role_arn",
            "arn:aws:iam::123456789012:role/uc-test",
        ])
        .await;
    assert_eq!(credential["purpose"], "STORAGE");
    let updated = s
        .json(&["credential", "update", "--name", "cred", "--comment", "c2"])
        .await;
    assert_eq!(updated["comment"], "c2");
    let credentials = s.json(&["credential", "list"]).await;
    assert_eq!(len(&credentials), 1);
    let got = s.json(&["credential", "get", "--name", "cred"]).await;
    assert_eq!(got["id"], credential["id"]);
    assert_eq!(
        got["aws_iam_role"]["role_arn"],
        "arn:aws:iam::123456789012:role/uc-test"
    );
    let location = s
        .json(&[
            "external_location",
            "create",
            "--name",
            "loc",
            "--url",
            "s3://bucket/path",
            "--credential_name",
            "cred",
        ])
        .await;
    assert_eq!(location["url"], "s3://bucket/path");
    let moved = s
        .json(&[
            "external_location",
            "update",
            "--name",
            "loc",
            "--new_name",
            "loc2",
            "--url",
            "s3://bucket/other",
        ])
        .await;
    assert_eq!(moved["name"], "loc2");
    assert_eq!(moved["url"], "s3://bucket/other");
    let locations = s.json(&["external_location", "list"]).await;
    assert_eq!(len(&locations), 1);
    let got = s
        .json(&["external_location", "get", "--name", "loc2"])
        .await;
    assert_eq!(got["credential_name"], "cred");
    assert_eq!(keys(&got)[..3], ["name", "id", "url"]);
    assert_eq!(
        s.ok(&[
            "external_location",
            "delete",
            "--name",
            "loc2",
            "--output",
            "json"
        ])
        .await,
        "{}\n"
    );
    assert_eq!(
        s.ok(&["credential", "delete", "--name", "cred", "--output", "json"])
            .await,
        "{}\n"
    );

    s.ok(&["user", "delete", "--id", &id]).await;
    // A deleted user stays, disabled.
    let got = s.json(&["user", "get", "--id", &id]).await;
    assert_eq!(got["state"], "DISABLED");

    // A bad identity token is refused by the token exchange.
    let (code, _, err) = s
        .uc(&["auth", "login", "--identity_token", "not-a-jwt"])
        .await;
    assert_eq!(code, 1);
    assert!(err.contains("Error authenticating - "), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn server_and_token_come_from_uc_toml_in_a_folder_above() {
    let s = Server::start().await;
    std::fs::write(
        s.dir.path().join("uc.toml"),
        format!("# The CLI's settings\n[cli]\nserver = \"{}\"\n", s.url),
    )
    .unwrap();
    let below = s.dir.path().join("project/notebooks");
    std::fs::create_dir_all(&below).unwrap();
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_uc"))
        .args(["catalog", "list", "--output", "json"])
        .env_remove("UC_CLI_SERVER")
        .env_remove("UC_CLI_TOKEN")
        .env_remove("UC_CONFIG")
        .env("XDG_STATE_HOME", s.dir.path().join("state"))
        .current_dir(&below)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[]");
}

#[tokio::test(flavor = "multi_thread")]
async fn personal_access_tokens_need_a_signed_in_user() {
    let s = Server::start().await;
    let err = s.fails(&["token", "list"]).await;
    assert!(err.contains("listTokens call failed with: 401 - "), "{err}");
    assert!(err.contains("No authorization found."), "{err}");

    let expires_at = chrono::Utc::now().timestamp_millis() + 3_600_000;
    let created = s
        .admin_json(&[
            "token",
            "create",
            "--name",
            "ci",
            "--expires_at",
            &expires_at.to_string(),
        ])
        .await;
    assert_eq!(created["name"], "ci");
    assert_eq!(created["owner"], "admin");
    assert_eq!(created["expires_at"], expires_at);
    let secret = created["token_value"].as_str().unwrap().to_string();
    let id = created["id"].as_str().unwrap().to_string();

    // The token signs the CLI in.
    let listed = s.json(&["token", "list", "--auth_token", &secret]).await;
    assert_eq!(len(&listed), 1);
    assert_eq!(listed[0]["id"], id.as_str());
    // A token's value is shown once.
    assert!(listed[0].get("token_value").is_none());

    assert_eq!(
        s.admin_ok(&["token", "delete", "--id", &id, "--output", "json"])
            .await,
        "\n"
    );
    assert_eq!(len(&s.admin_json(&["token", "list"]).await), 0);
    let err = s.fails(&["token", "list", "--auth_token", &secret]).await;
    assert!(err.contains("401"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn groups() {
    let s = Server::start().await;
    let group = s.json(&["group", "create", "--name", "admins"]).await;
    assert_eq!(group["displayName"], "admins");
    let id = group["id"].as_str().unwrap().to_string();
    s.json(&["group", "create", "--name", "readers"]).await;

    assert_eq!(len(&s.json(&["group", "list"]).await), 2);
    let filtered = s
        .json(&["group", "list", "--filter", r#"displayName eq "admins""#])
        .await;
    assert_eq!(len(&filtered), 1);
    assert_eq!(filtered[0]["id"], id.as_str());
    let page = s
        .json(&["group", "list", "--start_index", "2", "--count", "1"])
        .await;
    assert_eq!(len(&page), 1);
    assert_eq!(page[0]["displayName"], "readers");

    let got = s.json(&["group", "get", "--id", &id]).await;
    assert_eq!(got["displayName"], "admins");
    s.ok(&["group", "delete", "--id", &id]).await;
    assert_eq!(len(&s.json(&["group", "list"]).await), 1);
    let err = s.fails(&["group", "get", "--id", &id]).await;
    assert!(err.contains("getGroup call failed with: 404 - "), "{err}");
}

/// Upstream prints only the page of a list. The option that gives the next
/// page goes to stderr.
#[tokio::test(flavor = "multi_thread")]
async fn lists_say_how_to_get_the_next_page() {
    let s = Server::start().await;
    for name in ["a", "b"] {
        s.ok(&["catalog", "create", "--name", name]).await;
        s.ok(&["group", "create", "--name", name]).await;
    }
    let (code, out, err) = s
        .uc(&["catalog", "list", "--max_results", "1", "--output", "json"])
        .await;
    assert_eq!(code, 0, "{err}");
    assert_eq!(serde_json::from_str::<Value>(&out).unwrap()[0]["name"], "a");
    assert_eq!(err, "Next page: --page_token a\n");
    let next = s
        .json(&["catalog", "list", "--max_results", "1", "--page_token", "a"])
        .await;
    assert_eq!(next[0]["name"], "b");
    // A page that is not full is the last one.
    let (_, _, err) = s.uc(&["catalog", "list"]).await;
    assert!(!err.contains("Next page"), "{err}");

    // SCIM lists page with --start_index.
    let (_, _, err) = s.uc(&["group", "list", "--count", "1"]).await;
    assert_eq!(err, "Next page: --start_index 2\n");
    let (_, _, err) = s.uc(&["group", "list", "--start_index", "2"]).await;
    assert!(!err.contains("Next page"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_backs_up_restores_and_audits() {
    let s = Server::start().await;
    s.ok(&["catalog", "create", "--name", "kept"]).await;

    // Only the admin may.
    let err = s.fails(&["admin", "backup"]).await;
    assert!(err.contains("backup call failed with: 401 - "), "{err}");

    // A snapshot prints as JSON whatever the format, for `admin restore`.
    let snapshot = s.admin_ok(&["admin", "backup"]).await;
    let parsed: Value = serde_json::from_str(snapshot.trim()).unwrap();
    assert_eq!(parsed["version"], 1);
    assert!(
        parsed["tables"]["catalog"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "kept")
    );
    let pretty = s
        .admin_ok(&["admin", "backup", "--output", "jsonPretty"])
        .await;
    assert!(pretty.starts_with("{\n  \""), "{pretty}");
    assert_eq!(serde_json::from_str::<Value>(&pretty).unwrap(), parsed);

    // Restoring replaces what is there with the snapshot.
    let file = s.path("snapshot.json");
    std::fs::write(&file, &snapshot).unwrap();
    s.ok(&["catalog", "create", "--name", "later"]).await;
    let report = s.admin_json(&["admin", "restore", "--path", &file]).await;
    assert!(report["skipped"].as_array().unwrap().is_empty(), "{report}");
    assert!(report["imported"]["catalog"].as_u64().unwrap() >= 1);
    let names: Vec<Value> = s
        .json(&["catalog", "list"])
        .await
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].clone())
        .collect();
    assert_eq!(names, [json!("kept")]);

    // `--path -` reads standard input.
    s.ok(&["catalog", "create", "--name", "later"]).await;
    let (code, out, err) = s
        .uc_with_input(
            &[
                "admin",
                "restore",
                "--path",
                "-",
                "--output",
                "json",
                "--auth_token",
                &s.admin,
            ],
            &snapshot,
        )
        .await;
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("\"imported\""), "{out}");
    assert_eq!(len(&s.json(&["catalog", "list"]).await), 1);

    let err = s
        .fails(&[
            "admin",
            "restore",
            "--path",
            &s.path("missing.json"),
            "--auth_token",
            &s.admin,
        ])
        .await;
    assert!(err.starts_with("Error: "), "{err}");

    // The audit log has the writes; the server records them in the
    // background.
    let audit = |args: &'static [&'static str]| {
        let s = &s;
        async move {
            let mut all = vec!["admin", "audit"];
            all.extend(args);
            s.admin_json(&all).await
        }
    };
    let start = Instant::now();
    let events = loop {
        let events = audit(&[
            "--method",
            "POST",
            "--path_prefix",
            "/api/2.1/unity-catalog/catalogs",
        ])
        .await;
        if len(&events) >= 3 {
            break events;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "{events}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    for event in events.as_array().unwrap() {
        assert_eq!(event["method"], "POST");
        assert!(
            event["path"]
                .as_str()
                .unwrap()
                .starts_with("/api/2.1/unity-catalog/catalogs")
        );
    }
    let limited = audit(&["--max_results", "1"]).await;
    assert_eq!(len(&limited), 1);
    let by_admin = audit(&["--principal", "admin"]).await;
    assert!(len(&by_admin) >= 1, "{by_admin}");
    assert!(
        by_admin
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["principal"] == "admin")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn iceberg_rest_catalog() {
    let s = Server::start().await;
    let root = format!("file://{}", s.path("root"));
    s.ok(&["catalog", "create", "--name", "c", "--storage_root", &root])
        .await;
    s.ok(&["schema", "create", "--catalog", "c", "--name", "s"])
        .await;
    s.ok(&["schema", "create", "--catalog", "c", "--name", "t"])
        .await;

    let config = s.json(&["iceberg", "config", "--catalog", "c"]).await;
    assert_eq!(config["overrides"]["prefix"], "catalogs/c");
    let err = s.fails(&["iceberg", "config", "--catalog", "nope"]).await;
    assert!(err.contains("getConfig call failed with: 404 - "), "{err}");

    let namespaces = s
        .json(&["iceberg", "list_namespaces", "--catalog", "c"])
        .await;
    assert_eq!(namespaces, json!([["s"], ["t"]]));
    // As the Iceberg REST catalog, a page size pages only with a token.
    let page = s
        .json(&[
            "iceberg",
            "list_namespaces",
            "--catalog",
            "c",
            "--max_results",
            "1",
            "--page_token",
            "",
        ])
        .await;
    assert_eq!(page, json!([["s"]]));
    let tables = s
        .json(&["iceberg", "list_tables", "--catalog", "c", "--schema", "s"])
        .await;
    assert_eq!(tables, json!([]));

    // A MANAGED Delta table with UniForm Iceberg metadata, made through the
    // Delta API, with upstream's metadata file.
    let staging = s
        .post(
            "/delta/v1/catalogs/c/schemas/s/staging-tables",
            json!({ "name": "uni" }),
        )
        .await;
    let location = staging["location"].as_str().unwrap().to_string();
    let file = std::path::PathBuf::from(location.strip_prefix("file://").unwrap())
        .join("metadata/v1.metadata.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let fixture = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../uc-server/tests/fixtures/iceberg.metadata.json"
    ))
    .unwrap();
    std::fs::write(
        &file,
        fixture.replace("file:/tmp/uniform_iceberg_table", &location),
    )
    .unwrap();
    s.post(
        "/delta/v1/catalogs/c/schemas/s/tables",
        json!({
            "name": "uni", "location": location, "table-type": "MANAGED",
            "columns": { "type": "struct", "fields": [
                { "name": "x", "type": "integer", "nullable": true, "metadata": {} },
            ] },
            "protocol": {
                "min-reader-version": 3, "min-writer-version": 7,
                "reader-features": ["catalogManaged", "deletionVectors", "v2Checkpoint",
                    "vacuumProtocolCheck"],
                "writer-features": ["catalogManaged", "deletionVectors", "v2Checkpoint",
                    "vacuumProtocolCheck", "inCommitTimestamp"],
            },
            "properties": {
                "delta.enableDeletionVectors": "true", "delta.checkpointPolicy": "v2",
                "delta.enableInCommitTimestamps": "true",
                "delta.checkpoint.writeStatsAsStruct": "true",
                "delta.checkpoint.writeStatsAsJson": "true",
                "io.unitycatalog.tableId": staging["table-id"],
                "delta.universalFormat.enabledFormats": "iceberg",
            },
            "last-commit-timestamp-ms": 1_717_798_710_940i64,
            "uniform": { "iceberg": {
                "metadata-location": format!("{location}/metadata/v1.metadata.json"),
                "converted-delta-version": 1,
                "converted-delta-timestamp": 1_717_798_710_940i64,
            } },
        }),
    )
    .await;

    let tables = s
        .json(&["iceberg", "list_tables", "--catalog", "c", "--schema", "s"])
        .await;
    assert_eq!(tables, json!([{ "namespace": ["s"], "name": "uni" }]));
    let table = s
        .json(&["iceberg", "load_table", "--full_name", "c.s.uni"])
        .await;
    assert_eq!(table["metadata-location"], file.to_str().unwrap());
    assert_eq!(
        table["metadata"]["table-uuid"],
        "55d4dc69-5b14-4483-bfc8-f33b80f99f99"
    );
    let err = s
        .fails(&["iceberg", "load_table", "--full_name", "c.s.nope"])
        .await;
    assert!(err.contains("loadTable call failed with: 404 - "), "{err}");
    assert!(err.contains("NoSuchTableException"), "{err}");
}
