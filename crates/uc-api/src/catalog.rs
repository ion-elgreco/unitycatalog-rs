use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken, SecurablePropertiesMap, serialize_properties};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct CatalogInfo {
    /// Name of catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// User-provided free-form text description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(serialize_with = "serialize_properties")]
    pub properties: Option<SecurablePropertiesMap>,

    /// Username of current owner of catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Time at which this catalog was created, in epoch milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<EpochMillis>,

    /// Username of catalog creator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// Time at which this catalog was last modified, in epoch milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<EpochMillis>,

    /// Username of user who last modified catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,

    /// Unique identifier for the catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Storage root URL for managed storage location of catalog.
    /// Example: `s3://bucket/ucroot`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_root: Option<String>,

    /// Storage Location URL (full path) for managed storage location of catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<String>,
    /// True when the object has only the discovery fields that `BROWSE`
    /// allows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browse_only: Option<bool>,
}

impl CatalogInfo {
    /// What `BROWSE` shows of a catalog: every field.
    pub fn into_browse_only(self) -> Self {
        Self {
            browse_only: Some(true),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCatalog {
    /// Name of catalog.
    pub name: String,

    /// User-provided free-form text description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SecurablePropertiesMap>,

    /// Storage root URL for managed storage location of catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct UpdateCatalog {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SecurablePropertiesMap>,

    /// New name for the catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ListCatalogsResponse {
    /// An array of catalog information objects.
    pub catalogs: Vec<CatalogInfo>,

    /// Opaque token to retrieve the next page of results.
    /// Absent if there are no more pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}
