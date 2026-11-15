//! The Delta commit coordinator rules. Upstream `DeltaCommitRepository`.
//!
//! The server tracks the commits of a MANAGED Delta table. A backfill flags
//! rows (`is_backfilled`) instead of deleting them. The unflagged rows are
//! the live window: what get-commits returns and what the commit limit
//! counts. New rows are unflagged and only a backfill flags rows, so the
//! flagged rows are one block at the bottom of the log. The backfilled
//! version is the highest flagged row, or one below the oldest row when no
//! row is flagged.
//!
//! Flagged rows are kept for [`BACKFILLED_COMMITS_RETAINED`] versions, so a
//! retry of a backfilled commit is answered from its file name: the same
//! name is a replay, another name a conflict. An older retry is settled by
//! comparing the staged file with the published one.
//!
//! For example, with 2 versions kept (flagged rows marked `*`): commit v1,
//! v2, v3 gives `[v1, v2, v3]`; backfill v1 gives `[v1*, v2, v3]`; commit
//! v4 with backfill v3 gives `[v1*, v2*, v3*, v4]`; commit v5 with backfill
//! v4 deletes v1 and gives `[v2*, v3*, v4*, v5]`.
//!
//! A backfill flags rows only after the server has seen the published
//! `_delta_log/<version>.json` of each newly backfilled version.
//!
//! This module holds the backend-free part: request validation and the
//! plan for one request against the tracked commits.

use delta_kernel::LogPath;
use delta_kernel::path::ParsedLogPath;
use serde::{Deserialize, Serialize};
use url::Url;

use uc_api::{
    DeltaCommit, DeltaCommitInfo, DeltaGetCommits, DeltaGetCommitsResponse, DeltaMetadata,
};
use uc_common::{Error, ErrorCode, NormalizedUrl, Result, SecurablePropertiesMap};

/// Unbackfilled commits allowed per table. Clients backfill right after
/// they commit, so this is a safety limit.
pub const MAX_COMMITS_PER_TABLE: i64 = 10;

/// Versions below the backfilled version that keep their row. A retry
/// targets the version its writer just committed, and Delta stops retrying
/// within minutes, so this covers nearly all retries.
pub const BACKFILLED_COMMITS_RETAINED: i64 = 100;

/// A tracked commit, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCommit {
    pub table_id: String,
    pub commit_version: i64,
    pub commit_filename: String,
    pub commit_filesize: i64,
    pub commit_file_modification_timestamp: i64,
    pub commit_timestamp: i64,
    #[serde(rename = "is_backfilled_latest_commit")]
    pub is_backfilled: bool,
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
            is_backfilled: false,
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
    /// The version is no longer tracked. The caller must compare
    /// `_delta_log/_staged_commits/<staged_file_name>` with the published
    /// `_delta_log/<version>.json`; see [`replay_unknown`] and
    /// [`published_by_other_writer`].
    ContentCheck {
        table_location: String,
        version: i64,
        staged_file_name: String,
    },
    /// Nothing was written: the request backfills versions `from..=to`, and
    /// the caller must first check that their published files exist (see
    /// [`missing_published_file`]), then send the request again with the
    /// files marked as checked.
    VerifyPublished {
        table_location: String,
        from: i64,
        to: i64,
    },
}

/// The changes one commit request makes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitPlan {
    pub save: Option<StoredCommit>,
    /// Delete the tracked commits up to and including this version: they
    /// are more than [`BACKFILLED_COMMITS_RETAINED`] versions behind.
    pub prune_through: Option<i64>,
    /// Flag the tracked commits in `(from, to]` as backfilled.
    pub flag: Option<(i64, i64)>,
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
    /// See [`CommitOutcome::VerifyPublished`].
    VerifyPublished {
        from: i64,
        to: i64,
    },
}

/// The state of a table's commit log, from the lowest and highest tracked
/// version and the highest flagged one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitLog {
    pub last_version: i64,
    pub backfilled_version: i64,
}

impl CommitLog {
    /// None for an empty log.
    pub fn new(
        first: Option<i64>,
        last: Option<i64>,
        highest_flagged: Option<i64>,
    ) -> Option<Self> {
        let (first, last) = (first?, last?);
        Some(Self {
            last_version: last,
            backfilled_version: highest_flagged.unwrap_or(first - 1),
        })
    }
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
    if file_name.contains(['/', '\\']) {
        return Err(invalid(
            "Field must be a single file name without path separators: file_name",
        ));
    }
    if staged_commit_version(file_name) != info.version.and_then(|v| u64::try_from(v).ok()) {
        return Err(invalid(format!(
            "Field must be a staged commit file name <version>.<uuid>.json of version {}: \
             file_name",
            info.version.unwrap_or_default()
        )));
    }
    positive(info.file_size, "file_size")?;
    positive(
        info.file_modification_timestamp,
        "file_modification_timestamp",
    )
}

