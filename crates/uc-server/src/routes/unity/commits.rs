//! `/delta/preview/commits`: the commit coordinator for MANAGED Delta
//! tables. Upstream `DeltaCommitsService`; the rules are in
//! [`uc_delta::commits`].

use axum::extract::State;
use futures::TryStreamExt;
use object_store::{ObjectStore, ObjectStoreExt, path::Path as ObjectPath};
use tracing::warn;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{DeltaCommit, DeltaGetCommits, DeltaGetCommitsResponse};
use uc_common::{Error, ErrorCode, NormalizedUrl};
use uc_delta::commits::{self, CommitOutcome};

use crate::{
    authn::principal::Principal,
    authz::{self, rules},
    http::extract::Json,
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
    storage::vendor::VendOp,
    storage::{ServerStorage, StorageError, same_content},
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
    let (actor, req, metastore) = (principal.actor(), &req, &state.metastore);
    settle_commit(&state, |files_checked| async move {
        metastore
            .delta_tables()
            .commit(req.clone(), actor, files_checked)
            .await
    })
    .await?;
    Ok(EmptyObject)
}

/// Upstream `withPublishedCommitVerification` and
/// `verifyContentReplayOrThrowConflict`: run a commit `attempt` and settle
/// what it leaves to storage.
///
/// When the attempt backfills versions whose published files were not
/// seen yet, nothing is written: the files are checked with no transaction
/// open, and the attempt runs again with the files marked as checked. If a
/// file is missing, the attempt first runs again unchecked, since another
/// request may have backfilled the range meanwhile (and log cleanup removed
/// the file); only a range that is still needed is refused. New commits
/// only append and a backfill only flags upward, so the checked range still
/// covers what the last attempt flags.
pub(crate) async fn settle_commit<F, Fut>(state: &AppState, attempt: F) -> uc_common::Result<()>
where
    F: Fn(bool) -> Fut,
    Fut: Future<Output = uc_common::Result<CommitOutcome>>,
{
    let mut outcome = attempt(false).await?;
    if let CommitOutcome::VerifyPublished {
        table_location,
        from,
        to,
    } = &outcome
    {
        if missing_published(state, table_location, *from, *to)
            .await?
            .is_some()
        {
            outcome = attempt(false).await?;
            if let CommitOutcome::VerifyPublished {
                table_location,
                from,
                to,
            } = &outcome
            {
                if let Some(path) = missing_published(state, table_location, *from, *to).await? {
                    return Err(commits::missing_published_file(*to, &path));
                }
                outcome = attempt(true).await?;
            }
        } else {
            outcome = attempt(true).await?;
        }
    }
    verify_replay(state, outcome).await
}

/// Upstream `findMissingPublishedCommitFile`: the first published
/// `_delta_log/<version>.json` in `from..=to` that does not exist. When
/// storage cannot tell, the client should retry. A refusal of the server
/// to open the store (no credential covers the path, say) stays as it is,
/// since a retry gets it again.
async fn missing_published(
    state: &AppState,
    table_location: &str,
    from: i64,
    to: i64,
) -> uc_common::Result<Option<String>> {
    let unknown = |reason: String| {
        warn!(table_location, to, %reason, "cannot check the published commit files");
        commits::published_unknown(table_location, to)
    };
    let location = NormalizedUrl::parse(table_location).map_err(|e| unknown(e.message().into()))?;
    let storage = ServerStorage::new(state.metastore.clone(), state.vendor.clone());
    let (store, root) = storage
        .open(&location, VendOp::Read)
        .await
        .map_err(|e| match e {
            StorageError::Server(
                code @ (ErrorCode::FailedPrecondition
                | ErrorCode::PermissionDenied
                | ErrorCode::InvalidArgument),
                message,
            ) => Error::new(code, message),
            e => unknown(e.to_string()),
        })?;
    let log = root.join("_delta_log");
    for version in from..=to {
        let path = log.clone().join(format!("{version:020}.json"));
        if !published_exists(store.as_ref(), &path)
            .await
            .map_err(|e| unknown(e.to_string()))?
        {
            return Ok(Some(commits::published_path(table_location, version)));
        }
    }
    Ok(None)
}

/// Whether an object exists. S3 answers a HEAD on a missing key with 403
/// when the caller may not list the bucket, and table-scoped credentials
/// may list only under the table's prefix. So a refused HEAD is settled by
/// listing the key itself.
async fn published_exists(
    store: &dyn ObjectStore,
    path: &ObjectPath,
) -> object_store::Result<bool> {
    match store.head(path).await {
        Ok(_) => Ok(true),
        Err(object_store::Error::NotFound { .. }) => Ok(false),
        Err(refused @ object_store::Error::PermissionDenied { .. }) => {
            let listed: Vec<_> = store
                .list(Some(path))
                .try_collect()
                .await
                .map_err(|_| refused)?;
            Ok(listed.iter().any(|meta| &meta.location == path))
        }
        Err(e) => Err(e),
    }
}

/// Upstream `verifyContentReplayOrThrowConflict`: settle a commit `outcome`
/// that needs its files compared. A commit for a purged version is a replay
/// when its staged file has the bytes of the published
/// `_delta_log/<version>.json`. If either file cannot be read, the outcome
/// is unknown and the client should retry.
async fn verify_replay(state: &AppState, outcome: CommitOutcome) -> uc_common::Result<()> {
    let (table_location, version, staged_file_name) = match outcome {
        CommitOutcome::Done => return Ok(()),
        CommitOutcome::ContentCheck {
            table_location,
            version,
            staged_file_name,
        } => (table_location, version, staged_file_name),
        CommitOutcome::VerifyPublished { .. } => {
            return Err(uc_common::Error::internal(
                "a commit with checked files asked for its files to be checked",
            ));
        }
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
        .map_err(|e| unknown(e.to_string()))?;
    let log = root.join("_delta_log");
    let published = log.clone().join(format!("{version:020}.json"));
    let staged = log.join("_staged_commits").join(staged_file_name.as_str());
    match same_content(store.as_ref(), &published, &staged).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(commits::published_by_other_writer(version)),
        Err(e) => Err(unknown(e.to_string())),
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
