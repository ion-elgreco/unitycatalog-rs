//! Sharing: shares, the objects in them, recipients, their bearer tokens
//! and the grants of shares to recipients.
//!
//! A recipient is not a principal of the catalog. It reads the shares that
//! it was granted with a bearer token, which it gets once from an
//! activation link.

pub(crate) mod sql;

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use uc_common::{Pagination, Result};

/// What a share object is in the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShareObjectType {
    Table,
    Volume,
    /// A volume that holds an agent skill.
    AgentSkill,
}

/// An object of the catalog in a share. A share holds an object once, by
/// its type and source name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareObject {
    /// The full name in the catalog, such as `main.default.events`.
    pub source_full_name: String,
    pub object_type: ShareObjectType,
    /// The schema and the name that a recipient sees.
    pub shared_as_schema: String,
    pub shared_as_name: String,
    /// Whether a recipient reads older versions of the table.
    #[serde(default)]
    pub history_shared: bool,
    /// The lowest version of the table that a recipient reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_version: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default)]
    pub added_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Share {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
    pub created_at: i64,
    pub created_by: Option<String>,
    pub updated_at: i64,
    pub updated_by: Option<String>,
    pub objects: Vec<ShareObject>,
}

/// A change of the objects of a share.
#[derive(Debug, Clone)]
pub enum ObjectChange {
    /// A new object; the share must not hold it.
    Add(ShareObject),
    /// New settings of an object that the share holds.
    Update(ShareObject),
    /// The object of this type and source name leaves the share.
    Remove(ShareObjectType, String),
}

#[derive(Debug, Clone, Default)]
pub struct ShareUpdate {
    pub new_name: Option<String>,
    pub owner: Option<String>,
    pub comment: Option<String>,
    pub objects: Vec<ObjectChange>,
}

/// A bearer token of a recipient. The token itself is not kept, only its
/// hash.
#[derive(Debug, Clone)]
pub struct RecipientToken {
    pub id: String,
    /// The code of the activation link, until the recipient opens it.
    pub activation_code: Option<String>,
    /// When the token ends, in epoch milliseconds; `None` if it does not.
    pub expiration_time: Option<i64>,
    pub created_at: i64,
    pub created_by: Option<String>,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Recipient {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
    pub properties: BTreeMap<String, String>,
    pub created_at: i64,
    pub created_by: Option<String>,
    pub updated_at: i64,
    pub updated_by: Option<String>,
    /// The tokens that are valid, oldest first.
    pub tokens: Vec<RecipientToken>,
}

#[derive(Debug, Clone, Default)]
pub struct NewRecipient {
    pub name: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
    pub properties: BTreeMap<String, String>,
    /// When the tokens of the recipient end, in epoch milliseconds.
    pub expiration_time: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct RecipientUpdate {
    pub new_name: Option<String>,
    pub owner: Option<String>,
    pub comment: Option<String>,
    /// All the properties, in place of the ones the recipient has.
    pub properties: Option<BTreeMap<String, String>>,
    /// A new end of the newest token, and of the tokens to come.
    pub expiration_time: Option<i64>,
}

/// The recipient of a bearer token, and the shares that it reads.
#[derive(Debug, Clone)]
pub struct TokenRecipient {
    pub name: String,
    pub shares: Vec<String>,
}

/// What an activation link gives, once.
#[derive(Debug, Clone)]
pub struct Activated {
    pub bearer_token: String,
    pub expiration_time: Option<i64>,
}

#[async_trait]
pub trait SharingOps: Send + Sync {
    async fn create_share(
        &self,
        name: &str,
        comment: Option<String>,
        actor: Option<&str>,
    ) -> Result<Share>;
    async fn get_share(&self, name: &str) -> Result<Share>;
    async fn list_shares(&self, page: &Pagination) -> Result<(Vec<Share>, Option<String>)>;
    async fn update_share(
        &self,
        name: &str,
        update: ShareUpdate,
        actor: Option<&str>,
    ) -> Result<Share>;
    /// Delete the share and its grants.
    async fn delete_share(&self, name: &str) -> Result<()>;

    /// The names of the recipients that read the share, in order.
    async fn share_recipients(&self, share: &str) -> Result<Vec<String>>;
    /// Let the recipient read the share. A grant that exists stays.
    async fn grant_share(&self, share: &str, recipient: &str, actor: Option<&str>) -> Result<()>;
    /// End the recipient's reads of the share, if it had them.
    async fn revoke_share(&self, share: &str, recipient: &str) -> Result<()>;

    /// The new recipient with its first token, which has an activation
    /// code.
    async fn create_recipient(&self, new: NewRecipient, actor: Option<&str>) -> Result<Recipient>;
    async fn get_recipient(&self, name: &str) -> Result<Recipient>;
    async fn list_recipients(&self, page: &Pagination) -> Result<(Vec<Recipient>, Option<String>)>;
    async fn update_recipient(
        &self,
        name: &str,
        update: RecipientUpdate,
        actor: Option<&str>,
    ) -> Result<Recipient>;
    /// Delete the recipient with its tokens and grants.
    async fn delete_recipient(&self, name: &str) -> Result<()>;
    /// Give the recipient a new token with an activation code. The tokens
    /// it has stay valid for `existing_expire_in_seconds` more at most, and
    /// not longer than they would have; `None` or `0` ends them at once.
    async fn rotate_recipient_token(
        &self,
        name: &str,
        existing_expire_in_seconds: Option<i64>,
        actor: Option<&str>,
    ) -> Result<Recipient>;

    /// The token of an activation code. A code works once, and only while
    /// its token is valid.
    async fn activate(&self, code: &str) -> Result<Activated>;

    /// The recipient of a bearer token that is valid, if there is one.
    async fn recipient_for_token(&self, token: &str) -> Result<Option<TokenRecipient>>;
}
