//! UC Delta REST API (`/delta/v1`) wire models, after upstream
//! `api/delta.yaml`. Field names are kebab-case.
//!
//! Column schemas (`columns`) travel as Delta type JSON; they are kept as
//! [`serde_json::Value`] here and parsed with
//! [`crate::delta_types`] ([`KeyStyle::Kebab`](crate::delta_types::KeyStyle)).
//! Request fields are optional so that missing fields get upstream's
//! validation messages, not a parse error.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The type of a Delta table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum DeltaTableType {
    #[default]
    Managed,
    External,
}

/// The access level of a storage credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeltaCredentialOperation {
    Read,
    ReadWrite,
}

/// A maintenance operation the catalog allows on a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeltaMaintenanceOperation {
    DataReorganization,
    DataCleanup,
    MetadataCleanup,
}

/// Cloud credential settings. Only the keys of one provider are set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaStorageCredentialConfig {
    #[serde(rename = "s3.access-key-id", skip_serializing_if = "Option::is_none")]
    pub s3_access_key_id: Option<String>,
    #[serde(
        rename = "s3.secret-access-key",
        skip_serializing_if = "Option::is_none"
    )]
    pub s3_secret_access_key: Option<String>,
    #[serde(rename = "s3.session-token", skip_serializing_if = "Option::is_none")]
    pub s3_session_token: Option<String>,
    #[serde(rename = "azure.sas-token", skip_serializing_if = "Option::is_none")]
    pub azure_sas_token: Option<String>,
    #[serde(rename = "gcs.oauth-token", skip_serializing_if = "Option::is_none")]
    pub gcs_oauth_token: Option<String>,
}

