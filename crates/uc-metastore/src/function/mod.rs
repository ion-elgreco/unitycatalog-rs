pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CreateFunction, FunctionInfo};
use uc_common::{Pagination, Result};

/// Upstream `FunctionService` has no update: only create, get, list and delete.
#[async_trait]
pub trait FunctionOps: Send + Sync {
    async fn create(&self, req: CreateFunction, created_by: Option<&str>) -> Result<FunctionInfo>;

    async fn get(&self, full_name: &str) -> Result<FunctionInfo>;

    async fn delete(&self, full_name: &str) -> Result<()>;

    async fn list(
        &self,
        catalog_name: &str,
        schema_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<FunctionInfo>, Option<String>)>;
}
