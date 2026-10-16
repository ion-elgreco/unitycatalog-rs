//! Managed storage layout, per upstream `ExternalLocationUtils` and
//! `NormalizedURL`: storage roots are normalized, and catalogs and schemas
//! with a root get `<root>/__unitystorage/{catalogs,schemas}/<id>`.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req_json};

async fn post(app: &Router, path: &str, body: Value) -> axum::response::Response {
    app.clone()
        .oneshot(req_json(Method::POST, path, body))
        .await
        .unwrap()
}

#[rstest]
#[tokio::test]
async fn storage_roots_get_managed_locations(#[future(awt)] app: Router) {
    let resp = post(
        &app,
        "/catalogs",
        json!({ "name": "c", "storage_root": "s3://bucket/root/" }),
    )
    .await;
    let catalog = json_body(assert_status(resp, StatusCode::OK, "catalog").await).await;
    assert_eq!(catalog["storage_root"], "s3://bucket/root");
    assert_eq!(
        catalog["storage_location"],
        format!(
            "s3://bucket/root/__unitystorage/catalogs/{}",
            catalog["id"].as_str().unwrap()
        )
    );

    let resp = post(
        &app,
        "/schemas",
        json!({ "name": "s", "catalog_name": "c", "storage_root": "file:/tmp/uc//schemas/" }),
    )
    .await;
    let schema = json_body(assert_status(resp, StatusCode::OK, "schema").await).await;
    assert_eq!(schema["storage_root"], "file:///tmp/uc/schemas");
    assert_eq!(
        schema["storage_location"],
        format!(
            "file:///tmp/uc/schemas/__unitystorage/schemas/{}",
            schema["schema_id"].as_str().unwrap()
        )
    );

    // No root, no managed location.
    let resp = post(&app, "/catalogs", json!({ "name": "plain" })).await;
    let plain = json_body(assert_status(resp, StatusCode::OK, "plain").await).await;
    assert!(plain.get("storage_location").is_none_or(Value::is_null));
}

#[rstest]
#[tokio::test]
async fn bad_storage_roots_are_rejected(#[future(awt)] app: Router) {
    let cases = [
        ("ftp://host/x", "Unsupported URI scheme: ftp"),
        ("bucket", "Ambiguous path: bucket"),
        (
            "s3://bucket/__unitystorage/x",
            "Input path 's3://bucket/__unitystorage/x' contains managed storage prefix __unitystorage.",
        ),
    ];
    for (root, message) in cases {
        let resp = post(
            &app,
            "/catalogs",
            json!({ "name": "c", "storage_root": root }),
        )
        .await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], message, "{root}");
    }
}

async fn ok_json(app: &Router, method: Method, path: &str, body: Option<Value>) -> Value {
    let request = match body {
        Some(b) => req_json(method.clone(), path, b),
        None => common::req(method.clone(), path),
    };
    let resp = app.clone().oneshot(request).await.unwrap();
    json_body(assert_status(resp, StatusCode::OK, &format!("{method} {path}")).await).await
}

fn local_path(url: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(url.strip_prefix("file://").unwrap())
}

#[rstest]
#[tokio::test]
async fn managed_volumes_and_models_get_locations(#[future(awt)] app: Router) {
    let root = common::temp_storage_root();
    let catalog = ok_json(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c", "storage_root": root })),
    )
    .await;
    let catalog_loc = catalog["storage_location"].as_str().unwrap().to_string();
    ok_json(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "c" })),
    )
    .await;

    // Managed volume: `<catalog location>/volumes/<id>`.
    let vol = ok_json(
        &app,
        Method::POST,
        "/volumes",
        Some(json!({
            "name": "v", "catalog_name": "c", "schema_name": "s", "volume_type": "MANAGED",
        })),
    )
    .await;
    assert_eq!(
        vol["storage_location"],
        format!(
            "{catalog_loc}/volumes/{}",
            vol["volume_id"].as_str().unwrap()
        )
    );

    // Model: `<catalog location>/models/<id>`, and the directory exists.
    let model = ok_json(
        &app,
        Method::POST,
        "/models",
        Some(json!({ "name": "m", "catalog_name": "c", "schema_name": "s" })),
    )
    .await;
    let model_loc = model["storage_location"].as_str().unwrap().to_string();
    assert_eq!(
        model_loc,
        format!("{catalog_loc}/models/{}", model["id"].as_str().unwrap())
    );
    assert!(local_path(&model_loc).is_dir());

    // Version: `<model location>/versions/<id>`, directory created.
    let version_body = json!({
        "model_name": "m", "catalog_name": "c", "schema_name": "s", "source": "s3://run",
    });
    let v1 = ok_json(
        &app,
        Method::POST,
        "/models/versions",
        Some(version_body.clone()),
    )
    .await;
    let v1_loc = v1["storage_location"].as_str().unwrap();
    assert_eq!(
        v1_loc,
        format!("{model_loc}/versions/{}", v1["id"].as_str().unwrap())
    );
    assert!(local_path(v1_loc).is_dir());

    // Version numbers are never reused.
    let v2 = ok_json(
        &app,
        Method::POST,
        "/models/versions",
        Some(version_body.clone()),
    )
    .await;
    assert_eq!(v2["version"], 2);
    let resp = app
        .clone()
        .oneshot(common::req(Method::DELETE, "/models/c.s.m/versions/2"))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "delete v2").await;
    let v3 = ok_json(&app, Method::POST, "/models/versions", Some(version_body)).await;
    assert_eq!(v3["version"], 3);
}

