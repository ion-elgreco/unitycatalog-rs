//! Upstream's observability listener: probes and metrics on their own
//! port, 404 on the API port.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{AppState, build_app, build_observability_app};

async fn status(app: &Router, path: &str) -> StatusCode {
    app.clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

async fn json(app: &Router, path: &str) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    (resp.status(), common::json_body(resp).await)
}

/// Upstream's bodies; `/readyz` reports the last check and is not ready
/// before one succeeds.
#[tokio::test]
async fn readiness_is_the_last_check() {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let state = AppState::new(metastore.clone());
    let app = build_observability_app(state.clone());
    let unhealthy = serde_json::json!({ "healthy": false });
    assert_eq!(
        json(&app, "/livez").await,
        (StatusCode::OK, serde_json::json!({ "healthy": true }))
    );
    assert_eq!(
        json(&app, "/readyz").await,
        (StatusCode::SERVICE_UNAVAILABLE, unhealthy)
    );
    assert!(
        state
            .readiness
            .check(&state.metastore, std::time::Duration::from_secs(2))
            .await
    );
    assert_eq!(
        json(&app, "/readyz").await,
        (StatusCode::OK, serde_json::json!({ "healthy": true }))
    );
}

#[tokio::test]
async fn probes_and_metrics_are_on_the_observability_port_only() {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let state = AppState::new(metastore.clone());
    state
        .readiness
        .check(&state.metastore, std::time::Duration::from_secs(2))
        .await;
    let api = build_app(state.clone());
    let observability = build_observability_app(state);
    for path in ["/livez", "/readyz", "/metrics"] {
        assert_eq!(status(&observability, path).await, StatusCode::OK, "{path}");
        assert_eq!(status(&api, path).await, StatusCode::NOT_FOUND, "{path}");
    }
    // The API is not on the observability port.
    assert_eq!(
        status(&observability, "/api/2.1/unity-catalog/catalogs").await,
        StatusCode::NOT_FOUND
    );
}

/// Upstream's metric names and labels.
#[tokio::test]
async fn metrics_use_upstream_names_and_labels() {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let state = AppState::new(metastore).with_api_port(8081);
    let api = build_app(state.clone());
    let observability = build_observability_app(state);
    let send = |method: axum::http::Method, path: &str, body: Option<serde_json::Value>| {
        let api = api.clone();
        let request = match body {
            Some(body) => common::req_json(method, path, body),
            None => common::req(method, path),
        };
        async move { api.oneshot(request).await.unwrap().status() }
    };
    let root = common::temp_storage_root();
    assert_eq!(
        send(
            axum::http::Method::POST,
            "/catalogs",
            Some(serde_json::json!({ "name": "m", "storage_root": root }))
        )
        .await,
        StatusCode::OK
    );
    send(
        axum::http::Method::POST,
        "/schemas",
        Some(serde_json::json!({ "name": "s", "catalog_name": "m" })),
    )
    .await;
    let table = serde_json::json!({ "name": "t", "catalog_name": "m", "schema_name": "s", "table_type": "EXTERNAL",
        "data_source_format": "DELTA", "storage_location": format!("{root}/t") });
    assert_eq!(
        send(axum::http::Method::POST, "/tables", Some(table)).await,
        StatusCode::OK
    );
    assert_eq!(
        send(axum::http::Method::GET, "/catalogs/nope", None).await,
        StatusCode::NOT_FOUND
    );
    let unrouted = api
        .clone()
        .oneshot(
            Request::builder()
                .uri("/nowhere")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unrouted.status(), StatusCode::NOT_FOUND);

    let resp = observability
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let text = String::from_utf8(
        axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let series = |labels: &[&str]| {
        text.lines().any(|l| {
            l.starts_with("http_server_requests_total{")
                && labels.iter().all(|label| l.contains(label))
        })
    };
    assert!(
        series(&[
            "hostname_pattern=\"*:8081\"",
            "service=\"io.unitycatalog.server.service.CatalogService\"",
            "method=\"createCatalog\"",
            "http_status=\"200\"",
        ]),
        "{text}"
    );
    assert!(
        series(&["method=\"getCatalog\"", "http_status=\"404\""]),
        "{text}"
    );
    assert!(
        series(&[
            "service=\"com.linecorp.armeria.server.FallbackService\"",
            "method=\"GET\""
        ]),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|l| l.starts_with("uc_securable_table_creations_total ")),
        "{text}"
    );
    for gauge in [
        "process_cpu_usage",
        "system_cpu_usage",
        "process_uptime_seconds",
    ] {
        assert!(
            text.lines().any(|l| l.starts_with(gauge)),
            "{gauge}: {text}"
        );
    }
}

/// Upstream's root answer.
#[tokio::test]
async fn root_says_hello() {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let api = build_app(AppState::new(metastore));
    let resp = api
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        resp.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"Hello, Unity Catalog!");
}

/// Upstream's docs page.
#[tokio::test]
async fn docs_are_at_docs() {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let api = build_app(AppState::new(metastore));
    assert_eq!(status(&api, "/docs/").await, StatusCode::OK);
    assert_eq!(status(&api, "/api-docs/openapi.json").await, StatusCode::OK);
}

/// Swagger UI finds an operation by its id: two operations with one id
/// send "Try it out" to the first, with the other's parameters.
#[tokio::test]
async fn operation_ids_are_unique() {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let api = build_app(AppState::new(metastore));
    let (code, doc) = json(&api, "/api-docs/openapi.json").await;
    assert_eq!(code, StatusCode::OK);
    let mut seen = std::collections::HashMap::new();
    for (path, item) in doc["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            let Some(id) = operation["operationId"].as_str() else {
                continue;
            };
            let at = format!("{} {path}", method.to_uppercase());
            if let Some(first) = seen.insert(id.to_string(), at.clone()) {
                panic!("operation id {id} is on {first} and {at}");
            }
        }
    }
    assert!(seen.contains_key("iceberg_load_table"));
}
