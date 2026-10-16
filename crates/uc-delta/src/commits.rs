//! The Delta commit coordinator rules. Upstream `DeltaCommitRepository`.
//!
//! The server tracks the commits of a MANAGED Delta table until the client
//! backfills them into `_delta_log`. The newest commit is never deleted:
//! when a backfill reaches it, it stays as a marker
//! (`is_backfilled_latest_commit`), so the latest version is always known.
//!
//! For example: commit v1, v2, v3 gives `[v1, v2, v3]`; backfill v1 gives
//! `[v2, v3]`; commit v4 with backfill v3 gives `[v4]`; backfill v4 gives
//! `[v4 (marker)]`; commit v5 gives `[v4 (marker), v5]`.
//!
//! This module holds the backend-free part: request validation and the
//! plan for one request against the tracked commits.

use serde::{Deserialize, Serialize};

use uc_api::{
    DeltaCommit, DeltaCommitInfo, DeltaGetCommits, DeltaGetCommitsResponse, DeltaMetadata,
};
use uc_common::{Error, ErrorCode, NormalizedUrl, Result, SecurablePropertiesMap};

/// Unbackfilled commits allowed per table. Clients backfill right after
/// they commit, so this is a safety limit.
pub const MAX_COMMITS_PER_TABLE: i64 = 10;

/// Most commits read for one table in one query.
pub const COMMITS_PER_BATCH: usize = 20;

/// A tracked commit, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCommit {
    pub table_id: String,
    pub commit_version: i64,
    pub commit_filename: String,
    pub commit_filesize: i64,
    pub commit_file_modification_timestamp: i64,
    pub commit_timestamp: i64,
    pub is_backfilled_latest_commit: bool,
}

impl StoredCommit {
    /// A new, unbackfilled commit. `info` must have passed
    /// [`validate_commit_info`].
    pub fn new(table_id: &str, info: &DeltaCommitInfo) -> Self {
        Self {
            table_id: table_id.to_string(),
            commit_version: info.version.unwrap_or_default(),
            commit_filename: info.file_name.clone().unwrap_or_default(),
            commit_filesize: info.file_size.unwrap_or_default(),
            commit_file_modification_timestamp: info
                .file_modification_timestamp
                .unwrap_or_default(),
            commit_timestamp: info.timestamp.unwrap_or_default(),
            is_backfilled_latest_commit: false,
        }
    }

    pub fn to_info(&self) -> DeltaCommitInfo {
        DeltaCommitInfo {
            version: Some(self.commit_version),
            timestamp: Some(self.commit_timestamp),
            file_name: Some(self.commit_filename.clone()),
            file_size: Some(self.commit_filesize),
            file_modification_timestamp: Some(self.commit_file_modification_timestamp),
        }
    }
}

/// How a commit request ended, when it did not fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    /// Applied, or recognized as a replay of an accepted commit.
    Done,
    /// The version was backfilled and is no longer tracked. The caller must
    /// compare `_delta_log/_staged_commits/<staged_file_name>` with the
    /// published `_delta_log/<version>.json`; see [`replay_unknown`] and
    /// [`published_by_other_writer`].
    ContentCheck {
        table_location: String,
        version: i64,
        staged_file_name: String,
    },
}

/// The changes one commit request makes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitPlan {
    pub save: Option<StoredCommit>,
    /// Delete the tracked commits up to and including this version.
    pub delete_up_to: Option<i64>,
    /// Mark this version as the backfilled newest commit.
    pub mark_backfilled: Option<i64>,
    /// Apply the request's metadata to the table.
    pub update_table: bool,
}

/// The result of planning a request against the tracked commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Planned {
    Apply(CommitPlan),
    /// A replay of an accepted commit: the whole request is a no-op.
    Replay,
    /// See [`CommitOutcome::ContentCheck`].
    ContentCheck {
        version: i64,
        staged_file_name: String,
    },
}

fn invalid(message: impl Into<String>) -> Error {
    Error::invalid_argument(message)
}

/// Java `UUID.fromString`, with its message.
pub fn parse_table_id(id: &str) -> Result<String> {
    uuid::Uuid::parse_str(id)
        .map(|u| u.to_string())
        .map_err(|_| invalid(format!("Invalid UUID string: {id}")))
}

