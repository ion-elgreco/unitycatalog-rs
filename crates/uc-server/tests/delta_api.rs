//! The UC Delta REST API (`/delta/v1`), after upstream `DeltaApiService`
//! and `SdkLoadTableTest` / `SdkUpdateTableTest`.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{assert_delta_error, assert_status, json_body, req, seeded};

#[rstest]
#[tokio::test]
async fn config_lists_endpoints_for_the_catalog(#[future(awt)] seeded: Router) {
    let resp = seeded
        .clone()
        .oneshot(req(
            Method::GET,
            "/delta/v1/config?catalog=main&protocol-versions=1.0",
        ))
        .await
        .unwrap();
    let config = json_body(assert_status(resp, StatusCode::OK, "config").await).await;
    assert_eq!(config["protocol-version"], "1.0");
    let endpoints = config["endpoints"].as_array().unwrap();
    assert_eq!(endpoints.len(), 12);
    assert!(endpoints.contains(&json!(
        "GET /v1/catalogs/main/schemas/{schema}/tables/{table}"
    )));
    assert!(endpoints.contains(&json!("GET /v1/staging-tables/{table_id}/credentials")));

    // Errors use the Delta body and error types.
    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/delta/v1/config?catalog="))
        .await
        .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Must supply a proper catalog in catalog parameter."
    );
    let resp = seeded
        .oneshot(req(
            Method::GET,
            "/delta/v1/config?catalog=nope&protocol-versions=1.0",
        ))
        .await
        .unwrap();
    let err = assert_delta_error(resp, StatusCode::NOT_FOUND, "NoSuchCatalogException").await;
    assert_eq!(err["message"], "Catalog not found: nope");
}

