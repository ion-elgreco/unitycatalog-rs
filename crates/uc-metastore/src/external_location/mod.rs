pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CreateExternalLocation, ExternalLocationInfo, UpdateExternalLocation};
use uc_common::{Pagination, Result};

#[async_trait]
pub trait ExternalLocationOps: Send + Sync {
    async fn create(
        &self,
        req: CreateExternalLocation,
        created_by: Option<&str>,
    ) -> Result<ExternalLocationInfo>;

    async fn get(&self, name: &str) -> Result<ExternalLocationInfo>;

    async fn update(
        &self,
        name: &str,
        req: UpdateExternalLocation,
        updated_by: Option<&str>,
    ) -> Result<ExternalLocationInfo>;

    async fn delete(&self, name: &str, force: bool) -> Result<()>;

    async fn list(&self, page: &Pagination) -> Result<(Vec<ExternalLocationInfo>, Option<String>)>;

    /// Find the external location whose `url` is the longest prefix of the
    /// supplied path. Used by temporary-credential vending to pick the right
    /// credential for a storage URI.
    async fn find_by_url_prefix(&self, url: &str) -> Result<Option<ExternalLocationInfo>>;
}
