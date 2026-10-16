mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::{fixture, rstest};
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_status, json_body, req, req_json};

/// An `app` with the catalog `main`, and no schema in it.
#[fixture]
async fn with_catalog(#[future(awt)] app: Router) -> Router {
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "main" }),
        ))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "make_catalog").await;
    app
}

#[rstest]
#[tokio::test]
async fn create_schema_under_existing_catalog(#[future(awt)] with_catalog: Router) {
    let resp = with_catalog
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({
                "name": "default",
                "catalog_name": "main",
                "comment": "default schema",
            }),
        ))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "create schema").await;
    let info = json_body(resp).await;
    assert_eq!(info["name"], "default");
    assert_eq!(info["catalog_name"], "main");
    assert_eq!(info["full_name"], "main.default");
    assert!(info["schema_id"].is_string());
}

#[rstest]
#[tokio::test]
async fn create_schema_under_missing_catalog_returns_404(#[future(awt)] app: Router) {
    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({
                "name": "x", "catalog_name": "does_not_exist",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn get_update_delete_schema_by_full_name(#[future(awt)] with_catalog: Router) {
    with_catalog
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({
                "name": "s1", "catalog_name": "main",
            }),
        ))
        .await
        .unwrap();

    let resp = with_catalog
        .clone()
        .oneshot(req(Method::GET, "/schemas/main.s1"))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "get").await;
    let got = json_body(resp).await;
    assert_eq!(got["full_name"], "main.s1");

    let resp = with_catalog
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/schemas/main.s1",
            json!({ "comment": "hi" }),
        ))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "patch").await;
    assert_eq!(json_body(resp).await["comment"], "hi");

    let resp = with_catalog
        .oneshot(req(Method::DELETE, "/schemas/main.s1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn list_schemas_requires_catalog_name(#[future(awt)] with_catalog: Router) {
    with_catalog
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({
                "name": "s1", "catalog_name": "main",
            }),
        ))
        .await
        .unwrap();
    with_catalog
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({
                "name": "s2", "catalog_name": "main",
            }),
        ))
        .await
        .unwrap();

    let resp = with_catalog
        .clone()
        .oneshot(req(Method::GET, "/schemas?catalog_name=main"))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "list").await;
    let list = json_body(resp).await;
    assert_eq!(list["schemas"].as_array().unwrap().len(), 2);

    let resp = with_catalog
        .oneshot(req(Method::GET, "/schemas"))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "missing catalog_name should be a 400"
    );
}

#[rstest]
#[tokio::test]
async fn rename_schema_moves_under_new_full_name(#[future(awt)] with_catalog: Router) {
    with_catalog
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({
                "name": "old", "catalog_name": "main",
            }),
        ))
        .await
        .unwrap();

    let resp = with_catalog
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/schemas/main.old",
            json!({ "new_name": "new" }),
        ))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "rename").await;

    let resp = with_catalog
        .clone()
        .oneshot(req(Method::GET, "/schemas/main.new"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = with_catalog
        .oneshot(req(Method::GET, "/schemas/main.old"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
