//! Iceberg REST catalog wire models, after the Apache Iceberg REST spec as
//! upstream serves it (kebab-case). Table metadata, schemas, partition
//! specs, sort orders, requirements and updates travel as JSON and are
//! parsed by the server with the `iceberg` crate.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergConfigResponse {
    pub defaults: BTreeMap<String, String>,
    pub overrides: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergListNamespacesResponse {
    pub namespaces: Vec<Vec<String>>,
    #[serde(rename = "next-page-token", skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergNamespaceResponse {
    pub namespace: Vec<String>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct IcebergCreateNamespaceRequest {
    #[serde(default)]
    pub namespace: Option<Vec<String>>,
    #[serde(default)]
    pub properties: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct IcebergUpdateNamespacePropertiesRequest {
    #[serde(default)]
    pub removals: Option<Vec<String>>,
    #[serde(default)]
    pub updates: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergUpdateNamespacePropertiesResponse {
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergTableIdentifier {
    pub namespace: Vec<String>,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergListTablesResponse {
    pub identifiers: Vec<IcebergTableIdentifier>,
    #[serde(rename = "next-page-token", skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct IcebergLoadTableResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_location: Option<String>,
    /// Iceberg table metadata.
    pub metadata: Value,
    pub config: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct IcebergCreateTableRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    /// An Iceberg schema.
    #[serde(default)]
    pub schema: Option<Value>,
    /// An Iceberg partition spec.
    #[serde(default)]
    pub partition_spec: Option<Value>,
    /// An Iceberg sort order.
    #[serde(default)]
    pub write_order: Option<Value>,
    #[serde(default)]
    pub stage_create: Option<bool>,
    #[serde(default)]
    pub properties: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct IcebergCommitTableRequest {
    #[serde(default)]
    pub identifier: Option<IcebergTableIdentifier>,
    /// Iceberg update requirements.
    #[serde(default)]
    pub requirements: Vec<Value>,
    /// Iceberg metadata updates.
    #[serde(default)]
    pub updates: Vec<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct IcebergRenameTableRequest {
    #[serde(default)]
    pub source: Option<IcebergTableIdentifier>,
    #[serde(default)]
    pub destination: Option<IcebergTableIdentifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergStorageCredential {
    pub prefix: String,
    pub config: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct IcebergLoadCredentialsResponse {
    pub storage_credentials: Vec<IcebergStorageCredential>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergErrorModel {
    pub message: String,
    /// The Iceberg exception, e.g. `NoSuchTableException`.
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: u16,
}

/// The body of every Iceberg REST error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IcebergErrorResponse {
    pub error: IcebergErrorModel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_round_trip;
    use serde_json::json;

    #[test]
    fn bodies_use_kebab_case_names() {
        assert_round_trip::<IcebergListNamespacesResponse>(json!({
            "namespaces": [["default"]],
            "next-page-token": "p2",
        }));
        assert_round_trip::<IcebergListTablesResponse>(json!({
            "identifiers": [{ "namespace": ["default"], "name": "t" }],
            "next-page-token": "p2",
        }));
        assert_round_trip::<IcebergCreateTableRequest>(json!({
            "name": "t",
            "location": "s3://bucket/t",
            "schema": { "type": "struct", "schema-id": 0, "fields": [] },
            "partition-spec": { "spec-id": 0, "fields": [] },
            "write-order": { "order-id": 0, "fields": [] },
            "stage-create": false,
            "properties": { "k": "v" },
        }));
        assert_round_trip::<IcebergLoadTableResponse>(json!({
            "metadata-location": "s3://bucket/t/metadata/00001.metadata.json",
            "metadata": { "format-version": 2 },
            "config": {},
        }));
        assert_round_trip::<IcebergLoadCredentialsResponse>(json!({
            "storage-credentials": [{ "prefix": "s3://bucket/t", "config": { "s3.access-key-id": "id" } }],
        }));
        assert_round_trip::<IcebergErrorResponse>(json!({
            "error": { "message": "Table does not exist: default.t", "type": "NoSuchTableException", "code": 404 },
        }));
    }

    #[test]
    fn requests_may_leave_out_every_field() {
        // The server answers with Iceberg's messages, such as `Invalid namespace: null`.
        let commit: IcebergCommitTableRequest = serde_json::from_value(json!({})).unwrap();
        assert!(commit.identifier.is_none());
        assert!(commit.requirements.is_empty());
        assert!(commit.updates.is_empty());
        serde_json::from_value::<IcebergCreateNamespaceRequest>(json!({})).unwrap();
        serde_json::from_value::<IcebergUpdateNamespacePropertiesRequest>(json!({})).unwrap();
        serde_json::from_value::<IcebergCreateTableRequest>(json!({})).unwrap();
        serde_json::from_value::<IcebergRenameTableRequest>(json!({})).unwrap();
    }
}
