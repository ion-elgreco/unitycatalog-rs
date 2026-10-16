mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::{fixture, rstest};
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req_json};

async fn cred_and_location(app: &Router) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({
                "name": "c", "purpose": "STORAGE",
                "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" },
            }),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "loc",
                "url": "s3://bucket/prefix",
                "credential_name": "c",
            }),
        ))
        .await
        .unwrap();
}

/// An app with the credential `c` and the external location `loc`.
#[fixture]
async fn with_cred_and_loc(#[future(awt)] app: Router) -> Router {
    cred_and_location(&app).await;
    app
}

#[rstest]
#[tokio::test]
async fn temp_path_credentials_returns_bound_credential(#[future(awt)] with_cred_and_loc: Router) {
    let resp = with_cred_and_loc
        .oneshot(req_json(
            Method::POST,
            "/temporary-path-credentials",
            json!({
                "url": "s3://bucket/prefix/some/sub/path",
                "operation": "PATH_READ_WRITE",
            }),
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "vend").await).await;

    // The placeholder vendor returns the role ARN as the session token.
    assert_eq!(
        body["aws_temp_credentials"]["session_token"],
        "arn:aws:iam::1:role/x"
    );
    assert!(body["expiration_time"].is_number());
}

#[rstest]
#[tokio::test]
async fn temp_path_credentials_need_a_location_or_bucket_configuration(
    #[future(awt)] with_cred_and_loc: Router,
) {
    // Upstream: no external location, so the bucket's configuration, of
    // which there is none.
    let resp = with_cred_and_loc
        .oneshot(req_json(
            Method::POST,
            "/temporary-path-credentials",
            json!({
                "url": "gs://other-bucket/somewhere",
                "operation": "PATH_READ",
            }),
        ))
        .await
        .unwrap();
    let err = common::assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
    assert_eq!(
        err["message"],
        "Unknown GCS storage configuration for gs://other-bucket."
    );
}

#[rstest]
#[tokio::test]
async fn parent_external_location_resolves_nested_path(#[future(awt)] app: Router) {
    // External locations do not overlap, as upstream; a path resolves to the
    // one location that holds it.
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({
                "name": "broad", "purpose": "STORAGE",
                "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/broad" },
            }),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "broad-loc", "url": "s3://bucket/", "credential_name": "broad",
            }),
        ))
        .await
        .unwrap();

    // Adding a nested EL must be rejected (would overlap with broad-loc).
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "nested",
                "url": "s3://bucket/proj/",
                "credential_name": "broad",
            }),
        ))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;

    // Resolution from a deep path still finds the broad parent.
    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/temporary-path-credentials",
            json!({
                "url": "s3://bucket/proj/file.parquet", "operation": "PATH_READ",
            }),
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "vend").await).await;
    assert_eq!(
        body["aws_temp_credentials"]["session_token"], "arn:aws:iam::1:role/broad",
        "parent external_location should resolve deep paths"
    );
}

async fn vend(app: &Router, path: &str, body: serde_json::Value) -> axum::response::Response {
    app.clone()
        .oneshot(req_json(Method::POST, path, body))
        .await
        .unwrap()
}