async fn external_table(app: &Router, name: &str) -> serde_json::Value {
    let resp = app.clone().oneshot(common::req_json(Method::POST, "/tables", json!({
        "name": name, "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("s3://lake/{name}"),
        "columns": [{ "name": "id", "type_name": "LONG", "type_text": "bigint", "position": 0,
            "type_json": r#"{"name":"id","type":"long","nullable":true,"metadata":{}}"# }],
    }))).await.unwrap();
    json_body(assert_status(resp, StatusCode::OK, "create table").await).await
}

const T: &str = "/delta/v1/catalogs/main/schemas/default/tables";

#[rstest]
#[tokio::test]
async fn head_rename_metrics_and_delete(#[future(awt)] seeded: Router) {
    let app = &seeded;
    let send = |method: Method, path: String, body: Option<serde_json::Value>| {
        let request = match body {
            Some(b) => common::req_json(method, &path, b),
            None => req(method, &path),
        };
        app.clone().oneshot(request)
    };
    let table = external_table(app, "t").await;
    external_table(app, "taken").await;

    let resp = send(Method::HEAD, format!("{T}/t"), None).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = send(Method::HEAD, format!("{T}/nope"), None).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Metrics are acknowledged when the body names the same table.
    let resp = send(Method::POST, format!("{T}/t/metrics"), Some(json!({})))
        .await
        .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    assert_eq!(err["message"], "table-id is required.");
    let other = "00000000-0000-0000-0000-000000000000";
    let resp = send(
        Method::POST,
        format!("{T}/t/metrics"),
        Some(json!({ "table-id": other })),
    )
    .await
    .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    assert_eq!(
        err["message"],
        format!("table-id {other} does not match table main.default.t.")
    );
    let resp = send(
        Method::POST,
        format!("{T}/t/metrics"),
        Some(json!({
            "table-id": table["table_id"], "report": { "commit-report": { "num-files-added": 1 } },
        })),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Rename within the schema.
    let resp = send(Method::POST, format!("{T}/t/rename"), Some(json!({})))
        .await
        .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    assert_eq!(err["message"], "New table name is required.");
    let resp = send(
        Method::POST,
        format!("{T}/t/rename"),
        Some(json!({ "new-name": "taken" })),
    )
    .await
    .unwrap();
    assert_delta_error(resp, StatusCode::CONFLICT, "AlreadyExistsException").await;
    let resp = send(
        Method::POST,
        format!("{T}/t/rename"),
        Some(json!({ "new-name": "renamed" })),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = send(Method::HEAD, format!("{T}/renamed"), None)
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = send(Method::DELETE, format!("{T}/renamed"), None)
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = send(Method::DELETE, format!("{T}/renamed"), None)
        .await
        .unwrap();
    let err = assert_delta_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
    assert_eq!(err["message"], "Table not found: main.default.renamed");
    let resp = send(
        Method::DELETE,
        "/delta/v1/catalogs/main/schemas/nope/tables/x".into(),
        None,
    )
    .await
    .unwrap();
    assert_delta_error(resp, StatusCode::NOT_FOUND, "NoSuchSchemaException").await;
}

/// An app with a catalog `c.s` rooted in a fresh temp directory.
async fn managed_schema() -> Router {
    let (app, _) = common::app_with_metastore().await;
    let root = common::temp_storage_root();
    for (path, body) in [
        ("/catalogs", json!({ "name": "c", "storage_root": root })),
        ("/schemas", json!({ "name": "s", "catalog_name": "c" })),
    ] {
        let resp = app
            .clone()
            .oneshot(common::req_json(Method::POST, path, body))
            .await
            .unwrap();
        assert_status(resp, StatusCode::OK, path).await;
    }
    app
}

const STAGING: &str = "/delta/v1/catalogs/c/schemas/s/staging-tables";

#[tokio::test]
async fn staging_tables_carry_the_managed_contract() {
    let app = managed_schema().await;
    let resp = app
        .clone()
        .oneshot(common::req_json(Method::POST, STAGING, json!({})))
        .await
        .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    assert_eq!(err["message"], "Staging table name is required.");

    let resp = app
        .clone()
        .oneshot(common::req_json(
            Method::POST,
            STAGING,
            json!({ "name": "sales" }),
        ))
        .await
        .unwrap();
    let staging = json_body(assert_status(resp, StatusCode::OK, "staging").await).await;
    let id = staging["table-id"].as_str().unwrap();
    let location = staging["location"].as_str().unwrap();
    assert_eq!(staging["table-type"], "MANAGED");
    assert!(location.ends_with(&format!("/tables/{id}")));
    assert_eq!(
        staging["storage-credentials"],
        json!([
            { "prefix": location, "operation": "READ_WRITE", "config": {} },
        ])
    );
    assert_eq!(
        staging["required-protocol"],
        json!({
            "min-reader-version": 3, "min-writer-version": 7,
            "reader-features": ["catalogManaged", "v2Checkpoint", "vacuumProtocolCheck", "deletionVectors"],
            "writer-features": ["catalogManaged", "v2Checkpoint", "vacuumProtocolCheck", "deletionVectors",
                "inCommitTimestamp"],
        })
    );
    assert_eq!(
        staging["suggested-protocol"],
        json!({
            "reader-features": ["columnMapping"],
            "writer-features": ["columnMapping", "domainMetadata", "rowTracking"],
        })
    );
    assert_eq!(
        staging["required-properties"],
        json!({
            "delta.checkpoint.writeStatsAsJson": "true", "delta.checkpoint.writeStatsAsStruct": "true",
            "delta.checkpointPolicy": "v2", "delta.enableDeletionVectors": "true",
            "delta.enableInCommitTimestamps": "true", "io.unitycatalog.tableId": id,
        })
    );
    assert_eq!(
        staging["suggested-properties"]["delta.columnMapping.maxColumnId"],
        serde_json::Value::Null
    );
    assert_eq!(
        staging["suggested-properties"]["delta.parquet.compression.codec"],
        "zstd"
    );

    // Staging credentials: by id, always read-write.
    let resp = app
        .clone()
        .oneshot(req(
            Method::GET,
            &format!("/delta/v1/staging-tables/{id}/credentials"),
        ))
        .await
        .unwrap();
    let creds = json_body(assert_status(resp, StatusCode::OK, "staging creds").await).await;
    assert_eq!(creds["storage-credentials"][0]["operation"], "READ_WRITE");
    assert_eq!(creds["storage-credentials"][0]["prefix"], location);
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/delta/v1/staging-tables/x/credentials"))
        .await
        .unwrap();
    assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    let missing = "00000000-0000-0000-0000-000000000000";
    let resp = app
        .oneshot(req(
            Method::GET,
            &format!("/delta/v1/staging-tables/{missing}/credentials"),
        ))
        .await
        .unwrap();
    let err = assert_delta_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
    assert_eq!(
        err["message"],
        format!("Staging table not found with id: {missing}")
    );
}

#[rstest]
#[tokio::test]
async fn table_credentials_follow_the_operation(#[future(awt)] seeded: Router) {
    let resp = seeded.clone().oneshot(common::req_json(Method::POST, "/tables", json!({
        "name": "local", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": "file:///tmp/uc-delta-ext/local",
        "columns": [{ "name": "id", "type_name": "LONG", "type_text": "bigint", "position": 0,
            "type_json": r#"{"name":"id","type":"long","nullable":true,"metadata":{}}"# }],
    }))).await.unwrap();
    assert_status(resp, StatusCode::OK, "table").await;
    let resp = seeded
        .clone()
        .oneshot(req(
            Method::GET,
            &format!("{T}/local/credentials?operation=READ"),
        ))
        .await
        .unwrap();
    let creds = json_body(assert_status(resp, StatusCode::OK, "creds").await).await;
    assert_eq!(
        creds,
        json!({ "storage-credentials": [
        { "prefix": "file:///tmp/uc-delta-ext/local", "operation": "READ", "config": {} },
    ] })
    );
    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, &format!("{T}/local/credentials")))
        .await
        .unwrap();
    assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "InvalidParameterValueException",
    )
    .await;
    let resp = seeded
        .oneshot(req(
            Method::GET,
            &format!("{T}/nope/credentials?operation=READ"),
        ))
        .await
        .unwrap();
    assert_delta_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
}

#[rstest]
#[tokio::test]
async fn load_table_returns_delta_metadata(#[future(awt)] seeded: Router) {
    let resp = seeded.clone().oneshot(common::req_json(Method::POST, "/tables", json!({
        "name": "ext", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA", "storage_location": "s3://lake//ext/",
        "properties": { "delta.lastUpdateVersion": "4", "delta.lastCommitTimestamp": "1700000000004", "k": "v" },
        "columns": [
            { "name": "id", "type_name": "LONG", "type_text": "bigint", "position": 0, "nullable": false,
              "type_json": r#"{"name":"id","type":"long","nullable":false,"metadata":{"comment":"key"}}"# },
            { "name": "tags", "type_name": "ARRAY", "type_text": "array<string>", "position": 1,
              "type_json": r#"{"name":"tags","type":{"type":"array","elementType":"string","containsNull":true},"nullable":true,"metadata":{}}"# },
        ],
    }))).await.unwrap();
    let table = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, &format!("{T}/ext")))
        .await
        .unwrap();
    let loaded = json_body(assert_status(resp, StatusCode::OK, "load").await).await;
    let m = &loaded["metadata"];
    assert_eq!(m["etag"], format!("etag-{}", table["updated_at"]));
    assert_eq!(m["table-type"], "EXTERNAL");
    assert_eq!(m["table-uuid"], table["table_id"]);
    assert_eq!(m["location"], "s3://lake/ext");
    assert_eq!(m["created-time"], table["created_at"]);
    assert_eq!(
        m["columns"],
        json!({ "type": "struct", "fields": [
        { "name": "id", "type": "long", "nullable": false, "metadata": { "comment": "key" } },
        { "name": "tags", "type": { "type": "array", "element-type": "string", "contains-null": true },
          "nullable": true, "metadata": {} },
    ] })
    );
    assert_eq!(m["partition-columns"], json!([]));
    assert_eq!(m["properties"]["k"], "v");
    assert_eq!(m["last-commit-version"], 4);
    assert_eq!(m["last-commit-timestamp-ms"], 1_700_000_000_004i64);
    assert!(loaded.get("commits").is_none() && loaded.get("latest-table-version").is_none());
    assert_eq!(
        loaded["allowed-maintenance-operations"],
        json!(["DATA_REORGANIZATION", "DATA_CLEANUP", "METADATA_CLEANUP"])
    );

    // HEAD on the same path still answers 204.
    let resp = seeded
        .clone()
        .oneshot(req(Method::HEAD, &format!("{T}/ext")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = seeded.clone().oneshot(common::req_json(Method::POST, "/tables", json!({
        "name": "p", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "PARQUET", "storage_location": "s3://lake/p",
    }))).await.unwrap();
    assert_status(resp, StatusCode::OK, "parquet").await;
    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, &format!("{T}/p")))
        .await
        .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "UnsupportedTableFormatException",
    )
    .await;
    assert_eq!(err["message"], "Table is not a Delta table: main.default.p");
    let resp = seeded
        .oneshot(req(Method::GET, &format!("{T}/nope")))
        .await
        .unwrap();
    assert_delta_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
}

#[tokio::test]
async fn load_table_lists_unbackfilled_commits() {
    let app = managed_schema().await;
    let resp = app
        .clone()
        .oneshot(common::req_json(
            Method::POST,
            "/staging-tables",
            json!({ "name": "m", "catalog_name": "c", "schema_name": "s" }),
        ))
        .await
        .unwrap();
    let staging = json_body(assert_status(resp, StatusCode::OK, "staging").await).await;
    let resp = app.clone().oneshot(common::req_json(Method::POST, "/tables", json!({
        "name": "m", "catalog_name": "c", "schema_name": "s",
        "table_type": "MANAGED", "data_source_format": "DELTA",
        "storage_location": staging["staging_location"],
        "properties": { "io.unitycatalog.tableId": staging["id"] },
        "columns": [{ "name": "id", "type_name": "LONG", "type_text": "bigint", "position": 0,
            "type_json": r#"{"name":"id","type":"long","nullable":true,"metadata":{}}"# }],
    }))).await.unwrap();
    let table = json_body(assert_status(resp, StatusCode::OK, "managed").await).await;
    let path = "/delta/v1/catalogs/c/schemas/s/tables/m";
    let resp = app.clone().oneshot(req(Method::GET, path)).await.unwrap();
    let loaded = json_body(assert_status(resp, StatusCode::OK, "load").await).await;
    assert_eq!(
        (
            loaded["commits"].clone(),
            loaded["latest-table-version"].clone()
        ),
        (json!([]), json!(0))
    );

    let commit = json!({
        "table_id": table["table_id"], "table_uri": table["storage_location"],
        "commit_info": { "version": 1, "timestamp": 11, "file_name": "c1.json", "file_size": 7,
            "file_modification_timestamp": 12 },
    });
    let resp = app
        .clone()
        .oneshot(common::req_json(
            Method::POST,
            "/delta/preview/commits",
            commit,
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "commit").await;
    let resp = app.oneshot(req(Method::GET, path)).await.unwrap();
    let loaded = json_body(assert_status(resp, StatusCode::OK, "load").await).await;
    assert_eq!(loaded["latest-table-version"], 1);
    assert_eq!(
        loaded["commits"],
        json!([
            { "version": 1, "timestamp": 11, "file-name": "c1.json", "file-size": 7, "file-modification-timestamp": 12 },
        ])
    );
    assert_eq!(loaded["metadata"]["table-type"], "MANAGED");
}

/// A staging table `c.s.<name>` through the Delta API.
async fn delta_staging(app: &Router, name: &str) -> serde_json::Value {
    let resp = app
        .clone()
        .oneshot(common::req_json(
            Method::POST,
            STAGING,
            json!({ "name": name }),
        ))
        .await
        .unwrap();
    json_body(assert_status(resp, StatusCode::OK, "staging").await).await
}

/// After upstream `managedTableRequest`: the full contract, plus the
/// clustering feature and domain.
fn managed_request(name: &str, staging: &serde_json::Value) -> serde_json::Value {
    let id = staging["table-id"].as_str().unwrap();
    json!({
        "name": name, "location": staging["location"], "table-type": "MANAGED",
        "columns": { "type": "struct", "fields": [
            { "name": "id", "type": "long", "nullable": false, "metadata": {} },
            { "name": "amount", "type": "decimal(10,2)", "nullable": true, "metadata": { "comment": "usd" } },
        ] },
        "protocol": {
            "min-reader-version": 3, "min-writer-version": 7,
            "reader-features": ["catalogManaged", "deletionVectors", "v2Checkpoint", "vacuumProtocolCheck"],
            "writer-features": ["catalogManaged", "deletionVectors", "v2Checkpoint", "vacuumProtocolCheck",
                "inCommitTimestamp", "clustering"],
        },
        "properties": {
            "delta.enableDeletionVectors": "true", "delta.checkpointPolicy": "v2",
            "delta.enableInCommitTimestamps": "true", "delta.checkpoint.writeStatsAsStruct": "true",
            "delta.checkpoint.writeStatsAsJson": "true", "io.unitycatalog.tableId": id,
            "delta.feature.catalogManaged": "client-override", "clusteringColumns": "[[\"wrong\"]]",
        },
        "domain-metadata": { "delta.clustering": { "clusteringColumns": [["id"]] } },
        "last-commit-timestamp-ms": 1_700_000_000_000i64,
    })
}

const TABLES: &str = "/delta/v1/catalogs/c/schemas/s/tables";

#[tokio::test]
async fn create_table_follows_upstream() {
    let app = managed_schema().await;
    let create = |body: serde_json::Value| {
        app.clone()
            .oneshot(common::req_json(Method::POST, TABLES, body))
    };

    let staging = delta_staging(&app, "happy").await;
    let resp = create(managed_request("happy", &staging)).await.unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "managed").await).await;
    let m = &created["metadata"];
    assert_eq!(m["table-type"], "MANAGED");
    assert_eq!(m["location"], staging["location"]);
    assert_eq!(m["table-uuid"], staging["table-id"]);
    let names: Vec<_> = m["columns"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].clone())
        .collect();
    assert_eq!(names, [json!("id"), json!("amount")]);
    assert_eq!(m["columns"]["fields"][1]["metadata"]["comment"], "usd");
    // Protocol and domain projections win over client properties.
    let props = &m["properties"];
    for feature in [
        "catalogManaged",
        "clustering",
        "deletionVectors",
        "inCommitTimestamp",
        "v2Checkpoint",
        "vacuumProtocolCheck",
    ] {
        assert_eq!(
            props[format!("delta.feature.{feature}")],
            "supported",
            "{feature}"
        );
    }
    assert_eq!(props["clusteringColumns"], "[[\"id\"]]");
    assert_eq!(props["delta.minReaderVersion"], "3");
    assert_eq!(props["delta.lastUpdateVersion"], "0");
    assert_eq!(m["last-commit-version"], 0);
    assert_eq!(m["last-commit-timestamp-ms"], 1_700_000_000_000i64);
    assert_eq!(
        (
            created["commits"].clone(),
            created["latest-table-version"].clone()
        ),
        (json!([]), json!(0))
    );
    // The UC API sees the same table.
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/tables/c.s.happy"))
        .await
        .unwrap();
    let uc = json_body(assert_status(resp, StatusCode::OK, "uc get").await).await;
    assert_eq!(uc["columns"][1]["type_text"], "decimal(10,2)");
    assert_eq!(uc["columns"][0]["type_name"], "LONG");

    // EXTERNAL tables skip the contract.
    let resp = create(json!({
        "name": "ext", "location": "s3://lake/ext", "table-type": "EXTERNAL",
        "columns": { "type": "struct", "fields": [{ "name": "id", "type": "long", "nullable": true, "metadata": {} }] },
        "protocol": { "min-reader-version": 1, "min-writer-version": 2 },
        "last-commit-timestamp-ms": 1,
    })).await.unwrap();
    let ext = json_body(assert_status(resp, StatusCode::OK, "external").await).await;
    assert_eq!(ext["metadata"]["table-type"], "EXTERNAL");
    assert!(ext.get("commits").is_none());

    // Validation messages, as upstream.
    let staging = delta_staging(&app, "bad").await;
    let with = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut body = managed_request("bad", &staging);
        edit(&mut body);
        body
    };
    let cases: Vec<(serde_json::Value, String)> = vec![
        (with(&|b| { b.as_object_mut().unwrap().remove("name"); }), "Table name is required.".into()),
        (with(&|b| { b.as_object_mut().unwrap().remove("location"); }), "Table location is required.".into()),
        (with(&|b| { b.as_object_mut().unwrap().remove("columns"); }), "columns is required.".into()),
        (with(&|b| b["columns"] = json!({ "type": "struct", "fields": [] })),
            "columns.fields must contain at least one field.".into()),
        (with(&|b| b["columns"]["fields"][0]["type"] = json!("int")),
            "columns.fields[0].type: Unsupported Delta primitive type: int".into()),
        (with(&|b| b["columns"]["fields"][0]["type"] = json!({ "type": "decimal", "scale": 2 })),
            "columns.fields[0].type.precision is required.".into()),
        (with(&|b| { b.as_object_mut().unwrap().remove("table-type"); }), "table-type is required.".into()),
        (with(&|b| { b.as_object_mut().unwrap().remove("protocol"); }), "protocol is required.".into()),
        (with(&|b| { b.as_object_mut().unwrap().remove("last-commit-timestamp-ms"); }),
            "last-commit-timestamp-ms is required.".into()),
        (with(&|b| b["protocol"]["writer-features"] = json!(["deletionVectors"])),
            "Feature 'catalogManaged' is in reader-features but not writer-features. Per the Delta protocol, every reader-feature must also be a writer-feature.".into()),
        (with(&|b| b["domain-metadata"] = json!({ "delta.rowTracking": { "rowIdHighWaterMark": 100 } })),
            "domain-metadata.delta.rowTracking requires the 'rowTracking' writer feature.".into()),
        (with(&|b| b["partition-columns"] = json!(["nope"])),
            "partition-columns references unknown column: nope".into()),
        (with(&|b| b["properties"]["io.unitycatalog.tableId"] = json!("00000000-0000-0000-0000-000000000000")),
            format!("the table id ({}) does not match the properties io.unitycatalog.tableId(00000000-0000-0000-0000-000000000000).",
                staging["table-id"].as_str().unwrap())),
        (with(&|b| b["properties"]["delta.universalFormat.enabledFormats"] = json!("iceberg")),
            "Uniform metadata must be set when 'delta.universalFormat.enabledFormats' includes 'iceberg'.".into()),
    ];
    for (body, message) in cases {
        let err = assert_delta_error(
            create(body.clone()).await.unwrap(),
            StatusCode::BAD_REQUEST,
            "InvalidParameterValueException",
        )
        .await;
        assert_eq!(err["message"], message.as_str(), "{body}");
    }

    // Partition columns are stored by index.
    let staging = delta_staging(&app, "part").await;
    let mut body = managed_request("part", &staging);
    body["partition-columns"] = json!(["ID"]);
    let resp = create(body).await.unwrap();
    let part = json_body(assert_status(resp, StatusCode::OK, "partitioned").await).await;
    assert_eq!(part["metadata"]["partition-columns"], json!(["id"]));

    // A name in use is 409.
    let staging = delta_staging(&app, "dup").await;
    let mut body = managed_request("happy", &staging);
    body["name"] = json!("happy");
    let resp = create(body).await.unwrap();
    assert_delta_error(resp, StatusCode::CONFLICT, "AlreadyExistsException").await;
}

/// A Delta API table to update: name, id and current etag.
struct Handle {
    name: String,
    id: String,
    etag: String,
}

/// After upstream `createDeltaManaged`: the contract plus row tracking,
/// with a row-tracking high water mark.
async fn delta_managed(app: &Router, name: &str, extra: serde_json::Value) -> Handle {
    let staging = delta_staging(app, name).await;
    let mut body = managed_request(name, &staging);
    let writers = body["protocol"]["writer-features"].as_array_mut().unwrap();
    writers.retain(|f| f != "clustering");
    writers.push(json!("rowTracking"));
    body["domain-metadata"] = json!({ "delta.rowTracking": { "rowIdHighWaterMark": 10 } });
    body["properties"]
        .as_object_mut()
        .unwrap()
        .remove("clusteringColumns");
    for (k, v) in extra.as_object().unwrap() {
        body["properties"][k] = v.clone();
    }
    let resp = app
        .clone()
        .oneshot(common::req_json(Method::POST, TABLES, body))
        .await
        .unwrap();
    handle(
        name,
        json_body(assert_status(resp, StatusCode::OK, "create managed").await).await,
    )
}

/// After upstream `createDeltaExternal`: columns `id long, amount double`.
async fn delta_external(app: &Router, name: &str) -> Handle {
    let resp = app
        .clone()
        .oneshot(common::req_json(
            Method::POST,
            TABLES,
            json!({
                "name": name, "location": format!("s3://lake/{name}"), "table-type": "EXTERNAL",
                "columns": { "type": "struct", "fields": [
                    { "name": "id", "type": "long", "nullable": false, "metadata": {} },
                    { "name": "amount", "type": "double", "nullable": true, "metadata": {} },
                ] },
                "protocol": { "min-reader-version": 1, "min-writer-version": 2 },
                "last-commit-timestamp-ms": 1,
            }),
        ))
        .await
        .unwrap();
    handle(
        name,
        json_body(assert_status(resp, StatusCode::OK, "create external").await).await,
    )
}

fn handle(name: &str, loaded: serde_json::Value) -> Handle {
    Handle {
        name: name.into(),
        id: loaded["metadata"]["table-uuid"].as_str().unwrap().into(),
        etag: loaded["metadata"]["etag"].as_str().unwrap().into(),
    }
}

/// POST an update with `assert-table-uuid` and `assert-etag` from `h`.
async fn update(app: &Router, h: &Handle, updates: serde_json::Value) -> axum::response::Response {
    let body = json!({
        "requirements": [
            { "type": "assert-table-uuid", "uuid": h.id },
            { "type": "assert-etag", "etag": h.etag },
        ],
        "updates": updates,
    });
    app.clone()
        .oneshot(common::req_json(
            Method::POST,
            &format!("{TABLES}/{}", h.name),
            body,
        ))
        .await
        .unwrap()
}

async fn updated(app: &Router, h: &mut Handle, updates: serde_json::Value) -> serde_json::Value {
    let resp = update(app, h, updates.clone()).await;
    let body = json_body(assert_status(resp, StatusCode::OK, &updates.to_string()).await).await;
    h.etag = body["metadata"]["etag"].as_str().unwrap().into();
    body
}

async fn update_fails(
    app: &Router,
    h: &Handle,
    updates: serde_json::Value,
    error_type: &str,
    message: &str,
) {
    let status = match error_type {
        "UpdateRequirementConflictException" | "CommitVersionConflictException" => {
            StatusCode::CONFLICT
        }
        "NoSuchTableException" => StatusCode::NOT_FOUND,
        _ => StatusCode::BAD_REQUEST,
    };
    let err = assert_delta_error(update(app, h, updates).await, status, error_type).await;
    let got = err["message"].as_str().unwrap();
    assert!(got.contains(message), "{got:?} should contain {message:?}");
}

fn features(props: &serde_json::Value) -> Vec<String> {
    let mut f: Vec<String> = props
        .as_object()
        .unwrap()
        .keys()
        .filter_map(|k| k.strip_prefix("delta.feature.").map(str::to_string))
        .collect();
    f.sort();
    f
}

const IVE: &str = "InvalidParameterValueException";

#[tokio::test]
async fn update_table_metadata_actions() {
    let app = managed_schema().await;
    // Every metadata action at once.
    let mut h = delta_managed(
        &app,
        "umbrella",
        json!({ "keep": "v1", "update_me": "v_old", "drop_me": "v3" }),
    )
    .await;
    let t0 = h.etag.clone();
    let writers = json!([
        "catalogManaged",
        "v2Checkpoint",
        "vacuumProtocolCheck",
        "deletionVectors",
        "inCommitTimestamp",
        "rowTracking",
        "clustering"
    ]);
    let r1 = updated(&app, &mut h, json!([
        { "action": "set-properties", "updates": { "update_me": "v_new", "added": "v4" } },
        { "action": "remove-properties", "removals": ["drop_me", "missing"] },
        { "action": "set-protocol", "protocol": { "min-reader-version": 3, "min-writer-version": 7,
            "reader-features": ["catalogManaged", "v2Checkpoint", "vacuumProtocolCheck", "deletionVectors"],
            "writer-features": writers } },
        { "action": "set-domain-metadata", "updates": { "delta.clustering": { "clusteringColumns": [["id"]] } } },
        { "action": "set-table-comment", "comment": "umbrella comment" },
    ])).await;
    let props = &r1["metadata"]["properties"];
    assert_eq!(
        (
            props["keep"].clone(),
            props["update_me"].clone(),
            props["added"].clone()
        ),
        (json!("v1"), json!("v_new"), json!("v4"))
    );
    assert!(props.get("drop_me").is_none());
    assert_eq!(
        features(props),
        [
            "catalogManaged",
            "clustering",
            "deletionVectors",
            "inCommitTimestamp",
            "rowTracking",
            "v2Checkpoint",
            "vacuumProtocolCheck"
        ]
    );
    assert_eq!(props["clusteringColumns"], "[[\"id\"]]");
    assert_eq!(props["delta.rowTracking.rowIdHighWaterMark"], "10");
    assert_ne!(h.etag, t0);
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/tables/c.s.umbrella"))
        .await
        .unwrap();
    assert_eq!(json_body(resp).await["comment"], "umbrella comment");

    let r2 = updated(
        &app,
        &mut h,
        json!([{ "action": "remove-domain-metadata", "domains": ["delta.rowTracking"] }]),
    )
    .await;
    assert!(
        r2["metadata"]["properties"]
            .get("delta.rowTracking.rowIdHighWaterMark")
            .is_none()
    );

    // set-protocol that drops a feature a domain still needs.
    let mut h = delta_managed(&app, "drops", json!({})).await;
    let required = json!({ "min-reader-version": 3, "min-writer-version": 7,
        "reader-features": ["catalogManaged", "v2Checkpoint", "vacuumProtocolCheck", "deletionVectors"],
        "writer-features": ["catalogManaged", "v2Checkpoint", "vacuumProtocolCheck", "deletionVectors", "inCommitTimestamp"] });
    update_fails(
        &app,
        &h,
        json!([{ "action": "set-protocol", "protocol": required }]),
        IVE,
        "domain-metadata.delta.rowTracking requires the 'rowTracking' writer feature.",
    )
    .await;
    let r = updated(
        &app,
        &mut h,
        json!([
            { "action": "set-protocol", "protocol": required },
            { "action": "remove-domain-metadata", "domains": ["delta.rowTracking"] },
        ]),
    )
    .await;
    assert!(
        r["metadata"]["properties"]
            .get("delta.feature.rowTracking")
            .is_none()
    );
    update_fails(
        &app,
        &h,
        json!([{ "action": "set-domain-metadata",
        "updates": { "delta.rowTracking": { "rowIdHighWaterMark": 1 } } }]),
        IVE,
        "rowTracking",
    )
    .await;

    // Snapshot versions are for EXTERNAL tables.
    let mut ext = delta_external(&app, "snap").await;
    update_fails(
        &app,
        &ext,
        json!([{ "action": "update-metadata-snapshot-version", "last-commit-timestamp-ms": 1 }]),
        IVE,
        "update-metadata-snapshot-version requires last-commit-version.",
    )
    .await;
    let r = updated(
        &app,
        &mut ext,
        json!([{ "action": "update-metadata-snapshot-version",
        "last-commit-version": 42, "last-commit-timestamp-ms": 1_700_000_000_001i64 }]),
    )
    .await;
    assert_eq!(
        (
            r["metadata"]["last-commit-version"].clone(),
            r["metadata"]["last-commit-timestamp-ms"].clone()
        ),
        (json!(42), json!(1_700_000_000_001i64))
    );
    let managed = delta_managed(&app, "snap_managed", json!({})).await;
    update_fails(
        &app,
        &managed,
        json!([{ "action": "update-metadata-snapshot-version",
        "last-commit-version": 1, "last-commit-timestamp-ms": 1 }]),
        IVE,
        "only supported for EXTERNAL",
    )
    .await;

    // Columns and partition columns.
    let mut cols = delta_external(&app, "cols").await;
    let r = updated(
        &app,
        &mut cols,
        json!([
            { "action": "set-columns", "columns": { "type": "struct", "fields": [
                { "name": "new_id", "type": "long", "nullable": false, "metadata": {} },
                { "name": "flag", "type": "boolean", "nullable": true, "metadata": {} } ] } },
            { "action": "set-partition-columns", "partition-columns": ["flag"] },
        ]),
    )
    .await;
    assert_eq!(r["metadata"]["columns"]["fields"][0]["name"], "new_id");
    assert_eq!(r["metadata"]["partition-columns"], json!(["flag"]));
    let mut part = delta_external(&app, "part").await;
    let r = updated(
        &app,
        &mut part,
        json!([{ "action": "set-partition-columns", "partition-columns": ["id"] }]),
    )
    .await;
    assert_eq!(r["metadata"]["partition-columns"], json!(["id"]));
    // A new schema keeps the partition columns by name, or fails without them.
    let r = updated(
        &app,
        &mut part,
        json!([{ "action": "set-columns", "columns": { "type": "struct", "fields": [
        { "name": "id", "type": "long", "nullable": false, "metadata": {} },
        { "name": "flag", "type": "boolean", "nullable": true, "metadata": {} } ] } }]),
    )
    .await;
    assert_eq!(r["metadata"]["partition-columns"], json!(["id"]));
    update_fails(
        &app,
        &part,
        json!([{ "action": "set-columns", "columns": { "type": "struct", "fields": [
        { "name": "id2", "type": "long", "nullable": false, "metadata": {} } ] } }]),
        IVE,
        "partition-columns references unknown column: id",
    )
    .await;
    for (updates, message) in [
        (
            json!([{ "action": "set-partition-columns", "partition-columns": ["nope"] }]),
            "partition-columns references unknown column: nope",
        ),
        (
            json!([{ "action": "set-columns" }]),
            "set-columns requires a columns block.",
        ),
        (
            json!([{ "action": "set-columns", "columns": { "type": "struct", "fields": [] } }]),
            "set-columns requires at least one column.",
        ),
        (
            json!([{ "action": "set-partition-columns" }]),
            "set-partition-columns requires a partition-columns list.",
        ),
    ] {
        update_fails(&app, &part, updates, IVE, message).await;
    }

    // A stale etag conflicts.
    let mut h = delta_managed(&app, "etag", json!({})).await;
    let stale = Handle {
        name: h.name.clone(),
        id: h.id.clone(),
        etag: h.etag.clone(),
    };
    updated(
        &app,
        &mut h,
        json!([{ "action": "set-properties", "updates": { "k": "v" } }]),
    )
    .await;
    update_fails(
        &app,
        &stale,
        json!([{ "action": "set-properties", "updates": { "x": "y" } }]),
        "UpdateRequirementConflictException",
        "assert-etag failed",
    )
    .await;
}

#[tokio::test]
async fn update_table_rejects_bad_requests() {
    let app = managed_schema().await;
    let h = delta_managed(&app, "rejects", json!({})).await;
    let set_kv = json!({ "action": "set-properties", "updates": { "k": "v" } });
    let post = |name: &str, body: serde_json::Value| {
        app.clone().oneshot(common::req_json(
            Method::POST,
            &format!("{TABLES}/{name}"),
            body,
        ))
    };
    let resp = post(
        "rejects",
        json!({ "requirements": [], "updates": [set_kv] }),
    )
    .await
    .unwrap();
    let err = assert_delta_error(resp, StatusCode::BAD_REQUEST, IVE).await;
    assert_eq!(err["message"], "assert-table-uuid requirement is required.");
    let other = "00000000-0000-0000-0000-00000000000a";
    let resp = post(
        "rejects",
        json!({ "requirements": [{ "type": "assert-table-uuid", "uuid": other }],
        "updates": [set_kv] }),
    )
    .await
    .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::CONFLICT,
        "UpdateRequirementConflictException",
    )
    .await;
    assert_eq!(
        err["message"],
        format!(
            "assert-table-uuid failed: expected {other} but table has {}",
            h.id
        )
    );
    let resp = post(
        "nope",
        json!({ "requirements": [{ "type": "assert-table-uuid", "uuid": other }],
        "updates": [set_kv] }),
    )
    .await
    .unwrap();
    assert_delta_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
    let cases = [
        (json!([]), "At least one update is required."),
        (
            json!([{ "action": "set-properties", "updates": { "a": "1" } }, { "action": "set-properties", "updates": { "b": "2" } }]),
            "At most one set-properties is allowed per request.",
        ),
        (
            json!([set_kv, { "action": "remove-properties", "removals": ["k"] }]),
            "set-properties and remove-properties overlap on keys: [k]",
        ),
        (
            json!([{ "action": "set-domain-metadata", "updates": { "delta.rowTracking": { "rowIdHighWaterMark": 1 } } },
                { "action": "remove-domain-metadata", "domains": ["delta.rowTracking"] }]),
            "set-domain-metadata and remove-domain-metadata overlap on domains: [delta.rowTracking]",
        ),
        (
            json!([{ "action": "set-protocol" }]),
            "set-protocol requires a protocol.",
        ),
        (
            json!([{ "action": "set-domain-metadata" }]),
            "set-domain-metadata requires an updates block.",
        ),
        (
            json!([{ "action": "set-domain-metadata", "updates": {} }]),
            "set-domain-metadata requires at least one domain entry. Supported domains: [delta.clustering, delta.rowTracking]",
        ),
        (
            json!([{ "action": "set-table-comment" }]),
            "set-table-comment requires a comment.",
        ),
        (
            json!([{ "action": "remove-domain-metadata", "domains": ["delta.unknown"] }]),
            "Unknown domain in remove-domain-metadata: delta.unknown",
        ),
        (
            json!([{ "action": "set-protocol", "protocol": { "min-reader-version": 3, "min-writer-version": 7,
            "reader-features": ["v2Checkpoint"], "writer-features": ["v2Checkpoint"] } }]),
            "MANAGED table missing required reader-feature 'catalogManaged'.",
        ),
        (json!([{ "action": "no-such-action" }]), "unknown variant"),
    ];
    for (updates, message) in cases {
        update_fails(&app, &h, updates, IVE, message).await;
    }

    // EXTERNAL tables take any protocol.
    let mut ext = delta_external(&app, "ext_proto").await;
    let r = updated(&app, &mut ext, json!([{ "action": "set-protocol", "protocol": { "min-reader-version": 3,
        "min-writer-version": 7, "reader-features": ["v2Checkpoint"], "writer-features": ["v2Checkpoint"] } }])).await;
    assert_eq!(features(&r["metadata"]["properties"]), ["v2Checkpoint"]);

    // Non-Delta tables are rejected.
    let resp = app
        .clone()
        .oneshot(common::req_json(
            Method::POST,
            "/tables",
            json!({
                "name": "pq", "catalog_name": "c", "schema_name": "s", "table_type": "EXTERNAL",
                "data_source_format": "PARQUET", "storage_location": "s3://lake/pq",
            }),
        ))
        .await
        .unwrap();
    let pq = json_body(assert_status(resp, StatusCode::OK, "parquet").await).await;
    let resp = post(
        "pq",
        json!({ "requirements": [{ "type": "assert-table-uuid", "uuid": pq["table_id"] }],
        "updates": [set_kv] }),
    )
    .await
    .unwrap();
    let err = assert_delta_error(
        resp,
        StatusCode::BAD_REQUEST,
        "UnsupportedTableFormatException",
    )
    .await;
    assert_eq!(err["message"], "Table is not a Delta table: c.s.pq");
}

