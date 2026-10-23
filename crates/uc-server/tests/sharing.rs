//! Delta Sharing: shares and recipients, and what a recipient reads of
//! Delta tables on local disk.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
};
use delta_kernel::arrow::array::{Int64Array, RecordBatch};
use delta_kernel::arrow::datatypes::{DataType, Field, Schema};
use delta_kernel::parquet::arrow::ArrowWriter;
use http_body_util::BodyExt;
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{BASE, app, as_admin, assert_status, json_body, req_json, seeded};

/// A Delta table on local disk, written by hand: Parquet files of an `id`
/// column, and commits with in-commit timestamps, so that their times are
/// exact.
struct DeltaTable {
    dir: PathBuf,
    protocol: Value,
    partitioned: bool,
    configuration: Value,
    next_version: u64,
}

impl DeltaTable {
    /// A table with in-commit timestamps and `configuration`, partitioned by
    /// `date` (a string) if `partitioned`, in a new directory.
    fn new(partitioned: bool, configuration: Value) -> Self {
        let protocol = json!({
            "minReaderVersion": 1, "minWriterVersion": 7, "writerFeatures": ["inCommitTimestamp"],
        });
        let dir = tempfile::Builder::new()
            .prefix("uc-share-")
            .tempdir()
            .unwrap()
            .keep();
        Self::at(dir, protocol, partitioned, configuration)
    }

    /// A table in `dir` with `protocol`.
    fn at(dir: PathBuf, protocol: Value, partitioned: bool, configuration: Value) -> Self {
        std::fs::create_dir_all(dir.join("_delta_log")).unwrap();
        let mut config = json!({ "delta.enableInCommitTimestamps": "true" });
        for (k, v) in configuration.as_object().unwrap() {
            config[k] = v.clone();
        }
        Self {
            dir,
            protocol,
            partitioned,
            configuration: config,
            next_version: 0,
        }
    }

    fn url(&self) -> String {
        format!("file://{}", self.dir.display())
    }

    /// Write a Parquet file of `ids` at `path` under the table; its size.
    fn parquet(&self, path: &str, ids: &[i64]) -> i64 {
        let file = self.dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, true)]));
        let column = Arc::new(Int64Array::from(ids.to_vec()));
        let batch = RecordBatch::try_new(schema.clone(), vec![column]).unwrap();
        let file = std::fs::File::create(&file).unwrap();
        let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        std::fs::metadata(self.dir.join(path)).unwrap().len() as i64
    }

    /// An `add` of a new file of `ids`, in the partition `date` if any.
    fn add(&self, path: &str, ids: &[i64], date: Option<&str>) -> Value {
        let size = self.parquet(path, ids);
        let stats = json!({
            "numRecords": ids.len(),
            "minValues": { "id": ids.iter().min() },
            "maxValues": { "id": ids.iter().max() },
            "nullCount": { "id": 0 },
        });
        let partitions = match date {
            Some(date) => json!({ "date": date }),
            None => json!({}),
        };
        json!({ "add": {
            "path": path, "partitionValues": partitions, "size": size,
            "modificationTime": 0, "dataChange": true, "stats": stats.to_string(),
        } })
    }

    /// The `remove` of the file that `add` added.
    fn remove(add: &Value) -> Value {
        let add = &add["add"];
        json!({ "remove": {
            "path": add["path"], "partitionValues": add["partitionValues"], "size": add["size"],
            "deletionTimestamp": 0, "dataChange": true,
        } })
    }

    /// A change data file of `ids`.
    fn cdc(&self, path: &str, ids: &[i64]) -> Value {
        let size = self.parquet(path, ids);
        json!({ "cdc": { "path": path, "partitionValues": {}, "size": size, "dataChange": false } })
    }

    /// The lines of the next commit: `actions` at time `timestamp`, after
    /// the protocol and metadata for version 0.
    fn lines(&self, timestamp: i64, actions: Vec<Value>) -> String {
        let mut lines = vec![json!({ "commitInfo": {
            "inCommitTimestamp": timestamp, "timestamp": timestamp, "operation": "WRITE",
        } })];
        if self.next_version == 0 {
            let mut fields =
                vec![json!({ "name": "id", "type": "long", "nullable": true, "metadata": {} })];
            if self.partitioned {
                fields.push(
                    json!({ "name": "date", "type": "string", "nullable": true, "metadata": {} }),
                );
            }
            lines.push(json!({ "protocol": self.protocol }));
            lines.push(json!({ "metaData": {
                "id": "6d1a3e0f-0000-4000-8000-000000000001",
                "format": { "provider": "parquet", "options": {} },
                "schemaString": json!({ "type": "struct", "fields": fields }).to_string(),
                "partitionColumns": if self.partitioned { json!(["date"]) } else { json!([]) },
                "configuration": self.configuration,
                "createdTime": timestamp,
            } }));
        }
        lines.extend(actions);
        lines.iter().map(|l| format!("{l}\n")).collect()
    }

    /// The next version: `actions` at time `timestamp`.
    fn commit(&mut self, timestamp: i64, actions: Vec<Value>) {
        let file = format!("_delta_log/{:020}.json", self.next_version);
        std::fs::write(self.dir.join(file), self.lines(timestamp, actions)).unwrap();
        self.next_version += 1;
    }
}

/// How a test signs in to the admin API.
type Auth = fn(Request<Body>) -> Request<Body>;

fn anonymous(request: Request<Body>) -> Request<Body> {
    request
}

async fn send(app: &Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.unwrap()
}

/// `POST`, `PATCH`, … of `body` to the admin API, which must answer 200.
async fn admin(app: &Router, auth: Auth, method: Method, path: &str, body: Value) -> Value {
    let resp = send(app, auth(req_json(method, path, body))).await;
    json_body(assert_status(resp, StatusCode::OK, path).await).await
}

