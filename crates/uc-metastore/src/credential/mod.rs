pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CreateCredentialRequest, CredentialInfo, CredentialPurpose, UpdateCredentialRequest};
use uc_common::{Pagination, Result};

#[async_trait]
pub trait CredentialOps: Send + Sync {
    async fn create(
        &self,
        req: CreateCredentialRequest,
        created_by: Option<&str>,
    ) -> Result<CredentialInfo>;

    async fn get(&self, name: &str) -> Result<CredentialInfo>;

    async fn update(
        &self,
        name: &str,
        req: UpdateCredentialRequest,
        updated_by: Option<&str>,
    ) -> Result<CredentialInfo>;

    async fn delete(&self, name: &str, force: bool) -> Result<()>;

    /// One page of credentials, only those with `purpose` when given.
    async fn list(
        &self,
        purpose: Option<CredentialPurpose>,
        page: &Pagination,
    ) -> Result<(Vec<CredentialInfo>, Option<String>)>;
}
