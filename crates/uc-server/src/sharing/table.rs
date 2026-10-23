//! A shared table's Delta log, read with the Delta kernel. The kernel's
//! calls block, so callers run them on a blocking thread; its reads of the
//! object store run on the IO runtime.
//!
//! Files and commits come back as the Delta log's actions, in JSON: the
//! `delta` response format passes them on, and the `parquet` format picks
//! fields from them.

use std::sync::Arc;

use delta_kernel::arrow::json::writer::{LineDelimited, WriterBuilder};
use delta_kernel::commit_range::{CommitRange, DeltaAction};
use delta_kernel::engine::arrow_data::ArrowEngineData;
use delta_kernel::history_manager::{self, HistoryCommitType};
use delta_kernel::{
    Engine, EngineData, Error as KernelError, LogPath, PredicateRef, Snapshot, SnapshotRef, Version,
};
use delta_kernel_default_engine::DefaultEngineBuilder;
use delta_kernel_default_engine::executor::tokio::TokioMultiThreadExecutor;
use object_store::ObjectStore;
use serde_json::{Map, Value};
use tracing::warn;
use url::Url;

use uc_common::{Error, ErrorCode, Result};
use uc_delta::commits::StoredCommit;
use uc_storage::runtime::io_handle;

/// A Delta log action in JSON, such as the fields of an `add`.
pub type Action = Map<String, Value>;

/// A shared table's Delta log.
pub struct SharedTable {
    /// The table's location, ending in `/`.
    root: Url,
    engine: Arc<dyn Engine>,
    catalog: Option<CatalogTail>,
}

/// What the server holds of a catalog-managed table's log: its latest
/// version, and the commits that may not be in `_delta_log` yet. The
/// server coordinates the commits of MANAGED tables.
#[derive(Debug, Clone)]
pub struct CatalogTail {
    pub latest: Version,
    pub commits: Vec<LogPath>,
}

impl CatalogTail {
    /// The tail of the table at `root`, from its unbackfilled commits and
    /// latest version (as `DeltaTableOps::unbackfilled_commits` gives
    /// them).
    pub fn new(root: &Url, commits: &[StoredCommit], latest: i64) -> Result<Self> {
        let mut staged: Vec<&StoredCommit> = commits
            .iter()
            .filter(|c| !c.is_backfilled_latest_commit)
            .collect();
        staged.sort_by_key(|c| c.commit_version);
        let commits = staged
            .into_iter()
            .map(|c| {
                LogPath::staged_commit(
                    root.clone(),
                    &c.commit_filename,
                    c.commit_file_modification_timestamp,
                    c.commit_filesize.max(0) as u64,
                )
                .map_err(kernel_error)
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            latest: latest.max(0) as Version,
            commits,
        })
    }
}

/// One commit of the log: its version, its time (epoch milliseconds) and
/// its `add`, `remove`, `cdc`, `metaData` and `protocol` actions, each
/// under its name (`{"add": {...}}`).
#[derive(Debug, Clone)]
pub struct Commit {
    pub version: Version,
    pub timestamp: i64,
    pub actions: Vec<Action>,
}

impl SharedTable {
    /// The table at `root` (ending in `/`), whose files `store` reads.
    pub fn new(root: Url, store: Arc<dyn ObjectStore>, catalog: Option<CatalogTail>) -> Self {
        let executor = Arc::new(TokioMultiThreadExecutor::new(io_handle()));
        let engine = DefaultEngineBuilder::new(store)
            .with_task_executor(executor)
            .build();
        Self {
            root,
            engine: Arc::new(engine),
            catalog,
        }
    }

    pub fn root(&self) -> &Url {
        &self.root
    }