/// Upstream `validateCommit`: the request shape, before any lookup.
pub fn validate_commit(commit: &DeltaCommit) -> Result<()> {
    let table_id = commit.table_id.as_deref().filter(|s| !s.is_empty());
    let table_id = table_id.ok_or_else(|| invalid("Field can not be empty: table_id"))?;
    if commit.table_uri.as_deref().is_none_or(str::is_empty) {
        return Err(invalid("Field can not be empty: table_uri"));
    }
    match &commit.commit_info {
        Some(info) => {
            validate_commit_info(info)?;
            if let Some(metadata) = &commit.metadata {
                let properties = metadata
                    .properties
                    .as_ref()
                    .and_then(|p| p.properties.as_ref());
                let has_properties = properties.is_some_and(|p| !p.is_empty());
                let has_schema = metadata
                    .schema
                    .as_ref()
                    .and_then(|s| s.columns.as_ref())
                    .is_some_and(|c| !c.is_empty());
                if metadata.description.is_none() && !has_properties && !has_schema {
                    return Err(invalid(
                        "At least one of description, properties, or schema must be set in commit.metadata",
                    ));
                }
                if let Some(properties) = properties {
                    let map: SecurablePropertiesMap = properties
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    crate::contract::validate_table_id_property(Some(&map), table_id)?;
                }
            }
        }
        None => {
            if commit.latest_backfilled_version.is_none() {
                return Err(invalid(
                    "Either commit_info or latest_backfilled_version must be defined",
                ));
            }
            if commit.metadata.is_some() {
                return Err(invalid(
                    "metadata shouldn't be set for backfill only commit",
                ));
            }
        }
    }
    Ok(())
}

/// Upstream `validateCommitInfo`.
pub fn validate_commit_info(info: &DeltaCommitInfo) -> Result<()> {
    let positive = |value: Option<i64>, field: &str| match value {
        Some(v) if v > 0 => Ok(()),
        _ => Err(invalid(format!("Field must be positive: {field}"))),
    };
    positive(info.version, "version")?;
    positive(info.timestamp, "timestamp")?;
    let file_name = info.file_name.as_deref().unwrap_or_default();
    if file_name.is_empty() {
        return Err(invalid("Field can not be empty: file_name"));
    }
    // The server reads `_delta_log/_staged_commits/<file_name>`, so the name
    // must not leave that directory.
    if file_name.contains(['/', '\\']) || file_name == "." || file_name == ".." {
        return Err(invalid(
            "Field must be a single file name without path separators: file_name",
        ));
    }
    positive(info.file_size, "file_size")?;
    positive(
        info.file_modification_timestamp,
        "file_modification_timestamp",
    )
}

/// Upstream `validateTable`: only MANAGED Delta tables with a location take
/// commits.
pub fn validate_table(
    table_id: &str,
    table_type: &str,
    data_source_format: Option<&str>,
    location: Option<&str>,
) -> Result<()> {
    if table_type != "MANAGED" {
        return Err(invalid(
            "Only managed tables are supported for Delta commits",
        ));
    }
    if data_source_format != Some("DELTA") {
        return Err(invalid("Only delta tables are supported for Delta commits"));
    }
    if location.is_none() {
        return Err(Error::new(
            ErrorCode::DataLoss,
            format!("Managed table doesn't have a URI: {table_id}"),
        ));
    }
    Ok(())
}

/// Upstream `validateTableForCommit`: the commit's `table_uri` must name the
/// table's location after normalization.
pub fn validate_table_uri(commit_uri: &str, location: &str) -> Result<()> {
    if NormalizedUrl::parse(commit_uri)? != NormalizedUrl::parse(location)? {
        return Err(invalid(format!(
            "Table URI in commit {commit_uri} does not match the table path {location}"
        )));
    }
    Ok(())
}

