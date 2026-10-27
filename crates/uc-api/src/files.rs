//! The Files API, as the Databricks Files API has it.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::EpochMillis;

/// A file or directory in a directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DirectoryEntry {
    /// `/Volumes/<catalog>/<schema>/<volume>/...` or
    /// `/Tables/<catalog>/<schema>/<table>/...`.
    pub path: String,
    /// The last part of the path.
    pub name: String,
    pub is_directory: bool,
    /// Absent for a directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    /// Absent for a directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<EpochMillis>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListDirectoryContentsResponse {
    /// In each page, the directories come first.
    pub contents: Vec<DirectoryEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}
