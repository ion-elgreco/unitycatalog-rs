//! The Iceberg REST catalog's tables in SQL: native Iceberg tables and the
//! Iceberg metadata pointer of UniForm Delta tables, in the `tbl` table.

use async_trait::async_trait;

use uc_api::{ColumnInfo, CreateTable, DataSourceFormat, TableInfo, TableType};
use uc_common::{
    Error, ErrorCode, NormalizedUrl, Pagination, Result, SecurablePropertiesMap, now_epoch_millis,
};

use crate::sql::rows::TBL;
use crate::sql::{SqlMetastore, db_err, enum_str, json_text, parse_enum};
use crate::{IcebergTableOps, IcebergTableRef, IcebergTableState};
use uc_storage::Value;

#[async_trait]
impl IcebergTableOps for SqlMetastore {
    async fn table_state(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
    ) -> Result<IcebergTableState> {
        let schema_ref = self.resolve_schema(catalog, schema).await?;
        let row = self
            .table_row(&schema_ref.schema_id, table)
            .await?
            .ok_or_else(|| {
                Error::table_not_found(format!("Table not found: {catalog}.{schema}.{table}"))
            })?;
        Ok(IcebergTableState {
            table_id: row.id_string,
            table_type: parse_enum::<TableType>(&row.table_type)?,
            data_source_format: row
                .data_source_format
                .as_deref()
                .map(parse_enum::<DataSourceFormat>)
                .transpose()?,
            metadata_location: row.iceberg_metadata_location,
            storage_location: row.storage_location,
            owner: row.owner,
        })
    }

    async fn create_table(
        &self,
        req: CreateTable,
        metadata_location: &NormalizedUrl,
        created_by: Option<&str>,
    ) -> Result<TableInfo> {
        self.persist_table(req, None, Some(metadata_location), created_by)
            .await
    }

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
    ) -> Result<()> {
        let full_name = format!("{catalog}.{schema}.{table}");
        let schema_ref = self.resolve_schema(catalog, schema).await?;
        let not_found = || Error::table_not_found(format!("Table not found: {full_name}"));
        let row = self
            .table_row(&schema_ref.schema_id, table)
            .await?
            .ok_or_else(not_found)?;
        if row.data_source_format.as_deref() != Some(enum_str(&DataSourceFormat::Iceberg).as_str())
        {
            return Err(Error::invalid_argument(format!(
                "Table is not a native Iceberg table and cannot be committed to via the Iceberg \
                 REST catalog: {full_name}"
            )));
        }
        let conflict = |found: Option<&str>| {
            Error::new(
                ErrorCode::UpdateRequirementConflict,
                format!(
                    "Metadata location for {full_name} changed concurrently: expected {expected} \
                     but found {}",
                    found.unwrap_or("null")
                ),
            )
        };
        if row.iceberg_metadata_location.as_deref() != Some(expected) {
            return Err(conflict(row.iceberg_metadata_location.as_deref()));
        }
        // The pointer is the version: the update applies only if it still
        // names `expected`.
        let properties = match properties.is_empty() {
            true => Value::Null,
            false => json_text(&properties)?.into(),
        };
        let written = self
            .db
            .execute(
                "UPDATE tbl SET columns = $1, properties = $2, iceberg_metadata_location = $3, \
                     updated_at = $4, updated_by = $5 \
                 WHERE id_string = $6 AND iceberg_metadata_location = $7",
                &[
                    json_text(&columns)?.into(),
                    properties,
                    new_location.as_str().into(),
                    now_epoch_millis().max(row.updated_at + 1).into(),
                    updated_by.into(),
                    row.id_string.as_str().into(),
                    expected.into(),
                ],
            )
            .await
            .map_err(db_err("commit iceberg table"))?;
        if written == 0 {
            let current = self.table_row(&schema_ref.schema_id, table).await?;
            return Err(conflict(
                current.and_then(|r| r.iceberg_metadata_location).as_deref(),
            ));
        }
        Ok(())
    }

    async fn list_tables(
        &self,
        catalog: &str,
        schema: &str,
        page: &Pagination,
    ) -> Result<(Vec<IcebergTableRef>, Option<String>)> {
        let schema = self.resolve_schema(catalog, schema).await?;
        self.paged_list::<IcebergTableRef, _>(
            &TBL,
            &["schema_id = $1", "iceberg_metadata_location IS NOT NULL"],
            vec![schema.schema_id.as_str().into()],
            "name",
            page,
            |r| r.name.clone(),
        )
        .await
    }
}