/// Upstream `postCommitCore`. `first` and `last` are the tracked commits
/// with the lowest and highest version; `at_version` is the tracked commit
/// at the request's version, if any. `commit` must have passed
/// [`validate_commit`].
pub fn plan_commit(
    table_id: &str,
    commit: &DeltaCommit,
    first: Option<&StoredCommit>,
    last: Option<&StoredCommit>,
    at_version: Option<&StoredCommit>,
) -> Result<Planned> {
    let update_table = commit.metadata.is_some();
    let (first, last) = match (first, last) {
        (Some(f), Some(l)) => (f, l),
        // Onboarding: the first commit the server sees for this table.
        _ => {
            let info = commit.commit_info.as_ref().ok_or_else(|| {
                invalid(
                    "Backfill request requires a prior commit; the table's commit log is empty.",
                )
            })?;
            return Ok(Planned::Apply(CommitPlan {
                save: Some(StoredCommit::new(table_id, info)),
                update_table,
                ..Default::default()
            }));
        }
    };
    if first.commit_version > last.commit_version {
        return Err(invalid(
            "Inconsistent commit log: first commit version > last commit version.",
        ));
    }
    let (first_v, last_v) = (first.commit_version, last.commit_version);
    let Some(info) = &commit.commit_info else {
        // Backfill only.
        let backfilled = commit.latest_backfilled_version.unwrap_or_default();
        if backfilled > last_v {
            return Err(invalid(format!(
                "Should not backfill version {backfilled} while the last version committed is {last_v}"
            )));
        }
        let mut plan = CommitPlan::default();
        backfill(&mut plan, backfilled, first_v, last_v, None);
        return Ok(Planned::Apply(plan));
    };

    let new_v = info.version.unwrap_or_default();
    let file_name = info.file_name.clone().unwrap_or_default();
    if new_v <= last_v {
        return already_taken(table_id, new_v, file_name, first, last, at_version);
    }
    if new_v > last_v + 1 {
        return Err(invalid(format!(
            "Commit version must be the next version after the latest commit {last_v}, but got {new_v}"
        )));
    }
    let backfilled = commit.latest_backfilled_version;
    if let Some(b) = backfilled.filter(|b| *b > last_v) {
        return Err(invalid(format!(
            "Latest backfilled version {b} cannot be greater than the last commit version = {last_v}"
        )));
    }
    check_commit_limit(table_id, new_v, backfilled, first, last)?;
    let mut plan = CommitPlan {
        save: Some(StoredCommit::new(table_id, info)),
        update_table,
        ..Default::default()
    };
    if let Some(b) = backfilled {
        backfill(&mut plan, b, first_v, last_v, Some(new_v));
    }
    Ok(Planned::Apply(plan))
}

/// Upstream `exceptionForAlreadyTakenVersion`. A tracked version with the
/// same file name is a replay, with another name a conflict. A version below
/// the tracked range was backfilled and purged, so only the file contents
/// can tell.
fn already_taken(
    table_id: &str,
    version: i64,
    file_name: String,
    first: &StoredCommit,
    last: &StoredCommit,
    at_version: Option<&StoredCommit>,
) -> Result<Planned> {
    let existing = if version == last.commit_version {
        last
    } else if version == first.commit_version {
        first
    } else if version > first.commit_version {
        at_version.filter(|c| c.commit_version == version).ok_or_else(|| {
            Error::internal(format!(
                "Inconsistent uc_delta_commits table for table {table_id}: no row tracked at in-range commit version {version}"
            ))
        })?
    } else {
        return Ok(Planned::ContentCheck {
            version,
            staged_file_name: file_name,
        });
    };
    if existing.commit_filename == file_name {
        return Ok(Planned::Replay);
    }
    Err(Error::new(
        ErrorCode::CommitVersionConflict,
        format!(
            "Commit version already accepted. Version {version} was accepted with commit file {}, but the request carried {file_name}.",
            existing.commit_filename
        ),
    ))
}

/// Upstream `getEffectiveBackfilledVersion`: the backfilled version once
/// the request is done.
fn effective_backfilled_version(
    backfilled: Option<i64>,
    first: &StoredCommit,
    last: &StoredCommit,
) -> i64 {
    if last.is_backfilled_latest_commit {
        last.commit_version
    } else if let Some(b) = backfilled.filter(|b| *b >= first.commit_version) {
        b
    } else if first.is_backfilled_latest_commit {
        first.commit_version
    } else {
        first.commit_version - 1
    }
}

