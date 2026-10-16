//! Full-state export and import, for the admin endpoints.
//!
//! A backup is a full snapshot, held in memory: there is no incremental
//! backup or per-table filter. For large deployments on PostgreSQL, use
//! `pg_dump`. A snapshot also moves a catalog between databases.

pub(crate) mod sql;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use uc_common::Result;

/// Snapshot format version; restore takes only this one.
pub const BACKUP_VERSION: u32 = 1;

/// Whole-catalog snapshot. Map keyed by table name → array of row objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub tables: BTreeMap<String, Vec<serde_json::Value>>,
}

impl Snapshot {
    pub fn empty() -> Self {
        Self {
            version: BACKUP_VERSION,
            tables: BTreeMap::new(),
        }
    }
}

#[async_trait]
pub trait BackupOps: Send + Sync {
    /// Dump all UC tables (excludes refinery's `refinery_schema_history`
    /// and the audit log) into a single [`Snapshot`] value.
    async fn export_snapshot(&self) -> Result<Snapshot>;

    /// Restore a snapshot. Wipes the destination tables first, then inserts
    /// each row. A `version` other than [`BACKUP_VERSION`] fails fast.
    async fn import_snapshot(&self, snap: Snapshot) -> Result<RestoreReport>;
}

/// What a restore did: the rows imported per table, and the rows skipped.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RestoreReport {
    pub imported: BTreeMap<String, usize>,
    pub skipped: Vec<String>,
}
