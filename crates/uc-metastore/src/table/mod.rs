pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{CreateStagingTable, CreateTable, StagingTableInfo, TableInfo, TableType};
use uc_common::{NormalizedUrl, Pagination, Result};

/// Tables through the UC REST API, and the staging tables that MANAGED
/// tables start from. The Delta and Iceberg REST APIs have their own
/// traits: [`crate::DeltaTableOps`] and [`crate::IcebergTableOps`].
#[async_trait]
pub trait TableOps: Send + Sync {
    async fn create(&self, req: CreateTable, created_by: Option<&str>) -> Result<TableInfo>;

    /// `full_name` is `"<catalog>.<schema>.<table>"`.
    async fn get(&self, full_name: &str) -> Result<TableInfo>;

    /// Look up by the persistent table UUID (`table_id` in API responses).
    /// Used by `/temporary-table-credentials` to resolve the storage URL.
    async fn get_by_id(&self, table_id: &str) -> Result<TableInfo>;

    /// Upstream `getTableStorageLocation`: the storage location and type of
    /// the table with this id, or of the uncommitted staging table with this
    /// id (as MANAGED). Used by `/temporary-table-credentials`.
    async fn storage_location_by_id(&self, table_id: &str) -> Result<(Option<String>, TableType)>;

    async fn delete(&self, full_name: &str) -> Result<()>;

    /// Rename a table within the same schema: its name and `updated_*`
    /// fields change; its id stays.
    async fn rename(
        &self,
        full_name: &str,
        new_name: &str,
        updated_by: Option<&str>,
    ) -> Result<TableInfo>;

    async fn list(
        &self,
        catalog_name: &str,
        schema_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<TableInfo>, Option<String>)>;

    // Staging tables only reserve a storage location. Their rows are kept
    // apart from the tables.
    async fn create_staging(
        &self,
        req: CreateStagingTable,
        created_by: Option<&str>,
    ) -> Result<StagingTableInfo>;

    /// Upstream `getStagingTableStorageLocation`: the location of a staging
    /// table (regular tables are not accepted).
    async fn staging_location(&self, staging_id: &str) -> Result<String>;

    /// Upstream `requireOwnedStagingTable`: the staging table at `location`
    /// exists and `caller` created it.
    async fn require_owned_staging(
        &self,
        location: &NormalizedUrl,
        caller: Option<&str>,
    ) -> Result<()>;
}