    /// The latest snapshot, or that of `version`.
    pub fn snapshot(&self, version: Option<Version>) -> Result<SnapshotRef> {
        let build = |catalog: Option<&CatalogTail>| {
            let mut builder = Snapshot::builder_for(self.root.as_str());
            if let Some(tail) = catalog {
                builder = builder
                    .with_max_catalog_version(tail.latest)
                    .with_log_tail(tail.commits.clone());
            }
            if let Some(version) = version {
                builder = builder.at_version(version);
            }
            builder.build(self.engine.as_ref())
        };
        match build(self.catalog.as_ref()) {
            // A MANAGED table whose log does not have the catalogManaged
            // feature: its log alone.
            Err(KernelError::MaxCatalogVersion(_)) if self.catalog.is_some() => build(None),
            snapshot => snapshot,
        }
        .map_err(kernel_error)
    }

    /// The version a query as of `timestamp` (epoch milliseconds) reads:
    /// the last commit at or before it.
    pub fn version_as_of(&self, latest: &SnapshotRef, timestamp: i64) -> Result<Version> {
        history_manager::latest_version_as_of(
            latest,
            self.engine.as_ref(),
            timestamp,
            HistoryCommitType::Recreatable,
        )
        .map(|commit| commit.version)
        .map_err(invalid_argument)
    }

    /// The first commit at or after `timestamp` (epoch milliseconds); the
    /// first commit of all for a time before it.
    pub fn version_from(&self, latest: &SnapshotRef, timestamp: i64) -> Result<Version> {
        history_manager::first_version_after(
            latest,
            self.engine.as_ref(),
            timestamp,
            HistoryCommitType::Published,
        )
        .map(|commit| commit.version)
        .map_err(invalid_argument)
    }

    /// The last commit at or before `timestamp`, of any version.
    pub fn version_until(&self, latest: &SnapshotRef, timestamp: i64) -> Result<Version> {
        history_manager::latest_version_as_of(
            latest,
            self.engine.as_ref(),
            timestamp,
            HistoryCommitType::Published,
        )
        .map(|commit| commit.version)
        .map_err(invalid_argument)
    }

    /// When the commit of `snapshot` was made, in epoch milliseconds.
    pub fn timestamp(&self, snapshot: &SnapshotRef) -> Result<i64> {
        snapshot
            .get_timestamp(self.engine.as_ref())
            .map_err(kernel_error)
    }

    /// The `add` actions of the files of `snapshot`, without the files
    /// that `predicate` rules out. With `limit`, the files stop once they
    /// hold that many rows.
    pub fn files(
        &self,
        snapshot: &SnapshotRef,
        predicate: Option<PredicateRef>,
        limit: Option<u64>,
    ) -> Result<Vec<Action>> {
        let scan = match predicate {
            Some(predicate) => snapshot
                .clone()
                .scan_builder()
                .with_predicate(predicate)
                .build()
                .or_else(|error| {
                    // The predicate is a hint: without it, all files come.
                    warn!(%error, table = %self.root, "predicate hints not usable");
                    snapshot.clone().scan_builder().build()
                }),
            None => snapshot.clone().scan_builder().build(),
        }
        .map_err(kernel_error)?;
        let mut files = Vec::new();
        let mut rows = 0;
        for metadata in scan
            .scan_metadata(self.engine.as_ref())
            .map_err(kernel_error)?
        {
            let data = metadata
                .map_err(kernel_error)?
                .scan_files
                .apply_selection_vector()
                .map_err(kernel_error)?;
            for row in json_rows(data)? {
                if limit.is_some_and(|limit| rows >= limit) {
                    return Ok(files);
                }
                let add = scan_file_add(row);
                rows += records(&add);
                files.push(add);
            }
        }
        Ok(files)
    }