#[rstest]
#[tokio::test]
async fn volume_location_rules(#[future(awt)] app: Router) {
    ok_json(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c", "storage_root": "s3://bucket/root" })),
    )
    .await;
    ok_json(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "c" })),
    )
    .await;
    ok_json(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "bare" })),
    )
    .await;
    ok_json(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "bare" })),
    )
    .await;

    let volume = |catalog: &str, kind: &str, location: Option<&str>| {
        let mut body = json!({
            "name": "v", "catalog_name": catalog, "schema_name": "s", "volume_type": kind,
        });
        if let Some(l) = location {
            body["storage_location"] = json!(l);
        }
        body
    };
    let cases = [
        (
            volume("c", "MANAGED", Some("s3://bucket/x")),
            "INVALID_ARGUMENT",
            "Storage location should not be specified for managed volume",
        ),
        (
            volume("bare", "MANAGED", None),
            "FAILED_PRECONDITION",
            "None of catalog or schema has managed location configured.",
        ),
        (
            volume("c", "EXTERNAL", None),
            "INVALID_ARGUMENT",
            "Storage location is required for external volume",
        ),
        (
            volume("c", "EXTERNAL", Some("s3://bucket/")),
            "INVALID_ARGUMENT",
            "External volume storage location must include a non-empty path prefix: s3://bucket/",
        ),
        (
            volume("c", "EXTERNAL", Some("s3://bucket/root")),
            "INVALID_ARGUMENT",
            "Input path 's3://bucket/root' overlaps with managed storage.",
        ),
    ];
    for (body, code, message) in cases {
        let resp = post(&app, "/volumes", body.clone()).await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, code).await;
        assert_eq!(err["message"], message, "{body}");
    }

    let vol = ok_json(
        &app,
        Method::POST,
        "/volumes",
        Some(volume("c", "EXTERNAL", Some("s3://other//vol/"))),
    )
    .await;
    assert_eq!(vol["storage_location"], "s3://other/vol");
}

/// Without a catalog or schema location, a managed table or model names the
/// server's setting that would place it.
#[rstest]
#[tokio::test]
async fn managed_objects_without_a_location_name_the_setting(#[future(awt)] app: Router) {
    ok_json(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "bare" })),
    )
    .await;
    ok_json(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "bare" })),
    )
    .await;
    let names = json!({ "name": "x", "catalog_name": "bare", "schema_name": "s" });
    for (path, setting) in [
        ("/staging-tables", "storage.tables_root"),
        ("/models", "storage.models_root"),
    ] {
        let resp = post(&app, path, names.clone()).await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
        assert_eq!(
            err["message"],
            format!(
                "None of catalog, schema or {setting} server setting has managed location \
                 configured."
            ),
            "{path}"
        );
    }
}

#[tokio::test]
async fn models_fall_back_to_the_models_storage_root() {
    use uc_metastore::{ManagedStorageConfig, MetastoreConfig, SqlMetastore};
    let root = common::temp_storage_root();
    let config = MetastoreConfig::for_tests().with_storage(ManagedStorageConfig {
        models_root: Some(uc_common::NormalizedUrl::parse(&root).unwrap()),
        ..Default::default()
    });
    let metastore = SqlMetastore::connect(config).await.unwrap();
    let app = uc_server::build_app(uc_server::AppState::new(metastore));

    ok_json(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c" })),
    )
    .await;
    ok_json(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "c" })),
    )
    .await;
    let model = ok_json(
        &app,
        Method::POST,
        "/models",
        Some(json!({ "name": "m", "catalog_name": "c", "schema_name": "s" })),
    )
    .await;
    assert_eq!(
        model["storage_location"],
        format!(
            "{root}/__unitystorage/models/{}",
            model["id"].as_str().unwrap()
        )
    );
}
