//! The catalog API, through the whole app and a test database.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req, req_json};

#[rstest]
#[tokio::test]
async fn create_get_list_update_delete_catalog(#[future(awt)] app: Router) {
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({
                "name": "main",
                "comment": "primary catalog",
                "storage_root": "s3://bucket/ucroot",
            }),
        ))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "create main").await;
    let created = json_body(resp).await;
    assert_eq!(created["name"], "main");
    assert_eq!(created["comment"], "primary catalog");
    assert_eq!(created["storage_root"], "s3://bucket/ucroot");
    assert!(created["created_at"].is_number());
    assert!(created["id"].is_string());

    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "main" }),
        ))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "CATALOG_ALREADY_EXISTS").await;
    assert_eq!(err["message"], "Catalog already exists: main");

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/catalogs/main"))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "get main").await;
    let got = json_body(resp).await;
    assert_eq!(got["name"], "main");

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/catalogs/nope"))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::NOT_FOUND, "CATALOG_NOT_FOUND").await;
    assert_eq!(err["message"], "Catalog not found: nope");

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/catalogs"))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "list").await;
    let list = json_body(resp).await;
    assert_eq!(list["catalogs"].as_array().unwrap().len(), 1);

    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/catalogs/main",
            json!({ "comment": "edited" }),
        ))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "patch").await;
    let updated = json_body(resp).await;
    assert_eq!(updated["comment"], "edited");

    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, "/catalogs/main"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(req(Method::GET, "/catalogs/main"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn rename_catalog_moves_under_new_key(#[future(awt)] app: Router) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "old" }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/catalogs/old",
            json!({ "new_name": "new" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/catalogs/old"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = app
        .oneshot(req(Method::GET, "/catalogs/new"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn pagination_returns_next_token_when_more_pages_exist(#[future(awt)] app: Router) {
    for i in 0..5 {
        let resp = app
            .clone()
            .oneshot(req_json(
                Method::POST,
                "/catalogs",
                json!({ "name": format!("cat_{i}") }),
            ))
            .await
            .unwrap();
        let _ = assert_status(resp, StatusCode::OK, &format!("create cat_{i}")).await;
    }

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/catalogs?max_results=2"))
        .await
        .unwrap();
    let page1 = json_body(resp).await;
    assert_eq!(page1["catalogs"].as_array().unwrap().len(), 2);
    assert!(page1["next_page_token"].is_string());

    let token = page1["next_page_token"].as_str().unwrap();
    let resp = app
        .oneshot(req(
            Method::GET,
            &format!("/catalogs?max_results=2&page_token={token}"),
        ))
        .await
        .unwrap();
    let page2 = json_body(resp).await;
    assert_eq!(page2["catalogs"].as_array().unwrap().len(), 2);
}

#[rstest]
#[tokio::test]
async fn cascade_delete_catalog_requires_force_when_schemas_exist(#[future(awt)] app: Router) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "cat" }),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/schemas",
            json!({ "name": "s1", "catalog_name": "cat" }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, "/catalogs/cat"))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;

    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, "/catalogs/cat?force=true"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(req(Method::GET, "/schemas/cat.s1"))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "child schema should be gone"
    );
}
