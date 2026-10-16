mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{assert_error, assert_status, json_body, req, req_json, seeded};

#[rstest]
#[tokio::test]
async fn registered_model_crud(#[future(awt)] seeded: Router) {
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/models",
            json!({
                "name": "m1", "catalog_name": "main", "schema_name": "default",
                "comment": "first model",
            }),
        ))
        .await
        .unwrap();
    let info = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(info["full_name"], "main.default.m1");

    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/models/main.default.m1"))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "get").await;

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/models/main.default.m1",
            json!({ "comment": "updated" }),
        ))
        .await
        .unwrap();
    let updated = json_body(assert_status(resp, StatusCode::OK, "patch").await).await;
    assert_eq!(updated["comment"], "updated");

    let resp = seeded
        .oneshot(req(Method::DELETE, "/models/main.default.m1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn model_versions_auto_increment_and_finalize(#[future(awt)] seeded: Router) {
    seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/models",
            json!({
                "name": "m1", "catalog_name": "main", "schema_name": "default",
            }),
        ))
        .await
        .unwrap();

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/models/versions",
            json!({
                "model_name": "m1", "catalog_name": "main", "schema_name": "default",
                "source": "s3://bucket/m1/v1", "run_id": "abc",
            }),
        ))
        .await
        .unwrap();
    let v1 = json_body(assert_status(resp, StatusCode::OK, "create v1").await).await;
    assert_eq!(v1["version"], 1);
    assert_eq!(v1["status"], "PENDING_REGISTRATION");

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/models/versions",
            json!({
                "model_name": "m1", "catalog_name": "main", "schema_name": "default",
                "source": "s3://bucket/m1/v2",
            }),
        ))
        .await
        .unwrap();
    let v2 = json_body(assert_status(resp, StatusCode::OK, "create v2").await).await;
    assert_eq!(v2["version"], 2);

    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/models/main.default.m1/versions"))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    assert_eq!(list["model_versions"].as_array().unwrap().len(), 2);

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/models/main.default.m1/versions/1/finalize",
            json!({ "full_name": "main.default.m1", "version": 1 }),
        ))
        .await
        .unwrap();
    let finalized = json_body(assert_status(resp, StatusCode::OK, "finalize").await).await;
    assert_eq!(finalized["status"], "READY");

    // Upstream: only a pending version can be finalized, and the body must
    // name the version in the path.
    let finalize = |body: serde_json::Value| {
        seeded.clone().oneshot(req_json(
            Method::PATCH,
            "/models/main.default.m1/versions/1/finalize",
            body,
        ))
    };
    let cases = [
        (
            json!({ "full_name": "main.default.m1", "version": 1 }),
            "Model version not in a pending registration state: main.default.m1/1",
        ),
        (
            json!({ "full_name": "main.default.m1", "version": 2 }),
            "Path and request-body model version must match.",
        ),
        (json!({}), "Path and request-body model version must match."),
    ];
    for (body, message) in cases {
        let err = common::assert_error(
            finalize(body).await.unwrap(),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
        )
        .await;
        assert_eq!(err["message"], message);
    }

    // A version update needs a comment.
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/models/main.default.m1/versions/1",
            json!({}),
        ))
        .await
        .unwrap();
    let err = common::assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "No updated fields defined.");
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/models/main.default.m1/versions/0",
            json!({ "comment": "c" }),
        ))
        .await
        .unwrap();
    let err = common::assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "No valid model version specified: 0");

    let resp = seeded
        .oneshot(req(Method::DELETE, "/models/main.default.m1/versions/2"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn delete_registered_model_with_versions_requires_force(#[future(awt)] seeded: Router) {
    seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/models",
            json!({
                "name": "m1", "catalog_name": "main", "schema_name": "default",
            }),
        ))
        .await
        .unwrap();
    seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/models/versions",
            json!({
                "model_name": "m1", "catalog_name": "main", "schema_name": "default",
                "source": "s3://x",
            }),
        ))
        .await
        .unwrap();

    let resp = seeded
        .clone()
        .oneshot(req(Method::DELETE, "/models/main.default.m1"))
        .await
        .unwrap();
    assert_error(resp, StatusCode::CONFLICT, "ABORTED").await;

    let resp = seeded
        .clone()
        .oneshot(req(Method::DELETE, "/models/main.default.m1?force=true"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = seeded
        .oneshot(req(Method::GET, "/models/main.default.m1/versions/1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
