//! Object identity, per upstream: rows have UUIDs and link to their parent
//! by id. Names that differ only in where the dots are never collide,
//! renames keep children, and malformed full names are `INVALID_ARGUMENT`.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req, req_json, seeded};

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
    json_body(assert_status(resp, StatusCode::OK, &format!("{method} {path}")).await).await
}

fn external_table(catalog: &str, schema: &str, name: &str) -> Value {
    json!({
        "name": name, "catalog_name": catalog, "schema_name": schema,
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("s3://bucket/{catalog}/{schema}/{name}"), "columns": [],
    })
}

#[rstest]
#[tokio::test]
async fn dotted_names_do_not_collide(#[future(awt)] app: Router) {
    // `a.b_c` and `a_b.c` are two schemas, each with its own row.
    for (catalog, schema) in [("a", "b_c"), ("a_b", "c")] {
        ok(
            &app,
            Method::POST,
            "/catalogs",
            Some(json!({ "name": catalog })),
        )
        .await;
        ok(
            &app,
            Method::POST,
            "/schemas",
            Some(json!({ "name": schema, "catalog_name": catalog })),
        )
        .await;
        ok(
            &app,
            Method::POST,
            "/tables",
            Some(external_table(catalog, schema, "t")),
        )
        .await;
    }
    let s1 = ok(&app, Method::GET, "/schemas/a.b_c", None).await;
    let s2 = ok(&app, Method::GET, "/schemas/a_b.c", None).await;
    assert_eq!(s1["catalog_name"], "a");
    assert_eq!(s2["catalog_name"], "a_b");
    assert_ne!(s1["schema_id"], s2["schema_id"]);

    // Deleting one table leaves the other.
    let resp = call(&app, Method::DELETE, "/tables/a.b_c.t", None).await;
    assert_status(resp, StatusCode::OK, "delete a.b_c.t").await;
    let t = ok(&app, Method::GET, "/tables/a_b.c.t", None).await;
    assert_eq!(t["catalog_name"], "a_b");
}

#[rstest]
#[tokio::test]
async fn renamed_parents_keep_their_children(#[future(awt)] seeded: Router) {
    let app = &seeded;
    ok(
        app,
        Method::POST,
        "/tables",
        Some(external_table("main", "default", "t1")),
    )
    .await;
    ok(
        app,
        Method::POST,
        "/volumes",
        Some(json!({
            "name": "v1", "catalog_name": "main", "schema_name": "default",
            "volume_type": "EXTERNAL", "storage_location": "s3://bucket/v1",
        })),
    )
    .await;
    ok(
        app,
        Method::POST,
        "/models",
        Some(json!({ "name": "m1", "catalog_name": "main", "schema_name": "default" })),
    )
    .await;
    ok(app, Method::POST, "/models/versions", Some(json!({
        "model_name": "m1", "catalog_name": "main", "schema_name": "default", "source": "s3://run",
    }))).await;

    // Catalog rename: everything below follows.
    ok(
        app,
        Method::PATCH,
        "/catalogs/main",
        Some(json!({ "new_name": "prod" })),
    )
    .await;
    let table = ok(app, Method::GET, "/tables/prod.default.t1", None).await;
    assert_eq!(table["catalog_name"], "prod");
    let resp = call(app, Method::GET, "/tables/main.default.t1", None).await;
    assert_error(resp, StatusCode::NOT_FOUND, "CATALOG_NOT_FOUND").await;

    // Schema rename: tables, volumes and models follow.
    ok(
        app,
        Method::PATCH,
        "/schemas/prod.default",
        Some(json!({ "new_name": "gold" })),
    )
    .await;
    ok(app, Method::GET, "/tables/prod.gold.t1", None).await;
    let vol = ok(app, Method::GET, "/volumes/prod.gold.v1", None).await;
    assert_eq!(vol["full_name"], "prod.gold.v1");
    let tables = ok(
        app,
        Method::GET,
        "/tables?catalog_name=prod&schema_name=gold",
        None,
    )
    .await;
    assert_eq!(tables["tables"].as_array().unwrap().len(), 1);

    // Model rename: versions follow.
    ok(
        app,
        Method::PATCH,
        "/models/prod.gold.m1",
        Some(json!({ "new_name": "m2" })),
    )
    .await;
    let version = ok(app, Method::GET, "/models/prod.gold.m2/versions/1", None).await;
    assert_eq!(version["model_name"], "m2");
}

#[rstest]
#[tokio::test]
async fn renamed_credentials_keep_their_locations(#[future(awt)] app: Router) {
    ok(
        &app,
        Method::POST,
        "/credentials",
        Some(json!({
            "name": "c1", "purpose": "STORAGE",
            "aws_iam_role": { "role_arn": "arn:aws:iam::123456789012:role/r" },
        })),
    )
    .await;
    ok(
        &app,
        Method::POST,
        "/external-locations",
        Some(json!({ "name": "l1", "url": "s3://bucket/data", "credential_name": "c1" })),
    )
    .await;
    ok(
        &app,
        Method::PATCH,
        "/credentials/c1",
        Some(json!({ "new_name": "c2" })),
    )
    .await;
    let loc = ok(&app, Method::GET, "/external-locations/l1", None).await;
    assert_eq!(loc["credential_name"], "c2");
    // The renamed credential is still in use.
    let resp = call(&app, Method::DELETE, "/credentials/c2", None).await;
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
}

