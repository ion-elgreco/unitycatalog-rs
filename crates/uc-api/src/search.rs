use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken, SecurableType};

/// An object whose name or comment holds the words of a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SearchResult {
    /// `catalog`, `schema`, `table`, `volume`, `function` or
    /// `registered_model`.
    pub securable_type: SecurableType,
    /// `<catalog>`, `<catalog>.<schema>` or `<catalog>.<schema>.<name>`.
    pub full_name: String,
    pub catalog_name: String,
    /// Absent for a catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub created_at: EpochMillis,
    pub updated_at: EpochMillis,
    pub id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SearchObjectsResponse {
    pub results: Vec<SearchResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}
