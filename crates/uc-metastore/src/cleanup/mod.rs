//! Storage cleanup tasks. Upstream `StorageCleanupTaskRepository`.
//!
//! Dropping a managed table, volume, model or model version queues a task
//! in the same transaction as the drop. A background worker claims tasks
//! with a lease, deletes the files, then finishes the task or reports a
//! failure that delays the next attempt.

pub(crate) mod sql;

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use uc_common::Result;

/// A claimed task. The lease token proves ownership to
/// [`CleanupOps::finish`] and
/// [`CleanupOps::report_failure`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupClaim {
    /// Upstream `ManagedResourceType` name (`TABLE`, `VOLUME`, ...).
    pub resource_type: String,
    pub resource_id: String,
    pub storage_location: String,
    pub lease_token: String,
}

/// A queued task. Times are epoch millis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupTask {
    pub resource_type: String,
    #[serde(rename = "id_string")]
    pub resource_id: String,
    pub name: String,
    pub storage_location: String,
    pub deleted_at: i64,
    #[serde(default)]
    pub lease_expires_at: Option<i64>,
    pub failure_count: i64,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[async_trait]
pub trait CleanupOps: Send + Sync {
    /// Claim one task that was deleted at least `initial_delay` ago and has
    /// no live lease, the most overdue first. `None` when no task is ready.
    async fn claim(&self, lease: Duration, initial_delay: Duration)
    -> Result<Option<CleanupClaim>>;

    /// Remove a task while its lease is live. `false` when the lease expired
    /// or changed hands.
    async fn finish(&self, resource_id: &str, lease_token: &str) -> Result<bool>;

    /// Release a live lease, count the failure and wait `retry_backoff`
    /// before the next attempt. `false` when the lease is no longer held.
    async fn report_failure(
        &self,
        resource_id: &str,
        lease_token: &str,
        error: &str,
        retry_backoff: Duration,
    ) -> Result<bool>;

    /// All queued tasks, oldest first.
    async fn list_tasks(&self) -> Result<Vec<CleanupTask>>;
}
