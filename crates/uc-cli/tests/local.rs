//! `uc serve`, `uc dev` and `uc web`: uc-server and the web UI on this
//! machine.

use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStdout, Command};

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{AppState, build_app};

/// A uc-server in this process; its URL.
async fn server() -> String {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database");
    let app = build_app(AppState::new(metastore));
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
    format!("http://{addr}")
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// `uc` with no settings from this machine: an empty settings file, no
/// server, no UI (a `ui.dir` that does not exist) and no sign-ins.
fn uc(dir: &Path) -> Command {
    let settings = dir.join("empty.toml");
    std::fs::write(&settings, "").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_uc"));
    command
        .env_remove("UC_CLI_SERVER")
        .env_remove("UC_CLI_TOKEN")
        .env_remove("UC_CLI_SERVER_BIN")
        .env_remove("UC_UI_ENABLED")
        .env_remove("UC_DB_URL")
        .env("UC_UI_DIR", dir.join("no-ui"))
        .env("UC_CONFIG", settings)
        .env("XDG_STATE_HOME", dir.join("state"))
        .current_dir(dir)
        .kill_on_drop(true);
    command
}

/// Wait until `url` answers, while `child` runs.
async fn wait_for(url: &str, child: &mut Child) {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("uc stopped: {status}");
        }
        if reqwest::get(url).await.is_ok() {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "{url} does not answer"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn web_serves_the_ui_and_sends_api_calls_to_the_server() {
    let server = server().await;
    let dir = tempfile::tempdir().unwrap();
    let ui = dir.path().join("ui");
    std::fs::create_dir_all(ui.join("assets")).unwrap();
    std::fs::write(ui.join("index.html"), "<html>ui</html>").unwrap();
    std::fs::write(ui.join("uc_ui.js"), "js").unwrap();
    let js = "export function start() {}\n".repeat(1000);
    std::fs::write(ui.join("assets/uc_ui.js"), &js).unwrap();
    let port = free_port();
    let mut web = uc(dir.path())
        .args(["web", "--port", &port.to_string(), "--no_browser"])
        .args(["--server", &server])
        .env("UC_UI_DIR", &ui)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let url = format!("http://127.0.0.1:{port}");
    wait_for(&url, &mut web).await;

    let client = reqwest::Client::new();
    let text = |path: &str| {
        let request = client.get(format!("{url}{path}"));
        async move {
            let response = request.send().await.unwrap();
            (response.status().as_u16(), response.text().await.unwrap())
        }
    };
    assert_eq!(text("/").await, (200, "<html>ui</html>".into()));
    assert_eq!(text("/uc_ui.js").await, (200, "js".into()));
    // Client-side routes load the UI.
    assert_eq!(text("/data/main").await, (200, "<html>ui</html>".into()));
    // The UI's files go as uc-server sends them: compressed, and checked
    // with the server before a browser uses a kept copy.
    let asset = client
        .get(format!("{url}/assets/uc_ui.js"))
        .header("accept-encoding", "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(asset.status(), 200);
    assert_eq!(asset.headers()["content-encoding"], "gzip");
    assert_eq!(asset.headers()["cache-control"], "no-cache");
    assert!(asset.bytes().await.unwrap().len() < js.len() / 10);
    let page = client.get(&url).send().await.unwrap();
    assert_eq!(page.headers()["cache-control"], "no-cache");

    // The UI signs in with the token of the `uc` commands.
    let mode = client.get(format!("{url}/auth/mode")).send().await.unwrap();
    assert_eq!(mode.status(), 200);
    assert_eq!(mode.headers()["cache-control"], "no-store");
    let mode: serde_json::Value = mode.json().await.unwrap();
    assert_eq!(
        mode,
        serde_json::json!({ "mode": "token", "logout_url": null })
    );

    // API calls go to the server: its responses, errors included.
    let created = client
        .post(format!("{url}/api/2.1/unity-catalog/catalogs"))
        .json(&serde_json::json!({"name": "main", "comment": "via uc web"}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), 200);
    let (status, body) = text("/api/2.1/unity-catalog/catalogs/main?x=1").await;
    assert_eq!(status, 200);
    let catalog: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(catalog["comment"], "via uc web");
    let (status, body) = text("/api/2.1/unity-catalog/catalogs/missing").await;
    assert_eq!(status, 404);
    assert!(body.contains("CATALOG_NOT_FOUND"), "{body}");

    // So do the API docs that the UI links to: `/docs` sends the browser to
    // the server's Swagger UI at `/docs/`, whose files load its OpenAPI
    // document.
    let (status, body) = text("/docs").await;
    assert_eq!(status, 200);
    assert!(body.contains("swagger-ui"), "{body}");
    let (status, body) = text("/docs/swagger-initializer.js").await;
    assert_eq!(status, 200);
    assert!(body.contains("/api-docs/openapi.json"), "{body}");
    let (status, body) = text("/api-docs/openapi.json").await;
    assert_eq!(status, 200);
    assert!(body.contains("/api/2.1/unity-catalog/catalogs"), "{body}");
}

/// A uc-server in this process that needs a sign-in, and the token of a
/// user of it.
async fn signed_in_server() -> (String, String) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let security = uc_server::authn::security::SecurityContext::generate().unwrap();
    let token = security
        .create_access_token("alice@example.com", Duration::from_secs(600))
        .unwrap();
    uc_metastore::Metastore::scim(&*metastore)
        .create_user(uc_api::ScimUser {
            user_name: "alice@example.com".into(),
            emails: Some(vec![uc_api::ScimEmail {
                value: "alice@example.com".into(),
                primary: Some(true),
                r#type: None,
            }]),
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(std::sync::Arc::new(security));
    let app = build_app(state);
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
    (format!("http://{addr}"), token)
}

#[tokio::test]
async fn web_signs_the_uis_calls_in_with_the_uc_token() {
    let (server, token) = signed_in_server().await;
    let dir = tempfile::tempdir().unwrap();
    let ui = dir.path().join("ui");
    std::fs::create_dir(&ui).unwrap();
    std::fs::write(ui.join("index.html"), "<html>ui</html>").unwrap();
    let port = free_port();
    let mut web = uc(dir.path())
        .args(["web", "--port", &port.to_string(), "--no_browser"])
        .args(["--server", &server, "--auth_token", &token])
        .env("UC_UI_DIR", &ui)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let url = format!("http://127.0.0.1:{port}");
    wait_for(&url, &mut web).await;

    let me = |site: Option<&'static str>| {
        let mut request =
            reqwest::Client::new().get(format!("{url}/api/1.0/unity-control/scim2/Me"));
        if let Some(site) = site {
            request = request.header("sec-fetch-site", site);
        }
        async move { request.send().await.unwrap() }
    };
    // The page's own calls carry the `uc` token.
    let signed_in = me(Some("same-origin")).await;
    assert_eq!(signed_in.status(), 200);
    let user: serde_json::Value = signed_in.json().await.unwrap();
    assert_eq!(user["userName"], "alice@example.com");
    // Calls from other sites, or not from a browser, do not.
    assert_eq!(me(Some("cross-site")).await.status(), 401);
    assert_eq!(me(None).await.status(), 401);
    // The server's sign-in is not for this address.
    let login = reqwest::get(format!("{url}/auth/login")).await.unwrap();
    assert_eq!(login.status(), 404);
    assert!(login.text().await.unwrap().contains("--auth_token"));
}

#[tokio::test]
async fn web_needs_the_ui() {
    let dir = tempfile::tempdir().unwrap();
    let output = uc(dir.path())
        .args(["web", "--no_browser"])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unitycatalog-rs[ui]"), "{stderr}");
}

#[cfg(unix)]
fn script(path: &Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("#!/bin/sh\n{text}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn serve_runs_uc_server_with_its_options() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("uc-server");
    script(&fake, "echo \"$@\"\nexit 3");
    let output = uc(dir.path())
        .args(["serve", "-p", "9000", "--help"])
        .env("UC_CLI_SERVER_BIN", &fake)
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "-p 9000 --help\n");

    let output = uc(dir.path())
        .arg("serve")
        .env("UC_CLI_SERVER_BIN", dir.path().join("missing"))
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing"));
}

/// `uc dev` with a stand-in for uc-server: Python's HTTP server, which
/// writes down its settings.
#[cfg(unix)]
#[tokio::test]
async fn dev_runs_uc_server_with_its_folder_and_the_ui() {
    let dir = tempfile::tempdir().unwrap();
    let ui = dir.path().join("ui");
    std::fs::create_dir(&ui).unwrap();
    let fake = dir.path().join("uc-server");
    let record = dir.path().join("record");
    script(
        &fake,
        &format!(
            "echo \"$UC_SERVER_BIND $UC_UI_DIR $UC_SERVER_DATA_DIR $UC_CONFIG $UC_DB_URL\" > {}\n\
             exec python3 -m http.server --bind 127.0.0.1 \"${{UC_SERVER_BIND##*:}}\"",
            record.display()
        ),
    );
    let port = free_port();
    let mut dev = uc(dir.path())
        .args(["dev", "--port", &port.to_string(), "--dir", "data"])
        .arg("--no_browser")
        .env("UC_CLI_SERVER_BIN", &fake)
        .env("UC_UI_DIR", &ui)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(dev.stdout.take().unwrap()).lines();
    // uc-server prints the URLs; with the UI, uc dev says where the catalog
    // is and how to stop.
    let banner = banner(&mut lines).await;
    assert!(!banner.contains("No web UI"), "{banner}");
    assert!(banner.contains("The catalog is in memory"), "{banner}");

    // uc-server gets the folder, the UI, uc's settings file and a database
    // in memory.
    let data = std::fs::canonicalize(dir.path().join("data")).unwrap();
    let settings = std::fs::canonicalize(dir.path().join("empty.toml")).unwrap();
    let recorded = std::fs::read_to_string(&record).unwrap();
    assert_eq!(
        recorded.trim(),
        format!(
            "127.0.0.1:{port} {} {} {} memory",
            ui.display(),
            data.display(),
            settings.display()
        )
    );

    // A stop signal to `uc` alone stops the server too.
    let pid = dev.id().unwrap().to_string();
    let killed = std::process::Command::new("kill")
        .args(["-INT", &pid])
        .status()
        .unwrap();
    assert!(killed.success());
    tokio::time::timeout(Duration::from_secs(30), dev.wait())
        .await
        .expect("uc dev stops")
        .unwrap();
    assert!(
        reqwest::get(format!("http://127.0.0.1:{port}/"))
            .await
            .is_err(),
        "the server still runs"
    );
}

/// With `db.url`, `uc dev` leaves the database of the settings.
#[cfg(unix)]
#[tokio::test]
async fn dev_keeps_the_database_of_the_settings() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("uc-server");
    let record = dir.path().join("record");
    script(
        &fake,
        &format!(
            "echo \"$UC_DB_URL\" > {}\n\
             exec python3 -m http.server --bind 127.0.0.1 \"${{UC_SERVER_BIND##*:}}\"",
            record.display()
        ),
    );
    let mut dev = uc(dir.path())
        .args(["dev", "--port", &free_port().to_string(), "--no_browser"])
        .env("UC_CLI_SERVER_BIN", &fake)
        .env("UC_DB_URL", "file:keep.db")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(dev.stdout.take().unwrap()).lines();
    let banner = banner(&mut lines).await;
    assert!(!banner.contains("The catalog is in memory"), "{banner}");
    let recorded = std::fs::read_to_string(&record).unwrap();
    assert_eq!(recorded.trim(), "file:keep.db");
    let pid = dev.id().unwrap().to_string();
    let killed = std::process::Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .unwrap();
    assert!(killed.success());
    tokio::time::timeout(Duration::from_secs(30), dev.wait())
        .await
        .expect("uc dev stops")
        .unwrap();
}

/// What `uc dev` prints once the server answers.
#[cfg(unix)]
async fn banner(lines: &mut Lines<BufReader<ChildStdout>>) -> String {
    let mut banner = String::new();
    tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(line) = lines.next_line().await.unwrap() {
            banner.push_str(&line);
            banner.push('\n');
            if line.starts_with("Press Ctrl+C") {
                return;
            }
        }
        panic!("uc dev stopped: {banner}");
    })
    .await
    .expect("uc dev prints its banner");
    banner
}

/// Without the web UI, `uc dev` says why: it is off, or not installed.
#[cfg(unix)]
#[tokio::test]
async fn dev_says_why_it_has_no_ui() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("uc-server");
    script(
        &fake,
        "exec python3 -m http.server --bind 127.0.0.1 \"${UC_SERVER_BIND##*:}\"",
    );
    for (enabled, why) in [
        ("false", "No web UI: ui.enabled (UC_UI_ENABLED) is false."),
        // On: `ui.dir` of `uc()` has no UI.
        ("true", "No web UI. The web UI is not installed."),
    ] {
        let mut dev = uc(dir.path())
            .args(["dev", "--port", &free_port().to_string(), "--no_browser"])
            .env("UC_CLI_SERVER_BIN", &fake)
            .env("UC_UI_ENABLED", enabled)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(dev.stdout.take().unwrap()).lines();
        let banner = banner(&mut lines).await;
        assert!(banner.contains(why), "{banner}");
        let pid = dev.id().unwrap().to_string();
        let killed = std::process::Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .unwrap();
        assert!(killed.success());
        tokio::time::timeout(Duration::from_secs(30), dev.wait())
            .await
            .expect("uc dev stops")
            .unwrap();
    }
}
