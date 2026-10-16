pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{
    CreateRecipient, CreateShare, RecipientInfo, ShareInfo, UpdateRecipient, UpdateShare,
};
use uc_common::{Pagination, Result};

#[async_trait]
pub trait SharingOps: Send + Sync {
    async fn create_share(&self, req: CreateShare, created_by: Option<&str>) -> Result<ShareInfo>;
    async fn get_share(&self, name: &str) -> Result<ShareInfo>;
    async fn list_shares(&self, page: &Pagination) -> Result<(Vec<ShareInfo>, Option<String>)>;
    async fn update_share(
        &self,
        name: &str,
        req: UpdateShare,
        updated_by: Option<&str>,
    ) -> Result<ShareInfo>;
    async fn delete_share(&self, name: &str) -> Result<()>;

    /// The new recipient, with `bearer_token` set. The caller cannot read the
    /// token later.
    async fn create_recipient(
        &self,
        req: CreateRecipient,
        created_by: Option<&str>,
    ) -> Result<RecipientInfo>;
    async fn get_recipient(&self, name: &str) -> Result<RecipientInfo>;
    async fn list_recipients(
        &self,
        page: &Pagination,
    ) -> Result<(Vec<RecipientInfo>, Option<String>)>;
    async fn update_recipient(
        &self,
        name: &str,
        req: UpdateRecipient,
        updated_by: Option<&str>,
    ) -> Result<RecipientInfo>;
    async fn delete_recipient(&self, name: &str) -> Result<()>;
    /// Generate a fresh bearer for `name`, replacing the old hash. Returns
    /// the new plaintext.
    async fn rotate_recipient_token(&self, name: &str) -> Result<RecipientInfo>;

    /// Resolve a bearer token to the owning recipient (or `None`).
    async fn recipient_for_token(&self, token: &str) -> Result<Option<RecipientInfo>>;
}
