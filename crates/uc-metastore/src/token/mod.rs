pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CreatePat, PatInfo};
use uc_common::{Pagination, Result};

/// Persistence for Personal Access Tokens.
///
/// Tokens are stored as their SHA-256 hash; only the answer to `create_pat`
/// carries the plaintext.
#[async_trait]
pub trait TokenOps: Send + Sync {
    /// The new PAT, with its plaintext `token_value`.
    async fn create_pat(&self, req: CreatePat, owner: &str) -> Result<PatInfo>;

    async fn get_pat(&self, id: &str) -> Result<PatInfo>;

    async fn list_pats(
        &self,
        owner_filter: Option<&str>,
        page: &Pagination,
    ) -> Result<(Vec<PatInfo>, Option<String>)>;

    async fn revoke_pat(&self, id: &str) -> Result<()>;

    /// Verify a bearer token; returns the owning principal name on success.
    async fn principal_for_token(&self, token: &str) -> Result<Option<String>>;
}
