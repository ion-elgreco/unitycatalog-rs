//! Probe endpoints.
//!
//! * `/healthz` and `/version`, on the API port (not upstream).
//! * `/livez` and `/readyz`, upstream's, on the observability port only
//!   (`--obs-port`).

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use crate::http::extract::Json;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_metastore::DynMetastore;

use crate::server::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct VersionResponse {
    pub server: &'static str,
    pub version: &'static str,
    pub api_version: &'static str,
}

#[utoipa::path(get, path = "/healthz", responses((status = 200, body = HealthResponse)), tag = "Health")]
pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Upstream's probe body.
#[derive(Serialize)]
pub struct Healthy {
    pub healthy: bool,
}

/// Upstream `/livez`: the process serves; the database is not asked.
pub async fn livez() -> Json<Healthy> {
    Json(Healthy { healthy: true })
}

/// Upstream `/readyz`: the last result of the readiness check, never a
/// database call on the request path; not ready until a check succeeds.
pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let healthy = state.readiness.is_ready();
    let status = match healthy {
        true => StatusCode::OK,
        false => StatusCode::SERVICE_UNAVAILABLE,
    };
    (status, Json(Healthy { healthy }))
}

/// The result of upstream's background readiness probe.
#[derive(Clone, Default)]
pub struct Readiness(Arc<AtomicBool>);

impl Readiness {
    pub fn is_ready(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Check the database once, within `timeout`.
    pub async fn check(&self, metastore: &DynMetastore, timeout: Duration) -> bool {
        let ready = matches!(
            tokio::time::timeout(timeout, metastore.permissions().metastore_id()).await,
            Ok(Ok(_))
        );
        self.0.store(ready, Ordering::Relaxed);
        ready
    }

    /// Check every `interval` in the background.
    pub fn spawn(
        &self,
        metastore: DynMetastore,
        interval: Duration,
        timeout: Duration,
    ) -> tokio::task::JoinHandle<()> {
        let readiness = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                readiness.check(&metastore, timeout).await;
            }
        })
    }
}

#[utoipa::path(
    get, path = "/version",
    responses((status = 200, body = VersionResponse)),
    tag = "Health",
)]
pub async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        server: "unitycatalog-rs",
        version: env!("CARGO_PKG_VERSION"),
        api_version: "2.1",
    })
}

/// `/healthz` and `/version`, on the API port.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(healthz))
        .routes(routes!(version))
}

/// Upstream's observability endpoints, on the observability port only:
/// `/livez`, `/readyz` and `/metrics`.
pub fn observability_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/livez", axum::routing::get(livez))
        .route("/readyz", axum::routing::get(readyz))
        .merge(crate::observability::metrics::metrics_router())
}
