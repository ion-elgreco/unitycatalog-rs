//! The `[tables]` flags of managed tables, after upstream's `SdkUseDeltaApiOnlyFlagTest`.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_metastore::{MetastoreConfig, SqlMetastore, TableSettings};
use uc_server::{AppState, build_app};

use common::{assert_delta_error, assert_error, assert_status, json_body, req, req_json};

async fn app(tables: TableSettings, delta_api_only: bool) -> Router {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests().with_tables(tables))
        .await
        .unwrap();
    let app = build_app(AppState::new(metastore).with_delta_api_only(delta_api_only));
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
    app
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let request = match body {
        Some(b) => req_json(method, path, b),
        None => req(method, path),
    };
    app.clone().oneshot(request).await.unwrap()
}

fn external_table() -> Value {
    json!({ "name": "ext", "catalog_name": "c", "schema_name": "s", "table_type": "EXTERNAL",
        "data_source_format": "DELTA", "storage_location": "s3://lake/ext" })
}

#[tokio::test]
async fn managed_tables_can_be_disabled() {
    let app = app(
        TableSettings {
            managed_tables_enabled: false,
            ..Default::default()
        },
        false,
    )
    .await;
    let disabled = "MANAGED table is an is currently disabled. To enable it, set tables.managed = true (UC_TABLES_MANAGED)";
    let staging = json!({ "name": "t", "catalog_name": "c", "schema_name": "s" });
    let err = assert_error(
        send(&app, Method::POST, "/staging-tables", Some(staging)).await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(err["message"], disabled);
    let resp = send(
        &app,
        Method::POST,
        "/delta/v1/catalogs/c/schemas/s/staging-tables",
        Some(json!({ "name": "t" })),
    )
    .await;
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    assert_eq!(err["message"], disabled);
    let managed = json!({ "name": "m", "catalog_name": "c", "schema_name": "s", "table_type": "MANAGED",
        "data_source_format": "DELTA", "storage_location": "s3://lake/m" });
    let err = assert_error(
        send(&app, Method::POST, "/tables", Some(managed)).await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(err["message"], disabled);
    let get = json!({ "table_id": "00000000-0000-0000-0000-000000000000", "table_uri": "s3://x/y", "start_version": 0 });
    let err = assert_error(
        send(&app, Method::GET, "/delta/preview/commits", Some(get)).await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(err["message"], disabled);
    // EXTERNAL tables are unaffected.
    assert_status(
        send(&app, Method::POST, "/tables", Some(external_table())).await,
        StatusCode::OK,
        "external",
    )
    .await;
}

#[tokio::test]
async fn delta_api_only_refuses_the_uc_writers_of_managed_tables() {
    let app = app(TableSettings::default(), true).await;
    let only = |endpoint: &str| {
        format!(
            "This Unity Catalog endpoint is disabled when tables.delta_api_only = true (UC_TABLES_DELTA_API_ONLY). Use the Delta endpoint {endpoint} instead."
        )
    };
    let staging = json!({ "name": "t", "catalog_name": "c", "schema_name": "s" });
    let err = assert_error(
        send(&app, Method::POST, "/staging-tables", Some(staging)).await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(
        err["message"],
        only("POST /delta/v1/catalogs/{catalog}/schemas/{schema}/staging-tables")
    );
    let managed = json!({ "name": "m", "catalog_name": "c", "schema_name": "s", "table_type": "MANAGED",
        "data_source_format": "DELTA", "storage_location": "s3://lake/m" });
    let err = assert_error(
        send(&app, Method::POST, "/tables", Some(managed)).await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(
        err["message"],
        "This Unity Catalog endpoint is disabled for MANAGED Delta tables when tables.delta_api_only = true (UC_TABLES_DELTA_API_ONLY). Use the Delta endpoint POST /delta/v1/catalogs/{catalog}/schemas/{schema}/tables instead."
    );
    let commit = json!({ "table_id": "00000000-0000-0000-0000-000000000000", "table_uri": "s3://x/y",
        "latest_backfilled_version": 1 });
    let err = assert_error(
        send(&app, Method::POST, "/delta/preview/commits", Some(commit)).await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(
        err["message"],
        only(
            "POST /delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table} with action add-commit"
        )
    );

    // The Delta API still creates MANAGED tables, and the UC API still
    // serves EXTERNAL ones.
    let resp = send(
        &app,
        Method::POST,
        "/delta/v1/catalogs/c/schemas/s/staging-tables",
        Some(json!({ "name": "t" })),
    )
    .await;
    let staging = json_body(assert_status(resp, StatusCode::OK, "delta staging").await).await;
    let resp = send(
        &app,
        Method::POST,
        "/temporary-table-credentials",
        Some(json!({ "table_id": staging["table-id"], "operation": "READ_WRITE" })),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .starts_with("This Unity Catalog endpoint is disabled for MANAGED Delta tables")
    );
    let resp = send(&app, Method::POST, "/tables", Some(external_table())).await;
    let ext = json_body(assert_status(resp, StatusCode::OK, "external").await).await;
    let resp = send(
        &app,
        Method::POST,
        "/temporary-table-credentials",
        Some(json!({ "table_id": ext["table_id"], "operation": "READ" })),
    )
    .await;
    let body = json_body(resp).await;
    let message = body["message"].as_str().unwrap_or_default();
    assert!(
        !message.starts_with("This Unity Catalog endpoint is disabled"),
        "EXTERNAL tables keep their UC credentials"
    );
}

#[tokio::test]
async fn iceberg_compat_v2_tables_may_skip_deletion_vectors() {
    for allow in [false, true] {
        let app = app(
            TableSettings {
                allow_missing_deletion_vectors: allow,
                ..Default::default()
            },
            false,
        )
        .await;
        let resp = send(
            &app,
            Method::POST,
            "/delta/v1/catalogs/c/schemas/s/staging-tables",
            Some(json!({ "name": "t" })),
        )
        .await;
        let staging = json_body(assert_status(resp, StatusCode::OK, "staging").await).await;
        let features = json!(["catalogManaged", "v2Checkpoint", "vacuumProtocolCheck"]);
        let body = json!({
            "name": "t", "location": staging["location"], "table-type": "MANAGED",
            "columns": { "type": "struct", "fields": [{ "name": "id", "type": "long", "nullable": true, "metadata": {} }] },
            "protocol": { "min-reader-version": 3, "min-writer-version": 7, "reader-features": features,
                "writer-features": ["catalogManaged", "v2Checkpoint", "vacuumProtocolCheck", "inCommitTimestamp"] },
            "properties": { "delta.checkpointPolicy": "v2", "delta.enableInCommitTimestamps": "true",
                "delta.checkpoint.writeStatsAsStruct": "true", "delta.checkpoint.writeStatsAsJson": "true",
                "delta.enableIcebergCompatV2": "true", "io.unitycatalog.tableId": staging["table-id"] },
            "last-commit-timestamp-ms": 1,
        });
        let resp = send(
            &app,
            Method::POST,
            "/delta/v1/catalogs/c/schemas/s/tables",
            Some(body),
        )
        .await;
        if allow {
            assert_status(resp, StatusCode::OK, "allowed").await;
        } else {
            let err = assert_delta_error(
                resp,
                StatusCode::BAD_REQUEST,
                "InvalidParameterValueException",
            )
            .await;
            assert_eq!(
                err["message"],
                "MANAGED table missing required reader-feature 'deletionVectors'."
            );
        }
    }
}
