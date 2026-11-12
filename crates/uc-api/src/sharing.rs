//! The admin types of sharing: shares and recipients, with the shape of
//! upstream UC's sharing service. Their callers are UC principals.
//!
//! A recipient is not a UC principal: it authenticates with a bearer token
//! bound to a recipient row, and reads its shares with the Delta Sharing or
//! Open Sharing protocol.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ShareInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    /// The objects of the share: each UC table or volume and the
    /// `<share-schema>.<name>` that recipients see.
    #[serde(default)]
    pub objects: Vec<SharedDataObject>,
}

/// What a share object is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SharedObjectType {
    #[default]
    Table,
    Volume,
    /// A volume that holds an agent skill: a `SKILL.md` and its files.
    AgentSkill,
}

/// Whether recipients read the history of a shared table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HistoryDataSharingStatus {
    /// Recipients read the latest version only.
    #[default]
    Disabled,
    /// Recipients read older versions too, by version or by time.
    Enabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SharedDataObject {
    /// `<catalog>.<schema>.<name>` of the UC table or volume. An agent skill
    /// names the volume that holds it.
    pub source_full_name: String,
    #[serde(default)]
    pub data_object_type: SharedObjectType,
    /// Schema the recipient sees the object under (e.g. `default`).
    pub shared_as_schema: String,
    /// Name the recipient sees.
    pub shared_as_name: String,
    /// Of a table: whether recipients read its history.
    #[serde(default)]
    pub history_data_sharing_status: HistoryDataSharingStatus,
    /// Of a table that shares its history: the lowest version recipients
    /// read. Without it, they read from version 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_version: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CreateShare {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateShare {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
    #[serde(default)]
    pub add_objects: Vec<SharedDataObject>,
    #[serde(default)]
    pub remove_objects: Vec<SharedDataObject>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListSharesResponse {
    pub items: Vec<ShareInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct RecipientInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `TOKEN` (bearer) or `OAUTH_CLIENT_CREDENTIALS`. Only `TOKEN` is
    /// implemented.
    #[serde(default)]
    pub authentication_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Bearer-token plaintext. Only returned by `POST /recipients` (or
    /// `POST /recipients/{name}/rotate-token`); subsequent `GET`s redact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_prefix: Option<String>,
    /// Names of shares this recipient is granted access to.
    #[serde(default)]
    pub shares: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CreateRecipient {
    pub name: String,
    #[serde(default)]
    pub authentication_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateRecipient {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default)]
    pub add_shares: Vec<String>,
    #[serde(default)]
    pub remove_shares: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListRecipientsResponse {
    pub items: Vec<RecipientInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}