    /// The commits from `start` to `end`, both included.
    pub fn commits(&self, start: Version, end: Version) -> Result<Vec<Commit>> {
        let mut builder = CommitRange::builder_for(self.root.as_str(), start).with_end_version(end);
        if let Some(tail) = &self.catalog {
            builder = builder.with_log_tail(tail.commits.clone(), tail.latest);
        }
        let range = builder
            .build(self.engine.as_ref())
            .map_err(invalid_argument)?;
        let kinds = [
            DeltaAction::Add,
            DeltaAction::Remove,
            DeltaAction::Cdc,
            DeltaAction::Metadata,
            DeltaAction::Protocol,
        ];
        let mut commits = Vec::new();
        for commit in range
            .commits(self.engine.clone(), None, &kinds)
            .map_err(kernel_error)?
        {
            let commit = commit.map_err(kernel_error)?;
            let mut actions = Vec::new();
            for batch in commit
                .get_actions(self.engine.as_ref())
                .map_err(kernel_error)?
            {
                actions.extend(json_rows(batch.map_err(kernel_error)?)?);
            }
            commits.push(Commit {
                version: commit.version(),
                timestamp: commit.timestamp(),
                actions,
            });
        }
        Ok(commits)
    }

    /// The absolute URL of a file the log names by `path`: relative to the
    /// table, as writers name them. A file outside the table's location is
    /// an error: the share covers the table's location only.
    pub fn file_url(&self, path: &str) -> Result<Url> {
        let url = self
            .root
            .join(path)
            .map_err(|e| Error::internal(format!("file path {path}: {e}")))?;
        if !url.as_str().starts_with(self.root.as_str()) {
            return Err(Error::new(
                ErrorCode::FailedPrecondition,
                format!("A table with files outside its location cannot be shared: {path}"),
            ));
        }
        Ok(url)
    }

    /// The absolute URL of the file of a deletion vector, none for one
    /// stored inline.
    pub fn deletion_vector_url(&self, descriptor: &Value) -> Result<Option<Url>> {
        let descriptor: delta_kernel::actions::deletion_vector::DeletionVectorDescriptor =
            serde_json::from_value(descriptor.clone())
                .map_err(|e| Error::internal(format!("deletion vector {descriptor}: {e}")))?;
        let url = descriptor.absolute_path(&self.root).map_err(kernel_error)?;
        match url {
            Some(url) if !url.as_str().starts_with(self.root.as_str()) => Err(Error::new(
                ErrorCode::FailedPrecondition,
                format!(
                    "A table with deletion vectors outside its location cannot be shared: {url}"
                ),
            )),
            url => Ok(url),
        }
    }
}

/// The number of rows a file holds, as its stats say (0 without stats),
/// less those its deletion vector removes.
pub fn records(add: &Action) -> u64 {
    let stats = add
        .get("stats")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str::<Value>(s).ok());
    let rows = stats
        .and_then(|s| s.get("numRecords").and_then(Value::as_u64))
        .unwrap_or(0);
    let deleted = add
        .get("deletionVector")
        .and_then(|dv| dv.get("cardinality"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    rows.saturating_sub(deleted)
}

/// A scan file of the kernel as the `add` action of the log.
fn scan_file_add(mut row: Action) -> Action {
    let mut add = Map::new();
    let mut constants = match row.remove("fileConstantValues") {
        Some(Value::Object(constants)) => constants,
        _ => Map::new(),
    };
    let mut put = |key: &str, value: Option<Value>| {
        if let Some(value) = value {
            add.insert(key.to_string(), value);
        }
    };
    put("path", row.remove("path"));
    put(
        "partitionValues",
        Some(
            constants
                .remove("partitionValues")
                .unwrap_or_else(|| Value::Object(Map::new())),
        ),
    );
    put("size", row.remove("size"));
    put("modificationTime", row.remove("modificationTime"));
    put("dataChange", Some(Value::Bool(true)));
    put("stats", row.remove("stats"));
    put("tags", constants.remove("tags"));
    put("deletionVector", row.remove("deletionVector"));
    put("baseRowId", constants.remove("baseRowId"));
    put(
        "defaultRowCommitVersion",
        constants.remove("defaultRowCommitVersion"),
    );
    put("clusteringProvider", constants.remove("clusteringProvider"));
    add
}

/// The rows of a batch as JSON objects, without null fields; rows with no
/// field left are dropped.
fn json_rows(data: Box<dyn EngineData>) -> Result<Vec<Action>> {
    let batch = ArrowEngineData::try_from_engine_data(data).map_err(kernel_error)?;
    let mut writer = WriterBuilder::new()
        .with_explicit_nulls(false)
        .build::<_, LineDelimited>(Vec::new());
    let arrow = |e: delta_kernel::arrow::error::ArrowError| Error::internal(format!("JSON: {e}"));
    writer.write(batch.record_batch()).map_err(arrow)?;
    writer.finish().map_err(arrow)?;
    let text = writer.into_inner();
    let mut rows = Vec::new();
    for line in text.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        let row: Action =
            serde_json::from_slice(line).map_err(|e| Error::internal(format!("JSON: {e}")))?;
        if !row.is_empty() {
            rows.push(row);
        }
    }
    Ok(rows)
}

