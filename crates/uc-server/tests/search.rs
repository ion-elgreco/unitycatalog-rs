//! Search: the objects whose name or comment holds some words, in the
//! order of their names, in pages, and only those the caller may see. On
//! Turso a word matches anywhere in a word, on PostgreSQL at its start:
//! these tests use words that match the same objects on both.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{assert_error, assert_status, json_body, req, req_bearer, req_json, seeded};

async fn call(app: &Router, request: axum::http::Request<axum::body::Body>) -> Value {
    let resp = app.clone().oneshot(request).await.unwrap();
    json_body(assert_status(resp, StatusCode::OK, "request").await).await
}

async fn post(app: &Router, path: &str, body: Value) -> Value {
    call(app, req_json(Method::POST, path, body)).await
}

/// The full names of the results of `GET /search?{query}`, and the token
/// of the next page.
async fn search(app: &Router, query: &str) -> (Vec<String>, Option<String>) {
    let body = call(app, req(Method::GET, &format!("/search?{query}"))).await;
    (full_names(&body), next_token(&body))
}

fn full_names(body: &Value) -> Vec<String> {
    body["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["full_name"].as_str().unwrap().to_string())
        .collect()
}

fn next_token(body: &Value) -> Option<String> {
    body["next_page_token"].as_str().map(str::to_string)
}

fn table(name: &str, comment: Option<&str>) -> Value {
    json!({ "name": name, "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("s3://lake/{name}"), "comment": comment })
}

/// `main.default` with objects of each type whose name or comment holds
/// `orders`, and some that do not.
async fn objects(app: &Router) {
    post(app, "/catalogs", json!({ "name": "orders" })).await;
    post(
        app,
        "/schemas",
        json!({ "name": "orders_raw", "catalog_name": "main" }),
    )
    .await;
    post(app, "/tables", table("orders", None)).await;
    post(app, "/tables", table("sales_orders", None)).await;
    post(
        app,
        "/tables",
        table("customers", Some("who places orders")),
    )
    .await;
    post(app, "/tables", table("invoices", None)).await;
    post(
        app,
        "/volumes",
        json!({ "name": "landing", "catalog_name": "main", "schema_name": "default",
            "volume_type": "MANAGED", "comment": "Raw ORDERS files" }),
    )
    .await;
    post(
        app,
        "/functions",
        json!({ "function_info": { "name": "orders_total", "catalog_name": "main",
            "schema_name": "default", "routine_body": "EXTERNAL", "parameter_style": "S",
            "security_type": "DEFINER" } }),
    )
    .await;
    post(
        app,
        "/models",
        json!({ "name": "order_forecast", "catalog_name": "main", "schema_name": "default",
            "comment": "next month's orders" }),
    )
    .await;
}

/// Names equal to the text first, then names that start with it, names
/// that hold it, and names of objects whose comment holds it.
#[rstest]
#[tokio::test]
async fn finds_names_and_comments_in_order(#[future(awt)] seeded: Router) {
    let app = seeded;
    objects(&app).await;
    let (found, next) = search(&app, "query=orders").await;
    assert_eq!(
        found,
        [
            "main.default.orders",
            "orders",
            "main.orders_raw",
            "main.default.orders_total",
            "main.default.sales_orders",
            "main.default.customers",
            "main.default.landing",
            "main.default.order_forecast",
        ]
    );
    assert_eq!(next, None);

    let body = call(&app, req(Method::GET, "/search?query=ORDERS%20sales")).await;
    let result = &body["results"][0];
    assert_eq!(result["securable_type"], "table");
    assert_eq!(result["full_name"], "main.default.sales_orders");
    assert_eq!(result["catalog_name"], "main");
    assert_eq!(result["schema_name"], "default");
    assert_eq!(result["name"], "sales_orders");
    assert!(result["id"].as_str().is_some_and(|id| !id.is_empty()));
    assert!(result["created_at"].as_i64().unwrap() > 0);
    assert_eq!(body["results"].as_array().unwrap().len(), 1);

    let body = call(&app, req(Method::GET, "/search?query=orders")).await;
    assert_eq!(body["results"][1]["securable_type"], "catalog");
    assert_eq!(body["results"][1].get("schema_name"), None);
    assert_eq!(body["results"][6]["comment"], "Raw ORDERS files");
}

#[rstest]
#[tokio::test]
async fn filters_by_type(#[future(awt)] seeded: Router) {
    let app = seeded;
    objects(&app).await;
    let (tables, _) = search(&app, "query=orders&securable_type=table").await;
    assert_eq!(
        tables,
        [
            "main.default.orders",
            "main.default.sales_orders",
            "main.default.customers",
        ]
    );
    let (models, _) = search(&app, "query=orders&securable_type=registered_model").await;
    assert_eq!(models, ["main.default.order_forecast"]);
    let (catalogs, _) = search(&app, "query=orders&securable_type=catalog").await;
    assert_eq!(catalogs, ["orders"]);

    let resp = app
        .clone()
        .oneshot(req(
            Method::GET,
            "/search?query=orders&securable_type=credential",
        ))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
}

#[rstest]
#[tokio::test]
async fn pages(#[future(awt)] seeded: Router) {
    let app = seeded;
    objects(&app).await;
    let (all, _) = search(&app, "query=orders").await;
    let mut paged = Vec::new();
    let mut query = "query=orders&max_results=3".to_string();
    loop {
        let (found, next) = search(&app, &query).await;
        assert!(found.len() <= 3);
        paged.extend(found);
        let Some(token) = next else { break };
        query = format!("query=orders&max_results=3&page_token={token}");
    }
    assert_eq!(paged, all);

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/search?query=orders&page_token=x"))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
}

/// Renames, new comments and deletes change what the search finds; text
/// without words finds nothing.
#[rstest]
#[tokio::test]
async fn follows_writes(#[future(awt)] seeded: Router) {
    let app = seeded;
    objects(&app).await;
    let patch = |path: &str, body: Value| req_json(Method::PATCH, path, body);
    call(
        &app,
        patch(
            "/tables/main.default.sales_orders",
            json!({ "comment": "sold items" }),
        ),
    )
    .await;
    call(
        &app,
        patch(
            "/volumes/main.default.landing",
            json!({ "new_name": "drop" }),
        ),
    )
    .await;
    call(&app, req(Method::DELETE, "/catalogs/orders")).await;
    call(
        &app,
        req(Method::DELETE, "/models/main.default.order_forecast"),
    )
    .await;

    let (found, _) = search(&app, "query=orders").await;
    assert_eq!(
        found,
        [
            "main.default.orders",
            "main.orders_raw",
            "main.default.orders_total",
            "main.default.sales_orders",
            "main.default.customers",
            "main.default.drop",
        ]
    );
    let (found, _) = search(&app, "query=sold%20items").await;
    assert_eq!(found, ["main.default.sales_orders"]);
    let (found, next) = search(&app, "query=%22-*%20!").await;
    assert_eq!((found, next), (vec![], None));
}

/// A request signed in with `token`.
fn signed(
    method: Method,
    path: &str,
    body: Option<Value>,
    token: &str,
) -> axum::http::Request<axum::body::Body> {
    let mut request = match body {
        Some(body) => req_json(method, path, body),
        None => req_bearer(method, path, token),
    };
    request
        .headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    request
}

/// Each result needs the right of its type's `get`: hidden objects do not
/// fill pages.
#[tokio::test]
async fn shows_only_what_the_caller_may_see() {
    let (app, metastore) = common::signed_in_app().await;
    let admin = common::active_user(&metastore, "admin@example.com").await;
    metastore
        .permissions()
        .set_metastore_owner("admin@example.com")
        .await
        .unwrap();
    let bob = common::active_user(&metastore, "bob@example.com").await;
    let as_admin = async |method: Method, path: &str, body: Option<Value>| {
        let resp = app
            .clone()
            .oneshot(signed(method, path, body, &admin))
            .await
            .unwrap();
        assert_status(resp, StatusCode::OK, path).await;
    };
    as_admin(
        Method::POST,
        "/credentials",
        Some(json!({ "name": "cred", "purpose": "STORAGE",
        "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } })),
    )
    .await;
    as_admin(
        Method::POST,
        "/external-locations",
        Some(json!({ "name": "el",
        "url": "s3://lake", "credential_name": "cred" })),
    )
    .await;
    for catalog in ["c", "d"] {
        as_admin(
            Method::POST,
            "/catalogs",
            Some(json!({ "name": catalog, "comment": "orders" })),
        )
        .await;
    }
    as_admin(
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "c",
        "comment": "orders" })),
    )
    .await;
    for name in ["orders", "orders_secret"] {
        as_admin(
            Method::POST,
            "/tables",
            Some(json!({ "name": name, "catalog_name": "c",
            "schema_name": "s", "table_type": "EXTERNAL", "data_source_format": "DELTA",
            "storage_location": format!("s3://lake/{name}") })),
        )
        .await;
    }
    let grant = async |securable: &str, name: &str, privilege: &str| {
        let body = json!({ "changes": [{ "principal": "bob@example.com",
            "add": [privilege], "remove": [] }] });
        as_admin(
            Method::PATCH,
            &format!("/permissions/{securable}/{name}"),
            Some(body),
        )
        .await;
    };
    grant("catalog", "c", "USE CATALOG").await;
    grant("schema", "c.s", "USE SCHEMA").await;
    grant("table", "c.s.orders", "SELECT").await;

    let search_as = async |token: &str, query: &str| {
        let resp = app
            .clone()
            .oneshot(signed(
                Method::GET,
                &format!("/search?{query}"),
                None,
                token,
            ))
            .await
            .unwrap();
        let body = json_body(assert_status(resp, StatusCode::OK, query).await).await;
        (full_names(&body), next_token(&body))
    };
    assert_eq!(
        search_as(&admin, "query=orders").await.0,
        ["c.s.orders", "c.s.orders_secret", "c", "d", "c.s"]
    );
    assert_eq!(
        search_as(&bob, "query=orders").await,
        (vec!["c.s.orders".into(), "c".into(), "c.s".into()], None)
    );

    let mut pages = Vec::new();
    let mut query = "query=orders&max_results=1".to_string();
    loop {
        let (found, next) = search_as(&bob, &query).await;
        pages.push(found);
        let Some(token) = next else { break };
        query = format!("query=orders&max_results=1&page_token={token}");
    }
    assert_eq!(pages, [vec!["c.s.orders"], vec!["c"], vec!["c.s"], vec![]]);
}