/// Upstream `checkCommitLimit`.
fn check_commit_limit(
    table_id: &str,
    new_v: i64,
    backfilled: Option<i64>,
    first: &StoredCommit,
    last: &StoredCommit,
) -> Result<()> {
    let first_after = effective_backfilled_version(backfilled, first, last) + 1;
    if new_v - first_after + 1 > MAX_COMMITS_PER_TABLE {
        return Err(Error::new(
            ErrorCode::ResourceExhausted,
            format!("Max number of commits per table reached: {table_id}"),
        ));
    }
    Ok(())
}

/// Upstream `backfillCommits`. The newest commit is never deleted; a
/// backfill-only request that reaches it marks it instead.
fn backfill(plan: &mut CommitPlan, backfilled: i64, first_v: i64, last_v: i64, new_v: Option<i64>) {
    if backfilled < first_v {
        return;
    }
    let highest = new_v.unwrap_or(last_v);
    let delete_up_to = backfilled.min(highest - 1);
    if new_v.is_none() && backfilled == last_v {
        plan.mark_backfilled = Some(last_v);
    }
    if delete_up_to >= first_v {
        plan.delete_up_to = Some(delete_up_to);
    }
}

/// Upstream `getCommits` argument checks. Returns the canonical table id.
pub fn validate_get_commits(req: &DeltaGetCommits) -> Result<String> {
    let id = req
        .table_id
        .as_deref()
        .ok_or_else(|| invalid("Field can not be null: table_id"))?;
    let start = req
        .start_version
        .ok_or_else(|| invalid("Field can not be null: start_version"))?;
    let id = parse_table_id(id)?;
    if start < 0 {
        return Err(invalid("Field must be >=0: start_version"));
    }
    if req.end_version.is_some_and(|end| end < start) {
        return Err(invalid("end_version must be >=start_version if set"));
    }
    Ok(id)
}

/// Upstream `getCommits` over the newest [`COMMITS_PER_BATCH`] tracked
/// commits (`newest_first`). At most [`MAX_COMMITS_PER_TABLE`] commits are
/// returned, the oldest in range first to go into the page.
pub fn commits_response(
    req: &DeltaGetCommits,
    newest_first: &[StoredCommit],
) -> DeltaGetCommitsResponse {
    let (Some(newest), Some(oldest)) = (newest_first.first(), newest_first.last()) else {
        return DeltaGetCommitsResponse {
            commits: vec![],
            latest_table_version: 0,
        };
    };
    let latest_table_version = newest.commit_version;
    if newest.is_backfilled_latest_commit {
        return DeltaGetCommitsResponse {
            commits: vec![],
            latest_table_version,
        };
    }
    let start = req.start_version.unwrap_or_default();
    let page_end = start.max(oldest.commit_version) + MAX_COMMITS_PER_TABLE - 1;
    let end = req.end_version.unwrap_or(i64::MAX).min(page_end);
    let commits = newest_first
        .iter()
        .filter(|c| !c.is_backfilled_latest_commit)
        .filter(|c| (start..=end).contains(&c.commit_version))
        .map(StoredCommit::to_info)
        .collect();
    DeltaGetCommitsResponse {
        commits,
        latest_table_version,
    }
}

/// The metadata a commit applies to its table: replaced properties and
/// columns, and the comment.
pub fn metadata_changes(
    metadata: &DeltaMetadata,
) -> (
    Option<SecurablePropertiesMap>,
    Option<Vec<uc_api::ColumnInfo>>,
    Option<String>,
) {
    let properties = metadata.properties.as_ref().map(|p| {
        p.properties
            .iter()
            .flatten()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    });
    let columns = metadata
        .schema
        .as_ref()
        .map(|s| s.columns.clone().unwrap_or_default());
    (properties, columns, metadata.description.clone())
}

/// Upstream `verifyContentReplayOrThrowConflict` when a file cannot be
/// read: the outcome is unknown and the client should retry.
pub fn replay_unknown(table_location: &str, version: i64) -> Error {
    Error::new(
        ErrorCode::CommitStateUnknown,
        format!(
            "Could not determine whether commit version {version} is a replay: unable to read the staged or published commit file for the table at {table_location}; retry the request."
        ),
    )
}