/// Register the table at `url` as the EXTERNAL Delta table
/// `main.default.<name>`.
async fn register(app: &Router, auth: Auth, url: &str, name: &str) {
    let table = json!({
        "name": name, "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "columns": [{ "name": "id", "type_name": "LONG", "type_text": "bigint", "position": 0,
            "type_json": r#"{"name":"id","type":"long","nullable":true,"metadata":{}}"# }],
        "storage_location": url,
    });
    admin(app, auth, Method::POST, "/tables", table).await;
}

/// Share `main.default.<table>` in `share` (made if new) with the settings
/// of `object`.
async fn add_table(app: &Router, auth: Auth, share: &str, table: &str, object: Value) {
    let resp = send(
        app,
        auth(req_json(
            Method::GET,
            &format!("/shares/{share}"),
            json!({}),
        )),
    )
    .await;
    if resp.status() == StatusCode::NOT_FOUND {
        admin(app, auth, Method::POST, "/shares", json!({ "name": share })).await;
    }
    let mut object = object;
    object["source_table_full_name"] = format!("main.default.{table}").into();
    let update = json!({ "add_objects": [object] });
    admin(
        app,
        auth,
        Method::PATCH,
        &format!("/shares/{share}"),
        update,
    )
    .await;
}

/// A new recipient of `shares`; its token.
async fn recipient(app: &Router, auth: Auth, name: &str, shares: &[&str]) -> String {
    let rec = admin(
        app,
        auth,
        Method::POST,
        "/recipients",
        json!({ "name": name }),
    )
    .await;
    let grant = json!({ "add_shares": shares });
    admin(
        app,
        auth,
        Method::PATCH,
        &format!("/recipients/{name}"),
        grant,
    )
    .await;
    rec["bearer_token"].as_str().unwrap().to_string()
}

/// Register `table` as `main.default.events`, share it as
/// `analytics.default.events` with the settings of `object`, and grant
/// the share to the recipient `partner`; its token.
async fn share(app: &Router, table: &DeltaTable, object: Value) -> String {
    register(app, anonymous, &table.url(), "events").await;
    add_table(app, anonymous, "analytics", "events", object).await;
    recipient(app, anonymous, "partner", &["analytics"]).await
}

const TABLE: &str = "/delta-sharing/shares/analytics/schemas/default/tables/events";

/// A protocol request with `token`, `capabilities` and a JSON `body`.
fn protocol(
    method: Method,
    path: &str,
    token: &str,
    capabilities: &str,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"));
    if !capabilities.is_empty() {
        builder = builder.header("delta-sharing-capabilities", capabilities);
    }
    match body {
        Some(body) => builder
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

fn get(path: &str, token: &str) -> Request<Body> {
    protocol(Method::GET, path, token, "", None)
}

/// The NDJSON lines of a protocol response.
async fn ndjson(resp: Response) -> Vec<Value> {
    let resp = assert_status(resp, StatusCode::OK, "ndjson").await;
    assert_eq!(
        resp.headers()["content-type"],
        "application/x-ndjson; charset=utf-8"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec())
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The version header and the lines of a response.
async fn versioned(resp: Response) -> (String, Vec<Value>) {
    let version = resp
        .headers()
        .get("delta-table-version")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    (version, ndjson(resp).await)
}

/// `query` with `capabilities` and `body`.
async fn query(app: &Router, token: &str, capabilities: &str, body: Value) -> (String, Vec<Value>) {
    let path = format!("{TABLE}/query");
    let request = protocol(Method::POST, &path, token, capabilities, Some(body));
    versioned(send(app, request).await).await
}

/// `changes` with the query string `params`.
async fn changes(
    app: &Router,
    token: &str,
    capabilities: &str,
    params: &str,
) -> (String, Vec<Value>) {
    let path = format!("{TABLE}/changes?{params}");
    versioned(send(app, protocol(Method::GET, &path, token, capabilities, None)).await).await
}

/// The values of `key` in the lines that have it.
fn pick<'a>(lines: &'a [Value], key: &str) -> Vec<&'a Value> {
    lines.iter().filter_map(|l| l.get(key)).collect()
}

/// The `(version, size)` of each line of `key`.
fn files(lines: &[Value], key: &str) -> Vec<(Value, Value)> {
    pick(lines, key)
        .into_iter()
        .map(|f| (f["version"].clone(), f["size"].clone()))
        .collect()
}

/// A protocol error: its status and `errorCode`, and the body.
async fn assert_protocol_error(resp: Response, status: StatusCode, code: &str) -> Value {
    let body = json_body(assert_status(resp, status, code).await).await;
    assert_eq!(body["errorCode"], code, "{body}");
    assert_eq!(
        body.as_object().unwrap().len(),
        2,
        "errorCode and message: {body}"
    );
    body
}

/// The bytes at the file URL `url`, through the app, with `range` if any.
async fn fetch(app: &Router, url: &str, range: Option<&str>) -> (StatusCode, Vec<u8>) {
    let path = url
        .strip_prefix("http://localhost:8080")
        .expect("a URL of the server");
    let mut builder = Request::builder().uri(path);
    if let Some(range) = range {
        builder = builder.header("range", range);
    }
    let resp = send(app, builder.body(Body::empty()).unwrap()).await;
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, body.to_vec())
}

