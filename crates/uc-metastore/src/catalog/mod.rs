pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CatalogInfo, CreateCatalog, UpdateCatalog};
use uc_common::{Pagination, Result};

/// Persistence operations for the Catalogs resource.
///
/// Implementations keep `name` unique and map storage errors to
/// [`uc_common::Error`].
#[async_trait]
pub trait CatalogOps: Send + Sync {
    async fn create(&self, req: CreateCatalog, created_by: Option<&str>) -> Result<CatalogInfo>;

    async fn get(&self, name: &str) -> Result<CatalogInfo>;

    async fn update(
        &self,
        name: &str,
        req: UpdateCatalog,
        updated_by: Option<&str>,
    ) -> Result<CatalogInfo>;

    async fn delete(&self, name: &str, force: bool) -> Result<()>;

    async fn list(&self, page: &Pagination) -> Result<(Vec<CatalogInfo>, Option<String>)>;
}