#[rstest]
#[tokio::test]
async fn malformed_full_names_are_invalid(#[future(awt)] seeded: Router) {
    let cases = [
        ("/schemas/main", "Invalid schema full name: main"),
        ("/schemas/a.b.c", "Invalid schema full name: a.b.c"),
        ("/tables/main.default", "Invalid table name: main.default"),
        ("/volumes/a.b.c.d", "Invalid volume name: a.b.c.d"),
        ("/functions/f", "Invalid function name: f"),
        (
            "/models/main.default",
            "Invalid registered model name: main.default",
        ),
    ];
    for (path, message) in cases {
        let resp = call(&seeded, Method::GET, path, None).await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], message, "{path}");
    }
}

#[rstest]
#[tokio::test]
async fn lists_under_missing_parents_are_not_found(#[future(awt)] seeded: Router) {
    let resp = call(&seeded, Method::GET, "/schemas?catalog_name=nope", None).await;
    assert_error(resp, StatusCode::NOT_FOUND, "CATALOG_NOT_FOUND").await;
    for kind in ["tables", "volumes", "functions", "models"] {
        let resp = call(
            &seeded,
            Method::GET,
            &format!("/{kind}?catalog_name=main&schema_name=nope"),
            None,
        )
        .await;
        assert_error(resp, StatusCode::NOT_FOUND, "SCHEMA_NOT_FOUND").await;
    }
}

#[rstest]
#[tokio::test]
async fn model_list_rules(#[future(awt)] seeded: Router) {
    for (path, message) in [
        (
            "/models?catalog_name=main",
            "Cannot specify catalog w/o schema for list registered models.",
        ),
        (
            "/models?schema_name=default",
            "Cannot specify schema w/o catalog for list registered models.",
        ),
    ] {
        let resp = call(&seeded, Method::GET, path, None).await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], message);
    }

    // The metastore-wide list pages by id, so equal names in different
    // schemas are never skipped.
    ok(
        &seeded,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "other", "catalog_name": "main" })),
    )
    .await;
    for schema in ["default", "other"] {
        ok(
            &seeded,
            Method::POST,
            "/models",
            Some(json!({ "name": "same", "catalog_name": "main", "schema_name": schema })),
        )
        .await;
    }
    let page1 = ok(&seeded, Method::GET, "/models?max_results=1", None).await;
    let token = page1["next_page_token"].as_str().unwrap().to_string();
    let page2 = ok(
        &seeded,
        Method::GET,
        &format!("/models?max_results=1&page_token={token}"),
        None,
    )
    .await;
    let schemas: Vec<&str> = [&page1, &page2]
        .iter()
        .map(|p| p["registered_models"][0]["schema_name"].as_str().unwrap())
        .collect();
    assert_eq!(schemas.len(), 2);
    assert_ne!(schemas[0], schemas[1]);

    let resp = call(&seeded, Method::GET, "/models?page_token=not-a-uuid", None).await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Invalid page token received: not-a-uuid");

    // Deleting a missing model is a 404.
    let resp = call(&seeded, Method::DELETE, "/models/main.default.nope", None).await;
    assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
}

#[rstest]
#[tokio::test]
async fn renaming_to_an_existing_name_fails(#[future(awt)] seeded: Router) {
    // Upstream checks whether the new name exists — the current name too.
    let resp = call(
        &seeded,
        Method::PATCH,
        "/catalogs/main",
        Some(json!({ "new_name": "main" })),
    )
    .await;
    assert_error(resp, StatusCode::BAD_REQUEST, "CATALOG_ALREADY_EXISTS").await;
    let resp = call(
        &seeded,
        Method::PATCH,
        "/schemas/main.default",
        Some(json!({ "new_name": "default" })),
    )
    .await;
    assert_error(resp, StatusCode::BAD_REQUEST, "SCHEMA_ALREADY_EXISTS").await;

    ok(
        &seeded,
        Method::POST,
        "/models",
        Some(json!({ "name": "m", "catalog_name": "main", "schema_name": "default" })),
    )
    .await;
    let resp = call(
        &seeded,
        Method::PATCH,
        "/models/main.default.m",
        Some(json!({})),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "No updated fields defined.");
}

#[rstest]
#[tokio::test]
async fn patch_properties_replace_the_map(#[future(awt)] app: Router) {
    ok(
        &app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c", "properties": { "a": "1" } })),
    )
    .await;
    ok(
        &app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "c", "properties": { "a": "1" } })),
    )
    .await;

    for path in ["/catalogs/c", "/schemas/c.s"] {
        // A non-empty map replaces all properties.
        let updated = ok(
            &app,
            Method::PATCH,
            path,
            Some(json!({ "properties": { "b": "2" } })),
        )
        .await;
        assert_eq!(updated["properties"], json!({ "b": "2" }), "{path}");
        let stamp = updated["updated_at"].clone();

        // An empty map, or no fields at all, changes nothing (not even updated_at).
        for body in [json!({ "properties": {} }), json!({})] {
            let same = ok(&app, Method::PATCH, path, Some(body)).await;
            assert_eq!(same["properties"], json!({ "b": "2" }), "{path}");
            assert_eq!(same["updated_at"], stamp, "{path}");
        }
    }
}