#[rstest]
#[tokio::test]
async fn a_recipient_reads_the_files_of_the_latest_version(#[future(awt)] seeded: Router) {
    let mut table = DeltaTable::new(false, json!({}));
    let first = table.add("part-0.parquet", &[1, 2, 3], None);
    table.commit(1000, vec![first.clone()]);
    let second = table.add("part-1.parquet", &[4, 5], None);
    table.commit(2000, vec![second]);
    let token = share(&seeded, &table, json!({})).await;

    let resp = send(&seeded, get(&format!("{TABLE}/version"), &token)).await;
    let resp = assert_status(resp, StatusCode::OK, "version").await;
    assert_eq!(resp.headers()["delta-table-version"], "1");
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(body.is_empty());
    // The protocol's older form of the same request.
    let resp = send(&seeded, protocol(Method::HEAD, TABLE, &token, "", None)).await;
    assert_eq!(resp.headers()["delta-table-version"], "1");

    let resp = send(&seeded, get(&format!("{TABLE}/metadata"), &token)).await;
    assert_eq!(
        resp.headers()["delta-sharing-capabilities"],
        "responseformat=parquet"
    );
    let (version, lines) = versioned(resp).await;
    assert_eq!(version, "1");
    assert_eq!(lines[0], json!({ "protocol": { "minReaderVersion": 1 } }));
    let metadata = &lines[1]["metaData"];
    assert_eq!(metadata["id"], "6d1a3e0f-0000-4000-8000-000000000001");
    assert_eq!(metadata["format"], json!({ "provider": "parquet" }));
    assert_eq!(metadata["partitionColumns"], json!([]));
    assert_eq!(metadata["configuration"], json!({}));
    assert!(
        metadata["schemaString"]
            .as_str()
            .unwrap()
            .contains("\"id\"")
    );
    assert!(metadata.get("version").is_none());
    assert_eq!(lines.len(), 2);

    // An empty body is `{}`.
    let resp = send(
        &seeded,
        protocol(Method::POST, &format!("{TABLE}/query"), &token, "", None),
    )
    .await;
    let lines = ndjson(resp).await;
    let files = pick(&lines, "file");
    assert_eq!(files.len(), 2);
    let file = files
        .iter()
        .find(|f| f["size"] == first["add"]["size"])
        .unwrap();
    assert_eq!(file["stats"], first["add"]["stats"]);
    assert_eq!(file["partitionValues"], json!({}));
    assert!(file.get("version").is_none() && file.get("timestamp").is_none());
    assert!(file["expirationTimestamp"].as_i64().unwrap() > 0);
    assert_eq!(file["id"].as_str().unwrap().len(), 32);
    let url = file["url"].as_str().unwrap();
    assert!(
        url.starts_with("http://localhost:8080/delta-sharing/files/"),
        "{url}"
    );

    // The URL reads the file without a bearer token, in ranges too.
    let on_disk = std::fs::read(table.dir.join("part-0.parquet")).unwrap();
    assert_eq!(fetch(&seeded, url, None).await, (StatusCode::OK, on_disk));
    assert_eq!(
        fetch(&seeded, url, Some("bytes=0-3")).await,
        (StatusCode::PARTIAL_CONTENT, b"PAR1".to_vec())
    );
    let tampered = format!("{}x", &url[..url.len() - 1]);
    let expiry = url.split("se=").nth(1).unwrap().split('&').next().unwrap();
    let later = url.replace(
        &format!("se={expiry}"),
        &format!("se={}", expiry.parse::<i64>().unwrap() + 3600),
    );
    for bad in [tampered, later] {
        let (status, body) = fetch(&seeded, &bad, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{bad}");
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["errorCode"], "PERMISSION_DENIED");
    }
}

/// What kernel clients do with the `delta` format: write the actions to a
/// Delta log of their own, then read the files over HTTP.
#[rstest]
#[tokio::test]
async fn kernel_clients_read_the_delta_format(#[future(awt)] seeded: Router) {
    use delta_kernel::Snapshot;
    use delta_kernel::engine::arrow_data::ArrowEngineData;
    use delta_kernel_default_engine::DefaultEngineBuilder;

    let mut table = DeltaTable::new(false, json!({}));
    let first = table.add("part-0.parquet", &[1, 2, 3], None);
    table.commit(1000, vec![first]);
    let second = table.add("part-1.parquet", &[4, 5], None);
    table.commit(2000, vec![second]);
    let token = share(&seeded, &table, json!({})).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = listener.local_addr().unwrap().to_string();
    tokio::spawn(axum::serve(listener, seeded.clone()).into_future());
    let mut request = protocol(
        Method::POST,
        &format!("{TABLE}/query"),
        &token,
        "responseformat=delta",
        Some(json!({})),
    );
    request.headers_mut().insert("host", host.parse().unwrap());
    let lines = ndjson(send(&seeded, request).await).await;
    let mut log = vec![
        json!({ "protocol": lines[0]["protocol"]["deltaProtocol"] }),
        json!({ "metaData": lines[1]["metaData"]["deltaMetadata"] }),
    ];
    for file in pick(&lines, "file") {
        log.push(file["deltaSingleAction"].clone());
    }
    let local = tempfile::tempdir().unwrap();
    std::fs::create_dir(local.path().join("_delta_log")).unwrap();
    let text: String = log.iter().map(|l| format!("{l}\n")).collect();
    std::fs::write(
        local.path().join("_delta_log/00000000000000000000.json"),
        text,
    )
    .unwrap();

    let root = url::Url::from_directory_path(local.path()).unwrap();
    let rows = tokio::task::spawn_blocking(move || {
        let store = Arc::new(delta_kernel::object_store::local::LocalFileSystem::new());
        let engine = Arc::new(DefaultEngineBuilder::new(store).build());
        let snapshot = Snapshot::builder_for(root.as_str())
            .build(engine.as_ref())
            .unwrap();
        let scan = snapshot.scan_builder().build().unwrap();
        let mut rows = 0;
        for data in scan.execute(engine).unwrap() {
            let batch = ArrowEngineData::try_from_engine_data(data.unwrap()).unwrap();
            rows += batch.record_batch().num_rows();
        }
        rows
    })
    .await
    .unwrap();
    assert_eq!(rows, 5);
}

