//! Cascading deletes, per upstream `SchemaRepository.deleteSchema`: a
//! schema with tables, volumes, functions or models needs `force`, and
//! `force` (on a schema or its catalog) deletes all of them.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{assert_error, assert_status, json_body, req, req_json, seeded};

async fn call(
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

async fn ok(app: &Router, method: Method, path: &str, body: Option<Value>) {
    let resp = call(app, method.clone(), path, body).await;
    assert_status(resp, StatusCode::OK, &format!("{method} {path}")).await;
}

async fn count(app: &Router, path: &str, key: &str) -> usize {
    let resp = call(app, Method::GET, path, None).await;
    let body = json_body(assert_status(resp, StatusCode::OK, path).await).await;
    body[key].as_array().map_or(0, Vec::len)
}

fn volume(schema: &str) -> Value {
    json!({
        "name": "v", "catalog_name": "main", "schema_name": schema,
        "volume_type": "EXTERNAL", "storage_location": format!("s3://bucket/{schema}/v"),
    })
}

fn function(schema: &str) -> Value {
    json!({ "function_info": {
        "name": "f", "catalog_name": "main", "schema_name": schema,
        "input_params": { "parameters": [] }, "data_type": "INT", "full_data_type": "int",
        "routine_body": "EXTERNAL", "routine_definition": "return 1", "external_language": "python",
        "parameter_style": "S", "is_deterministic": true, "sql_data_access": "NO_SQL",
        "is_null_call": false, "security_type": "DEFINER", "specific_name": "f",
    }})
}

async fn add_model(app: &Router, schema: &str) {
    ok(
        app,
        Method::POST,
        "/models",
        Some(json!({ "name": "m", "catalog_name": "main", "schema_name": schema })),
    )
    .await;
    ok(
        app,
        Method::POST,
        "/models/versions",
        Some(json!({
            "model_name": "m", "catalog_name": "main", "schema_name": schema, "source": "s3://run",
        })),
    )
    .await;
}

#[rstest]
#[case::volumes("volumes")]
#[case::functions("functions")]
#[case::models("models")]
#[tokio::test]
async fn schema_with_any_child_needs_force(#[future(awt)] seeded: Router, #[case] kind: &str) {
    let app = &seeded;
    ok(
        app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "main" })),
    )
    .await;
    match kind {
        "volumes" => ok(app, Method::POST, "/volumes", Some(volume("s"))).await,
        "functions" => ok(app, Method::POST, "/functions", Some(function("s"))).await,
        _ => add_model(app, "s").await,
    }
    let resp = call(app, Method::DELETE, "/schemas/main.s", None).await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
    assert_eq!(err["message"], format!("Cannot delete schema with {kind}"));

    ok(app, Method::DELETE, "/schemas/main.s?force=true", None).await;

    // Re-creating the schema shows no leftovers.
    ok(
        app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "main" })),
    )
    .await;
    assert_eq!(
        count(
            app,
            &format!("/{kind}?catalog_name=main&schema_name=s"),
            if kind == "models" {
                "registered_models"
            } else {
                kind
            }
        )
        .await,
        0
    );
}

#[rstest]
#[tokio::test]
async fn force_deleting_a_catalog_removes_everything_inside(#[future(awt)] seeded: Router) {
    let app = &seeded;
    ok(app, Method::POST, "/volumes", Some(volume("default"))).await;
    ok(app, Method::POST, "/functions", Some(function("default"))).await;
    add_model(app, "default").await;

    let resp = call(app, Method::DELETE, "/catalogs/main", None).await;
    assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
    ok(app, Method::DELETE, "/catalogs/main?force=true", None).await;

    // Re-create the same names: nothing comes back.
    common::seed_main_default(app).await;
    let scope = "catalog_name=main&schema_name=default";
    assert_eq!(count(app, &format!("/volumes?{scope}"), "volumes").await, 0);
    assert_eq!(
        count(app, &format!("/functions?{scope}"), "functions").await,
        0
    );
    assert_eq!(
        count(app, &format!("/models?{scope}"), "registered_models").await,
        0
    );
    // The model's versions went too: the metastore-wide model list is empty.
    assert_eq!(count(app, "/models", "registered_models").await, 0);
}
