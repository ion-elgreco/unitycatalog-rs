mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_status, json_body, req, req_json, seeded};

#[rstest]
#[tokio::test]
async fn create_get_list_update_delete_volume(#[future(awt)] seeded: Router) {
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/volumes",
            json!({
                "catalog_name": "main", "schema_name": "default",
                "name": "vol1", "volume_type": "MANAGED",
                "comment": "scratch",
            }),
        ))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "create vol").await;
    let info = json_body(resp).await;
    assert_eq!(info["name"], "vol1");
    assert_eq!(info["full_name"], "main.default.vol1");
    assert_eq!(info["volume_type"], "MANAGED");
    assert!(info["volume_id"].is_string());

    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/volumes/main.default.vol1"))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "get").await;

    let resp = seeded
        .clone()
        .oneshot(req(
            Method::GET,
            "/volumes?catalog_name=main&schema_name=default",
        ))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    assert_eq!(list["volumes"].as_array().unwrap().len(), 1);

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/volumes/main.default.vol1",
            json!({ "comment": "edited" }),
        ))
        .await
        .unwrap();
    let updated = json_body(assert_status(resp, StatusCode::OK, "patch").await).await;
    assert_eq!(updated["comment"], "edited");

    let resp = seeded
        .clone()
        .oneshot(req(Method::DELETE, "/volumes/main.default.vol1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn external_volume_requires_storage_location(#[future(awt)] seeded: Router) {
    let resp = seeded
        .oneshot(req_json(
            Method::POST,
            "/volumes",
            json!({
                "catalog_name": "main", "schema_name": "default",
                "name": "ext", "volume_type": "EXTERNAL",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn volume_under_missing_schema_returns_404(#[future(awt)] app: Router) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "main" }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/volumes",
            json!({
                "catalog_name": "main", "schema_name": "ghost",
                "name": "x", "volume_type": "MANAGED",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