#[rstest]
#[tokio::test]
async fn the_delta_format_wraps_the_log_actions(#[future(awt)] seeded: Router) {
    let mut table = DeltaTable::new(false, json!({}));
    let add = table.add("part-0.parquet", &[1, 2], None);
    table.commit(1000, vec![add.clone()]);
    let token = share(&seeded, &table, json!({})).await;

    let path = format!("{TABLE}/query");
    let capabilities = "responseformat=delta;includeendstreamaction=true";
    let resp = send(
        &seeded,
        protocol(Method::POST, &path, &token, capabilities, Some(json!({}))),
    )
    .await;
    assert_eq!(resp.headers()["delta-sharing-capabilities"], capabilities);
    let lines = ndjson(resp).await;
    assert_eq!(
        lines[0],
        json!({ "protocol": { "deltaProtocol": {
            "minReaderVersion": 1, "minWriterVersion": 7, "writerFeatures": ["inCommitTimestamp"],
        } } })
    );
    let metadata = &lines[1]["metaData"]["deltaMetadata"];
    assert_eq!(
        metadata["configuration"]["delta.enableInCommitTimestamps"],
        "true"
    );
    let file = &lines[2]["file"];
    let action = &file["deltaSingleAction"]["add"];
    assert!(
        action["path"]
            .as_str()
            .unwrap()
            .contains("/delta-sharing/files/")
    );
    assert_eq!(action["stats"], add["add"]["stats"]);
    assert_eq!(action["size"], add["add"]["size"]);
    assert_eq!(action["dataChange"], true);
    let end = &lines[3]["endStreamAction"];
    assert_eq!(
        end["minUrlExpirationTimestamp"],
        file["expirationTimestamp"]
    );
    assert_eq!(lines.len(), 4);

    // A plain table is answered in parquet when both formats do.
    let (_, lines) = query(&seeded, &token, "responseformat=delta,parquet", json!({})).await;
    assert!(lines[2]["file"]["url"].is_string());

    // The file id scheme is echoed, and checked.
    let mut request = protocol(Method::POST, &path, &token, "", Some(json!({})));
    request
        .headers_mut()
        .insert("fileidhash", "Delta".parse().unwrap());
    let resp = send(&seeded, request).await;
    assert_eq!(resp.headers()["fileidhash"], "delta");
    let mut request = protocol(Method::POST, &path, &token, "", Some(json!({})));
    request
        .headers_mut()
        .insert("fileidhash", "md5".parse().unwrap());
    let resp = send(&seeded, request).await;
    assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;
}

#[rstest]
#[tokio::test]
async fn hints_skip_files(#[future(awt)] seeded: Router) {
    let mut table = DeltaTable::new(true, json!({}));
    let jan = table.add(
        "date=2024-01-01/part-0.parquet",
        &[1, 2, 3],
        Some("2024-01-01"),
    );
    let feb = table.add(
        "date=2024-02-01/part-0.parquet",
        &[4, 5, 6],
        Some("2024-02-01"),
    );
    table.commit(1000, vec![jan, feb]);
    let token = share(&seeded, &table, json!({})).await;

    let hint = |op: &str, column: &str, value: &str| {
        json!({ "op": op, "children": [
            { "op": "column", "name": column, "valueType": "string" },
            { "op": "literal", "value": value, "valueType": "string" },
        ] })
        .to_string()
    };
    let body = json!({ "jsonPredicateHints": hint("equal", "date", "2024-02-01") });
    let (_, lines) = query(&seeded, &token, "", body).await;
    let files = pick(&lines, "file");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["partitionValues"], json!({ "date": "2024-02-01" }));
    assert_eq!(lines[1]["metaData"]["partitionColumns"], json!(["date"]));

    // On stats too.
    let body = json!({ "jsonPredicateHints": hint("lessThan", "id", "3") });
    let (_, lines) = query(&seeded, &token, "", body).await;
    assert_eq!(pick(&lines, "file").len(), 1);

    // A hint that does not convert keeps all files.
    let body = json!({ "jsonPredicateHints": "{\"op\":" });
    let (_, lines) = query(&seeded, &token, "", body).await;
    assert_eq!(pick(&lines, "file").len(), 2);

    // The first file holds the 3 rows of the limit.
    let (_, lines) = query(&seeded, &token, "", json!({ "limitHint": 3 })).await;
    assert_eq!(pick(&lines, "file").len(), 1);
    let (_, lines) = query(&seeded, &token, "", json!({ "limitHint": 4 })).await;
    assert_eq!(pick(&lines, "file").len(), 2);
}

/// v0 adds `a.parquet` at 1 s, v1 adds `b.parquet` at 2 s, v2 removes
/// `a.parquet` with a change data file at 3 s. The change data feed is on.
fn history_table() -> (DeltaTable, [Value; 3]) {
    let mut table = DeltaTable::new(false, json!({ "delta.enableChangeDataFeed": "true" }));
    let a = table.add("a.parquet", &[1, 2], None);
    table.commit(1000, vec![a.clone()]);
    let b = table.add("b.parquet", &[3], None);
    table.commit(2000, vec![b.clone()]);
    let cdc = table.cdc("_change_data/cdc-0.parquet", &[1, 2, 3, 4]);
    table.commit(3000, vec![DeltaTable::remove(&a), cdc.clone()]);
    let size = |action: &Value, key: &str| action[key]["size"].clone();
    (table, [size(&a, "add"), size(&b, "add"), size(&cdc, "cdc")])
}

