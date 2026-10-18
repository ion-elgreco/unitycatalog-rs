use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, Method, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
};
use tower::ServiceExt;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use uc_common::{API_BASE, CONTROL_BASE};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    authn::middleware::auth_layer, observability::audit::layer::audit_layer,
    observability::metrics::metrics_layer, routes, server::openapi::ApiDoc,
    server::state::AppState,
};

/// What the API listener serves, and its limits (`[server]` and `[ui]`
/// settings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpSettings {
    /// `server.max_request_length`: the largest request body, in bytes.
    pub max_request_length: usize,
    /// `server.request_timeout`; none for `0s`.
    pub request_timeout: Option<std::time::Duration>,
    /// `server.rate_limit_per_sec`: requests per second and client address;
    /// 0 for no limit.
    pub rate_limit_per_sec: u32,
    /// `server.cors_allow_origins`; none allows every origin.
    pub cors_allow_origins: Vec<String>,
    /// The web UI's folder (`ui.dir`, else the UI installed next to the
    /// binary); none for no UI.
    pub ui_dir: Option<std::path::PathBuf>,
}

impl HttpSettings {
    /// The limits of `[server]`, without a UI.
    pub fn new(server: &uc_common::settings::Server) -> Self {
        Self {
            max_request_length: server.max_request_length,
            request_timeout: Some(server.request_timeout).filter(|t| !t.is_zero()),
            rate_limit_per_sec: server.rate_limit_per_sec,
            // `*` allows every origin, as none.
            cors_allow_origins: match server.cors_allow_origins.as_slice() {
                [any] if any == "*" => Vec::new(),
                origins => origins.to_vec(),
            },
            ui_dir: None,
        }
    }
}

impl Default for HttpSettings {
    /// The defaults of `[server]`: Armeria's limits, as upstream (10 MiB
    /// request bodies and a 10 s timeout); no rate limit, every origin, no
    /// UI.
    fn default() -> Self {
        Self::new(&uc_common::settings::Server::default())
    }
}

/// Build the API app: every resource router under the API and control
/// paths ([`API_BASE`], [`CONTROL_BASE`]), plus the OpenAPI doc, Swagger UI
/// and the web UI. Probes and `/metrics` are on the observability app.
pub fn build_app(state: AppState) -> Router {
    crate::observability::metrics::init_metrics();
    let (resource_router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(API_BASE, routes::unity::catalogs::router())
        .nest(API_BASE, routes::unity::schemas::router())
        .nest(API_BASE, routes::unity::tables::router())
        .nest(API_BASE, routes::unity::tables::staging_router())
        .nest(API_BASE, routes::unity::commits::router())
        .nest(API_BASE, routes::delta_api::router())
        .nest(API_BASE, routes::unity::volumes::router())
        .nest(API_BASE, routes::unity::functions::router())
        .nest(API_BASE, routes::unity::models::router())
        .nest(API_BASE, routes::unity::credentials::router())
        .nest(API_BASE, routes::unity::external_locations::router())
        .nest(API_BASE, routes::unity::temp_credentials::router())
        .nest(API_BASE, routes::unity::permissions::router())
        .nest(API_BASE, routes::unity::metastore::router())
        .nest(API_BASE, routes::unity::tokens::router())
        .nest(API_BASE, routes::iceberg_rest::router())
        .nest(API_BASE, routes::admin::router())
        .nest(API_BASE, routes::sharing::router())
        .nest(CONTROL_BASE, routes::control::router())
        .nest(CONTROL_BASE, routes::control::scim::router())
        .merge(routes::health::router())
        .split_for_parts();

    // Per-IP token-bucket rate limiter: `server.rate_limit_per_sec`. Off by
    // default: the layer needs the `ConnectInfo` extension, which requests
    // sent to the router directly, such as in tests, do not have.
    let http = state.http.clone();
    let rate = http.rate_limit_per_sec;
    let governor_layer = (rate > 0).then(|| {
        let config = GovernorConfigBuilder::default()
            .per_second(rate.into())
            .burst_size((rate * 2).max(1))
            .finish()
            .expect("valid governor config");
        GovernorLayer {
            config: Arc::new(config),
        }
    });

    let timeout = http.request_timeout;
    let mut router = resource_router
        // Upstream's docs page.
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api))
        // The web UI's sign-in, outside the API (`crate::authn::sign_in`). Before
        // the layers: a router merged after them would get a fallback
        // without them.
        .merge(crate::authn::sign_in::router())
        // Inside auth, so that the audit layer sees the resolved
        // `Principal` and records the status of the final response.
        .layer(middleware::from_fn_with_state(state.clone(), audit_layer))
        .layer(middleware::from_fn_with_state(state.clone(), auth_layer))
        // Outside auth, so that sign-in errors get the Delta body too.
        .layer(middleware::from_fn(routes::delta_api::error_layer))
        .layer(middleware::from_fn(routes::iceberg_rest::error_layer))
        .layer(middleware::from_fn_with_state(state.clone(), metrics_layer))
        // Armeria's defaults: 10 MiB request bodies and a 10 s timeout.
        .layer(axum::extract::DefaultBodyLimit::max(http.max_request_length))
        .layer(middleware::from_fn(move |req, next| request_timeout(timeout, req, next)))
        .layer(build_cors_layer(&http.cors_allow_origins))
        .layer(TraceLayer::new_for_http());

    if let Some(governor) = governor_layer {
        router = router.layer(governor);
    }

    // The web UI, if there is one: `ui.dir`, else the UI installed next to
    // the binary (`uc_common::install`). Its files are served from the
    // root, compressed and with cache headers, as `uc web` serves them
    // (`uc_common::ui`); other paths load the UI, for `leptos_router`'s
    // client-side routes.
    let mut router = router.with_state(state);
    let ui = http.ui_dir.as_deref().map(uc_common::ui::files);
    router = router.route(
        "/",
        axum::routing::get({
            let ui = ui.clone();
            move |request: Request| root(ui.clone(), request)
        }),
    );
    if let Some(ui) = ui {
        router = router.fallback_service(ui);
    }
    router
}

