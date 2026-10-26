//! Protection against deletes: a protected catalog, schema, table, volume or
//! registered model cannot be deleted, also not with its catalog or schema,
//! until its protection is removed.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{
    assert_delta_error, assert_error, assert_iceberg_error, assert_status, json_body, req,
    req_json, seeded,
};

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

async fn ok(app: &Router, method: Method, path: &str, body: Option<Value>) -> Value {
    let resp = call(app, method.clone(), path, body).await;
    let resp = assert_status(resp, StatusCode::OK, &format!("{method} {path}")).await;
    json_body(resp).await
}

/// Protect the securable `name` of `kind`, or remove its protection.
async fn protect(app: &Router, kind: &str, name: &str, protected: bool) -> Value {
    let body = json!({ "protected": protected });
    ok(
        app,
        Method::PUT,
        &format!("/protection/{kind}/{name}"),
        Some(body),
    )
    .await
}

async fn schema(app: &Router, name: &str) {
    let body = json!({ "name": name, "catalog_name": "main" });
    ok(app, Method::POST, "/schemas", Some(body)).await;
}

async fn table(app: &Router, schema: &str, name: &str) {
    let body = json!({
        "name": name, "catalog_name": "main", "schema_name": schema,
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("s3://lake/{schema}/{name}"),
    });
    ok(app, Method::POST, "/tables", Some(body)).await;
}

async fn volume(app: &Router, schema: &str, name: &str) {
    let body = json!({
        "name": name, "catalog_name": "main", "schema_name": schema,
        "volume_type": "EXTERNAL", "storage_location": format!("s3://lake/{schema}/{name}"),
    });
    ok(app, Method::POST, "/volumes", Some(body)).await;
}

async fn model(app: &Router, schema: &str, name: &str) {
    let body = json!({ "name": name, "catalog_name": "main", "schema_name": schema });
    ok(app, Method::POST, "/models", Some(body)).await;
}

#[rstest]
#[tokio::test]
async fn protection_is_set_and_removed(#[future(awt)] seeded: Router) {
    let app = &seeded;
    table(app, "default", "t").await;
    let path = "/protection/table/main.default.t";
    assert_eq!(
        ok(app, Method::GET, path, None).await,
        json!({ "protected": false })
    );
    let on = protect(app, "table", "main.default.t", true).await;
    assert_eq!(on["protected"], true);
    assert!(on["protected_at"].is_i64(), "{on}");
    assert_eq!(ok(app, Method::GET, path, None).await, on);
    // Setting it again changes nothing.
    assert_eq!(protect(app, "table", "main.default.t", true).await, on);
    assert_eq!(
        protect(app, "table", "main.default.t", false).await,
        json!({ "protected": false })
    );
}

#[rstest]
#[case::table("table", "tables")]
#[case::volume("volume", "volumes")]
#[case::registered_model("registered_model", "models")]
#[tokio::test]
async fn a_protected_securable_is_not_deleted(
    #[future(awt)] seeded: Router,
    #[case] kind: &str,
    #[case] route: &str,
) {
    let app = &seeded;
    match kind {
        "table" => table(app, "default", "x").await,
        "volume" => volume(app, "default", "x").await,
        _ => model(app, "default", "x").await,
    }
    protect(app, kind, "main.default.x", true).await;
    let delete = format!("/{route}/main.default.x");
    let resp = call(app, Method::DELETE, &delete, None).await;
    let err = assert_error(resp, StatusCode::CONFLICT, "RESOURCE_PROTECTED").await;
    let label = kind.replace('_', " ");
    assert_eq!(
        err["message"],
        format!(
            "Cannot delete {label} main.default.x: it is protected. Remove its protection first."
        )
    );
    protect(app, kind, "main.default.x", false).await;
    ok(app, Method::DELETE, &delete, None).await;
}