#[rstest]
#[tokio::test]
async fn shared_history_is_read_by_version_time_and_change(#[future(awt)] seeded: Router) {
    let (table, [a, b, cdc]) = history_table();
    let history = json!({ "history_data_sharing_status": "ENABLED" });
    let token = share(&seeded, &table, history).await;

    let version = |query: &str| {
        let request = get(&format!("{TABLE}/version{query}"), &token);
        let seeded = seeded.clone();
        async move {
            let resp = send(&seeded, request).await;
            assert_status(resp, StatusCode::OK, "version")
                .await
                .headers()["delta-table-version"]
                .to_str()
                .unwrap()
                .to_string()
        }
    };
    assert_eq!(version("").await, "2");
    assert_eq!(
        version("?startingTimestamp=1970-01-01T00:00:01.500Z").await,
        "1"
    );
    assert_eq!(
        version("?startingTimestamp=1970-01-01T00:00:00Z").await,
        "0"
    );
    let resp = send(
        &seeded,
        get(
            &format!("{TABLE}/version?startingTimestamp=2030-01-01T00:00:00Z"),
            &token,
        ),
    )
    .await;
    assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;

    // The change data feed is announced to parquet clients.
    let resp = send(&seeded, get(&format!("{TABLE}/metadata?version=1"), &token)).await;
    let (header, lines) = versioned(resp).await;
    assert_eq!(header, "1");
    let metadata = &lines[1]["metaData"];
    assert_eq!(
        metadata["configuration"],
        json!({ "enableChangeDataFeed": "true" })
    );
    assert_eq!(metadata["version"], 1);

    // Time travel: the files of a version carry it and its time.
    let (header, lines) = query(&seeded, &token, "", json!({ "version": 0 })).await;
    assert_eq!(header, "0");
    assert_eq!(files(&lines, "file"), [(json!(0), a.clone())]);
    assert_eq!(pick(&lines, "file")[0]["timestamp"], 1000);
    let at = json!({ "timestamp": "1970-01-01T00:00:02.500Z" });
    let (header, lines) = query(&seeded, &token, "", at).await;
    assert_eq!(header, "1");
    let mut got = files(&lines, "file");
    got.sort_by_key(|(_, size)| size.as_i64());
    let mut want = vec![(json!(1), a.clone()), (json!(1), b.clone())];
    want.sort_by_key(|(_, size)| size.as_i64());
    assert_eq!(got, want);
    let resp = send(
        &seeded,
        protocol(
            Method::POST,
            &format!("{TABLE}/query"),
            &token,
            "",
            Some(json!({ "version": 5 })),
        ),
    )
    .await;
    assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;
    let both = json!({ "version": 0, "timestamp": "1970-01-01T00:00:01Z" });
    let resp = send(
        &seeded,
        protocol(
            Method::POST,
            &format!("{TABLE}/query"),
            &token,
            "",
            Some(both),
        ),
    )
    .await;
    assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;

    // Streaming: what each commit since a version added and removed.
    let (header, lines) = query(&seeded, &token, "", json!({ "startingVersion": 1 })).await;
    assert_eq!(header, "1");
    assert_eq!(lines[1]["metaData"]["version"], 1);
    assert_eq!(files(&lines, "add"), [(json!(1), b.clone())]);
    assert_eq!(files(&lines, "remove"), [(json!(2), a.clone())]);
    assert_eq!(pick(&lines, "remove")[0]["timestamp"], 3000);
    assert!(pick(&lines, "cdf").is_empty());
    let window = json!({ "startingVersion": 1, "endingVersion": 1 });
    let (_, lines) = query(&seeded, &token, "", window).await;
    assert_eq!(files(&lines, "add"), [(json!(1), b.clone())]);
    assert!(pick(&lines, "remove").is_empty());

    // Changes: a commit with change data files has only those.
    let (header, lines) = changes(&seeded, &token, "", "startingVersion=0").await;
    assert_eq!(header, "0");
    assert_eq!(
        files(&lines, "add"),
        [(json!(0), a.clone()), (json!(1), b.clone())]
    );
    assert_eq!(files(&lines, "cdf"), [(json!(2), cdc.clone())]);
    assert!(pick(&lines, "remove").is_empty());
    let params = "startingTimestamp=1970-01-01T00:00:01.500Z&endingVersion=1";
    let (header, lines) = changes(&seeded, &token, "", params).await;
    assert_eq!(header, "1");
    assert_eq!(files(&lines, "add"), [(json!(1), b.clone())]);
    let (_, lines) = changes(&seeded, &token, "responseformat=delta", "startingVersion=2").await;
    let cdc_line = &pick(&lines, "file")[0]["deltaSingleAction"]["cdc"];
    assert_eq!(cdc_line["size"], cdc);
    assert!(
        cdc_line["path"]
            .as_str()
            .unwrap()
            .contains("/delta-sharing/files/")
    );
    let resp = send(
        &seeded,
        get(&format!("{TABLE}/changes?startingVersion=3"), &token),
    )
    .await;
    assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;

    // A share of the history since version 1 only.
    let since = json!({ "history_data_sharing_status": "ENABLED", "start_version": 1 });
    add_table(&seeded, anonymous, "recent", "events", since).await;
    let late = recipient(&seeded, anonymous, "late", &["recent"]).await;
    let path = "/delta-sharing/shares/recent/schemas/default/tables/events/query";
    let resp = send(
        &seeded,
        protocol(Method::POST, path, &late, "", Some(json!({ "version": 0 }))),
    )
    .await;
    let body =
        assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("since version 1"),
        "{body}"
    );
    let resp = send(
        &seeded,
        protocol(Method::POST, path, &late, "", Some(json!({ "version": 1 }))),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn without_shared_history_only_the_latest_version_is_read(#[future(awt)] seeded: Router) {
    let (table, _) = history_table();
    let token = share(&seeded, &table, json!({})).await;
    for request in [
        get(
            &format!("{TABLE}/version?startingTimestamp=1970-01-01T00:00:00Z"),
            &token,
        ),
        get(&format!("{TABLE}/metadata?version=0"), &token),
        get(&format!("{TABLE}/changes?startingVersion=0"), &token),
        protocol(
            Method::POST,
            &format!("{TABLE}/query"),
            &token,
            "",
            Some(json!({ "version": 0 })),
        ),
        protocol(
            Method::POST,
            &format!("{TABLE}/query"),
            &token,
            "",
            Some(json!({ "startingVersion": 0 })),
        ),
    ] {
        let resp = send(&seeded, request).await;
        assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;
    }
    // And the change data feed is not announced.
    let (_, lines) = query(&seeded, &token, "", json!({})).await;
    assert_eq!(lines[1]["metaData"]["configuration"], json!({}));
}

#[rstest]
#[tokio::test]
async fn deletion_vectors_need_the_delta_format(#[future(awt)] seeded: Router) {
    let protocol_v3 = json!({
        "minReaderVersion": 3, "minWriterVersion": 7,
        "readerFeatures": ["deletionVectors"],
        "writerFeatures": ["deletionVectors", "inCommitTimestamp"],
    });
    let dir = tempfile::tempdir().unwrap().keep();
    let config = json!({ "delta.enableDeletionVectors": "true" });
    let mut table = DeltaTable::at(dir, protocol_v3, false, config);
    let mut add = table.add("part-0.parquet", &[1, 2, 3], None);
    // The example of the Delta protocol: `ab/deletion_vector_<uuid>.bin`.
    add["add"]["deletionVector"] = json!({
        "storageType": "u", "pathOrInlineDv": "ab^-aqEH.-t@S}K{vb[*k^",
        "offset": 1, "sizeInBytes": 36, "cardinality": 2,
    });
    table.commit(1000, vec![add]);
    let token = share(&seeded, &table, json!({})).await;

    let path = format!("{TABLE}/query");
    for capabilities in ["", "responseformat=delta", "responseformat=delta,parquet"] {
        let request = protocol(Method::POST, &path, &token, capabilities, Some(json!({})));
        let resp = send(&seeded, request).await;
        let body =
            assert_protocol_error(resp, StatusCode::BAD_REQUEST, "UNSUPPORTED_OPERATION").await;
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("deletion vectors"),
            "{body}"
        );
    }

    let capabilities = "responseformat=delta,parquet;readerfeatures=deletionvectors";
    let (_, lines) = query(&seeded, &token, capabilities, json!({})).await;
    assert_eq!(
        lines[0]["protocol"]["deltaProtocol"]["readerFeatures"],
        json!(["deletionVectors"])
    );
    let file = &pick(&lines, "file")[0];
    assert_eq!(file["deletionVectorFileId"].as_str().unwrap().len(), 32);
    let dv = &file["deltaSingleAction"]["add"]["deletionVector"];
    assert_eq!(dv["storageType"], "p");
    assert_eq!(dv["cardinality"], 2);
    assert!(
        dv["pathOrInlineDv"]
            .as_str()
            .unwrap()
            .contains("/delta-sharing/files/")
    );
}