/// A temporary credential for a storage prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaStorageCredential {
    pub prefix: String,
    pub operation: DeltaCredentialOperation,
    pub config: DeltaStorageCredentialConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_time_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaCredentialsResponse {
    pub storage_credentials: Vec<DeltaStorageCredential>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaCatalogConfig {
    pub endpoints: Vec<String>,
    pub protocol_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaCreateStagingTableRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Delta protocol versions and features.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaProtocol {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_reader_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_writer_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reader_features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writer_features: Option<Vec<String>>,
}

/// Features a client should enable when it can.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaSuggestedProtocol {
    pub reader_features: Vec<String>,
    pub writer_features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaStagingTableResponse {
    pub table_id: String,
    pub table_type: DeltaTableType,
    pub location: String,
    pub storage_credentials: Vec<DeltaStorageCredential>,
    pub required_protocol: DeltaProtocol,
    pub suggested_protocol: DeltaSuggestedProtocol,
    /// A `null` value means any valid value.
    pub required_properties: BTreeMap<String, Option<String>>,
    /// A `null` value means the client generates the value.
    pub suggested_properties: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaClusteringDomainMetadata {
    #[serde(rename = "clusteringColumns", skip_serializing_if = "Option::is_none")]
    pub clustering_columns: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaRowTrackingDomainMetadata {
    #[serde(rename = "rowIdHighWaterMark", skip_serializing_if = "Option::is_none")]
    pub row_id_high_water_mark: Option<i64>,
}

/// Domain metadata entries keyed by domain name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaDomainMetadataUpdates {
    #[serde(rename = "delta.clustering", skip_serializing_if = "Option::is_none")]
    pub delta_clustering: Option<DeltaClusteringDomainMetadata>,
    #[serde(rename = "delta.rowTracking", skip_serializing_if = "Option::is_none")]
    pub delta_row_tracking: Option<DeltaRowTrackingDomainMetadata>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaUniformMetadataIceberg {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_delta_version: Option<i64>,
    /// Epoch millis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_delta_timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_converted_delta_version: Option<i64>,
}

/// UniForm conversion metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaUniformMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iceberg: Option<DeltaUniformMetadataIceberg>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaCreateTableRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_type: Option<DeltaTableType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// A Delta struct type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_columns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<DeltaProtocol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_metadata: Option<DeltaDomainMetadataUpdates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit_timestamp_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform: Option<DeltaUniformMetadata>,
}

/// A catalog-coordinated commit (upstream `delta.yaml` `DeltaCommit`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
#[schema(as = DeltaApiCommit)]
pub struct DeltaCommitWire {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_modification_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaTableMetadata {
    pub etag: String,
    pub table_type: DeltaTableType,
    pub table_uuid: String,
    pub location: String,
    pub created_time: i64,
    pub updated_time: i64,
    /// A Delta struct type.
    pub columns: Value,
    pub partition_columns: Vec<String>,
    pub properties: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit_timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaLoadTableResponse {
    pub metadata: DeltaTableMetadata,
    /// Unbackfilled commits, newest first (MANAGED tables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits: Option<Vec<DeltaCommitWire>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform: Option<DeltaUniformMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_table_version: Option<i64>,
    pub allowed_maintenance_operations: Vec<DeltaMaintenanceOperation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaRenameTableRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
}

/// A precondition of an update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DeltaTableRequirement {
    AssertTableUuid {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uuid: Option<String>,
    },
    AssertEtag {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        etag: Option<String>,
    },
}

/// One update action. All actions of a request apply atomically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(
    tag = "action",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
pub enum DeltaTableUpdate {
    SetProperties {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        updates: Option<BTreeMap<String, String>>,
    },
    RemoveProperties {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        removals: Option<Vec<String>>,
    },
    SetColumns {
        /// A Delta struct type.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<Value>,
    },
    SetTableComment {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
    AddCommit {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        commit: Option<DeltaCommitWire>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uniform: Option<DeltaUniformMetadata>,
    },
    SetLatestBackfilledVersion {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        latest_published_version: Option<i64>,
    },
    SetProtocol {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        protocol: Option<DeltaProtocol>,
    },
    SetDomainMetadata {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        updates: Option<DeltaDomainMetadataUpdates>,
    },
    RemoveDomainMetadata {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        domains: Option<Vec<String>>,
    },
    SetPartitionColumns {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        partition_columns: Option<Vec<String>>,
    },
    UpdateMetadataSnapshotVersion {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_commit_version: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_commit_timestamp_ms: Option<i64>,
    },
}

impl DeltaTableUpdate {
    /// The wire name of the action, e.g. `set-properties`.
    pub fn action(&self) -> &'static str {
        match self {
            Self::SetProperties { .. } => "set-properties",
            Self::RemoveProperties { .. } => "remove-properties",
            Self::SetColumns { .. } => "set-columns",
            Self::SetTableComment { .. } => "set-table-comment",
            Self::AddCommit { .. } => "add-commit",
            Self::SetLatestBackfilledVersion { .. } => "set-latest-backfilled-version",
            Self::SetProtocol { .. } => "set-protocol",
            Self::SetDomainMetadata { .. } => "set-domain-metadata",
            Self::RemoveDomainMetadata { .. } => "remove-domain-metadata",
            Self::SetPartitionColumns { .. } => "set-partition-columns",
            Self::UpdateMetadataSnapshotVersion { .. } => "update-metadata-snapshot-version",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DeltaUpdateTableRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<DeltaTableRequirement>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updates: Option<Vec<DeltaTableUpdate>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DeltaReportMetricsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
    /// Commit metrics; accepted and not stored, as upstream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaErrorModel {
    pub message: String,
    /// A `DeltaErrorType` such as `NoSuchTableException`.
    #[serde(rename = "type")]
    pub error_type: String,
    /// The HTTP status.
    pub code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<Vec<String>>,
}

/// The body of every UC Delta API error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeltaErrorResponse {
    pub error: DeltaErrorModel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{assert_round_trip, assert_wire_names};
    use serde_json::json;

    #[test]
    fn enums_match_upstream() {
        assert_wire_names(&[
            (DeltaTableType::Managed, "MANAGED"),
            (DeltaTableType::External, "EXTERNAL"),
        ]);
        assert_wire_names(&[
            (DeltaCredentialOperation::Read, "READ"),
            (DeltaCredentialOperation::ReadWrite, "READ_WRITE"),
        ]);
        assert_wire_names(&[
            (
                DeltaMaintenanceOperation::DataReorganization,
                "DATA_REORGANIZATION",
            ),
            (DeltaMaintenanceOperation::DataCleanup, "DATA_CLEANUP"),
            (
                DeltaMaintenanceOperation::MetadataCleanup,
                "METADATA_CLEANUP",
            ),
        ]);
    }

    #[test]
    fn requirements_and_updates_parse_without_their_fields() {
        for kind in ["assert-table-uuid", "assert-etag"] {
            let requirement: DeltaTableRequirement =
                serde_json::from_value(json!({ "type": kind })).unwrap();
            assert_eq!(
                serde_json::to_value(requirement).unwrap(),
                json!({ "type": kind })
            );
        }
        // Upstream's actions, in `delta.yaml` order.
        for action in [
            "set-properties",
            "remove-properties",
            "set-columns",
            "set-table-comment",
            "add-commit",
            "set-latest-backfilled-version",
            "set-protocol",
            "set-domain-metadata",
            "remove-domain-metadata",
            "set-partition-columns",
            "update-metadata-snapshot-version",
        ] {
            let update: DeltaTableUpdate =
                serde_json::from_value(json!({ "action": action })).unwrap();
            assert_eq!(update.action(), action);
            assert_eq!(
                serde_json::to_value(update).unwrap(),
                json!({ "action": action })
            );
        }
    }

    /// Every field of every requirement and update.
    #[test]
    fn update_request_uses_upstream_names() {
        assert_round_trip::<DeltaUpdateTableRequest>(json!({
            "requirements": [
                { "type": "assert-table-uuid", "uuid": "8e1b0c3e-0f6c-4a4e-9a4a-6c1f0e6f9d2b" },
                { "type": "assert-etag", "etag": "etag-1000" },
            ],
            "updates": [
                { "action": "set-properties", "updates": { "k": "v" } },
                { "action": "remove-properties", "removals": ["old"] },
                { "action": "set-columns", "columns": { "type": "struct", "fields": [] } },
                { "action": "set-table-comment", "comment": "c" },
                {
                    "action": "add-commit",
                    "commit": {
                        "version": 2,
                        "timestamp": 2000,
                        "file-name": "00000000000000000002.3b5f1e1e-6a2b-4c51-9d2b-5d6a3e8f1c2a.json",
                        "file-size": 100,
                        "file-modification-timestamp": 2001,
                    },
                    "uniform": {
                        "iceberg": {
                            "metadata-location": "s3://bucket/t/metadata/00002.metadata.json",
                            "converted-delta-version": 2,
                            "converted-delta-timestamp": 2002,
                            "base-converted-delta-version": 1,
                        },
                    },
                },
                { "action": "set-latest-backfilled-version", "latest-published-version": 1 },
                {
                    "action": "set-protocol",
                    "protocol": {
                        "min-reader-version": 3,
                        "min-writer-version": 7,
                        "reader-features": ["deletionVectors"],
                        "writer-features": ["catalogManaged"],
                    },
                },
                {
                    "action": "set-domain-metadata",
                    "updates": {
                        "delta.clustering": { "clusteringColumns": [["a"], ["b", "c"]] },
                        "delta.rowTracking": { "rowIdHighWaterMark": 9 },
                    },
                },
                { "action": "remove-domain-metadata", "domains": ["delta.clustering"] },
                { "action": "set-partition-columns", "partition-columns": ["p"] },
                {
                    "action": "update-metadata-snapshot-version",
                    "last-commit-version": 2,
                    "last-commit-timestamp-ms": 2000,
                },
            ],
        }));
    }

    #[test]
    fn requests_use_upstream_names() {
        assert_round_trip::<DeltaCreateTableRequest>(json!({
            "name": "t",
            "location": "s3://bucket/t",
            "table-type": "MANAGED",
            "comment": "c",
            "columns": { "type": "struct", "fields": [] },
            "partition-columns": ["p"],
            "protocol": { "min-reader-version": 3, "min-writer-version": 7 },
            "properties": { "k": "v" },
            "domain-metadata": { "delta.clustering": { "clusteringColumns": [["p"]] } },
            "last-commit-timestamp-ms": 1000,
            "uniform": { "iceberg": { "converted-delta-version": 0 } },
        }));
        assert_round_trip::<DeltaRenameTableRequest>(json!({ "new-name": "u" }));
        assert_round_trip::<DeltaReportMetricsRequest>(json!({
            "table-id": "t1",
            "report": { "num-files": 1 },
        }));
    }

    #[test]
    fn responses_use_upstream_names() {
        assert_round_trip::<DeltaStagingTableResponse>(json!({
            "table-id": "t1",
            "table-type": "MANAGED",
            "location": "s3://bucket/t1",
            "storage-credentials": [{
                "prefix": "s3://bucket/t1",
                "operation": "READ_WRITE",
                "config": {
                    "s3.access-key-id": "id",
                    "s3.secret-access-key": "secret",
                    "s3.session-token": "token",
                },
                "expiration-time-ms": 1000,
            }],
            "required-protocol": { "min-reader-version": 3, "min-writer-version": 7 },
            "suggested-protocol": { "reader-features": [], "writer-features": ["rowTracking"] },
            // `null`: any value, or one that the client makes.
            "required-properties": {
                "delta.enableRowTracking": "true",
                "delta.rowTracking.materializedRowIdColumnName": null,
            },
            "suggested-properties": { "delta.columnMapping.maxColumnId": null },
        }));
        assert_round_trip::<DeltaCredentialsResponse>(json!({
            "storage-credentials": [
                {
                    "prefix": "abfss://container@account.dfs.core.windows.net/t1",
                    "operation": "READ",
                    "config": { "azure.sas-token": "sas" },
                },
                { "prefix": "gs://bucket/t1", "operation": "READ", "config": { "gcs.oauth-token": "token" } },
            ],
        }));
        assert_round_trip::<DeltaLoadTableResponse>(json!({
            "metadata": {
                "etag": "etag-2000",
                "table-type": "MANAGED",
                "table-uuid": "t1",
                "location": "s3://bucket/t1",
                "created-time": 1000,
                "updated-time": 2000,
                "columns": { "type": "struct", "fields": [] },
                "partition-columns": [],
                "properties": {},
                "last-commit-version": 2,
                "last-commit-timestamp-ms": 2000,
            },
            "commits": [{
                "version": 2,
                "timestamp": 2000,
                "file-name": "00000000000000000002.3b5f1e1e-6a2b-4c51-9d2b-5d6a3e8f1c2a.json",
                "file-size": 100,
                "file-modification-timestamp": 2001,
            }],
            "uniform": { "iceberg": { "metadata-location": "s3://bucket/t1/metadata/00002.metadata.json" } },
            "latest-table-version": 2,
            "allowed-maintenance-operations": ["DATA_CLEANUP"],
        }));
        assert_round_trip::<DeltaCatalogConfig>(json!({
            "endpoints": ["GET /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}"],
            "protocol-version": "1.0",
        }));
        assert_round_trip::<DeltaErrorResponse>(json!({
            "error": { "message": "m", "type": "NoSuchTableException", "code": 404, "stack": [] },
        }));
    }
}
