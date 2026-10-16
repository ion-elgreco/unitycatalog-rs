//! `/admin/backup` and `/admin/restore`: a snapshot of the whole catalog,
//! and what a restore imported.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A snapshot of the catalog: the rows of each table of the store.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BackupResponse {
    pub version: u32,
    #[schema(value_type = Object)]
    pub tables: BTreeMap<String, Vec<serde_json::Value>>,
}

/// What a restore imported: rows per table, and the tables it skipped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RestoreResponse {
    #[schema(value_type = Object)]
    pub imported: BTreeMap<String, usize>,
    pub skipped: Vec<String>,
}
