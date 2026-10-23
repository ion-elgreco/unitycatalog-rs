//! Delta Sharing protocol types.
//!
//! The admin types (`Share` and `Recipient` CRUD) have the shape of upstream
//! UC's sharing service; their callers are UC principals. The protocol types
//! (`/shares/{share}/...` reads) are for the recipient of a share. A
//! recipient is not a UC principal: it authenticates with a bearer token
//! bound to a recipient row.
//!
//! The Delta Sharing REST API spec:
//! <https://github.com/delta-io/delta-sharing/blob/main/PROTOCOL.md>.

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
    /// The tables of the share: each UC table `<catalog>.<schema>.<table>`
    /// and the `<share-schema>.<share-table>` name that recipients see.
    #[serde(default)]
    pub objects: Vec<SharedDataObject>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SharedDataObject {
    /// The shared Delta table, `<catalog>.<schema>.<table>`.
    pub source_table_full_name: String,
    /// The schema recipients see the table in; by default the table's
    /// schema.
    #[serde(default)]
    pub shared_as_schema: String,
    /// The name recipients see; by default the table's name.
    #[serde(default)]
    pub shared_as_name: String,
    /// The table's id in the share, which the server assigns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Whether recipients may read the table's earlier versions and its
    /// changes, or only its latest version.
    #[serde(default)]
    pub history_data_sharing_status: HistoryDataSharingStatus,
    /// With history: the first version recipients may read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_version: Option<i64>,
}

impl SharedDataObject {
    pub fn history_shared(&self) -> bool {
        self.history_data_sharing_status == HistoryDataSharingStatus::Enabled
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HistoryDataSharingStatus {
    Enabled,
    #[default]
    Disabled,
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
    /// Tables to share. A table already in the share stays as it is:
    /// remove it and add it in one update to change it.
    #[serde(default)]
    pub add_objects: Vec<SharedDataObject>,
    /// Tables to stop sharing, by `source_table_full_name`; removed
    /// before `add_objects` are added.
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

// As the protocol, the page token is `nextPageToken`, and a table's version
// travels in the `Delta-Table-Version` header, not in a body.

/// `GET /shares` (recipient-side) — list of shares the recipient can see.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SharingListSharesResponse {
    pub items: Vec<SharingShare>,
    #[serde(
        rename = "nextPageToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SharingShare {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// `GET /shares/{share}` (recipient-side).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SharingGetShareResponse {
    pub share: SharingShare,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SharingListSchemasResponse {
    pub items: Vec<SharingSchema>,
    #[serde(
        rename = "nextPageToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SharingSchema {
    pub name: String,
    pub share: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SharingListTablesResponse {
    pub items: Vec<SharingTable>,
    #[serde(
        rename = "nextPageToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SharingTable {
    pub name: String,
    pub schema: String,
    pub share: String,
    #[serde(rename = "shareId", default, skip_serializing_if = "Option::is_none")]
    pub share_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_round_trip;
    use serde_json::json;

    #[test]
    fn protocol_lists_use_protocol_names() {
        assert_round_trip::<SharingListSharesResponse>(json!({
            "items": [{ "name": "analytics", "id": "1" }],
            "nextPageToken": "p2",
        }));
        assert_round_trip::<SharingListSchemasResponse>(json!({
            "items": [{ "name": "default", "share": "analytics" }],
            "nextPageToken": "p2",
        }));
        assert_round_trip::<SharingListTablesResponse>(json!({
            "items": [{
                "name": "events", "schema": "default", "share": "analytics", "shareId": "1",
                "id": "2",
            }],
            "nextPageToken": "p2",
        }));
    }

    #[test]
    fn shared_tables_default_to_no_history() {
        let object: SharedDataObject =
            serde_json::from_value(json!({ "source_table_full_name": "main.default.events" }))
                .unwrap();
        assert!(!object.history_shared());
        assert_round_trip::<SharedDataObject>(json!({
            "source_table_full_name": "main.default.events",
            "shared_as_schema": "default",
            "shared_as_name": "events",
            "id": "1",
            "history_data_sharing_status": "ENABLED",
            "start_version": 3,
        }));
    }
}
