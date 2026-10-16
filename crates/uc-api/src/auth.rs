use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken};

/// Personal Access Token metadata. Only `createPat` returns the plaintext
/// `token_value`; the server does not store it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PatInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The first 8 characters of the token, for display only. The rest
    /// cannot be recovered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<EpochMillis>,
    /// Plaintext token — only populated on the response of `createPat`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePat {
    pub name: String,
    /// Optional expiry in epoch milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<EpochMillis>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListPatsResponse {
    pub tokens: Vec<PatInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}