#[tokio::test]
async fn catalog_managed_tables_include_the_commits_the_server_holds() {
    let (app, _) = common::app_with_metastore().await;
    common::seed_main_default(&app).await;
    let staging = admin(
        &app,
        anonymous,
        Method::POST,
        "/staging-tables",
        json!({ "name": "events", "catalog_name": "main", "schema_name": "default" }),
    )
    .await;
    let table_id = staging["id"].as_str().unwrap().to_string();
    let location = staging["staging_location"].as_str().unwrap().to_string();
    let protocol_v3 = json!({
        "minReaderVersion": 3, "minWriterVersion": 7,
        "readerFeatures": ["catalogManaged"],
        "writerFeatures": ["catalogManaged", "inCommitTimestamp"],
    });
    let dir = PathBuf::from(location.strip_prefix("file://").unwrap());
    let config = json!({ "io.unitycatalog.tableId": table_id });
    let mut table = DeltaTable::at(dir, protocol_v3, false, config);
    let first = table.add("part-0.parquet", &[1], None);
    table.commit(1000, vec![first]);
    let created = json!({
        "name": "events", "catalog_name": "main", "schema_name": "default",
        "table_type": "MANAGED", "data_source_format": "DELTA", "storage_location": location,
        "columns": [{ "name": "id", "type_name": "LONG", "type_text": "bigint", "position": 0,
            "type_json": r#"{"name":"id","type":"long","nullable":true,"metadata":{}}"# }],
        "properties": {
            "io.unitycatalog.tableId": table_id,
            "delta.feature.catalogManaged": "supported",
            "delta.enableInCommitTimestamps": "true",
        },
    });
    admin(&app, anonymous, Method::POST, "/tables", created).await;

    // Version 1 is ratified by the server and not yet published.
    let second = table.add("part-1.parquet", &[2], None);
    let staged = format!("{:020}.4a5f6b5e-0000-4000-8000-000000000002.json", 1);
    let text = table.lines(2000, vec![second]);
    let staged_dir = table.dir.join("_delta_log/_staged_commits");
    std::fs::create_dir_all(&staged_dir).unwrap();
    std::fs::write(staged_dir.join(&staged), &text).unwrap();
    let commit = json!({
        "table_id": table_id, "table_uri": location,
        "commit_info": {
            "version": 1, "file_name": staged, "file_size": text.len(),
            "timestamp": 2000, "file_modification_timestamp": 2000,
        },
        "metadata": { "properties": { "properties": { "io.unitycatalog.tableId": table_id } } },
    });
    let resp = send(
        &app,
        req_json(Method::POST, "/delta/preview/commits", commit),
    )
    .await;
    assert_status(resp, StatusCode::OK, "commit").await;

    add_table(&app, anonymous, "analytics", "events", json!({})).await;
    let token = recipient(&app, anonymous, "partner", &["analytics"]).await;
    let (header, lines) = query(&app, &token, "", json!({})).await;
    assert_eq!(header, "1");
    assert_eq!(pick(&lines, "file").len(), 2);
    // A recipient's own log needs no catalog.
    let (_, lines) = query(&app, &token, "responseformat=delta", json!({})).await;
    let protocol = &lines[0]["protocol"]["deltaProtocol"];
    assert_eq!(protocol["readerFeatures"], json!([]));
    assert_eq!(protocol["writerFeatures"], json!(["inCommitTimestamp"]));
}

#[rstest]
#[tokio::test]
async fn lists_page_and_names_ignore_case(#[future(awt)] seeded: Router) {
    let mut table = DeltaTable::new(false, json!({}));
    table.commit(1000, vec![]);
    for name in ["events", "orders", "users"] {
        register(&seeded, anonymous, &table.url(), name).await;
    }
    add_table(&seeded, anonymous, "analytics", "events", json!({})).await;
    let in_sales = json!({ "shared_as_schema": "sales" });
    add_table(&seeded, anonymous, "analytics", "orders", in_sales).await;
    add_table(&seeded, anonymous, "analytics", "users", json!({})).await;
    add_table(&seeded, anonymous, "archive", "events", json!({})).await;
    let token = recipient(&seeded, anonymous, "partner", &["archive", "analytics"]).await;

    let list = |path: &str| {
        let request = get(path, &token);
        let seeded = seeded.clone();
        async move {
            json_body(assert_status(send(&seeded, request).await, StatusCode::OK, "list").await)
                .await
        }
    };
    let page = list("/delta-sharing/shares?maxResults=1").await;
    assert_eq!(page["items"][0]["name"], "analytics");
    assert!(page["items"][0]["id"].is_string());
    let token_2 = page["nextPageToken"].as_str().unwrap();
    let page = list(&format!(
        "/delta-sharing/shares?maxResults=1&pageToken={token_2}"
    ))
    .await;
    assert_eq!(page["items"][0]["name"], "archive");
    assert!(page.get("nextPageToken").is_none());

    let share = list("/delta-sharing/shares/ANALYTICS").await;
    assert_eq!(share["share"]["name"], "analytics");
    let schemas = list("/delta-sharing/shares/analytics/schemas").await;
    let names: Vec<&Value> = schemas["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["name"])
        .collect();
    assert_eq!(names, [&json!("default"), &json!("sales")]);
    let tables = list("/delta-sharing/shares/analytics/schemas/Default/tables").await;
    let names: Vec<&Value> = tables["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| &t["name"])
        .collect();
    assert_eq!(names, [&json!("events"), &json!("users")]);
    let first = &tables["items"][0];
    assert_eq!(first["share"], "analytics");
    assert_eq!(first["schema"], "default");
    assert_eq!(first["shareId"], share["share"]["id"]);
    assert!(first["id"].is_string());
    let all = list("/delta-sharing/shares/analytics/all-tables?maxResults=2").await;
    assert_eq!(all["items"].as_array().unwrap().len(), 2);
    assert!(all["nextPageToken"].is_string());

    // Names match in any case.
    let path = "/delta-sharing/shares/Analytics/schemas/DEFAULT/tables/Events/version";
    let resp = send(&seeded, get(path, &token)).await;
    assert_eq!(resp.headers()["delta-table-version"], "0");

    let not_found = [
        "/delta-sharing/shares/analytics/schemas/nope/tables",
        "/delta-sharing/shares/analytics/schemas/default/tables/nope/version",
    ];
    for path in not_found {
        let resp = send(&seeded, get(path, &token)).await;
        assert_protocol_error(resp, StatusCode::NOT_FOUND, "RESOURCE_DOES_NOT_EXIST").await;
    }
    let resp = send(&seeded, get("/delta-sharing/shares?maxResults=-1", &token)).await;
    assert_protocol_error(resp, StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE").await;
}

#[rstest]
#[tokio::test]
async fn shares_check_their_tables_and_grants_follow_their_shares(#[future(awt)] seeded: Router) {
    let mut table = DeltaTable::new(false, json!({}));
    table.commit(1000, vec![]);
    register(&seeded, anonymous, &table.url(), "events").await;
    register(&seeded, anonymous, &table.url(), "events_copy").await;
    admin(
        &seeded,
        anonymous,
        Method::POST,
        "/shares",
        json!({ "name": "analytics" }),
    )
    .await;
    let patch = |body: Value| req_json(Method::PATCH, "/shares/analytics", body);

    // Only Delta tables, and each shared name once.
    let resp = send(
        &seeded,
        req_json(
            Method::POST,
            "/tables",
            json!({
                "name": "plain", "catalog_name": "main", "schema_name": "default",
                "table_type": "EXTERNAL", "data_source_format": "CSV",
                "columns": [], "storage_location": table.url(),
            }),
        ),
    )
    .await;
    assert_status(resp, StatusCode::OK, "csv table").await;
    let csv = json!({ "add_objects": [{ "source_table_full_name": "main.default.plain" }] });
    let resp = send(&seeded, patch(csv)).await;
    common::assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    let both = json!({ "add_objects": [
        { "source_table_full_name": "main.default.events" },
        { "source_table_full_name": "main.default.events_copy", "shared_as_name": "Events" },
    ] });
    let resp = send(&seeded, patch(both)).await;
    common::assert_error(resp, StatusCode::CONFLICT, "ALREADY_EXISTS").await;
    let dotted = json!({ "add_objects": [
        { "source_table_full_name": "main.default.events", "shared_as_schema": "a.b" },
    ] });
    let resp = send(&seeded, patch(dotted)).await;
    common::assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    let start = json!({ "add_objects": [
        { "source_table_full_name": "main.default.events", "start_version": 1 },
    ] });
    let resp = send(&seeded, patch(start)).await;
    common::assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;

    // Defaults: the table's schema and name, no history, an id.
    let add = json!({ "add_objects": [{ "source_table_full_name": "main.default.events" }] });
    let share = json_body(send(&seeded, patch(add)).await).await;
    let object = &share["objects"][0];
    assert_eq!(object["shared_as_schema"], "default");
    assert_eq!(object["shared_as_name"], "events");
    assert_eq!(object["history_data_sharing_status"], "DISABLED");
    let id = object["id"].clone();
    assert!(id.is_string());
    // Removing and adding in one update changes a table.
    let change = json!({
        "remove_objects": [{ "source_table_full_name": "main.default.events" }],
        "add_objects": [{ "source_table_full_name": "main.default.events",
                          "history_data_sharing_status": "ENABLED" }],
    });
    let share = json_body(send(&seeded, patch(change)).await).await;
    assert_eq!(
        share["objects"][0]["history_data_sharing_status"],
        "ENABLED"
    );

    // Grants name shares that exist, and follow renames and deletes.
    let resp = send(
        &seeded,
        req_json(Method::POST, "/recipients", json!({ "name": "partner" })),
    )
    .await;
    assert_status(resp, StatusCode::OK, "recipient").await;
    let grant = |shares: Value| {
        req_json(
            Method::PATCH,
            "/recipients/partner",
            json!({ "add_shares": shares }),
        )
    };
    let resp = send(&seeded, grant(json!(["nope"]))).await;
    common::assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
    let resp = send(&seeded, grant(json!(["analytics"]))).await;
    assert_status(resp, StatusCode::OK, "grant").await;
    let renamed = json!({ "new_name": "metrics" });
    let resp = send(&seeded, patch(renamed)).await;
    assert_status(resp, StatusCode::OK, "rename").await;
    let partner = admin(
        &seeded,
        anonymous,
        Method::GET,
        "/recipients/partner",
        json!({}),
    )
    .await;
    assert_eq!(partner["shares"], json!(["metrics"]));
    let resp = send(
        &seeded,
        req_json(Method::DELETE, "/shares/metrics", json!({})),
    )
    .await;
    assert_status(resp, StatusCode::NO_CONTENT, "delete").await;
    // A new share of the same name is not granted.
    admin(
        &seeded,
        anonymous,
        Method::POST,
        "/shares",
        json!({ "name": "metrics" }),
    )
    .await;
    let partner = admin(
        &seeded,
        anonymous,
        Method::GET,
        "/recipients/partner",
        json!({}),
    )
    .await;
    assert_eq!(partner["shares"], json!([]));

    let resp = send(
        &seeded,
        req_json(
            Method::POST,
            "/recipients",
            json!({ "name": "oauth", "authentication_type": "OAUTH_CLIENT_CREDENTIALS" }),
        ),
    )
    .await;
    common::assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
}

#[rstest]
#[tokio::test]
async fn profiles_name_the_protocol_where_the_client_called(#[future(awt)] app: Router) {
    admin(
        &app,
        anonymous,
        Method::POST,
        "/recipients",
        json!({ "name": "partner" }),
    )
    .await;
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("{BASE}/recipients/partner/profile"))
        .header("host", "uc.example.com")
        .header("x-forwarded-proto", "https")
        .body(Body::empty())
        .unwrap();
    let profile =
        json_body(assert_status(send(&app, request).await, StatusCode::OK, "profile").await).await;
    assert_eq!(profile["shareCredentialsVersion"], 1);
    assert_eq!(profile["endpoint"], "https://uc.example.com/delta-sharing");
    let token = profile["bearerToken"].as_str().unwrap();
    let resp = send(&app, get("/delta-sharing/shares", token)).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn protocol_calls_without_token_return_401(#[future(awt)] app: Router) {
    let request = Request::builder()
        .uri("/delta-sharing/shares")
        .body(Body::empty())
        .unwrap();
    let resp = send(&app, request).await;
    assert_protocol_error(resp, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED").await;
    let resp = send(&app, get("/delta-sharing/shares", "dsr_unknown")).await;
    assert_protocol_error(resp, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED").await;
}

#[rstest]
#[tokio::test]
async fn recipient_without_share_grant_gets_403(#[future(awt)] app: Router) {
    admin(
        &app,
        anonymous,
        Method::POST,
        "/shares",
        json!({ "name": "secret" }),
    )
    .await;
    let token = recipient(&app, anonymous, "noone", &[]).await;
    let resp = send(&app, get("/delta-sharing/shares/secret/schemas", &token)).await;
    assert_protocol_error(resp, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
}

#[tokio::test]
async fn with_sign_in_on_recipients_use_their_own_tokens() {
    let (app, metastore) = common::signed_in_app().await;
    // As the server does at start.
    uc_server::authn::security::initialize_admin(&metastore)
        .await
        .unwrap();
    admin(
        &app,
        as_admin,
        Method::POST,
        "/catalogs",
        json!({ "name": "main" }),
    )
    .await;
    let schema = json!({ "name": "default", "catalog_name": "main" });
    admin(&app, as_admin, Method::POST, "/schemas", schema).await;
    let mut table = DeltaTable::new(false, json!({}));
    let add = table.add("part-0.parquet", &[1], None);
    table.commit(1000, vec![add]);
    register(&app, as_admin, &table.url(), "events").await;
    add_table(&app, as_admin, "analytics", "events", json!({})).await;
    let token = recipient(&app, as_admin, "partner", &["analytics"]).await;

    let (_, lines) = query(&app, &token, "", json!({})).await;
    let url = pick(&lines, "file")[0]["url"].as_str().unwrap().to_string();
    let (status, _) = fetch(&app, &url, None).await;
    assert_eq!(status, StatusCode::OK);
    // A recipient token is not a UC token.
    let resp = send(&app, common::req_bearer(Method::GET, "/catalogs", &token)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