/// A kernel error as an API error: a missing log is a missing table, an
/// unsupported feature a refused request, anything else the server's.
pub fn kernel_error(e: KernelError) -> Error {
    match e {
        KernelError::FileNotFound(_) | KernelError::EmptyLog | KernelError::MissingVersion(_) => {
            Error::not_found(format!("The table's Delta log is missing: {e}"))
        }
        KernelError::Unsupported(_)
        | KernelError::ChangeDataFeedUnsupported(_)
        | KernelError::InvalidColumnMappingMode(_)
        | KernelError::InvalidProtocol(_) => Error::new(ErrorCode::Unimplemented, e.to_string()),
        KernelError::LogHistory(_) => invalid_argument(e),
        e => Error::internal(format!("Delta kernel: {e}")),
    }
}

/// A kernel error the request caused, such as a version or time the
/// table does not have.
fn invalid_argument(e: KernelError) -> Error {
    Error::invalid_argument(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scan_files_become_add_actions() {
        let row = json!({
            "path": "date=2024-01-01/part-0.parquet",
            "size": 10,
            "modificationTime": 5,
            "stats": "{\"numRecords\":7}",
            "deletionVector": { "storageType": "i", "pathOrInlineDv": "x", "sizeInBytes": 4, "cardinality": 2 },
            "fileConstantValues": { "partitionValues": { "date": "2024-01-01" }, "baseRowId": 3 },
        });
        let add = scan_file_add(row.as_object().unwrap().clone());
        assert_eq!(
            Value::Object(add.clone()),
            json!({
                "path": "date=2024-01-01/part-0.parquet",
                "partitionValues": { "date": "2024-01-01" },
                "size": 10,
                "modificationTime": 5,
                "dataChange": true,
                "stats": "{\"numRecords\":7}",
                "deletionVector": { "storageType": "i", "pathOrInlineDv": "x", "sizeInBytes": 4, "cardinality": 2 },
                "baseRowId": 3,
            })
        );
        assert_eq!(records(&add), 5);
    }

    #[test]
    fn files_must_be_under_the_table() {
        let store = Arc::new(object_store::memory::InMemory::new());
        let table = SharedTable::new(Url::parse("s3://lake/t/").unwrap(), store, None);
        assert_eq!(
            table.file_url("a%20b/part-0.parquet").unwrap().as_str(),
            "s3://lake/t/a%20b/part-0.parquet"
        );
        assert_eq!(
            table
                .file_url("s3://lake/t/part-1.parquet")
                .unwrap()
                .as_str(),
            "s3://lake/t/part-1.parquet"
        );
        for outside in ["s3://lake/other/part-0.parquet", "../other/part-0.parquet"] {
            assert_eq!(
                table.file_url(outside).unwrap_err().code(),
                ErrorCode::FailedPrecondition
            );
        }
    }
}
