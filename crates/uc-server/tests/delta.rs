//! `/delta/preview/commits`, after upstream `SdkDeltaCommitsCRUDTest`.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{assert_error, assert_status, json_body, req, req_json};

const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

/// An app with a MANAGED Delta table `c.s.t` in a temp directory.
async fn managed_table() -> (Router, Value) {
    let (app, _) = common::app_with_metastore().await;
    let root = common::temp_storage_root();
    for (path, body) in [
        ("/catalogs", json!({ "name": "c", "storage_root": root })),
        ("/schemas", json!({ "name": "s", "catalog_name": "c" })),
    ] {
        let resp = app
            .clone()
            .oneshot(req_json(Method::POST, path, body))
            .await
            .unwrap();
        assert_status(resp, StatusCode::OK, path).await;
    }
    let table = create_table(&app, "t", "MANAGED", None).await;
    (app, table)
}

async fn create_table(app: &Router, name: &str, kind: &str, location: Option<&str>) -> Value {
    let (location, properties) = match location {
        Some(l) => (json!(l), json!({})),
        None => {
            let resp = app
                .clone()
                .oneshot(req_json(
                    Method::POST,
                    "/staging-tables",
                    json!({ "name": name, "catalog_name": "c", "schema_name": "s" }),
                ))
                .await
                .unwrap();
            let staging = json_body(assert_status(resp, StatusCode::OK, "staging").await).await;
            (
                staging["staging_location"].clone(),
                json!({ "io.unitycatalog.tableId": staging["id"] }),
            )
        }
    };
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/tables",
            json!({
                "name": name, "catalog_name": "c", "schema_name": "s",
                "table_type": kind, "data_source_format": "DELTA",
                "storage_location": location, "properties": properties,
                "columns": [{ "name": "id", "type_name": "LONG", "type_text": "long", "position": 0,
                    "type_json": r#"{"name":"id","type":"long","nullable":true,"metadata":{}}"# }],
            }),
        ))
        .await
        .unwrap();
    json_body(assert_status(resp, StatusCode::OK, "create table").await).await
}

fn id(table: &Value) -> &str {
    table["table_id"].as_str().unwrap()
}

fn uri(table: &Value) -> &str {
    table["storage_location"].as_str().unwrap()
}

/// Upstream `createCommitObject`: file `file<v>`, timestamps `1700000000 + v`,
/// and metadata that carries the table id property.
fn commit(table: &Value, version: i64) -> Value {
    let ts = 1_700_000_000 + version;
    json!({
        "table_id": id(table),
        "table_uri": uri(table),
        "commit_info": {
            "version": version, "file_name": format!("file{version}"), "file_size": 100,
            "timestamp": ts, "file_modification_timestamp": ts,
        },
        "metadata": { "properties": { "properties": { "io.unitycatalog.tableId": id(table) } } },
    })
}

fn backfill(table: &Value, version: i64) -> Value {
    json!({ "table_id": id(table), "table_uri": uri(table), "latest_backfilled_version": version })
}

async fn post(app: &Router, body: &Value) -> axum::response::Response {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/delta/preview/commits",
            body.clone(),
        ))
        .await
        .unwrap()
}

async fn ok(app: &Router, body: &Value) {
    let resp = post(app, body).await;
    let resp = assert_status(resp, StatusCode::OK, &body.to_string()).await;
    let bytes = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .unwrap()
        .to_bytes();
    assert!(
        bytes.is_empty(),
        "upstream answers a commit with an empty body"
    );
}

async fn fails(app: &Router, body: &Value, status: StatusCode, code: &str, message: &str) {
    let err = assert_error(post(app, body).await, status, code).await;
    let got = err["message"].as_str().unwrap();
    assert!(got.contains(message), "{got:?} should contain {message:?}");
}

async fn get_commits(app: &Router, body: Value) -> axum::response::Response {
    app.clone()
        .oneshot(req_json(Method::GET, "/delta/preview/commits", body))
        .await
        .unwrap()
}

