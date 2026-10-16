//! `/delta/preview/commits`: the Unity Catalog commit coordinator for
//! MANAGED Delta tables. Upstream `DeltaCommit` and friends.
//!
//! Request fields are optional on the wire so that missing fields get
//! upstream's validation messages, not a parse error.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ColumnInfo;
use uc_common::EpochMillis;

/// A commit, a backfill report, or both. At least one of `commit_info` and
/// `latest_backfilled_version` must be present.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaCommit {
    /// The id of the table to commit to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
    /// The storage location of the table. Must match the registered one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_info: Option<DeltaCommitInfo>,
    /// Commits up to this version are backfilled into `_delta_log`, so the
    /// server stops tracking them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_backfilled_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<DeltaMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform: Option<DeltaUniform>,
}

/// One Delta commit file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaCommitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// The in-commit timestamp written by the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<EpochMillis>,
    /// The UUID-based commit file name under `_delta_log/_staged_commits`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_modification_timestamp: Option<EpochMillis>,
}

/// A Delta metadata change to apply to the table with the commit.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaMetadata {
    /// The table comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<ColumnInfos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<DeltaCommitMetadataProperties>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ColumnInfos {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ColumnInfo>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaCommitMetadataProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, String>>,
}

/// UniForm metadata committed with a Delta commit.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaUniform {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iceberg: Option<DeltaUniformIceberg>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaUniformIceberg {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_delta_version: Option<i64>,
    /// An ISO-8601 instant in UTC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_delta_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_converted_delta_version: Option<i64>,
}

/// The commit response has no fields; upstream sends an empty body.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaCommitResponse {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaGetCommits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_uri: Option<String>,
    /// First version to return (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_version: Option<i64>,
    /// Last version to return (inclusive). Does not change
    /// `latest_table_version`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_version: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaGetCommitsResponse {
    /// Unbackfilled commits, newest first.
    pub commits: Vec<DeltaCommitInfo>,
    /// The latest version tracked by the server; 0 before the first commit.
    pub latest_table_version: i64,
}
