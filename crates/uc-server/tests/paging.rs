//! List paging, per upstream `PagedListingHelper`: page size is
//! `min(max_results, 100)` (0 or absent = 100), a negative value is an
//! `INVALID_ARGUMENT`, results are ordered by name, and a full page carries
//! the last name as `next_page_token`. Model versions page by version.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{assert_error, assert_status, json_body, req, req_json, seeded};

async fn get(app: &Router, path: &str) -> Value {
    let resp = app.clone().oneshot(req(Method::GET, path)).await.unwrap();
    json_body(assert_status(resp, StatusCode::OK, path).await).await
}

async fn post(app: &Router, path: &str, body: Value) {
    let resp = app
        .clone()
        .oneshot(req_json(Method::POST, path, body))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, path).await;
}

fn names(list: &Value, key: &str) -> Vec<String> {
    list[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap().to_string())
        .collect()
}

#[rstest]
#[tokio::test]
async fn schemas_page_by_name(#[future(awt)] seeded: Router) {
    for s in ["b", "a"] {
        post(
            &seeded,
            "/schemas",
            json!({ "name": s, "catalog_name": "main" }),
        )
        .await;
    }
    // main has: a, b, default.
    let page1 = get(&seeded, "/schemas?catalog_name=main&max_results=2").await;
    assert_eq!(names(&page1, "schemas"), ["a", "b"]);
    assert_eq!(page1["next_page_token"], "b");

    let page2 = get(
        &seeded,
        "/schemas?catalog_name=main&max_results=2&page_token=b",
    )
    .await;
    assert_eq!(names(&page2, "schemas"), ["default"]);
    assert!(
        page2.get("next_page_token").is_none_or(Value::is_null),
        "{page2}"
    );

    // 0 means the default page size.
    let all = get(&seeded, "/schemas?catalog_name=main&max_results=0").await;
    assert_eq!(names(&all, "schemas"), ["a", "b", "default"]);

    // A full last page still yields a token; the following page is empty.
    let full = get(&seeded, "/schemas?catalog_name=main&max_results=3").await;
    assert_eq!(full["next_page_token"], "default");
    let empty = get(
        &seeded,
        "/schemas?catalog_name=main&max_results=3&page_token=default",
    )
    .await;
    assert!(empty["schemas"].as_array().unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn negative_max_results_is_invalid(#[future(awt)] seeded: Router) {
    for path in [
        "/catalogs?max_results=-1",
        "/schemas?catalog_name=main&max_results=-1",
    ] {
        let resp = seeded
            .clone()
            .oneshot(req(Method::GET, path))
            .await
            .unwrap();
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(
            err["message"],
            "maxResults must be greater than or equal to 0"
        );
    }
}

#[rstest]
#[tokio::test]
async fn max_results_works_on_every_list(#[future(awt)] seeded: Router) {
    for path in [
        "/catalogs?max_results=1",
        "/schemas?catalog_name=main&max_results=1",
        "/tables?catalog_name=main&schema_name=default&max_results=1",
        "/volumes?catalog_name=main&schema_name=default&max_results=1",
        "/functions?catalog_name=main&schema_name=default&max_results=1",
        "/models?catalog_name=main&schema_name=default&max_results=1",
        "/credentials?max_results=1",
        "/external-locations?max_results=1",
    ] {
        get(&seeded, path).await;
    }
}

#[rstest]
#[tokio::test]
async fn model_versions_page_by_version_number(#[future(awt)] seeded: Router) {
    post(
        &seeded,
        "/models",
        json!({ "name": "m", "catalog_name": "main", "schema_name": "default" }),
    )
    .await;
    for _ in 0..12 {
        post(
            &seeded,
            "/models/versions",
            json!({
                "model_name": "m", "catalog_name": "main", "schema_name": "default",
                "source": "s3://bucket/run",
            }),
        )
        .await;
    }
    let versions = |v: &Value| -> Vec<i64> {
        v["model_versions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["version"].as_i64().unwrap())
            .collect()
    };
    // Version 10 sorts after 9 (numeric, not text, order).
    let page1 = get(&seeded, "/models/main.default.m/versions?max_results=9").await;
    assert_eq!(versions(&page1), (1..=9).collect::<Vec<_>>());
    assert_eq!(page1["next_page_token"], "9");
    let page2 = get(
        &seeded,
        "/models/main.default.m/versions?max_results=9&page_token=9",
    )
    .await;
    assert_eq!(versions(&page2), [10, 11, 12]);

    let resp = seeded
        .clone()
        .oneshot(req(
            Method::GET,
            "/models/main.default.m/versions?page_token=abc",
        ))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Invalid page token received: abc");
}