/// `/`: upstream's greeting, which test harnesses poll until the server is
/// up. With the UI, browsers (asking for HTML) get the UI there instead.
async fn root(ui: Option<Router>, request: Request) -> Response {
    let html = request
        .headers()
        .get(header::ACCEPT)
        .and_then(|a| a.to_str().ok())
        .is_some_and(|a| a.contains("text/html"));
    if let Some(ui) = ui.filter(|_| html) {
        let Ok(response) = ui.oneshot(request).await;
        // A UI folder without `index.html` gives the greeting.
        if response.status() != StatusCode::NOT_FOUND {
            return response;
        }
    }
    "Hello, Unity Catalog!".into_response()
}

/// A request over the timeout answers Armeria's plain 503.
async fn request_timeout(
    timeout: Option<std::time::Duration>,
    req: Request,
    next: middleware::Next,
) -> Response {
    let Some(timeout) = timeout else {
        return next.run(req).await;
    };
    match tokio::time::timeout(timeout, next.run(req)).await {
        Ok(response) => response,
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "503 Service Unavailable").into_response(),
    }
}

/// Upstream's observability listener (`--obs-port`): `/livez`, `/readyz`
/// and `/metrics`, without sign-in. Its requests are not counted.
pub fn build_observability_app(state: AppState) -> Router {
    routes::health::observability_router().with_state(state)
}

/// The CORS layer of `server.cors_allow_origins`; none allows every origin.
fn build_cors_layer(allow_origins: &[String]) -> CorsLayer {
    if allow_origins.is_empty() {
        return CorsLayer::permissive();
    }
    let origins: Vec<HeaderValue> = allow_origins
        .iter()
        .filter_map(|s| HeaderValue::from_str(s).ok())
        .collect();
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(methods)
        .allow_headers(tower_http::cors::Any)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn slow_requests_answer_503() {
        let timeout = Some(std::time::Duration::from_millis(20));
        let app: Router = Router::new()
            .route(
                "/slow",
                get(|| async { tokio::time::sleep(std::time::Duration::from_millis(500)).await }),
            )
            .route("/fast", get(|| async { "ok" }))
            .layer(middleware::from_fn(move |req, next| {
                request_timeout(timeout, req, next)
            }));
        let status = |path: &'static str| {
            let app = app.clone();
            async move {
                app.oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status()
            }
        };
        assert_eq!(status("/slow").await, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(status("/fast").await, StatusCode::OK);
    }
}
