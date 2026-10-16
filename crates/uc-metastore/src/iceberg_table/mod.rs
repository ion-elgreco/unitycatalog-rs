pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{ColumnInfo, CreateTable, DataSourceFormat, TableInfo, TableType};
use uc_common::{NormalizedUrl, Pagination, Result, SecurablePropertiesMap};

/// What the Iceberg REST catalog needs of a table. Upstream
/// `TableRepository.IcebergTableState`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcebergTableState {
    pub table_id: String,
    pub table_type: TableType,
    pub data_source_format: Option<DataSourceFormat>,
    /// The current Iceberg metadata file. Native Iceberg tables and UniForm
    /// Delta tables have one.
    pub metadata_location: Option<String>,
    pub storage_location: Option<String>,
    pub owner: Option<String>,
}

/// A table the Iceberg REST catalog lists.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct IcebergTableRef {
    #[serde(rename = "id_string")]
    pub table_id: String,
    pub name: String,
    #[serde(default)]
    pub owner: Option<String>,
}

/// Tables through the Iceberg REST catalog: native Iceberg tables, and the
/// Iceberg metadata of UniForm Delta tables.
#[async_trait]
pub trait IcebergTableOps: Send + Sync {
    /// Upstream `getIcebergTableState`: the table `catalog.schema.table`.
    async fn table_state(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
    ) -> Result<IcebergTableState>;

    /// Upstream `createTableForIceberg`: create a native Iceberg table (an
    /// EXTERNAL one, or a MANAGED one over the caller's staging table) whose
    /// current metadata file is `metadata_location`.
    async fn create_table(
        &self,
        req: CreateTable,
        metadata_location: &NormalizedUrl,
        created_by: Option<&str>,
    ) -> Result<TableInfo>;

    /// Upstream `commitIcebergTable`: move the metadata pointer of the native
    /// Iceberg table from `expected` to `new_location` if no one moved it
    /// first (`UPDATE_REQUIREMENT_CONFLICT` otherwise), and replace its
    /// columns and properties with the new metadata's.
    #[allow(clippy::too_many_arguments)]
    async fn commit_table(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
        expected: &str,
        new_location: &NormalizedUrl,
        columns: Vec<ColumnInfo>,
        properties: SecurablePropertiesMap,
        updated_by: Option<&str>,
    ) -> Result<()>;

    /// Upstream `listIcebergTables`: a page of the tables in a schema that
    /// have Iceberg metadata, by name.
    async fn list_tables(
        &self,
        catalog: &str,
        schema: &str,
        page: &Pagination,
    ) -> Result<(Vec<IcebergTableRef>, Option<String>)>;
}
