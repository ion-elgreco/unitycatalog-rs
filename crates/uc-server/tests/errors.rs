//! Error envelope and error codes, per upstream `ErrorCode` and
//! `GlobalExceptionHandler`: resource-specific `*_NOT_FOUND` /
//! `*_ALREADY_EXISTS` codes, their HTTP statuses, and the
//! `google.rpc.ErrorInfo` detail.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, assert_error, assert_status, req, req_json, seeded};

fn external_table(catalog: &str, schema: &str, name: &str) -> Value {
    json!({
        "name": name, "catalog_name": catalog, "schema_name": schema,
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("s3://bucket/{catalog}/{schema}/{name}"),
        "columns": [],
    })
}

fn function(name: &str) -> Value {
    json!({ "function_info": {
        "name": name, "catalog_name": "main", "schema_name": "default",
        "input_params": { "parameters": [] },
        "data_type": "INT", "full_data_type": "int",
        "routine_body": "EXTERNAL", "routine_definition": "return 1",
        "external_language": "python",
        "parameter_style": "S", "is_deterministic": true, "sql_data_access": "NO_SQL",
        "is_null_call": false, "security_type": "DEFINER", "specific_name": name,
    }})
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let request = match body {
        Some(b) => req_json(method, path, b),
        None => req(method, path),
    };
    app.clone().oneshot(request).await.unwrap()
}

#[rstest]
#[tokio::test]
async fn missing_parents_report_catalog_or_schema_not_found(#[future(awt)] app: Router) {
    let resp = send(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "nope" })),
    )
    .await;
    let err = assert_error(resp, StatusCode::NOT_FOUND, "CATALOG_NOT_FOUND").await;
    assert_eq!(err["message"], "Catalog not found: nope");

    let resp = send(
        &app,
        Method::POST,
        "/tables",
        Some(external_table("nope", "s", "t")),
    )
    .await;
    assert_error(resp, StatusCode::NOT_FOUND, "CATALOG_NOT_FOUND").await;

    send(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "main" })),
    )
    .await;
    let resp = send(
        &app,
        Method::POST,
        "/tables",
        Some(external_table("main", "nope", "t")),
    )
    .await;
    let err = assert_error(resp, StatusCode::NOT_FOUND, "SCHEMA_NOT_FOUND").await;
    assert_eq!(err["message"], "Schema not found: main.nope");

    let resp = send(&app, Method::GET, "/schemas/main.nope", None).await;
    assert_error(resp, StatusCode::NOT_FOUND, "SCHEMA_NOT_FOUND").await;
}

#[rstest]
#[tokio::test]
async fn not_found_codes_per_resource(#[future(awt)] seeded: Router) {
    let cases = [
        (
            "/tables/main.default.t",
            "TABLE_NOT_FOUND",
            "Table not found: main.default.t",
        ),
        (
            "/volumes/main.default.v",
            "NOT_FOUND",
            "Volume not found: main.default.v",
        ),
        (
            "/functions/main.default.f",
            "NOT_FOUND",
            "Function not found: main.default.f",
        ),
        (
            "/models/main.default.m",
            "NOT_FOUND",
            "Registered model not found: main.default.m",
        ),
        (
            "/credentials/c",
            "NOT_FOUND",
            "Storage credential not found: c",
        ),
        (
            "/external-locations/l",
            "NOT_FOUND",
            "External location not found: l",
        ),
    ];
    for (path, code, message) in cases {
        let resp = send(&seeded, Method::GET, path, None).await;
        let err = assert_error(resp, StatusCode::NOT_FOUND, code).await;
        assert_eq!(err["message"], message, "{path}");
    }
}

