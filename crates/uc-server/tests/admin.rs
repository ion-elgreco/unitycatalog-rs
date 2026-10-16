//! The admin API: backup, restore and the audit log.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{app, as_admin, assert_status, json_body, req, req_json, seeded};

#[rstest]
#[tokio::test]
async fn backup_then_restore_round_trips_state(#[future(awt)] seeded: Router) {
    // Add a table to the pre-seeded main.default.
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

    let resp = seeded
        .clone()
        .oneshot(as_admin(req(Method::GET, "/admin/backup")))
        .await
        .unwrap();
    let snapshot = json_body(assert_status(resp, StatusCode::OK, "backup").await).await;
    assert_eq!(snapshot["version"], 1);
    let tables = &snapshot["tables"];
    assert!(
        tables["catalog"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "main")
    );
    assert!(
        tables["schema"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "default")
    );
    assert!(
        tables["tbl"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "events")
    );

    // Restore replaces all rows; the same snapshot leaves the catalog as it was.
    let resp = seeded
        .clone()
        .oneshot(as_admin(req_json(Method::POST, "/admin/restore", snapshot)))
        .await
        .unwrap();
    let report = json_body(assert_status(resp, StatusCode::OK, "restore").await).await;
    assert!(report["skipped"].as_array().unwrap().is_empty());
    assert!(report["imported"]["catalog"].as_u64().unwrap() >= 1);

    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/catalogs/main"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = seeded
        .oneshot(req(Method::GET, "/tables/main.default.events"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn audit_endpoint_returns_recorded_events_filterable_by_method(#[future(awt)] app: Router) {
    // A POST to /catalogs writes one audit event.
    let _ = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "auditme" }),
        ))
        .await
        .unwrap();
    // The audit layer writes in the background.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let resp = app
        .oneshot(as_admin(req(Method::GET, "/admin/audit?method=POST")))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "audit list").await).await;
    let events = body["events"].as_array().unwrap();
    assert!(events.iter().all(|e| e["method"] == "POST"));
    assert!(
        events
            .iter()
            .any(|e| e["path"].as_str().unwrap().contains("/catalogs"))
    );
}

#[rstest]
#[tokio::test]
async fn restore_rejects_unsupported_version(#[future(awt)] app: Router) {
    let snapshot = json!({ "version": 99, "tables": {} });
    let resp = app
        .oneshot(as_admin(req_json(Method::POST, "/admin/restore", snapshot)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// Backups hold credentials and users: a signed-in admin only, also with
/// authorization off.
#[rstest]
#[tokio::test]
async fn admin_endpoints_need_a_signed_in_admin(#[future(awt)] app: Router) {
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/admin/backup"))
        .await
        .unwrap();
    common::assert_error(resp, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED").await;
    let token = common::security()
        .create_access_token("alice@example.com", std::time::Duration::from_secs(60))
        .unwrap();
    let mut request = req(Method::GET, "/admin/backup");
    request
        .headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    let resp = app.clone().oneshot(request).await.unwrap();
    common::assert_error(resp, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    let resp = app
        .oneshot(as_admin(req(Method::GET, "/admin/backup")))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "admin backup").await;
}

/// Restored keys are bound, not spliced into SQL: a key made of query text
/// stays an ordinary key.
#[rstest]
#[tokio::test]
async fn restore_keys_are_not_query_text(#[future(awt)] app: Router) {
    let ts = 1_700_000_000_000i64;
    let hostile = "x'); DELETE FROM catalog; --";
    let snapshot = json!({
        "version": 1,
        "tables": {
            "catalog": [
                { "name": "first", "id_string": hostile, "created_at": ts, "updated_at": ts },
                { "name": "second", "id_string": "c-2", "created_at": ts, "updated_at": ts },
            ],
        },
    });
    let resp = app
        .clone()
        .oneshot(as_admin(req_json(Method::POST, "/admin/restore", snapshot)))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "restore").await;
    let resp = app.oneshot(req(Method::GET, "/catalogs")).await.unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    let mut names: Vec<&str> = body["catalogs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["first", "second"]);
    let first = body["catalogs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "first")
        .unwrap();
    assert_eq!(first["id"], hostile);
}
