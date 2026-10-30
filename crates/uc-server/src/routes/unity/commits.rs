//! `/delta/preview/commits`: the commit coordinator for MANAGED Delta
//! tables. Upstream `DeltaCommitsService`; the rules are in
//! [`uc_delta::commits`].

use axum::extract::State;
use tracing::warn;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{DeltaCommit, DeltaGetCommits, DeltaGetCommitsResponse};
use uc_common::NormalizedUrl;
use uc_delta::commits::{self, CommitOutcome};

use crate::{
    authn::principal::Principal,
    authz::{self, rules},
    http::extract::Json,
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
    storage::vendor::VendOp,
    storage::{ServerStorage, same_content},
};

#[utoipa::path(
    post,
    path = "/delta/preview/commits",
    request_body = DeltaCommit,
    responses(
        (status = 200, description = "Committed, or a replay of an accepted commit. Empty body."),
        (status = 400, description = "Invalid commit, version or table URI"),
        (status = 404, description = "Table not found"),
        (status = 409, description = "Another commit holds this version"),
        (status = 429, description = "Too many unbackfilled commits"),
    ),
    tag = "Delta Commits",
)]
pub async fn commit(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<DeltaCommit>,
) -> ApiResult<EmptyObject> {
    if let Some(table_id) = req.table_id.as_deref() {
        authz::check_table_id(&state, &principal, table_id, rules::update_table).await?;
    }
    state.check_delta_api_only(
        "POST /delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table} with action add-commit",
    )?;
    let outcome = state
        .metastore
        .delta_tables()
        .commit(req, principal.actor())
        .await?;
    verify_replay(&state, outcome).await?;
    Ok(EmptyObject)
}

/// Upstream `verifyContentReplayOrThrowConflict`: settle a commit `outcome`
/// that needs its files compared. A commit for a purged version is a replay
/// when its staged file has the bytes of the published
/// `_delta_log/<version>.json`. If either file cannot be read, the outcome
/// is unknown and the client should retry.
pub(crate) async fn verify_replay(
    state: &AppState,
    outcome: CommitOutcome,
) -> uc_common::Result<()> {
    let CommitOutcome::ContentCheck {
        table_location,
        version,
        staged_file_name,
    } = outcome
    else {
        return Ok(());
    };
    let table_location = table_location.as_str();
    let unknown = |reason: String| {
        warn!(table_location, version, %reason, "cannot compare commit files");
        commits::replay_unknown(table_location, version)
    };
    let location = NormalizedUrl::parse(table_location).map_err(|e| unknown(e.message().into()))?;
    let storage = ServerStorage::new(state.metastore.clone(), state.vendor.clone());
    let (store, root) = storage
        .open(&location, VendOp::Read)
        .await
        .map_err(|e| unknown(e.0))?;
    let log = root.child("_delta_log");
    let published = log.child(format!("{version:020}.json"));
    let staged = log
        .child("_staged_commits")
        .child(staged_file_name.as_str());
    match same_content(store.as_ref(), &published, &staged).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(commits::published_by_other_writer(version)),
        Err(e) => Err(unknown(e.0)),
    }
}

#[utoipa::path(
    get,
    path = "/delta/preview/commits",
    request_body = DeltaGetCommits,
    responses((status = 200, body = DeltaGetCommitsResponse)),
    tag = "Delta Commits",
)]
pub async fn get_commits(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<DeltaGetCommits>,
) -> ApiResult<Json<DeltaGetCommitsResponse>> {
    if let Some(table_id) = req.table_id.as_deref() {
        authz::check_table_id(&state, &principal, table_id, rules::get_commits).await?;
    }
    state.check_delta_api_only(
        "GET /delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table} (commits[])",
    )?;
    Ok(Json(state.metastore.delta_tables().get_commits(req).await?))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(commit, get_commits))
}
