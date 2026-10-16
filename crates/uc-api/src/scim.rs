//! SCIM 2.0 users, as upstream `Scim2Utils.asUserResource` shows them: the
//! user's email is `userName` and the one primary email, its name is
//! `displayName`, plus `externalId`, `active`, `photos` and `meta`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// SCIM 2.0 schema URIs. Used by request/response envelopes and by the
/// `/scim2/Schemas` discovery endpoint.
pub mod schema_uri {
    pub const USER: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
    pub const GROUP: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
    pub const SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Schema";
    pub const RESOURCE_TYPE: &str = "urn:ietf:params:scim:schemas:core:2.0:ResourceType";
    pub const SERVICE_PROVIDER_CONFIG: &str =
        "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig";
    pub const LIST_RESPONSE: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
    pub const PATCH_OP: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
    pub const BULK_REQUEST: &str = "urn:ietf:params:scim:api:messages:2.0:BulkRequest";
    pub const BULK_RESPONSE: &str = "urn:ietf:params:scim:api:messages:2.0:BulkResponse";
    pub const ERROR: &str = "urn:ietf:params:scim:api:messages:2.0:Error";
}

/// SCIM `meta`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ScimMeta {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// RFC 3339.
    #[serde(
        rename = "lastModified",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_modified: Option<String>,
}

/// A SCIM role (RFC 7643, section 4.1.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ScimRole {
    pub value: String,
}

impl ScimRole {
    /// The role of a metastore admin.
    pub const ADMIN: &str = "admin";
}

/// A SCIM photo: upstream keeps one picture URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ScimPhoto {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScimEmail {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// A SCIM user. In requests every field may be absent; responses have
/// `schemas`, `id`, `meta`, `userName`, `active`, `emails` and `photos`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ScimUser {
    /// `["urn:ietf:params:scim:schemas:core:2.0:User"]`.
    #[serde(default)]
    pub schemas: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The identity the user has at the client's provider. Token exchange
    /// resolves an OAuth client (`azp`, `client_id`) to the user by it.
    #[serde(
        rename = "externalId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<ScimMeta>,
    /// The user's email. Ignored in requests: upstream takes the primary
    /// email.
    #[serde(rename = "userName", default)]
    pub user_name: String,
    #[serde(
        rename = "displayName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub display_name: Option<String>,
    /// Absent in a request: unchanged (enabled for a new user).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emails: Option<Vec<ScimEmail>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photos: Option<Vec<ScimPhoto>>,
    /// Only in `/scim2/Me`, which names [`ScimRole::ADMIN`] for a caller
    /// that the server lets do what the metastore admin does; upstream
    /// shows no roles. The server ignores roles in requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<ScimRole>>,
}

impl ScimUser {
    /// Whether `/scim2/Me` names the caller a metastore admin.
    pub fn is_admin(&self) -> bool {
        self.roles
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|role| role.value == ScimRole::ADMIN)
    }

    /// Enabled, upstream state `ENABLED`.
    pub fn is_active(&self) -> bool {
        self.active != Some(false)
    }

    /// The primary email of a request, upstream's identity of a new user.
    pub fn primary_email(&self) -> Option<&str> {
        self.emails
            .as_deref()?
            .iter()
            .find(|e| e.primary == Some(true))
            .map(|e| e.value.as_str())
    }

    /// The user's email as UC names principals: the primary email, else
    /// the first, else the user name.
    pub fn principal(&self) -> &str {
        let emails = self.emails.as_deref().unwrap_or_default();
        emails
            .iter()
            .find(|e| e.primary == Some(true))
            .or_else(|| emails.first())
            .map(|e| e.value.as_str())
            .unwrap_or(&self.user_name)
    }
}