fn add_commit(version: i64, file: &str) -> serde_json::Value {
    json!({ "action": "add-commit", "commit": { "version": version, "timestamp": 1_700_000_000 + version,
        "file-name": file, "file-size": 1024, "file-modification-timestamp": 1_700_000_000 + version } })
}

#[tokio::test]
async fn update_table_commits() {
    let app = managed_schema().await;
    let mut h = delta_managed(&app, "commits", json!({})).await;
    let t0 = h.etag.clone();
    // A data-only commit leaves the metadata (and the etag) alone.
    let r1 = updated(&app, &mut h, json!([add_commit(1, "00000001.json")])).await;
    assert_eq!(r1["latest-table-version"], 1);
    assert_eq!(r1["commits"][0]["file-name"], "00000001.json");
    assert_eq!(h.etag, t0);
    // A replay is a whole-request no-op.
    let replay = updated(&app, &mut h, json!([add_commit(1, "00000001.json")])).await;
    assert_eq!(replay["commits"].as_array().unwrap().len(), 1);
    update_fails(
        &app,
        &h,
        json!([add_commit(1, "00000001b.json")]),
        "CommitVersionConflictException",
        "Commit version already accepted. Version 1 was accepted with commit file 00000001.json",
    )
    .await;
    update_fails(
        &app,
        &h,
        json!([add_commit(3, "00000003.json")]),
        IVE,
        "Commit version must be the next version after the latest commit 1, but got 3",
    )
    .await;

    // Middle versions replay by name too.
    for v in 2..=3 {
        updated(
            &app,
            &mut h,
            json!([add_commit(v, &format!("{v:08}.json"))]),
        )
        .await;
    }
    let r = updated(&app, &mut h, json!([add_commit(2, "00000002.json")])).await;
    assert_eq!(r["commits"].as_array().unwrap().len(), 3);
    update_fails(
        &app,
        &h,
        json!([add_commit(2, "other.json")]),
        "CommitVersionConflictException",
        "already accepted",
    )
    .await;

    // A metadata-changing commit stamps the snapshot properties.
    let r = updated(
        &app,
        &mut h,
        json!([add_commit(4, "00000004.json"),
        { "action": "set-properties", "updates": { "k": "v" } }]),
    )
    .await;
    assert_eq!(r["metadata"]["last-commit-version"], 4);
    assert_eq!(r["metadata"]["last-commit-timestamp-ms"], 1_700_000_004);
    // A replay ignores a stale etag.
    let stale = Handle {
        name: h.name.clone(),
        id: h.id.clone(),
        etag: t0.clone(),
    };
    let resp = update(
        &app,
        &stale,
        json!([add_commit(4, "00000004.json"),
        { "action": "set-properties", "updates": { "k": "v" } }]),
    )
    .await;
    assert_status(resp, StatusCode::OK, "stale replay").await;

    // Backfill through the update endpoint.
    let r = updated(
        &app,
        &mut h,
        json!([{ "action": "set-latest-backfilled-version", "latest-published-version": 4 }]),
    )
    .await;
    assert_eq!(
        (r["commits"].clone(), r["latest-table-version"].clone()),
        (json!([]), json!(4))
    );
    update_fails(
        &app,
        &h,
        json!([{ "action": "set-latest-backfilled-version" }]),
        IVE,
        "set-latest-backfilled-version requires latest-published-version.",
    )
    .await;
    update_fails(
        &app,
        &h,
        json!([{ "action": "add-commit" }]),
        IVE,
        "add-commit requires a commit block.",
    )
    .await;
    update_fails(
        &app,
        &h,
        json!([add_commit(0, "x.json")]),
        IVE,
        "Field must be positive: version",
    )
    .await;

    // Only MANAGED tables take commits.
    let ext = delta_external(&app, "ext_commit").await;
    update_fails(
        &app,
        &ext,
        json!([add_commit(1, "1.json")]),
        IVE,
        "add-commit and set-latest-backfilled-version require a MANAGED Delta table.",
    )
    .await;
}

