pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{
    CreateModelVersion, CreateRegisteredModel, ModelVersionInfo, RegisteredModelInfo,
    UpdateModelVersion, UpdateRegisteredModel,
};
use uc_common::{Pagination, Result};

#[async_trait]
pub trait ModelOps: Send + Sync {
    async fn create_model(
        &self,
        req: CreateRegisteredModel,
        created_by: Option<&str>,
    ) -> Result<RegisteredModelInfo>;

    async fn get_model(&self, full_name: &str) -> Result<RegisteredModelInfo>;

    async fn update_model(
        &self,
        full_name: &str,
        req: UpdateRegisteredModel,
        updated_by: Option<&str>,
    ) -> Result<RegisteredModelInfo>;

    /// `force=true` deletes all child model versions.
    async fn delete_model(&self, full_name: &str, force: bool) -> Result<()>;

    async fn list_models(
        &self,
        catalog_name: Option<&str>,
        schema_name: Option<&str>,
        page: &Pagination,
    ) -> Result<(Vec<RegisteredModelInfo>, Option<String>)>;

    async fn create_version(
        &self,
        req: CreateModelVersion,
        created_by: Option<&str>,
    ) -> Result<ModelVersionInfo>;

    async fn get_version(&self, full_name: &str, version: i64) -> Result<ModelVersionInfo>;

    async fn update_version(
        &self,
        full_name: &str,
        version: i64,
        req: UpdateModelVersion,
        updated_by: Option<&str>,
    ) -> Result<ModelVersionInfo>;

    async fn delete_version(&self, full_name: &str, version: i64) -> Result<()>;

    /// Upstream `finalizeModelVersion`: PENDING_REGISTRATION becomes READY.
    async fn finalize_version(
        &self,
        full_name: &str,
        version: i64,
        updated_by: Option<&str>,
    ) -> Result<ModelVersionInfo>;

    async fn list_versions(
        &self,
        full_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<ModelVersionInfo>, Option<String>)>;
}
