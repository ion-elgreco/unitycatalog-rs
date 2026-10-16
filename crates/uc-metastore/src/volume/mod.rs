pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CreateVolumeRequestContent, UpdateVolumeRequestContent, VolumeInfo};
use uc_common::{Pagination, Result};

#[async_trait]
pub trait VolumeOps: Send + Sync {
    async fn create(
        &self,
        req: CreateVolumeRequestContent,
        created_by: Option<&str>,
    ) -> Result<VolumeInfo>;

    /// `full_name` is `"<catalog>.<schema>.<volume>"`.
    async fn get(&self, full_name: &str) -> Result<VolumeInfo>;

    /// Look up by persistent volume UUID. Used by `/temporary-volume-credentials`.
    async fn get_by_id(&self, volume_id: &str) -> Result<VolumeInfo>;

    async fn update(
        &self,
        full_name: &str,
        req: UpdateVolumeRequestContent,
        updated_by: Option<&str>,
    ) -> Result<VolumeInfo>;

    async fn delete(&self, full_name: &str) -> Result<()>;

    async fn list(
        &self,
        catalog_name: &str,
        schema_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<VolumeInfo>, Option<String>)>;
}
