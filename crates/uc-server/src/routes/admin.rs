//! Admin operations: full-state backup and restore, and the audit log. Each
//! takes a signed-in admin, see [`authz::require_signed_in_admin`].

use crate::http::extract::{Json, Query};
use axum::{extract::State, http::StatusCode};
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{BackupResponse, ListAuditEventsResponse, RestoreResponse};
use uc_common::{PageToken, Pagination};
use uc_metastore::{AuditQuery, Snapshot};

use crate::{authn::principal::Principal, authz, http::error::ApiResult, server::state::AppState};

#[utoipa::path(
    get, path = "/admin/backup",
    responses(
        (status = 200, body = BackupResponse),
        (status = 403, description = "Not an admin"),
    ),
    tag = "Admin",
)]
pub async fn backup(
    State(state): State<AppState>,
    principal: Principal,
) -> ApiResult<Json<BackupResponse>> {
    authz::require_signed_in_admin(&state, &principal).await?;
    let snap = state.metastore.backups().export_snapshot().await?;
    Ok(Json(BackupResponse {
        version: snap.version,
        tables: snap.tables,
    }))
}

#[utoipa::path(
    post, path = "/admin/restore",
    request_body = BackupResponse,
    responses(
        (status = 200, body = RestoreResponse),
        (status = 400, description = "Malformed snapshot or unsupported version"),
        (status = 403, description = "Not an admin"),
    ),
    tag = "Admin",
)]
pub async fn restore(
    State(state): State<AppState>,
    principal: Principal,
    Json(body): Json<BackupResponse>,
) -> ApiResult<(StatusCode, Json<RestoreResponse>)> {
    authz::require_signed_in_admin(&state, &principal).await?;
    let snapshot = Snapshot {
        version: body.version,
        tables: body.tables,
    };
    let report = state.metastore.backups().import_snapshot(snapshot).await?;
    let response = RestoreResponse {
        imported: report.imported,
        skipped: report.skipped,
    };
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Debug, Default, Deserialize)]
pub struct AuditListParams {
    #[serde(default)]
    pub principal: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default, rename = "path_prefix")]
    pub path_prefix: Option<String>,
    /// Epoch millis lower bound (inclusive).
    #[serde(default)]
    pub since: Option<i64>,
    /// Epoch millis upper bound (inclusive).
    #[serde(default)]
    pub until: Option<i64>,
    #[serde(default)]
    pub max_results: Option<u32>,
    #[serde(default)]
    pub page_token: Option<String>,
}

#[utoipa::path(
    get, path = "/admin/audit",
    params(
        ("principal"   = Option<String>, Query),
        ("method"      = Option<String>, Query),
        ("path_prefix" = Option<String>, Query),
        ("since"       = Option<i64>,    Query),
        ("until"       = Option<i64>,    Query),
        ("max_results" = Option<u32>,    Query),
        ("page_token"  = Option<String>, Query),
    ),
    responses(
        (status = 200, body = ListAuditEventsResponse),
        (status = 403, description = "Not an admin"),
    ),
    tag = "Admin",
)]
pub async fn audit_list(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<AuditListParams>,
) -> ApiResult<Json<ListAuditEventsResponse>> {
    authz::require_signed_in_admin(&state, &principal).await?;
    let query = AuditQuery {
        principal: params.principal.as_deref(),
        method: params.method.as_deref(),
        path_prefix: params.path_prefix.as_deref(),
        since: params.since,
        until: params.until,
    };
    let page = Pagination {
        max_results: params.max_results.map(i64::from),
        page_token: params.page_token,
    };
    let (events, next) = state.metastore.audit_log().list(query, &page).await?;
    Ok(Json(ListAuditEventsResponse {
        events,
        next_page_token: next.map(PageToken::new),
    }))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(backup))
        .routes(routes!(restore))
        .routes(routes!(audit_list))
}
