pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CreateCredentialRequest, CredentialInfo, CredentialPurpose, UpdateCredentialRequest};
use uc_common::{Pagination, Result};

use crate::SecretValue;

/// What [`CredentialOps::refresh_secrets`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshReport {
    /// The secrets that the store keeps in a new form now, such as sealed
    /// with a new key.
    pub refreshed: usize,
    /// The credentials whose secrets the store cannot read.
    pub unreadable: Vec<String>,
}

#[async_trait]
pub trait CredentialOps: Send + Sync {
    async fn create(
        &self,
        req: CreateCredentialRequest,
        created_by: Option<&str>,
    ) -> Result<CredentialInfo>;

    async fn get(&self, name: &str) -> Result<CredentialInfo>;

    /// The credential named `name`, and its secret, if it has one: the
    /// client secret of its Azure service principal, or its GCP service
    /// account key, from the secret store. Only credential vending reads the
    /// secret: no answer of the API holds it.
    async fn get_with_secret(&self, name: &str) -> Result<(CredentialInfo, Option<SecretValue>)>;

    /// Give the store each secret that it keeps in an older form, such as
    /// sealed with an older key, so that it keeps it in its current form.
    /// Nothing for a store that does not refresh.
    async fn refresh_secrets(&self) -> Result<RefreshReport>;

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