/// Upstream's list: `totalResults` and `itemsPerPage` both count the page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListScimUsersResponse {
    pub schemas: Vec<String>,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimUser>,
    #[serde(rename = "startIndex", default)]
    pub start_index: i64,
    #[serde(rename = "itemsPerPage", default)]
    pub items_per_page: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<ScimMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScimGroupMember {
    /// `id` of the User entity.
    pub value: String,
    /// A display string, which clients set to the user's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// The SCIM resource type: `"User"`.
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ScimGroup {
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<ScimGroupMember>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListScimGroupsResponse {
    pub schemas: Vec<String>,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimGroup>,
    #[serde(rename = "startIndex", default)]
    pub start_index: i64,
    #[serde(rename = "itemsPerPage", default)]
    pub items_per_page: i64,
}

/// `urn:ietf:params:scim:api:messages:2.0:PatchOp` envelope.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScimPatchOp {
    #[serde(default)]
    pub schemas: Vec<String>,
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimPatchOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScimPatchOperation {
    /// `"add"` / `"remove"` / `"replace"` (case-insensitive per RFC 7644).
    pub op: String,
    /// SCIM attribute path — e.g. `"active"`, `"emails[type eq \"work\"].value"`,
    /// `"members"`. Optional for `replace` operations that pass a whole
    /// resource as `value`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_round_trip;
    use serde_json::json;

    fn email(value: &str, primary: Option<bool>) -> ScimEmail {
        ScimEmail {
            value: value.into(),
            primary,
            r#type: None,
        }
    }

    fn user(emails: Option<Vec<ScimEmail>>) -> ScimUser {
        ScimUser {
            user_name: "name@example.com".into(),
            emails,
            ..Default::default()
        }
    }

    #[test]
    fn active_unless_disabled() {
        for (active, expected) in [(None, true), (Some(true), true), (Some(false), false)] {
            let user = ScimUser {
                active,
                ..Default::default()
            };
            assert_eq!(user.is_active(), expected, "{active:?}");
        }
    }

    #[test]
    fn principal_is_the_primary_email_else_the_first_else_the_user_name() {
        let u = user(Some(vec![
            email("a@example.com", None),
            email("b@example.com", Some(true)),
        ]));
        assert_eq!(u.primary_email(), Some("b@example.com"));
        assert_eq!(u.principal(), "b@example.com");

        let u = user(Some(vec![
            email("a@example.com", Some(false)),
            email("b@example.com", None),
        ]));
        assert_eq!(u.primary_email(), None);
        assert_eq!(u.principal(), "a@example.com");

        for emails in [None, Some(vec![])] {
            let u = user(emails);
            assert_eq!(u.primary_email(), None);
            assert_eq!(u.principal(), "name@example.com");
        }
    }

    #[test]
    fn users_use_scim_names() {
        assert_round_trip::<ListScimUsersResponse>(json!({
            "schemas": [schema_uri::LIST_RESPONSE],
            "totalResults": 1,
            "Resources": [{
                "schemas": [schema_uri::USER],
                "id": "1",
                "externalId": "client-1",
                "meta": {
                    "resourceType": "User",
                    "created": "2026-10-14T08:00:00Z",
                    "lastModified": "2026-10-14T09:00:00Z",
                },
                "userName": "a@example.com",
                "displayName": "A",
                "active": true,
                "emails": [{ "value": "a@example.com", "primary": true, "type": "work" }],
                "photos": [{ "value": "https://example.com/a.png", "type": "photo" }],
            }],
            "startIndex": 1,
            "itemsPerPage": 1,
            "meta": { "resourceType": "User" },
        }));
    }

    #[test]
    fn groups_and_patches_use_scim_names() {
        assert_round_trip::<ListScimGroupsResponse>(json!({
            "schemas": [schema_uri::LIST_RESPONSE],
            "totalResults": 1,
            "Resources": [{
                "schemas": [schema_uri::GROUP],
                "id": "g1",
                "displayName": "admins",
                "members": [{ "value": "1", "display": "A", "type": "User" }],
            }],
            "startIndex": 1,
            "itemsPerPage": 1,
        }));
        assert_round_trip::<ScimPatchOp>(json!({
            "schemas": [schema_uri::PATCH_OP],
            "Operations": [{ "op": "replace", "path": "active", "value": false }],
        }));
    }

    #[test]
    fn requests_may_leave_out_every_field() {
        let user: ScimUser = serde_json::from_value(json!({})).unwrap();
        assert!(user.schemas.is_empty());
        assert!(user.user_name.is_empty());
        assert!(user.is_active());
        let patch: ScimPatchOp = serde_json::from_value(json!({ "Operations": [] })).unwrap();
        assert!(patch.schemas.is_empty());
    }
}
