use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken, SecurablePropertiesMap, serialize_properties};

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SchemaInfo {
    /// Name of schema, relative to parent catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Name of parent catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(serialize_with = "serialize_properties")]
    pub properties: Option<SecurablePropertiesMap>,

    /// Full name of schema, in form of `<catalog>.<schema>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSchema {
    /// Name of schema, relative to parent catalog.
    pub name: String,
    pub catalog_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SecurablePropertiesMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_root: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SecurablePropertiesMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListSchemasResponse {
    pub schemas: Vec<SchemaInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}