fn uniform_iceberg(location: &str, version: i64) -> serde_json::Value {
    json!({ "iceberg": { "metadata-location": location, "converted-delta-version": version,
        "converted-delta-timestamp": 1_700_000_000_000i64 } })
}

/// A UniForm-enabled managed create request.
fn uniform_request(
    name: &str,
    staging: &serde_json::Value,
    uniform: serde_json::Value,
) -> serde_json::Value {
    let mut body = managed_request(name, staging);
    body["properties"]["delta.universalFormat.enabledFormats"] = json!("iceberg");
    body["uniform"] = uniform;
    body
}

#[tokio::test]
async fn uniform_metadata_on_create_and_commit() {
    let app = managed_schema().await;
    let create = |body: serde_json::Value| {
        app.clone()
            .oneshot(common::req_json(Method::POST, TABLES, body))
    };

    let staging = delta_staging(&app, "uni").await;
    let loc = staging["location"].as_str().unwrap().to_string();
    let resp = create(uniform_request(
        "uni",
        &staging,
        uniform_iceberg(&format!("{loc}/_uniform/v1.json"), 0),
    ))
    .await
    .unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "uniform create").await).await;
    assert_eq!(
        created["uniform"],
        json!({ "iceberg": {
        "metadata-location": format!("{loc}/_uniform/v1.json"), "converted-delta-version": 0,
        "converted-delta-timestamp": 1_700_000_000_000i64 } })
    );
    let staging_v3 = delta_staging(&app, "uni_v3").await;
    let loc_v3 = staging_v3["location"].as_str().unwrap().to_string();
    let resp = create(uniform_request(
        "uni_v3",
        &staging_v3,
        uniform_iceberg(&format!("{loc_v3}/m.json"), 1),
    ))
    .await
    .unwrap();
    assert_status(resp, StatusCode::OK, "v3").await;

    // Create-time checks, as upstream.
    let staging = delta_staging(&app, "bad").await;
    let loc = staging["location"].as_str().unwrap().to_string();
    let inside = format!("{loc}/_uniform/v1.json");
    let cases: Vec<(serde_json::Value, String)> = vec![
        ({ let mut b = managed_request("bad", &staging);
           b["properties"]["delta.universalFormat.enabledFormats"] = json!("iceberg"); b },
            "Uniform metadata must be set when 'delta.universalFormat.enabledFormats' includes 'iceberg'.".into()),
        ({ let mut b = managed_request("bad", &staging); b["uniform"] = uniform_iceberg(&inside, 0); b },
            "Uniform metadata must not be set unless 'delta.universalFormat.enabledFormats' includes 'iceberg'.".into()),
        (uniform_request("bad", &staging, json!({})), "uniform.iceberg is required when uniform is set.".into()),
        (uniform_request("bad", &staging, json!({ "iceberg": {} })),
            "uniform.iceberg.metadata-location is required when uniform is set.".into()),
        (uniform_request("bad", &staging, json!({ "iceberg": { "metadata-location": inside,
            "converted-delta-timestamp": 1 } })),
            "uniform.iceberg.converted-delta-version is required when uniform is set.".into()),
        (uniform_request("bad", &staging, json!({ "iceberg": { "metadata-location": inside,
            "converted-delta-version": 0 } })),
            "uniform.iceberg.converted-delta-timestamp is required when uniform is set.".into()),
        (uniform_request("bad", &staging, uniform_iceberg("s3://elsewhere/v1.json", 0)),
            format!("uniform.iceberg.metadata-location ('s3://elsewhere/v1.json') must be a subpath of the table location ('{loc}').")),
        (uniform_request("bad", &staging, uniform_iceberg(&inside, 2)),
            "uniform.iceberg.converted-delta-version must be 0 or 1 at create time, got 2.".into()),
        (uniform_request("bad", &staging, json!({ "iceberg": { "metadata-location": inside,
            "converted-delta-version": 0, "converted-delta-timestamp": 1, "base-converted-delta-version": 0 } })),
            "uniform.iceberg.base-converted-delta-version must not be set at create time: the table has no prior converted-delta-version for it to reference.".into()),
    ];
    for (body, message) in cases {
        let err = assert_delta_error(
            create(body.clone()).await.unwrap(),
            StatusCode::BAD_REQUEST,
            IVE,
        )
        .await;
        assert_eq!(err["message"], message.as_str(), "{body}");
    }

    // Delta API commits carry the conversion of their version.
    let resp = app
        .clone()
        .oneshot(req(Method::GET, &format!("{TABLES}/uni")))
        .await
        .unwrap();
    let mut h = handle("uni", json_body(resp).await);
    let uni_loc = created["metadata"]["location"]
        .as_str()
        .unwrap()
        .to_string();
    let commit = |v: i64, uniform: Option<serde_json::Value>| {
        let mut c = add_commit(v, &format!("{v}.json"));
        if let Some(u) = uniform {
            c["uniform"] = u;
        }
        json!([c])
    };
    update_fails(&app, &h, commit(1, None), IVE,
        "Uniform metadata must be set when 'delta.universalFormat.enabledFormats' includes 'iceberg'.").await;
    update_fails(
        &app,
        &h,
        commit(1, Some(uniform_iceberg(&format!("{uni_loc}/v2.json"), 5))),
        IVE,
        "uniform.iceberg.converted-delta-version (5) must equal commit version (1)",
    )
    .await;
    update_fails(
        &app,
        &h,
        commit(1, Some(uniform_iceberg("s3://elsewhere/v2.json", 1))),
        IVE,
        "must be a subpath of the table location",
    )
    .await;
    let before = h.etag.clone();
    let r = updated(
        &app,
        &mut h,
        commit(1, Some(uniform_iceberg(&format!("{uni_loc}/v2.json"), 1))),
    )
    .await;
    assert_eq!(
        r["uniform"]["iceberg"]["metadata-location"],
        format!("{uni_loc}/v2.json")
    );
    assert_eq!(r["uniform"]["iceberg"]["converted-delta-version"], 1);
    assert_ne!(h.etag, before, "a UniForm commit updates the table");

    // The UC REST commit endpoint takes an ISO-8601 conversion time.
    let rest = |body: serde_json::Value| {
        app.clone().oneshot(common::req_json(
            Method::POST,
            "/delta/preview/commits",
            body,
        ))
    };
    let rest_commit = |v: i64, uniform: Option<serde_json::Value>| {
        let mut c = json!({ "table_id": h.id, "table_uri": uni_loc,
            "commit_info": { "version": v, "timestamp": 1, "file_name": format!("{v}.json"), "file_size": 1,
                "file_modification_timestamp": 1 } });
        if let Some(u) = uniform {
            c["uniform"] = u;
        }
        c
    };
    let rest_uniform = |v: i64, loc: &str| {
        json!({ "iceberg": { "metadata_location": loc,
        "converted_delta_version": v, "converted_delta_timestamp": "2025-01-04T03:13:11.423Z" } })
    };
    for (body, message) in [
        (rest_commit(2, Some(json!({}))), "Field cannot be null in uniform: iceberg".to_string()),
        (rest_commit(2, Some(rest_uniform(3, &format!("{uni_loc}/v3.json")))),
            "uniform.iceberg.converted-delta-version (3) must equal commit version (2)".into()),
        (rest_commit(2, None), "Uniform metadata must be set when 'delta.universalFormat.enabledFormats' includes 'iceberg'.".into()),
    ] {
        let err = common::assert_error(rest(body).await.unwrap(), StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], message.as_str());
    }
    let resp = rest(rest_commit(
        2,
        Some(rest_uniform(2, &format!("{uni_loc}/v3.json"))),
    ))
    .await
    .unwrap();
    assert_status(resp, StatusCode::OK, "rest uniform commit").await;
    let resp = app
        .clone()
        .oneshot(req(Method::GET, &format!("{TABLES}/uni")))
        .await
        .unwrap();
    let loaded = json_body(resp).await;
    assert_eq!(
        loaded["uniform"]["iceberg"]["metadata-location"],
        format!("{uni_loc}/v3.json")
    );
    assert_eq!(
        loaded["uniform"]["iceberg"]["converted-delta-timestamp"],
        1_735_960_391_423i64
    );
}