#[rstest]
#[tokio::test]
async fn already_exists_codes_per_resource(#[future(awt)] seeded: Router) {
    let cred = json!({
        "name": "c1", "purpose": "STORAGE",
        "aws_iam_role": { "role_arn": "arn:aws:iam::123456789012:role/r" },
    });
    let ext_loc = json!({ "name": "l1", "url": "s3://bucket/loc", "credential_name": "c1" });
    let volume = json!({
        "name": "v1", "catalog_name": "main", "schema_name": "default",
        "volume_type": "EXTERNAL", "storage_location": "s3://bucket/vol",
    });
    let model = json!({ "name": "m1", "catalog_name": "main", "schema_name": "default" });

    let cases: Vec<(&str, Value, StatusCode, &str)> = vec![
        (
            "/catalogs",
            json!({ "name": "main" }),
            StatusCode::BAD_REQUEST,
            "CATALOG_ALREADY_EXISTS",
        ),
        (
            "/schemas",
            json!({ "name": "default", "catalog_name": "main" }),
            StatusCode::BAD_REQUEST,
            "SCHEMA_ALREADY_EXISTS",
        ),
        (
            "/tables",
            external_table("main", "default", "t1"),
            StatusCode::BAD_REQUEST,
            "TABLE_ALREADY_EXISTS",
        ),
        (
            "/volumes",
            volume,
            StatusCode::BAD_REQUEST,
            "RESOURCE_ALREADY_EXISTS",
        ),
        (
            "/functions",
            function("f1"),
            StatusCode::BAD_REQUEST,
            "RESOURCE_ALREADY_EXISTS",
        ),
        ("/models", model, StatusCode::CONFLICT, "ALREADY_EXISTS"),
        (
            "/credentials",
            cred,
            StatusCode::BAD_REQUEST,
            "STORAGE_CREDENTIAL_ALREADY_EXISTS",
        ),
        (
            "/external-locations",
            ext_loc,
            StatusCode::BAD_REQUEST,
            "EXTERNAL_LOCATION_ALREADY_EXISTS",
        ),
    ];
    for (path, body, status, code) in cases {
        // First create succeeds (except the seeded catalog/schema), second fails.
        let first = send(&seeded, Method::POST, path, Some(body.clone())).await;
        if !matches!(path, "/catalogs" | "/schemas") {
            assert_status(first, StatusCode::OK, path).await;
        }
        let resp = send(&seeded, Method::POST, path, Some(body)).await;
        assert_error(resp, status, code).await;
    }
}

fn raw(
    method: Method,
    path: &str,
    content_type: Option<&str>,
    body: &str,
) -> axum::http::Request<axum::body::Body> {
    let mut b = axum::http::Request::builder()
        .method(method)
        .uri(format!("{}{path}", common::BASE));
    if let Some(ct) = content_type {
        b = b.header("content-type", ct);
    }
    b.body(axum::body::Body::from(body.to_string())).unwrap()
}

#[rstest]
#[tokio::test]
async fn unreadable_input_is_a_json_invalid_argument(#[future(awt)] app: Router) {
    let cases = [
        raw(
            Method::POST,
            "/catalogs",
            Some("application/json"),
            "{not json",
        ),
        raw(Method::POST, "/catalogs", Some("application/json"), ""),
        raw(
            Method::POST,
            "/catalogs",
            Some("application/json"),
            r#"{"name": 42}"#,
        ),
        raw(Method::GET, "/catalogs?max_results=abc", None, ""),
        raw(Method::DELETE, "/catalogs/x?force=maybe", None, ""),
    ];
    for request in cases {
        let what = format!("{} {}", request.method(), request.uri());
        let resp = app.clone().oneshot(request).await.unwrap();
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert!(
            err["message"].as_str().is_some_and(|m| !m.is_empty()),
            "{what}: {err}"
        );
    }
}

#[rstest]
#[tokio::test]
async fn json_body_without_content_type_is_accepted(#[future(awt)] app: Router) {
    let resp = app
        .clone()
        .oneshot(raw(Method::POST, "/catalogs", None, r#"{"name": "main"}"#))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "create without content-type").await;
}

#[rstest]
#[tokio::test]
async fn names_follow_upstream_rules(#[future(awt)] app: Router) {
    let bad = "Name cannot contain a period, space, forward-slash, or control characters";
    for name in ["a.b", "a b", "a/b", "a'b", "café"] {
        let resp = send(
            &app,
            Method::POST,
            "/catalogs",
            Some(json!({ "name": name })),
        )
        .await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], bad, "{name}");
    }
    let resp = send(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "x".repeat(256) })),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Name cannot be longer than 255 characters");

    // Upstream has no reserved words.
    for name in ["system", "information_schema", "A-b_c@d"] {
        let resp = send(
            &app,
            Method::POST,
            "/catalogs",
            Some(json!({ "name": name })),
        )
        .await;
        assert_status(resp, StatusCode::OK, name).await;
    }

    // Parent names are looked up, not validated: a bad one is a 404.
    let resp = send(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "no.such" })),
    )
    .await;
    assert_error(resp, StatusCode::NOT_FOUND, "CATALOG_NOT_FOUND").await;

    // Renames validate the new name.
    let resp = send(
        &app,
        Method::PATCH,
        "/catalogs/system",
        Some(json!({ "new_name": "bad name" })),
    )
    .await;
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
}

/// Armeria's 10 MiB request limit: larger bodies are a plain 413.
#[rstest]
#[tokio::test]
async fn bodies_over_10_mib_are_413(#[future(awt)] app: Router) {
    let comment = "x".repeat(10 * 1024 * 1024);
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "big", "comment": comment }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let comment = "x".repeat(9 * 1024 * 1024);
    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "big", "comment": comment }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