#[rstest]
#[tokio::test]
async fn a_cascade_names_every_protected_securable_and_deletes_nothing(
    #[future(awt)] seeded: Router,
) {
    let app = &seeded;
    schema(app, "s").await;
    table(app, "s", "kept").await;
    table(app, "s", "t").await;
    volume(app, "s", "v").await;
    model(app, "default", "m").await;
    protect(app, "table", "main.s.t", true).await;
    protect(app, "volume", "main.s.v", true).await;
    protect(app, "registered_model", "main.default.m", true).await;

    let resp = call(app, Method::DELETE, "/catalogs/main?force=true", None).await;
    let err = assert_error(resp, StatusCode::CONFLICT, "RESOURCE_PROTECTED").await;
    assert_eq!(
        err["message"],
        "Cannot delete catalog main: these securables are protected: registered model \
         main.default.m, table main.s.t, volume main.s.v. Remove their protection first."
    );
    let resp = call(app, Method::DELETE, "/schemas/main.s?force=true", None).await;
    let err = assert_error(resp, StatusCode::CONFLICT, "RESOURCE_PROTECTED").await;
    assert_eq!(
        err["message"],
        "Cannot delete schema main.s: these securables are protected: table main.s.t, volume \
         main.s.v. Remove their protection first."
    );
    // Nothing was deleted, also not the securables without protection.
    ok(app, Method::GET, "/tables/main.s.kept", None).await;

    protect(app, "table", "main.s.t", false).await;
    protect(app, "volume", "main.s.v", false).await;
    protect(app, "registered_model", "main.default.m", false).await;
    ok(app, Method::DELETE, "/catalogs/main?force=true", None).await;
}

#[rstest]
#[tokio::test]
async fn a_protected_catalog_or_schema_says_so_before_it_is_not_empty(
    #[future(awt)] seeded: Router,
) {
    let app = &seeded;
    table(app, "default", "t").await;
    protect(app, "schema", "main.default", true).await;
    let resp = call(app, Method::DELETE, "/schemas/main.default", None).await;
    let err = assert_error(resp, StatusCode::CONFLICT, "RESOURCE_PROTECTED").await;
    assert_eq!(
        err["message"],
        "Cannot delete schema main.default: it is protected. Remove its protection first."
    );
    protect(app, "catalog", "main", true).await;
    let resp = call(app, Method::DELETE, "/catalogs/main?force=true", None).await;
    let err = assert_error(resp, StatusCode::CONFLICT, "RESOURCE_PROTECTED").await;
    assert_eq!(
        err["message"],
        "Cannot delete catalog main: these securables are protected: catalog main, schema \
         main.default. Remove their protection first."
    );
}

#[rstest]
#[tokio::test]
async fn engines_get_a_refusal(#[future(awt)] seeded: Router) {
    let app = &seeded;
    // An Iceberg table, and an empty namespace.
    let iceberg = json!({ "name": "ice", "schema": { "type": "struct", "schema-id": 0, "fields": [
        { "id": 1, "name": "id", "required": true, "type": "long" },
    ] } });
    let tables = "/iceberg/v1/catalogs/main/namespaces/default/tables";
    ok(app, Method::POST, tables, Some(iceberg)).await;
    schema(app, "empty").await;
    protect(app, "table", "main.default.ice", true).await;
    protect(app, "schema", "main.empty", true).await;

    let resp = call(app, Method::DELETE, &format!("{tables}/ice"), None).await;
    let err = assert_iceberg_error(resp, StatusCode::FORBIDDEN, "ForbiddenException").await;
    assert_eq!(
        err["message"],
        "Cannot delete table main.default.ice: it is protected. Remove its protection first."
    );
    let namespace = "/iceberg/v1/catalogs/main/namespaces/empty";
    let resp = call(app, Method::DELETE, namespace, None).await;
    assert_iceberg_error(resp, StatusCode::FORBIDDEN, "ForbiddenException").await;

    // The Delta API.
    table(app, "default", "delta").await;
    protect(app, "table", "main.default.delta", true).await;
    let delta = "/delta/v1/catalogs/main/schemas/default/tables/delta";
    let resp = call(app, Method::DELETE, delta, None).await;
    assert_delta_error(resp, StatusCode::FORBIDDEN, "PermissionDeniedException").await;
}

#[rstest]
#[tokio::test]
async fn only_data_securables_are_protected(#[future(awt)] seeded: Router) {
    let app = &seeded;
    let body = Some(json!({ "protected": true }));
    let resp = call(
        app,
        Method::PUT,
        "/protection/function/main.default.f",
        body.clone(),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        "Only catalogs, schemas, tables, volumes and registered models can be protected, not \
         functions."
    );
    let resp = call(
        app,
        Method::PUT,
        "/protection/table/main.default.none",
        body,
    )
    .await;
    assert_error(resp, StatusCode::NOT_FOUND, "TABLE_NOT_FOUND").await;
}

#[rstest]
#[tokio::test]
async fn backups_keep_protections(#[future(awt)] seeded: Router) {
    let app = &seeded;
    protect(app, "schema", "main.default", true).await;
    let request = common::as_admin(req(Method::GET, "/admin/backup"));
    let resp = app.clone().oneshot(request).await.unwrap();
    let snapshot = json_body(assert_status(resp, StatusCode::OK, "backup").await).await;
    assert_eq!(
        snapshot["tables"]["protection"].as_array().unwrap().len(),
        1
    );
}