async fn vend_error(app: &Router, path: &str, body: serde_json::Value, message: &str) {
    let err = assert_error(
        vend(app, path, body).await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(err["message"], message);
}

/// Upstream `StorageCredentialVendor`: the path comes back in `url`, local
/// paths get empty credentials, bucket roots and empty privileges fail.
#[rstest]
#[tokio::test]
async fn path_credentials_follow_upstream(#[future(awt)] with_cred_and_loc: Router) {
    let app = &with_cred_and_loc;
    let resp = vend(
        app,
        "/temporary-path-credentials",
        json!({ "url": "s3://bucket/prefix//x/", "operation": "PATH_READ" }),
    )
    .await;
    let body = json_body(assert_status(resp, StatusCode::OK, "s3").await).await;
    assert_eq!(body["url"], "s3://bucket/prefix/x");

    let resp = vend(
        app,
        "/temporary-path-credentials",
        json!({ "url": "file:/tmp/uc-local", "operation": "PATH_READ_WRITE" }),
    )
    .await;
    let body = json_body(assert_status(resp, StatusCode::OK, "file").await).await;
    assert_eq!(body, json!({ "url": "file:///tmp/uc-local" }));

    vend_error(app, "/temporary-path-credentials", json!({ "url": "s3://bucket/", "operation": "PATH_READ" }),
        "Storage location must include a non-empty path prefix before credentials can be vended: s3://bucket").await;
    vend_error(
        app,
        "/temporary-path-credentials",
        json!({ "url": "s3://bucket/prefix/x", "operation": "UNKNOWN_PATH_OPERATION" }),
        "Privileges cannot be null or empty.",
    )
    .await;
}

#[tokio::test]
async fn table_volume_and_model_version_credentials() {
    let (app, _) = common::app_with_metastore().await;
    cred_and_location(&app).await;
    let root = common::temp_storage_root();
    for (path, body) in [
        (
            "/catalogs",
            json!({ "name": "local", "storage_root": root }),
        ),
        ("/schemas", json!({ "name": "s", "catalog_name": "local" })),
        (
            "/catalogs",
            json!({ "name": "cloud", "storage_root": "s3://bucket/prefix/root" }),
        ),
        ("/schemas", json!({ "name": "s", "catalog_name": "cloud" })),
    ] {
        assert_status(vend(&app, path, body).await, StatusCode::OK, path).await;
    }

    // Tables: the id must be a UUID; a local managed table gets its url.
    vend_error(
        &app,
        "/temporary-table-credentials",
        json!({ "table_id": "x", "operation": "READ" }),
        "Invalid UUID string: x",
    )
    .await;
    let resp = vend(
        &app,
        "/staging-tables",
        json!({ "name": "t", "catalog_name": "local", "schema_name": "s" }),
    )
    .await;
    let staging = json_body(assert_status(resp, StatusCode::OK, "staging").await).await;
    let resp = vend(
        &app,
        "/temporary-table-credentials",
        json!({ "table_id": staging["id"], "operation": "READ_WRITE" }),
    )
    .await;
    let body = json_body(assert_status(resp, StatusCode::OK, "table").await).await;
    assert_eq!(body["url"], staging["staging_location"]);
    vend_error(
        &app,
        "/temporary-table-credentials",
        json!({ "table_id": staging["id"], "operation": "UNKNOWN_TABLE_OPERATION" }),
        "Privileges cannot be null or empty.",
    )
    .await;

    // Volumes: the id is required; a MANAGED volume gets its url.
    vend_error(
        &app,
        "/temporary-volume-credentials",
        json!({ "volume_id": "", "operation": "READ_VOLUME" }),
        "Volume ID is required.",
    )
    .await;
    let resp = vend(
        &app,
        "/volumes",
        json!({
            "name": "v", "catalog_name": "local", "schema_name": "s", "volume_type": "MANAGED",
        }),
    )
    .await;
    let volume = json_body(assert_status(resp, StatusCode::OK, "volume").await).await;
    let resp = vend(
        &app,
        "/temporary-volume-credentials",
        json!({ "volume_id": volume["volume_id"], "operation": "WRITE_VOLUME" }),
    )
    .await;
    let body = json_body(assert_status(resp, StatusCode::OK, "volume creds").await).await;
    assert_eq!(body["url"], volume["storage_location"]);

    // Model versions: never for local storage; read-write only while pending.
    let version = |catalog: &str| {
        json!({
            "model_name": "m", "catalog_name": catalog, "schema_name": "s", "source": "s3://run",
        })
    };
    let creds = |catalog: &str, op: &str| {
        json!({
            "catalog_name": catalog, "schema_name": "s", "model_name": "m", "version": 1, "operation": op,
        })
    };
    for catalog in ["local", "cloud"] {
        let body = json!({ "name": "m", "catalog_name": catalog, "schema_name": "s" });
        assert_status(vend(&app, "/models", body).await, StatusCode::OK, "model").await;
        assert_status(
            vend(&app, "/models/versions", version(catalog)).await,
            StatusCode::OK,
            "version",
        )
        .await;
    }
    vend_error(&app, "/temporary-model-version-credentials", creds("local", "READ_MODEL_VERSION"),
        "Cannot request credentials on a model version with a file based storage location: local.s.m/1").await;
    let resp = vend(
        &app,
        "/temporary-model-version-credentials",
        creds("cloud", "READ_WRITE_MODEL_VERSION"),
    )
    .await;
    let body = json_body(assert_status(resp, StatusCode::OK, "pending rw").await).await;
    assert!(
        body["url"]
            .as_str()
            .unwrap()
            .starts_with("s3://bucket/prefix/root/__unitystorage/")
    );
    vend_error(
        &app,
        "/temporary-model-version-credentials",
        creds("cloud", "UNKNOWN_MODEL_VERSION_OPERATION"),
        "Unknown operation in the request: UNKNOWN_MODEL_VERSION_OPERATION",
    )
    .await;

    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/models/cloud.s.m/versions/1/finalize",
            json!({ "full_name": "cloud.s.m", "version": 1 }),
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "finalize").await;
    vend_error(&app, "/temporary-model-version-credentials", creds("cloud", "READ_WRITE_MODEL_VERSION"),
        "Cannot request read/write credentials on a model version that has been finalized: cloud.s.m/1").await;
    let resp = vend(
        &app,
        "/temporary-model-version-credentials",
        creds("cloud", "READ_MODEL_VERSION"),
    )
    .await;
    assert_status(resp, StatusCode::OK, "finalized read").await;
}
