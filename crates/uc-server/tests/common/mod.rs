//! Each integration test file compiles as its own binary, so a helper used
//! by only some tests looks "unused" to clippy in the others. Allow that.
#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
};
use http_body_util::BodyExt;
use rstest::fixture;
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{AppState, build_app};

pub const BASE: &str = "/api/2.1/unity-catalog";

/// rstest fixture for an app with a new test database. Use as
/// `#[rstest] #[tokio::test] async fn t(#[future(awt)] app: Router) { ... }`.
#[fixture]
pub async fn app() -> Router {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database");
    build_app(AppState::new(metastore).with_security(security()))
}

/// The server signing key, made once per test binary.
pub fn security() -> std::sync::Arc<uc_server::authn::security::SecurityContext> {
    static SECURITY: std::sync::OnceLock<
        std::sync::Arc<uc_server::authn::security::SecurityContext>,
    > = std::sync::OnceLock::new();
    SECURITY
        .get_or_init(|| {
            std::sync::Arc::new(uc_server::authn::security::SecurityContext::generate().unwrap())
        })
        .clone()
}

/// A token of the bootstrap `admin`, as `uc-server token` prints it.
pub fn admin_token() -> String {
    security()
        .create_access_token(
            uc_server::authn::security::ADMIN,
            std::time::Duration::from_secs(3600),
        )
        .unwrap()
}

/// An app with authorization on (sign-in required), and its metastore.
pub async fn signed_in_app() -> (Router, uc_metastore::DynMetastore) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database");
    let dyn_metastore: uc_metastore::DynMetastore = metastore.clone();
    (
        build_app(
            AppState::new(metastore)
                .with_authz(true)
                .with_security(security()),
        ),
        dyn_metastore,
    )
}

/// Add an active SCIM user `email` straight to the metastore and return a
/// UC access token for it.
pub async fn active_user(metastore: &uc_metastore::DynMetastore, email: &str) -> String {
    metastore
        .scim()
        .create_user(uc_api::ScimUser {
            user_name: email.to_string(),
            emails: Some(vec![uc_api::ScimEmail {
                value: email.to_string(),
                primary: Some(true),
                r#type: None,
            }]),
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    security()
        .create_access_token(email, std::time::Duration::from_secs(600))
        .unwrap()
}

/// An app with a new test database, and its metastore, for tests that
/// check state the API does not show.
pub async fn app_with_metastore() -> (Router, uc_metastore::DynMetastore) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database");
    let dyn_metastore: uc_metastore::DynMetastore = metastore.clone();
    (
        build_app(AppState::new(metastore).with_security(security())),
        dyn_metastore,
    )
}

/// `request` signed in as the bootstrap admin (for `/admin/*`).
pub fn as_admin(mut request: Request<Body>) -> Request<Body> {
    let token = admin_token();
    request
        .headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    request
}

/// Create a SCIM user named `name` (also its email). Grants need one.
pub async fn create_user(app: &Router, name: &str) {
    let resp = app
        .clone()
        .oneshot(control_json(
            Method::POST,
            "/scim2/Users",
            json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "userName": name,
                "emails": [{ "value": name, "primary": true }],
                "active": true,
            }),
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::CREATED, "create user").await;
}

/// rstest fixture for an [`app`] with the schema `main.default`, the parent
/// of the tables, volumes, functions and models of a test.
#[fixture]
pub async fn seeded(#[future(awt)] app: Router) -> Router {
    seed_main_default(&app).await;
    app
}

pub async fn json_body(resp: Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("body is json")
}

pub async fn assert_status(resp: Response, expected: StatusCode, ctx: &str) -> Response {
    let status = resp.status();
    if status == expected {
        return resp;
    }
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&bytes);
    panic!("[{ctx}] expected {expected}, got {status}. body: {body}");
}

/// Assert an upstream-shaped error: HTTP status, `error_code`, and the
/// `google.rpc.ErrorInfo` entry in `details`. Returns the body.
pub async fn assert_error(resp: Response, status: StatusCode, code: &str) -> Value {
    let resp = assert_status(resp, status, code).await;
    let body = json_body(resp).await;
    assert_eq!(body["error_code"], code, "error_code in {body}");
    assert_eq!(body["details"][0]["reason"], code, "details in {body}");
    assert_eq!(
        body["details"][0]["@type"], "type.googleapis.com/google.rpc.ErrorInfo",
        "details in {body}"
    );
    body
}

/// Assert a UC Delta API error (`{"error": {message, type, code}}`).
/// Returns the `error` object.
pub async fn assert_delta_error(resp: Response, status: StatusCode, error_type: &str) -> Value {
    let resp = assert_status(resp, status, error_type).await;
    let body = json_body(resp).await;
    assert_eq!(body["error"]["type"], error_type, "type in {body}");
    assert_eq!(body["error"]["code"], status.as_u16(), "code in {body}");
    assert_eq!(body.as_object().unwrap().len(), 1, "only `error` in {body}");
    body["error"].clone()
}

/// Assert an Iceberg REST error, the same body as the Delta API's.
/// Returns the `error` object.
pub async fn assert_iceberg_error(resp: Response, status: StatusCode, error_type: &str) -> Value {
    assert_delta_error(resp, status, error_type).await
}

/// Upstream's control-plane path.
pub const CONTROL: &str = "/api/1.0/unity-control";

/// A JSON request to `path` under the control plane.
pub fn control_json(method: Method, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{CONTROL}{path}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// A request to `path` under the control plane.
pub fn control(method: Method, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{CONTROL}{path}"))
        .body(Body::empty())
        .unwrap()
}

pub fn req_json(method: Method, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{BASE}{path}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn req(method: Method, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{BASE}{path}"))
        .body(Body::empty())
        .unwrap()
}

pub fn req_bearer(method: Method, path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{BASE}{path}"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// A fresh local directory to use as a storage root. Kept after the test
/// (the OS cleans up its temp dir).
pub fn temp_storage_root() -> String {
    let dir = tempfile::Builder::new()
        .prefix("uc-test-")
        .tempdir()
        .unwrap()
        .keep();
    format!("file://{}", dir.display())
}

/// Seed `main.default` so resource tests have a parent schema to attach to.
/// `main` has a local storage root, so managed volumes, tables and models
/// get a location (upstream requires one).
pub async fn seed_main_default(app: &axum::Router) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "main", "storage_root": temp_storage_root() }),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({ "name": "default", "catalog_name": "main" }),
        ))
        .await
        .unwrap();
}