/// Upstream `verifyContentReplayOrThrowConflict` when the contents differ.
pub fn published_by_other_writer(version: i64) -> Error {
    Error::new(
        ErrorCode::CommitVersionConflict,
        format!("Commit version already accepted. Version {version} is already published."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(v: i64, marker: bool) -> StoredCommit {
        StoredCommit {
            table_id: "t".into(),
            commit_version: v,
            commit_filename: format!("file{v}"),
            commit_filesize: 1,
            commit_file_modification_timestamp: 1,
            commit_timestamp: 1,
            is_backfilled_latest_commit: marker,
        }
    }

    fn commit(v: Option<i64>, backfilled: Option<i64>) -> DeltaCommit {
        DeltaCommit {
            table_id: Some("t".into()),
            table_uri: Some("s3://b/t".into()),
            commit_info: v.map(|v| DeltaCommitInfo {
                version: Some(v),
                timestamp: Some(1),
                file_name: Some(format!("file{v}")),
                file_size: Some(1),
                file_modification_timestamp: Some(1),
            }),
            latest_backfilled_version: backfilled,
            ..Default::default()
        }
    }

    fn plan(c: &DeltaCommit, first: i64, last: i64) -> Result<Planned> {
        plan_commit(
            "t",
            c,
            Some(&stored(first, false)),
            Some(&stored(last, false)),
            None,
        )
    }

    #[test]
    fn backfill_keeps_the_newest_commit() {
        // [1..=3] + commit 4 with backfill 3: delete up to 3.
        let Planned::Apply(p) = plan(&commit(Some(4), Some(3)), 1, 3).unwrap() else {
            panic!()
        };
        assert_eq!(p.delete_up_to, Some(3));
        assert_eq!(p.mark_backfilled, None);
        // [2..=4] + backfill-only 4: delete up to 3, mark 4.
        let Planned::Apply(p) = plan(&commit(None, Some(4)), 2, 4).unwrap() else {
            panic!()
        };
        assert_eq!((p.delete_up_to, p.mark_backfilled), (Some(3), Some(4)));
        // A backfill below the tracked range changes nothing.
        let Planned::Apply(p) = plan(&commit(None, Some(1)), 2, 4).unwrap() else {
            panic!()
        };
        assert_eq!(p, CommitPlan::default());
    }

    #[test]
    fn the_limit_counts_commits_after_the_backfill() {
        let err = plan(&commit(Some(11), None), 1, 10).unwrap_err();
        assert_eq!(err.code(), ErrorCode::ResourceExhausted);
        assert!(plan(&commit(Some(11), Some(1)), 1, 10).is_ok());
    }

    #[test]
    fn taken_versions_are_replays_or_conflicts() {
        assert_eq!(plan(&commit(Some(3), None), 1, 3).unwrap(), Planned::Replay);
        assert_eq!(plan(&commit(Some(1), None), 1, 3).unwrap(), Planned::Replay);
        let mut other = commit(Some(3), None);
        other.commit_info.as_mut().unwrap().file_name = Some("x".into());
        assert_eq!(
            plan(&other, 1, 3).unwrap_err().code(),
            ErrorCode::CommitVersionConflict
        );
        // In range but untracked: the log is inconsistent.
        assert_eq!(
            plan(&commit(Some(2), None), 1, 3).unwrap_err().code(),
            ErrorCode::Internal
        );
        // Below the range: purged, so compare contents.
        assert_eq!(
            plan(&commit(Some(1), None), 2, 3).unwrap(),
            Planned::ContentCheck {
                version: 1,
                staged_file_name: "file1".into()
            }
        );
    }

    #[test]
    fn pages_hold_at_most_ten_commits() {
        let newest_first: Vec<_> = (1..=12).rev().map(|v| stored(v, false)).collect();
        let req = DeltaGetCommits {
            start_version: Some(0),
            ..Default::default()
        };
        let resp = commits_response(&req, &newest_first);
        assert_eq!(resp.latest_table_version, 12);
        let versions: Vec<_> = resp.commits.iter().map(|c| c.version.unwrap()).collect();
        assert_eq!(versions, (1..=10).rev().collect::<Vec<_>>());
    }
}