/// `(latest_table_version, versions newest first)` from `start..=end`.
async fn commits(app: &Router, table: &Value, start: i64, end: Option<i64>) -> (i64, Vec<i64>) {
    let mut body =
        json!({ "table_id": id(table), "table_uri": uri(table), "start_version": start });
    if let Some(end) = end {
        body["end_version"] = json!(end);
    }
    let resp = assert_status(get_commits(app, body).await, StatusCode::OK, "get commits").await;
    let resp = json_body(resp).await;
    let versions = resp["commits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["version"].as_i64().unwrap())
        .collect();
    (resp["latest_table_version"].as_i64().unwrap(), versions)
}

fn write(table: &Value, relative: &str, content: &str) {
    let path = std::path::Path::new(uri(table).strip_prefix("file://").unwrap()).join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

#[tokio::test]
async fn basic_coordinated_commits() {
    let (app, t) = managed_table().await;
    assert_eq!(commits(&app, &t, 0, None).await, (0, vec![]));

    ok(&app, &commit(&t, 1)).await;
    // Replaying the same version and file name is a no-op success.
    ok(&app, &commit(&t, 1)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (1, vec![1]));
    let resp = json_body(
        get_commits(
            &app,
            json!({
                "table_id": id(&t), "table_uri": uri(&t), "start_version": 0,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(
        resp["commits"][0],
        json!({
            "version": 1, "file_name": "file1", "file_size": 100,
            "timestamp": 1_700_000_001, "file_modification_timestamp": 1_700_000_001,
        })
    );

    let mut other = commit(&t, 1);
    other["commit_info"]["file_name"] = json!("some_other_filename");
    fails(&app, &other, StatusCode::CONFLICT, "COMMIT_VERSION_CONFLICT",
        "Commit version already accepted. Version 1 was accepted with commit file file1, but the request carried some_other_filename.").await;
    fails(
        &app,
        &commit(&t, 3),
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
        "Commit version must be the next version after the latest commit 1, but got 3",
    )
    .await;

    let mut no_metadata = commit(&t, 2);
    no_metadata.as_object_mut().unwrap().remove("metadata");
    ok(&app, &no_metadata).await;
    ok(&app, &commit(&t, 3)).await;
    // An older, still tracked version: same name replays, another conflicts.
    ok(&app, &commit(&t, 1)).await;
    fails(
        &app,
        &other,
        StatusCode::CONFLICT,
        "COMMIT_VERSION_CONFLICT",
        "Commit version already accepted.",
    )
    .await;

    let err = assert_error(
        get_commits(
            &app,
            json!({
                "table_id": ZERO_UUID, "table_uri": uri(&t), "start_version": 0,
            }),
        )
        .await,
        StatusCode::NOT_FOUND,
        "TABLE_NOT_FOUND",
    )
    .await;
    assert_eq!(err["message"], format!("Table not found: {ZERO_UUID}"));

    assert_eq!(commits(&app, &t, 0, None).await, (3, vec![3, 2, 1]));
    assert_eq!(commits(&app, &t, 2, None).await, (3, vec![3, 2]));
    assert_eq!(commits(&app, &t, 2, Some(2)).await, (3, vec![2]));

    // Commit 4 and backfill up to 1 in one request.
    let mut c4 = commit(&t, 4);
    c4["latest_backfilled_version"] = json!(1);
    ok(&app, &c4).await;
    assert_eq!(commits(&app, &t, 0, None).await, (4, vec![4, 3, 2]));
    ok(&app, &backfill(&t, 1)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (4, vec![4, 3, 2]));
    ok(&app, &backfill(&t, 4)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (4, vec![]));
    // The backfilled newest commit stays as a marker, so its replay is
    // still recognized by name.
    ok(&app, &commit(&t, 4)).await;

    // Purged versions are settled by comparing the staged file with the
    // published one.
    write(
        &t,
        "_delta_log/00000000000000000002.json",
        "delta-commit-v2\n",
    );
    write(&t, "_delta_log/_staged_commits/file2", "delta-commit-v2\n");
    ok(&app, &commit(&t, 2)).await;
    write(&t, "_delta_log/_staged_commits/file2", "different\n");
    fails(
        &app,
        &commit(&t, 2),
        StatusCode::CONFLICT,
        "COMMIT_VERSION_CONFLICT",
        "Commit version already accepted. Version 2 is already published.",
    )
    .await;
    // Nothing published for v3: unknown, so the client should retry.
    fails(
        &app,
        &commit(&t, 3),
        StatusCode::INTERNAL_SERVER_ERROR,
        "COMMIT_STATE_UNKNOWN",
        "retry the request",
    )
    .await;

    ok(&app, &commit(&t, 5)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (5, vec![5]));

    // Dropping the table drops its commits.
    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, "/tables/c.s.t"))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "delete").await;
    assert_error(
        get_commits(
            &app,
            json!({
                "table_id": id(&t), "table_uri": uri(&t), "start_version": 0,
            }),
        )
        .await,
        StatusCode::NOT_FOUND,
        "TABLE_NOT_FOUND",
    )
    .await;
}

#[tokio::test]
async fn a_replay_ignores_the_rest_of_the_request() {
    let (app, t) = managed_table().await;
    ok(&app, &commit(&t, 1)).await;
    ok(&app, &commit(&t, 2)).await;
    let mut replay = commit(&t, 2);
    replay["latest_backfilled_version"] = json!(1);
    ok(&app, &replay).await;
    assert_eq!(commits(&app, &t, 0, None).await, (2, vec![2, 1]));
}

#[tokio::test]
async fn invalid_requests_follow_upstream() {
    let (app, t) = managed_table().await;
    let bad = |edit: &dyn Fn(&mut Value)| {
        let mut c = commit(&t, 1);
        edit(&mut c);
        c
    };
    let cases: Vec<(Value, &str)> = vec![
        (
            bad(&|c| {
                c.as_object_mut().unwrap().remove("table_id");
            }),
            "Field can not be empty: table_id",
        ),
        (
            bad(&|c| c["table_id"] = json!("")),
            "Field can not be empty: table_id",
        ),
        (
            bad(&|c| c["table_uri"] = json!("")),
            "Field can not be empty: table_uri",
        ),
        (
            bad(&|c| {
                c.as_object_mut().unwrap().remove("table_uri");
            }),
            "Field can not be empty: table_uri",
        ),
        (
            bad(&|c| c["commit_info"]["version"] = json!(0)),
            "Field must be positive: version",
        ),
        (
            bad(&|c| c["commit_info"]["version"] = json!(-1)),
            "Field must be positive: version",
        ),
        (
            bad(&|c| c["commit_info"]["version"] = Value::Null),
            "Field must be positive: version",
        ),
        (
            bad(&|c| {
                c.as_object_mut().unwrap().remove("commit_info");
            }),
            "Either commit_info or latest_backfilled_version must be defined",
        ),
        (
            bad(&|c| c["commit_info"]["file_name"] = json!("")),
            "Field can not be empty: file_name",
        ),
        (
            bad(&|c| c["commit_info"]["file_name"] = json!("../1.json")),
            "Field must be a single file name without path separators: file_name",
        ),
        (
            bad(&|c| c["commit_info"]["file_size"] = json!(0)),
            "Field must be positive: file_size",
        ),
        (
            bad(&|c| c["commit_info"]["timestamp"] = json!(-1)),
            "Field must be positive: timestamp",
        ),
        (
            bad(&|c| c["commit_info"]["file_modification_timestamp"] = json!(0)),
            "Field must be positive: file_modification_timestamp",
        ),
        (
            bad(&|c| c["metadata"] = json!({})),
            "At least one of description, properties, or schema must be set in commit.metadata",
        ),
        (
            bad(&|c| c["metadata"]["properties"]["properties"] = json!({ "random": "x" })),
            "Properties does not contain io.unitycatalog.tableId.",
        ),
        (
            bad(&|c| {
                c["metadata"]["properties"]["properties"] =
                    json!({ "io.unitycatalog.tableId": ZERO_UUID })
            }),
            "does not match the properties io.unitycatalog.tableId",
        ),
    ];
    for (body, message) in &cases {
        fails(
            &app,
            body,
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            message,
        )
        .await;
    }

    // A well-formed id of a missing table.
    let mut missing = commit(&t, 1);
    missing["table_id"] = json!(ZERO_UUID);
    missing["metadata"]["properties"]["properties"]["io.unitycatalog.tableId"] = json!(ZERO_UUID);
    fails(
        &app,
        &missing,
        StatusCode::NOT_FOUND,
        "TABLE_NOT_FOUND",
        "Table not found",
    )
    .await;

    ok(&app, &commit(&t, 1)).await;
    let mut wrong_uri = commit(&t, 2);
    wrong_uri["table_uri"] = json!("s3://wrong-bucket/wrong-path");
    fails(
        &app,
        &wrong_uri,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
        "Table URI in commit s3://wrong-bucket/wrong-path does not match the table path",
    )
    .await;
    // The URI is compared after normalization.
    let mut loose_uri = commit(&t, 2);
    loose_uri["table_uri"] = json!(uri(&t).replace("file:///", "file:/"));
    ok(&app, &loose_uri).await;

    // Only MANAGED tables take commits.
    let external = create_table(&app, "ext", "EXTERNAL", Some("s3://lake/ext")).await;
    let mut to_external = commit(&external, 1);
    to_external["metadata"]["properties"]["properties"]["io.unitycatalog.tableId"] =
        json!(id(&external));
    fails(
        &app,
        &to_external,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
        "Only managed tables are supported for Delta commits",
    )
    .await;

    let get_cases = [
        (
            json!({ "table_id": id(&t), "table_uri": uri(&t) }),
            "Field can not be null: start_version",
        ),
        (
            json!({ "table_id": id(&t), "table_uri": uri(&t), "start_version": -1 }),
            "Field must be >=0: start_version",
        ),
        (
            json!({ "table_id": id(&t), "table_uri": uri(&t), "start_version": 5, "end_version": 3 }),
            "end_version must be >=start_version if set",
        ),
        (
            json!({ "table_uri": uri(&t), "start_version": 0 }),
            "Field can not be null: table_id",
        ),
        (
            json!({ "table_id": "nope", "start_version": 0 }),
            "Invalid UUID string: nope",
        ),
    ];
    for (body, message) in get_cases {
        let err = assert_error(
            get_commits(&app, body).await,
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
        )
        .await;
        assert_eq!(err["message"], message);
    }
}

#[tokio::test]
async fn backfills_at_different_versions() {
    let (app, t) = managed_table().await;
    fails(
        &app,
        &backfill(&t, 1),
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
        "Backfill request requires a prior commit; the table's commit log is empty.",
    )
    .await;
    for v in 1..=5 {
        ok(&app, &commit(&t, v)).await;
    }
    assert_eq!(commits(&app, &t, 0, None).await, (5, vec![5, 4, 3, 2, 1]));

    let mut with_metadata = backfill(&t, 2);
    with_metadata["metadata"] = json!({ "description": "x" });
    fails(
        &app,
        &with_metadata,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
        "metadata shouldn't be set for backfill only commit",
    )
    .await;
    ok(&app, &backfill(&t, 2)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (5, vec![5, 4, 3]));

    fails(
        &app,
        &backfill(&t, 9),
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
        "Should not backfill version 9 while the last version committed is 5",
    )
    .await;
    for b in [9, 6] {
        let mut c6 = commit(&t, 6);
        c6["latest_backfilled_version"] = json!(b);
        fails(
            &app,
            &c6,
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            &format!(
                "Latest backfilled version {b} cannot be greater than the last commit version = 5"
            ),
        )
        .await;
    }
    ok(&app, &backfill(&t, 4)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (5, vec![5]));
    ok(&app, &backfill(&t, 5)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (5, vec![]));
    ok(&app, &commit(&t, 6)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (6, vec![6]));
}

#[tokio::test]
async fn at_most_ten_unbackfilled_commits() {
    let (app, t) = managed_table().await;
    for v in 1..=10 {
        ok(&app, &commit(&t, v)).await;
    }
    assert_eq!(
        commits(&app, &t, 0, None).await.1,
        (1..=10).rev().collect::<Vec<_>>()
    );
    fails(
        &app,
        &commit(&t, 11),
        StatusCode::TOO_MANY_REQUESTS,
        "RESOURCE_EXHAUSTED",
        &format!("Max number of commits per table reached: {}", id(&t)),
    )
    .await;

    ok(&app, &backfill(&t, 6)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (10, vec![10, 9, 8, 7]));
    for v in 11..=14 {
        ok(&app, &commit(&t, v)).await;
    }
    assert_eq!(
        commits(&app, &t, 0, None).await.1,
        (7..=14).rev().collect::<Vec<_>>()
    );
    ok(&app, &backfill(&t, 14)).await;
    assert_eq!(commits(&app, &t, 0, None).await, (14, vec![]));
    for v in 15..=24 {
        ok(&app, &commit(&t, v)).await;
    }
}

#[tokio::test]
async fn pages_of_commits() {
    let (app, t) = managed_table().await;
    for v in 1..=10 {
        ok(&app, &commit(&t, v)).await;
    }
    assert_eq!(commits(&app, &t, 0, Some(3)).await, (10, vec![3, 2, 1]));
    assert_eq!(commits(&app, &t, 4, Some(7)).await, (10, vec![7, 6, 5, 4]));
    assert_eq!(commits(&app, &t, 8, None).await, (10, vec![10, 9, 8]));
    assert_eq!(commits(&app, &t, 11, None).await, (10, vec![]));
    assert_eq!(commits(&app, &t, 3, Some(3)).await, (10, vec![3]));
}

#[tokio::test]
async fn commits_update_table_metadata() {
    let (app, t) = managed_table().await;
    let get_table = || async {
        let resp = app
            .clone()
            .oneshot(req(Method::GET, "/tables/c.s.t"))
            .await
            .unwrap();
        json_body(assert_status(resp, StatusCode::OK, "get table").await).await
    };

    let mut c1 = commit(&t, 1);
    c1["metadata"]["properties"]["properties"] = json!({
        "io.unitycatalog.tableId": id(&t), "customProperty": "customValue", "delta.feature.test": "enabled",
    });
    ok(&app, &c1).await;
    let t1 = get_table().await;
    assert_eq!(t1["properties"]["customProperty"], "customValue");
    assert_eq!(t1["properties"]["delta.feature.test"], "enabled");

    let mut c2 = commit(&t, 2);
    c2["metadata"] = json!({ "description": "Updated table description via commit" });
    ok(&app, &c2).await;
    let t2 = get_table().await;
    assert_eq!(t2["comment"], "Updated table description via commit");
    assert_eq!(t2["properties"], t1["properties"]);
    assert_eq!(t2["columns"], t1["columns"]);

    // Commit metadata columns are stored as sent.
    let mut c3 = commit(&t, 3);
    c3["metadata"] = json!({ "schema": { "columns": [
        { "name": "new_column_1", "type_name": "STRING", "type_text": "string",
          "type_json": "{\"type\":\"string\"}", "position": 0, "nullable": true },
        { "name": "new_column_2", "type_name": "INT", "type_text": "int",
          "type_json": "{\"type\":\"integer\"}", "position": 1, "nullable": false },
    ] } });
    ok(&app, &c3).await;
    let t3 = get_table().await;
    let names: Vec<_> = t3["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].clone())
        .collect();
    assert_eq!(names, [json!("new_column_1"), json!("new_column_2")]);
    assert_eq!(t3["properties"], t2["properties"]);
    assert_eq!(t3["comment"], t2["comment"]);

    // Properties are replaced, not merged.
    let mut c4 = commit(&t, 4);
    c4["metadata"] = json!({
        "description": "Complete metadata test table",
        "properties": { "properties": { "io.unitycatalog.tableId": id(&t), "customProperty": "customValueNew" } },
        "schema": { "columns": [{ "name": "test_column_new", "type_name": "STRING", "type_text": "string",
            "type_json": "{\"type\":\"string\"}", "position": 0, "nullable": true,
            "comment": "Test column comment new" }] },
    });
    ok(&app, &c4).await;
    let t4 = get_table().await;
    assert_eq!(t4["comment"], "Complete metadata test table");
    assert!(t4["properties"].get("delta.feature.test").is_none());
    assert_eq!(t4["properties"]["customProperty"], "customValueNew");
    assert_eq!(t4["columns"][0]["comment"], "Test column comment new");
}
