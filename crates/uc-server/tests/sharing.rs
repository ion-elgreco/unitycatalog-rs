//! Delta Sharing: shares and recipients, and what a recipient reads.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_status, json_body, req, req_bearer, req_json, seeded};

#[rstest]
#[tokio::test]
async fn shares_and_recipients_crud_round_trip(#[future(awt)] seeded: Router) {
    // The table to share, in the seeded `main.default`.
    let _ = seeded.clone().oneshot(req_json(Method::POST, "/tables", json!({
        "name": "events",
        "catalog_name": "main",
        "schema_name": "default",
        "table_type": "EXTERNAL",
        "data_source_format": "DELTA",
        "columns": [
            { "name": "ts", "type_name": "TIMESTAMP", "type_text": "timestamp", "type_json": r#"{"name":"ts","type":"timestamp","nullable":true,"metadata":{}}"#, "position": 0 },
        ],
        "storage_location": "s3://lake/main/default/events",
    }))).await.unwrap();

    // Create a share and add the table to it.
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/shares",
            json!({ "name": "analytics" }),
        ))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "create share").await;

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/shares/analytics",
            json!({
                "add_objects": [{
                    "source_table_full_name": "main.default.events",
                    "shared_as_schema": "default",
                    "shared_as_name": "events",
                }],
            }),
        ))
        .await
        .unwrap();
    let share = json_body(assert_status(resp, StatusCode::OK, "update share").await).await;
    assert_eq!(share["objects"].as_array().unwrap().len(), 1);

    // Create a recipient and grant access to the share.
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/recipients",
            json!({ "name": "partner-x" }),
        ))
        .await
        .unwrap();
    let rec = json_body(assert_status(resp, StatusCode::OK, "create recipient").await).await;
    let token = rec["bearer_token"]
        .as_str()
        .expect("plaintext on create")
        .to_string();
    assert!(token.starts_with("dsr_"));

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/recipients/partner-x",
            json!({
                "add_shares": ["analytics"],
            }),
        ))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "grant share").await;

    // Recipient-side protocol calls.
    let resp = seeded
        .clone()
        .oneshot(req_bearer(Method::GET, "/delta-sharing/shares", &token))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "proto list shares").await).await;
    assert_eq!(body["items"][0]["name"], "analytics");

    let resp = seeded
        .clone()
        .oneshot(req_bearer(
            Method::GET,
            "/delta-sharing/shares/analytics/all-tables",
            &token,
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "proto all-tables").await).await;
    assert_eq!(body["items"][0]["name"], "events");

    let resp = seeded
        .clone()
        .oneshot(req_bearer(
            Method::GET,
            "/delta-sharing/shares/analytics/schemas/default/tables/events/version",
            &token,
        ))
        .await
        .unwrap();
    // The protocol sends the version in a header, with an empty body.
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["delta-table-version"], "0");
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    assert!(body.is_empty());

    let resp = seeded
        .oneshot(req_bearer(
            Method::GET,
            "/delta-sharing/shares/analytics/schemas/default/tables/events/metadata",
            &token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["delta-table-version"], "0");
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.contains("application/x-ndjson"));
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2, "protocol + metaData lines");
    assert!(lines[0].contains("\"protocol\""));
    assert!(lines[1].contains("\"metaData\""));
}

#[rstest]
#[tokio::test]
async fn protocol_calls_without_token_return_401(#[future(awt)] app: Router) {
    let resp = app
        .oneshot(req(Method::GET, "/delta-sharing/shares"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[rstest]
#[tokio::test]
async fn recipient_without_share_grant_gets_403(#[future(awt)] app: Router) {
    // A share that is not granted to the recipient.
    let _ = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/shares",
            json!({ "name": "secret" }),
        ))
        .await
        .unwrap();
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/recipients",
            json!({ "name": "noone" }),
        ))
        .await
        .unwrap();
    let rec = json_body(assert_status(resp, StatusCode::OK, "create recipient").await).await;
    let token = rec["bearer_token"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(req_bearer(
            Method::GET,
            "/delta-sharing/shares/secret/schemas",
            &token,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