/// The version of a staged commit file name, `<version>.<uuid>.json`, as
/// the kernel reads it from the log tail. The kernel parses a name only
/// inside a table URL; any table root does. A name that the URL changes,
/// such as one with `#`, is refused.
fn staged_commit_version(file_name: &str) -> Option<u64> {
    let root = Url::parse("file:///table/").ok()?;
    let parsed = ParsedLogPath::from(LogPath::staged_commit(root, file_name, 0, 0).ok()?);
    (parsed.filename == file_name).then_some(parsed.version)
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

/// Upstream `postCommitCore`. `log` is the state of the tracked commits,
/// none before the first commit; `at_version` is the tracked commit at the
/// request's version, if any. `files_checked` says that the published files
/// of the versions this request backfills were seen. `commit` must have
/// passed [`validate_commit`].
pub fn plan_commit(
    table_id: &str,
    commit: &DeltaCommit,
    log: Option<CommitLog>,
    at_version: Option<&StoredCommit>,
    files_checked: bool,
) -> Result<Planned> {
    let update_table = commit.metadata.is_some();
    let Some(log) = log else {
        // Onboarding: the first commit the server sees for this table.
        let info = commit.commit_info.as_ref().ok_or_else(|| {
            invalid("Backfill request requires a prior commit; the table's commit log is empty.")
        })?;
        return Ok(Planned::Apply(CommitPlan {
            save: Some(StoredCommit::new(table_id, info)),
            update_table,
            ..Default::default()
        }));
    };
    let last_v = log.last_version;
    let Some(info) = &commit.commit_info else {
        // Backfill only.
        let backfilled = commit.latest_backfilled_version.unwrap_or_default();
        if backfilled > last_v {
            return Err(invalid(format!(
                "Should not backfill version {backfilled} while the last version committed is {last_v}"
            )));
        }
        if let Some(verify) = verify_published(log, backfilled, files_checked) {
            return Ok(verify);
        }
        let mut plan = CommitPlan::default();
        backfill(&mut plan, backfilled, log);
        return Ok(Planned::Apply(plan));
    };

    let new_v = info.version.unwrap_or_default();
    let file_name = info.file_name.clone().unwrap_or_default();
    if let Some(taken) = taken_version(table_id, new_v, file_name, log, at_version)? {
        return Ok(taken);
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
    check_commit_limit(table_id, new_v, backfilled, log)?;
    // The files are checked before anything is written.
    if let Some(verify) = backfilled.and_then(|b| verify_published(log, b, files_checked)) {
        return Ok(verify);
    }
    let mut plan = CommitPlan {
        save: Some(StoredCommit::new(table_id, info)),
        update_table,
        ..Default::default()
    };
    if let Some(b) = backfilled {
        backfill(&mut plan, b, log);
    }
    Ok(Planned::Apply(plan))
}

/// Upstream `exceptionForAlreadyTakenVersion`, for a commit at `version`:
/// none when the version is free. A tracked version with the same file name
/// is a replay, with another name a conflict. A version that is no longer
/// tracked was backfilled, so only the file contents can tell.
pub fn taken_version(
    table_id: &str,
    version: i64,
    file_name: String,
    log: CommitLog,
    at_version: Option<&StoredCommit>,
) -> Result<Option<Planned>> {
    if version > log.last_version {
        return Ok(None);
    }
    let Some(existing) = at_version.filter(|c| c.commit_version == version) else {
        if version > log.backfilled_version {
            // The live window has no gaps, so the log is inconsistent.
            return Err(Error::internal(format!(
                "Inconsistent uc_delta_commits table for table {table_id}: no row tracked at in-range commit version {version}"
            )));
        }
        return Ok(Some(Planned::ContentCheck {
            version,
            staged_file_name: file_name,
        }));
    };
    if existing.commit_filename == file_name {
        return Ok(Some(Planned::Replay));
    }
    Err(Error::new(
        ErrorCode::CommitVersionConflict,
        format!(
            "Commit version already accepted. Version {version} was accepted with commit file {}, but the request carried {file_name}.",
            existing.commit_filename
        ),
    ))
}

/// Upstream `checkCommitLimit`. A request only moves the backfilled version
/// forward, so the live window after it starts above the higher of the two.
fn check_commit_limit(
    table_id: &str,
    new_v: i64,
    backfilled: Option<i64>,
    log: CommitLog,
) -> Result<()> {
    let backfilled_after = log.backfilled_version.max(backfilled.unwrap_or(i64::MIN));
    if new_v - backfilled_after > MAX_COMMITS_PER_TABLE {
        return Err(Error::new(
            ErrorCode::ResourceExhausted,
            format!("Max number of commits per table reached: {table_id}"),
        ));
    }
    Ok(())
}

/// Upstream `ensurePublishedRangeVerified`: the versions that a backfill to
/// `backfilled` still has to see published.
fn verify_published(log: CommitLog, backfilled: i64, files_checked: bool) -> Option<Planned> {
    (!files_checked && backfilled > log.backfilled_version).then_some(Planned::VerifyPublished {
        from: log.backfilled_version + 1,
        to: backfilled,
    })
}

/// Upstream `backfillCommits`: prune the rows that aged out, and flag the
/// newly backfilled ones. A version that is already backfilled changes
/// nothing.
fn backfill(plan: &mut CommitPlan, backfilled: i64, log: CommitLog) {
    if backfilled <= log.backfilled_version {
        return;
    }
    if backfilled > BACKFILLED_COMMITS_RETAINED {
        plan.prune_through = Some(backfilled - BACKFILLED_COMMITS_RETAINED - 1);
    }
    plan.flag = Some((log.backfilled_version, backfilled));
}

/// The name of the published commit file of `version` in `_delta_log`.
pub fn published_file_name(version: i64) -> String {
    format!("{version:020}.json")
}

/// The path of the published commit file of `version` under a table.
pub fn published_path(table_location: &str, version: i64) -> String {
    format!(
        "{}/_delta_log/{}",
        table_location.trim_end_matches('/'),
        published_file_name(version)
    )
}

/// Upstream `requirePublishedCommitFiles`: a backfill reported a version
/// whose published file does not exist.
pub fn missing_published_file(to: i64, path: &str) -> Error {
    invalid(format!(
        "Cannot backfill through version {to}: published commit file is missing: {path}"
    ))
}

/// Upstream `findMissingPublishedCommitFile` when storage cannot tell
/// whether a file exists: the client should retry.
pub fn published_unknown(table_location: &str, to: i64) -> Error {
    Error::new(
        ErrorCode::CommitStateUnknown,
        format!(
            "Could not verify the published commit files under {}/_delta_log for backfill through version {to}; retry the request.",
            table_location.trim_end_matches('/')
        ),
    )
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

/// Upstream `getCommits` over the unbackfilled commits (newest first) and
/// the latest version. At most [`MAX_COMMITS_PER_TABLE`] commits are
/// returned, the oldest in range first to go into the page.
pub fn commits_response(
    req: &DeltaGetCommits,
    unbackfilled: &[StoredCommit],
    latest_table_version: i64,
) -> DeltaGetCommitsResponse {
    let oldest = unbackfilled
        .last()
        .map_or(latest_table_version, |c| c.commit_version);
    let start = req.start_version.unwrap_or_default();
    let page_end = start.max(oldest) + MAX_COMMITS_PER_TABLE - 1;
    let end = req.end_version.unwrap_or(i64::MAX).min(page_end);
    let commits = unbackfilled
        .iter()
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

    fn stored(v: i64, backfilled: bool) -> StoredCommit {
        StoredCommit {
            table_id: "t".into(),
            commit_version: v,
            commit_filename: format!("file{v}"),
            commit_filesize: 1,
            commit_file_modification_timestamp: 1,
            commit_timestamp: 1,
            is_backfilled: backfilled,
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

    /// A log of versions `first..=last`, backfilled through `backfilled`.
    fn log(first: i64, last: i64, backfilled: Option<i64>) -> Option<CommitLog> {
        CommitLog::new(Some(first), Some(last), backfilled)
    }

    fn plan(c: &DeltaCommit, log: Option<CommitLog>) -> Result<Planned> {
        let at_version = c
            .commit_info
            .as_ref()
            .and_then(|i| i.version)
            .map(|v| stored(v, false));
        let at_version =
            at_version.filter(|a| log.is_some_and(|l| a.commit_version <= l.last_version));
        plan_commit("t", c, log, at_version.as_ref(), true)
    }

    fn applied(planned: Result<Planned>) -> CommitPlan {
        match planned.unwrap() {
            Planned::Apply(p) => p,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_backfilled_version_is_the_highest_flagged_row() {
        assert_eq!(CommitLog::new(None, None, None), None);
        assert_eq!(log(3, 5, None).unwrap().backfilled_version, 2);
        assert_eq!(log(1, 5, Some(4)).unwrap().backfilled_version, 4);
    }

    #[test]
    fn a_backfill_flags_rows_and_prunes_old_ones() {
        // [1..=3] + commit 4 with backfill 3: flag (0, 3].
        let p = applied(plan(&commit(Some(4), Some(3)), log(1, 3, None)));
        assert_eq!((p.flag, p.prune_through), (Some((0, 3)), None));
        assert!(p.save.is_some());
        // A backfill-only request that reaches the newest commit flags it.
        let p = applied(plan(&commit(None, Some(4)), log(2, 4, Some(2))));
        assert_eq!(p.flag, Some((2, 4)));
        // A version that is already backfilled changes nothing.
        let p = applied(plan(&commit(None, Some(1)), log(1, 4, Some(2))));
        assert_eq!(p, CommitPlan::default());
        // The version exactly 100 behind is kept.
        let p = applied(plan(&commit(None, Some(150)), log(1, 150, None)));
        assert_eq!((p.flag, p.prune_through), (Some((0, 150)), Some(49)));
    }

    #[test]
    fn a_backfill_waits_for_its_files() {
        let unchecked = |c: &DeltaCommit, log| plan_commit("t", c, log, None, false).unwrap();
        assert_eq!(
            unchecked(&commit(None, Some(4)), log(1, 4, Some(1))),
            Planned::VerifyPublished { from: 2, to: 4 }
        );
        assert_eq!(
            unchecked(&commit(Some(5), Some(3)), log(1, 4, None)),
            Planned::VerifyPublished { from: 1, to: 3 }
        );
        // Nothing new to backfill: nothing to check.
        assert!(matches!(
            unchecked(&commit(None, Some(1)), log(1, 4, Some(2))),
            Planned::Apply(_)
        ));
        assert!(matches!(
            unchecked(&commit(Some(5), None), log(1, 4, None)),
            Planned::Apply(_)
        ));
    }

    #[test]
    fn the_limit_counts_commits_after_the_backfill() {
        let err = plan(&commit(Some(11), None), log(1, 10, None)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::ResourceExhausted);
        assert!(plan(&commit(Some(11), Some(1)), log(1, 10, None)).is_ok());
        assert!(plan(&commit(Some(15), None), log(1, 14, Some(5))).is_ok());
    }

    #[test]
    fn taken_versions_are_replays_or_conflicts() {
        let at = |v| stored(v, false);
        let taken = |v: i64, name: &str, log, at: Option<StoredCommit>| {
            taken_version("t", v, name.into(), log, at.as_ref())
        };
        let l = log(1, 3, None).unwrap();
        assert_eq!(taken(4, "file4", l, None).unwrap(), None);
        assert_eq!(
            taken(3, "file3", l, Some(at(3))).unwrap(),
            Some(Planned::Replay)
        );
        assert_eq!(
            taken(3, "x", l, Some(at(3))).unwrap_err().code(),
            ErrorCode::CommitVersionConflict
        );
        // In the live window but untracked: the log is inconsistent.
        assert_eq!(
            taken(2, "file2", l, None).unwrap_err().code(),
            ErrorCode::Internal
        );
        // A backfilled version that is still kept is answered by name.
        let l = log(1, 3, Some(2)).unwrap();
        assert_eq!(
            taken(1, "file1", l, Some(stored(1, true))).unwrap(),
            Some(Planned::Replay)
        );
        // Not kept: compare the contents.
        let l = log(5, 8, Some(6)).unwrap();
        assert_eq!(
            taken(2, "file2", l, None).unwrap(),
            Some(Planned::ContentCheck {
                version: 2,
                staged_file_name: "file2".into()
            })
        );
    }

    #[test]
    fn pages_hold_at_most_ten_commits() {
        let unbackfilled: Vec<_> = (1..=12).rev().map(|v| stored(v, false)).collect();
        let req = DeltaGetCommits {
            start_version: Some(0),
            ..Default::default()
        };
        let resp = commits_response(&req, &unbackfilled, 12);
        assert_eq!(resp.latest_table_version, 12);
        let versions: Vec<_> = resp.commits.iter().map(|c| c.version.unwrap()).collect();
        assert_eq!(versions, (1..=10).rev().collect::<Vec<_>>());
        let resp = commits_response(&req, &[], 12);
        assert!(resp.commits.is_empty());
        assert_eq!(resp.latest_table_version, 12);
    }

    #[test]
    fn published_paths_are_zero_padded() {
        assert_eq!(
            published_path("s3://b/t/", 7),
            "s3://b/t/_delta_log/00000000000000000007.json"
        );
    }

    #[test]
    fn staged_commit_names_carry_their_version() {
        let uuid = "3a0d65cd-4a56-49a8-937b-95f9e3ee90e5";
        let name = format!("00000000000000000010.{uuid}.json");
        assert_eq!(staged_commit_version(&name), Some(10));
        for name in [
            "00000000000000000010.json".to_string(),
            format!("10.{uuid}.json"),
            format!("+0000000000000000010.{uuid}.json"),
            "00000000000000000010.not-a-uuid.json".to_string(),
            format!("00000000000000000010.{uuid}.parquet"),
            format!("00000000000000000010.{uuid}.json#x"),
            format!("00000000000000000010.{uuid}.json?x"),
            "..".to_string(),
        ] {
            assert_eq!(staged_commit_version(&name), None, "{name}");
        }
    }
}
