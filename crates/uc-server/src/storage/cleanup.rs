//! Background storage cleanup. Upstream `StorageCleanupWorker`.
//!
//! Dropping a managed table, volume, model or model version queues a task
//! (see [`uc_metastore::CleanupOps`]). Each poll works through the ready
//! tasks one at a time: the worker claims a task, deletes everything under
//! its location within the attempt timeout, checks that nothing remains,
//! and then finishes the task. A failure is logged and retried after the
//! backoff, and ends the poll.

use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use object_store::{ObjectStore, path::Path as ObjectPath};
use tracing::warn;

use uc_common::{ManagedResource, NormalizedUrl, UriScheme};
use uc_metastore::{CleanupClaim, DynMetastore};

use crate::{
    storage::vendor::{DynVendor, VendOp},
    storage::{ServerStorage, delete_all},
};

/// The cleanup of dropped managed tables, volumes and models: the
/// `storage.cleanup_*` settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupConfig {
    pub poll_interval: Duration,
    pub attempt_timeout: Duration,
    pub lease_duration: Duration,
    /// How long a dropped object's files are kept before cleanup.
    pub initial_delay: Duration,
    pub retry_backoff: Duration,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        const MINUTE: u64 = 60;
        Self {
            poll_interval: Duration::from_secs(MINUTE),
            attempt_timeout: Duration::from_secs(30 * MINUTE),
            lease_duration: Duration::from_secs(120 * MINUTE),
            initial_delay: Duration::from_secs(7 * 24 * 60 * MINUTE),
            retry_backoff: Duration::from_secs(60 * MINUTE),
        }
    }
}

impl CleanupConfig {
    /// `storage.cleanup_poll_interval`, `cleanup_attempt_timeout`,
    /// `cleanup_lease_duration`, `cleanup_initial_delay` and
    /// `cleanup_retry_backoff`, over the defaults.
    pub fn from_settings(storage: &uc_common::settings::Storage) -> Result<Self, String> {
        let defaults = Self::default();
        let config = Self {
            poll_interval: storage
                .cleanup_poll_interval
                .unwrap_or(defaults.poll_interval),
            attempt_timeout: storage
                .cleanup_attempt_timeout
                .unwrap_or(defaults.attempt_timeout),
            lease_duration: storage
                .cleanup_lease_duration
                .unwrap_or(defaults.lease_duration),
            initial_delay: storage
                .cleanup_initial_delay
                .unwrap_or(defaults.initial_delay),
            retry_backoff: storage
                .cleanup_retry_backoff
                .unwrap_or(defaults.retry_backoff),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        let one_ms = Duration::from_millis(1);
        if self.poll_interval < one_ms {
            return Err("Cleanup poll interval must be at least one millisecond".into());
        }
        if self.attempt_timeout < one_ms {
            return Err("Storage cleanup attempt timeout must be at least one millisecond".into());
        }
        if self.lease_duration <= self.attempt_timeout {
            return Err("Storage cleanup lease duration must exceed the attempt timeout".into());
        }
        Ok(())
    }
}

/// Why a cleanup attempt failed.
#[derive(Debug, thiserror::Error)]
pub enum CleanupError {
    #[error("Cleanup task location does not match its {0} resource id")]
    InvalidTask(String),
    #[error("Storage cleanup attempt timed out")]
    TimedOut,
    #[error("Storage cleanup did not empty the task prefix")]
    FilesRemain,
    #[error("{0}")]
    Storage(String),
}

impl CleanupError {
    /// The sanitized form kept in the task row: no paths or provider
    /// messages, as upstream.
    pub fn safe_message(&self) -> String {
        let kind = match self {
            Self::InvalidTask(_) => "InvalidTask",
            Self::TimedOut => "TimedOut",
            Self::FilesRemain => "FilesRemain",
            Self::Storage(_) => "StorageError",
        };
        format!("Storage cleanup failed: {kind}")
    }
}

/// Deletes the files of a dropped object.
#[async_trait]
pub trait CleanupStorage: Send + Sync + 'static {
    /// Delete everything under `location`, then fail with
    /// [`CleanupError::FilesRemain`] if a fresh listing still finds a file.
    async fn delete_prefix(&self, location: &NormalizedUrl) -> Result<(), CleanupError>;
}

/// Local paths are removed from disk; cloud paths go through
/// [`ServerStorage`] with read-write credentials.
pub struct DefaultCleanupStorage {
    storage: ServerStorage,
}

impl DefaultCleanupStorage {
    pub fn new(metastore: DynMetastore, vendor: DynVendor) -> Self {
        Self {
            storage: ServerStorage::new(metastore, vendor),
        }
    }
}

#[async_trait]
impl CleanupStorage for DefaultCleanupStorage {
    async fn delete_prefix(&self, location: &NormalizedUrl) -> Result<(), CleanupError> {
        if location.scheme() == UriScheme::File {
            return delete_local(location.local_path()).await;
        }
        let (store, prefix) = self
            .storage
            .open(location, VendOp::ReadWrite)
            .await
            .map_err(|e| CleanupError::Storage(e.0))?;
        delete_objects(store.as_ref(), &prefix).await
    }
}

async fn delete_local(path: PathBuf) -> Result<(), CleanupError> {
    tokio::task::spawn_blocking(move || {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(CleanupError::Storage(e.to_string())),
        }
        if std::fs::symlink_metadata(&path).is_ok() {
            return Err(CleanupError::FilesRemain);
        }
        Ok(())
    })
    .await
    .map_err(|e| CleanupError::Storage(e.to_string()))?
}

async fn delete_objects(store: &dyn ObjectStore, prefix: &ObjectPath) -> Result<(), CleanupError> {
    let storage = |e: object_store::Error| CleanupError::Storage(e.to_string());
    delete_all(store, prefix).await.map_err(storage)?;
    // A file may appear after the listing above; keep the task if so.
    if store
        .list(Some(prefix))
        .next()
        .await
        .transpose()
        .map_err(storage)?
        .is_some()
    {
        return Err(CleanupError::FilesRemain);
    }
    Ok(())
}

/// Upstream `validateTask`: the stored location must be a plain URL that
/// ends with `/<segment>/<resource id>` before any storage is touched.
fn validate_task(claim: &CleanupClaim) -> Result<NormalizedUrl, CleanupError> {
    let invalid = || CleanupError::InvalidTask(claim.resource_type.clone());
    let resource = ManagedResource::parse(&claim.resource_type).ok_or_else(invalid)?;
    let location = NormalizedUrl::parse(&claim.storage_location).map_err(|_| invalid())?;
    let raw = location.as_str();
    let suffix = format!("/{}/{}", resource.path_segment(), claim.resource_id);
    if claim.resource_id.is_empty()
        || claim.resource_id.contains('/')
        || raw.contains(['?', '#'])
        || !raw.ends_with(&suffix)
    {
        return Err(invalid());
    }
    Ok(location)
}

/// What one [`StorageCleanupWorker::run_once`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    /// No task was ready.
    Idle,
    /// A task was cleaned up and finished.
    Finished,
    /// A task failed and waits for the retry backoff.
    Failed,
}

/// Processes cleanup tasks. See the module docs.
pub struct StorageCleanupWorker {
    metastore: DynMetastore,
    storage: Arc<dyn CleanupStorage>,
    config: CleanupConfig,
}

impl StorageCleanupWorker {
    pub fn new(
        metastore: DynMetastore,
        storage: Arc<dyn CleanupStorage>,
        config: CleanupConfig,
    ) -> Self {
        Self {
            metastore,
            storage,
            config,
        }
    }

    /// Claim and process at most one task.
    pub async fn run_once(&self) -> uc_common::Result<Attempt> {
        let tasks = self.metastore.cleanup();
        let Some(claim) = tasks
            .claim(self.config.lease_duration, self.config.initial_delay)
            .await?
        else {
            return Ok(Attempt::Idle);
        };
        let attempt = tokio::time::timeout(self.config.attempt_timeout, self.cleanup(&claim));
        let outcome = attempt.await.unwrap_or(Err(CleanupError::TimedOut));
        match outcome {
            Ok(()) => {
                tasks.finish(&claim.resource_id, &claim.lease_token).await?;
                Ok(Attempt::Finished)
            }
            Err(error) => {
                warn!(
                    resource_type = %claim.resource_type,
                    resource_id = %claim.resource_id,
                    location = %claim.storage_location,
                    %error,
                    "storage cleanup failed; will retry after backoff"
                );
                tasks
                    .report_failure(
                        &claim.resource_id,
                        &claim.lease_token,
                        &error.safe_message(),
                        self.config.retry_backoff,
                    )
                    .await?;
                Ok(Attempt::Failed)
            }
        }
    }

    /// Process ready tasks until none is left or one fails; the number
    /// finished.
    pub async fn run_ready(&self) -> uc_common::Result<usize> {
        let mut finished = 0;
        while self.run_once().await? == Attempt::Finished {
            finished += 1;
        }
        Ok(finished)
    }

    async fn cleanup(&self, claim: &CleanupClaim) -> Result<(), CleanupError> {
        let location = validate_task(claim)?;
        self.storage.delete_prefix(&location).await
    }

    /// Poll on a background task, one poll interval after the previous
    /// poll ends, until the handle is aborted. An aborted attempt keeps its
    /// task; the task is claimable again when the lease expires.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(self.config.poll_interval).await;
                if let Err(error) = self.run_ready().await {
                    warn!(%error, "storage cleanup poll failed; will retry on the next interval");
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_rules_follow_upstream() {
        assert!(CleanupConfig::default().validate().is_ok());
        let short_lease = CleanupConfig {
            lease_duration: Duration::from_secs(60),
            attempt_timeout: Duration::from_secs(60),
            ..Default::default()
        };
        assert_eq!(
            short_lease.validate().unwrap_err(),
            "Storage cleanup lease duration must exceed the attempt timeout"
        );
    }

    #[test]
    fn task_locations_must_end_with_the_resource_id() {
        let claim = |kind: &str, id: &str, location: &str| CleanupClaim {
            resource_type: kind.into(),
            resource_id: id.into(),
            storage_location: location.into(),
            lease_token: "t".into(),
        };
        let ok = [
            claim("TABLE", "a1", "s3://b/__unitystorage/catalogs/c/tables/a1"),
            claim("VOLUME", "v1", "file:///tmp/x/volumes/v1"),
            claim("REGISTERED_MODEL", "m1", "gs://b/models/m1"),
            claim(
                "MODEL_VERSION",
                "n1",
                "abfss://c@a.dfs.core.windows.net/models/m1/versions/n1",
            ),
        ];
        for c in ok {
            assert!(validate_task(&c).is_ok(), "{c:?}");
        }
        let bad = [
            claim("TABLE", "a1", "s3://b/tables/a2"),
            claim("TABLE", "a1", "s3://b/volumes/a1"),
            claim("TABLE", "", "s3://b/tables/"),
            claim("SCHEMA", "a1", "s3://b/schemas/a1"),
            claim("TABLE", "a1", "s3://b/tables/a1?x=1"),
            claim("TABLE", "a1", "s3://b/tables/a1#f"),
            claim("TABLE", "x/a1", "s3://b/tables/x/a1"),
        ];
        for c in bad {
            assert!(
                matches!(validate_task(&c), Err(CleanupError::InvalidTask(_))),
                "{c:?}"
            );
        }
    }
}
