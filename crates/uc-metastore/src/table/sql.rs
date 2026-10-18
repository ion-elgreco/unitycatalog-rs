use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    ColumnInfo, CreateStagingTable, CreateTable, DataSourceFormat, DependencyList,
    StagingTableInfo, TableInfo, TableType,
};
use uc_common::{
    Error, ErrorCode, ManagedResource, NormalizedUrl, Pagination, Result, SecurablePropertiesMap,
    now_epoch_millis, validate_sql_object_name,
};

use crate::TableOps;
use crate::cleanup::sql::queue_cleanup;
use crate::sql::rows::{STAGING_TABLE, TBL};
use crate::sql::{
    SchemaRef, SqlMetastore, by_id, create_err_as, db_err, enum_str, insert, parse_enum,
    select_one, split_three_part, update,
};
use uc_delta::contract::validate_table_id_property;
use uc_storage::{Exec, Value};

/// A table row (SQL table `tbl`), keyed by its UUID and linked to its schema
/// by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TableRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    pub(crate) schema_id: String,
    /// Serialized as the enum's string repr (`MANAGED`, `EXTERNAL`, ...).
    pub(crate) table_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data_source_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) columns: Option<Vec<ColumnInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) storage_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<SecurablePropertiesMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) owner: Option<String>,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) view_definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) view_dependencies: Option<DependencyList>,
    /// UniForm: the latest Iceberg metadata of a Delta table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) iceberg_metadata_location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) uniform_iceberg_converted_delta_version: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) uniform_iceberg_converted_delta_timestamp: Option<i64>,
}

impl TableRow {
    /// Upstream `updateUniformIcebergMetadata`.
    pub(crate) fn set_uniform(&mut self, uniform: &uc_delta::uniform::UniformIceberg) {
        self.iceberg_metadata_location = Some(uniform.metadata_location.as_str().to_string());
        self.uniform_iceberg_converted_delta_version = Some(uniform.converted_delta_version);
        self.uniform_iceberg_converted_delta_timestamp = Some(uniform.converted_delta_timestamp_ms);
    }

    /// The UniForm block of the Delta API, when the table has one.
    pub(crate) fn uniform(&self) -> Option<uc_api::delta_api::DeltaUniformMetadata> {
        let location = self.iceberg_metadata_location.clone()?;
        Some(uc_api::delta_api::DeltaUniformMetadata {
            iceberg: Some(uc_api::delta_api::DeltaUniformMetadataIceberg {
                metadata_location: Some(location),
                converted_delta_version: self.uniform_iceberg_converted_delta_version,
                converted_delta_timestamp: self.uniform_iceberg_converted_delta_timestamp,
                base_converted_delta_version: None,
            }),
        })
    }

    pub(crate) fn into_info(self, schema: &SchemaRef) -> Result<TableInfo> {
        Ok(TableInfo {
            catalog_name: Some(schema.catalog_name.clone()),
            schema_name: Some(schema.schema_name.clone()),
            table_type: Some(parse_enum::<TableType>(&self.table_type)?),
            data_source_format: self
                .data_source_format
                .as_deref()
                .map(parse_enum::<DataSourceFormat>)
                .transpose()?,
            columns: self.columns,
            storage_location: self.storage_location,
            comment: self.comment,
            properties: self.properties,
            owner: self.owner,
            created_at: Some(self.created_at),
            created_by: self.created_by,
            updated_at: Some(self.updated_at),
            updated_by: self.updated_by,
            table_id: Some(self.id_string),
            view_definition: self.view_definition,
            view_dependencies: self.view_dependencies,
            name: Some(self.name),
        })
    }
}

fn table_exists(full_name: &str) -> Error {
    Error::new(
        ErrorCode::TableAlreadyExists,
        format!("Table already exists: {full_name}"),
    )
}

/// A staging table: a reserved managed location for a table a client is
/// about to create (upstream `StagingTableDAO`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StagingRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    pub(crate) schema_id: String,
    pub(crate) staging_location: String,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) accessed_at: i64,
    pub(crate) stage_committed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stage_committed_at: Option<i64>,
    pub(crate) purge_state: i64,
    pub(crate) num_cleanup_retries: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_cleanup_at: Option<i64>,
}

/// Before dropping the tables that `filter` picks (binding `params`), in
/// the same transaction. Upstream `deleteTable`: MANAGED tables queue
/// storage cleanup and lose their Delta commits.
pub(crate) async fn drop_managed_tables<E: Exec + ?Sized>(
    ex: &E,
    filter: &str,
    params: &[Value],
) -> Result<()> {
    let managed = format!("({filter}) AND table_type = 'MANAGED'");
    queue_cleanup(ex, &TBL, ManagedResource::Table, &managed, params).await?;
    ex.execute(
        &format!(
            "DELETE FROM delta_commit WHERE table_id IN (SELECT id_string FROM tbl WHERE {managed})"
        ),
        params,
    )
    .await
    .map_err(db_err("delete delta commits"))?;
    Ok(())
}

/// Upstream column handling on create: `ColumnUtils.validateTypeJson` on
/// each column, `type_text` lowercased, and `nullable` defaulting to false.
fn validate_columns(columns: &mut [ColumnInfo], format: Option<DataSourceFormat>) -> Result<()> {
    for column in columns.iter_mut() {
        validate_type_json(column, format)?;
        if let Some(text) = column.type_text.as_mut() {
            *text = text.to_lowercase();
        }
        column.nullable.get_or_insert(false);
    }
    Ok(())
}

/// Upstream `ColumnUtils.validateTypeJson`: `type_json` must be a Spark
/// `StructField` whose name and nullability match the column. Delta tables
/// also get the Delta type checks.
fn validate_type_json(column: &ColumnInfo, format: Option<DataSourceFormat>) -> Result<()> {
    use uc_api::delta_types::{DeltaStructField, KeyStyle, validate_struct_field};
    let name = column.name.as_deref();
    let invalid = |message: String| {
        Error::invalid_argument(format!(
            "Invalid type_json for column {}: {message}",
            name.unwrap_or("<unknown>")
        ))
    };
    let type_json = column
        .type_json
        .as_deref()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            invalid(format!(
                "Column {} has null/empty typeJson",
                name.unwrap_or("null")
            ))
        })?;
    let unparsable = || {
        invalid(format!(
            "Failed to parse typeJson for column {}: {type_json}",
            name.unwrap_or("null")
        ))
    };
    let value: serde_json::Value = serde_json::from_str(type_json).map_err(|_| unparsable())?;
    let field = DeltaStructField::from_json(&value, KeyStyle::Camel).map_err(|_| unparsable())?;
    if format == Some(DataSourceFormat::Delta) {
        validate_struct_field(&field, "type_json").map_err(invalid)?;
    } else {
        // A shallow check for other formats; upstream does not prefix these.
        let missing = [
            ("name", field.name.is_none()),
            ("type", field.data_type.is_none()),
            ("nullable", field.nullable.is_none()),
            ("metadata", field.metadata.is_none()),
        ];
        if let Some((key, _)) = missing.iter().find(|(_, m)| *m) {
            return Err(Error::invalid_argument(format!(
                "type_json.{key} is required."
            )));
        }
    }
    let field_name = field.name.as_deref().unwrap_or_default();
    if Some(field_name) != name {
        return Err(invalid(format!(
            "field name {field_name} does not match column name {}",
            name.unwrap_or("null")
        )));
    }
    if let (Some(column_nullable), Some(field_nullable)) = (column.nullable, field.nullable)
        && column_nullable != field_nullable
    {
        return Err(invalid(format!(
            "field nullable {field_nullable} does not match column nullable {column_nullable}"
        )));
    }
    Ok(())
}

/// The dependencies of a create request (none when absent).
fn dependencies(req: &CreateTable) -> &[uc_api::Dependency] {
    req.view_dependencies
        .as_ref()
        .and_then(|d| d.dependencies.as_deref())
        .unwrap_or(&[])
}

/// Upstream `DependencyDAO.from`: the kind ("table" / "function") and the
/// three parts of a dependency's full name.
fn parse_dependency(dep: &uc_api::Dependency) -> Result<(&'static str, Vec<String>)> {
    let (kind, full_name) = match (&dep.table, &dep.function) {
        (Some(_), Some(_)) => {
            return Err(Error::invalid_argument(
                "Dependency must have exactly one of table or function set, not both",
            ));
        }
        (Some(t), None) => ("table", t.table_full_name.as_str()),
        (None, Some(f)) => ("function", f.function_full_name.as_str()),
        (None, None) => return Err(Error::invalid_argument("Unsupported dependency type")),
    };
    if full_name.is_empty() {
        return Err(Error::invalid_argument(format!(
            "Dependency {kind} full name must not be null or empty"
        )));
    }
    let parts: Vec<String> = crate::sql::split_dotted(full_name)
        .iter()
        .map(|s| s.to_string())
        .collect();
    if parts.len() != 3 {
        return Err(Error::invalid_argument(format!(
            "Dependency {kind} full name must be a three-part name (catalog.schema.{kind}); got: \
             {full_name}"
        )));
    }
    Ok((kind, parts))
}

impl SqlMetastore {
    async fn staging_row_at(&self, location: &NormalizedUrl) -> Result<Option<StagingRow>> {
        select_one(
            self.db.as_ref(),
            &STAGING_TABLE,
            "WHERE staging_location = $1",
            &[location.as_str().into()],
        )
        .await
    }

    /// Upstream `requireOwnedStagingTable`: the staging table at `location`,
    /// created by `caller`.
    async fn require_owned_staging(
        &self,
        location: &NormalizedUrl,
        caller: Option<&str>,
    ) -> Result<StagingRow> {
        let row = self.staging_row_at(location).await?.ok_or_else(|| {
            Error::table_not_found(format!("Staging table not found: {location}"))
        })?;
        if row.created_by.as_deref() != caller {
            return Err(Error::permission_denied(format!(
                "User attempts to create table on a staging location without ownership: {location}"
            )));
        }
        Ok(row)
    }

    /// Upstream `validateViewLike`: a definition, at least one column, and
    /// existing dependencies (when given).
    async fn validate_view_like(&self, req: &CreateTable, label: &str) -> Result<()> {
        if req.view_definition.as_deref().is_none_or(str::is_empty) {
            return Err(Error::invalid_argument(format!(
                "view_definition is required for {label}"
            )));
        }
        if req.columns.is_empty() {
            return Err(Error::invalid_argument(format!(
                "columns must contain at least one entry for {label}"
            )));
        }
        let parsed: Vec<(&str, Vec<String>)> = dependencies(req)
            .iter()
            .map(parse_dependency)
            .collect::<Result<_>>()?;
        for (kind, parts) in parsed {
            let full_name = parts.join(".");
            let schema = self.resolve_schema(&parts[0], &parts[1]).await?;
            let exists = if kind == "table" {
                self.table_row(&schema.schema_id, &parts[2])
                    .await?
                    .is_some()
            } else {
                self.function_row(&schema.schema_id, &parts[2])
                    .await?
                    .is_some()
            };
            if !exists {
                return Err(Error::not_found(format!(
                    "View dependency {kind} does not exist: {full_name}"
                )));
            }
        }
        Ok(())
    }

    /// The table `name` in schema `schema_id`, if any.
    pub(crate) async fn table_row(&self, schema_id: &str, name: &str) -> Result<Option<TableRow>> {
        select_one(
            self.db.as_ref(),
            &TBL,
            "WHERE schema_id = $1 AND name = $2",
            &[schema_id.into(), name.into()],
        )
        .await
    }

    /// Resolve `<catalog>.<schema>.<table>`, or fail with upstream's errors:
    /// `INVALID_ARGUMENT` for a malformed name, then `CATALOG_NOT_FOUND`,
    /// `SCHEMA_NOT_FOUND`, `TABLE_NOT_FOUND`.
    pub(crate) async fn resolve_table(&self, full_name: &str) -> Result<(SchemaRef, TableRow)> {
        let (c, s, t) = split_three_part(full_name, "table")?;
        let schema = self.resolve_schema(c, s).await?;
        // A query of its own: PostgreSQL plans a join with `tbl` for the
        // table's size when it prepares the statement, and keeps a
        // sequential scan of `tbl` while the table grows.
        let row = self
            .table_row(&schema.schema_id, t)
            .await?
            .ok_or_else(|| Error::table_not_found(format!("Table not found: {full_name}")))?;
        Ok((schema, row))
    }

    pub(crate) async fn insert_table(&self, row: TableRow, full_name: &str) -> Result<TableRow> {
        insert(self.db.as_ref(), &TBL, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::TableAlreadyExists,
                format!("Table already exists: {full_name}"),
            ))?;
        Ok(row)
    }
}

#[async_trait]
impl TableOps for SqlMetastore {
    async fn create(&self, req: CreateTable, created_by: Option<&str>) -> Result<TableInfo> {
        self.persist_table(req, None, None, created_by).await
    }

    async fn get(&self, full_name: &str) -> Result<TableInfo> {
        let (schema, row) = self.resolve_table(full_name).await?;
        row.into_info(&schema)
    }

    async fn get_by_id(&self, table_id: &str) -> Result<TableInfo> {
        let row: TableRow = by_id(self.db.as_ref(), &TBL, table_id)
            .await?
            .ok_or_else(|| Error::table_not_found(format!("Table not found: {table_id}")))?;
        let schema = self.schema_ref_by_id(&row.schema_id).await?;
        row.into_info(&schema)
    }

    async fn storage_location_by_id(&self, table_id: &str) -> Result<(Option<String>, TableType)> {
        let table: Option<TableRow> = by_id(self.db.as_ref(), &TBL, table_id).await?;
        if let Some(t) = table {
            return Ok((t.storage_location, parse_enum::<TableType>(&t.table_type)?));
        }
        let staging: Option<StagingRow> = by_id(self.db.as_ref(), &STAGING_TABLE, table_id).await?;
        // Staging rows always become MANAGED tables, so report them as such.
        staging
            .map(|s| (Some(s.staging_location), TableType::Managed))
            .ok_or_else(|| {
                Error::table_not_found(format!(
                    "Neither table nor staging table found with id: {table_id}"
                ))
            })
    }

    async fn staging_location(&self, staging_id: &str) -> Result<String> {
        let staging: Option<StagingRow> =
            by_id(self.db.as_ref(), &STAGING_TABLE, staging_id).await?;
        staging.map(|s| s.staging_location).ok_or_else(|| {
            Error::table_not_found(format!("Staging table not found with id: {staging_id}"))
        })
    }

    async fn delete(&self, full_name: &str) -> Result<()> {
        let (_, row) = self.resolve_table(full_name).await?;
        let failed = db_err("delete table");
        let tx = self.db.begin().await.map_err(&failed)?;
        let id = [row.id_string.as_str().into()];
        drop_managed_tables(tx.as_ref(), "id_string = $1", &id).await?;
        for sql in [
            "DELETE FROM permission WHERE resource_id = $1",
            "DELETE FROM tbl WHERE id_string = $1",
        ] {
            tx.execute(sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn rename(
        &self,
        full_name: &str,
        new_name: &str,
        updated_by: Option<&str>,
    ) -> Result<TableInfo> {
        validate_sql_object_name(new_name)?;
        let (schema, mut row) = self.resolve_table(full_name).await?;
        if row.name == new_name {
            return row.into_info(&schema);
        }
        if self.table_row(&schema.schema_id, new_name).await?.is_some() {
            return Err(table_exists(new_name));
        }
        row.name = new_name.to_string();
        crate::sql::touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);
        let written = update(self.db.as_ref(), &TBL, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::TableAlreadyExists,
                format!("Table already exists: {new_name}"),
            ))?;
        if written == 0 {
            return Err(Error::table_not_found(format!(
                "Table not found: {full_name}"
            )));
        }
        row.into_info(&schema)
    }

    async fn list(
        &self,
        catalog_name: &str,
        schema_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<TableInfo>, Option<String>)> {
        let schema = self.resolve_schema(catalog_name, schema_name).await?;
        let (rows, next) = self
            .paged_list::<TableRow, _>(
                &TBL,
                &["schema_id = $1"],
                vec![schema.schema_id.as_str().into()],
                "name",
                page,
                |r| r.name.clone(),
            )
            .await?;
        let infos: Result<Vec<TableInfo>> =
            rows.into_iter().map(|r| r.into_info(&schema)).collect();
        Ok((infos?, next))
    }

    async fn create_staging(
        &self,
        req: CreateStagingTable,
        created_by: Option<&str>,
    ) -> Result<StagingTableInfo> {
        // Upstream `StagingTableRepository.createStagingTable`: reserve
        // `<schema or catalog location>/tables/<id>` (or the
        // `storage-root.tables` fallback) for a table about to be created.
        self.check_managed_enabled()?;
        validate_sql_object_name(&req.name)?;
        let schema = self
            .resolve_schema(&req.catalog_name, &req.schema_name)
            .await?;
        let id = Uuid::new_v4().to_string();
        let location = schema
            .managed_parent_location(&self.roots, ManagedResource::Table)?
            .managed_child_location(ManagedResource::Table, &id)?;
        if self
            .table_row(&schema.schema_id, &req.name)
            .await?
            .is_some()
        {
            return Err(Error::new(
                ErrorCode::TableAlreadyExists,
                format!("Table already exists: {}", req.name),
            ));
        }
        if self.staging_row_at(&location).await?.is_some() {
            return Err(Error::new(
                ErrorCode::TableAlreadyExists,
                format!("Staging table already exists at: {location}"),
            ));
        }

        let now = now_epoch_millis();
        let row = StagingRow {
            id_string: id.clone(),
            name: req.name.clone(),
            schema_id: schema.schema_id.clone(),
            staging_location: location.to_string(),
            created_at: now,
            created_by: created_by.map(str::to_string),
            accessed_at: now,
            stage_committed: false,
            stage_committed_at: None,
            purge_state: 0,
            num_cleanup_retries: 0,
            last_cleanup_at: None,
        };
        insert(self.db.as_ref(), &STAGING_TABLE, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::TableAlreadyExists,
                format!("Staging table already exists at: {location}"),
            ))?;

        Ok(StagingTableInfo {
            name: Some(req.name),
            catalog_name: Some(schema.catalog_name),
            schema_name: Some(schema.schema_name),
            id: Some(id),
            staging_location: Some(location.into_string()),
        })
    }

    async fn require_owned_staging(
        &self,
        location: &NormalizedUrl,
        caller: Option<&str>,
    ) -> Result<()> {
        self.check_managed_enabled()?;
        SqlMetastore::require_owned_staging(self, location, caller)
            .await
            .map(|_| ())
    }
}

impl SqlMetastore {
    /// Upstream `persistTable`, with the UniForm fields of a Delta API
    /// create.
    pub(crate) async fn persist_table(
        &self,
        req: CreateTable,
        uniform: Option<uc_delta::uniform::UniformIceberg>,
        iceberg_metadata: Option<&NormalizedUrl>,
        created_by: Option<&str>,
    ) -> Result<TableInfo> {
        // Native Iceberg tables come only from the Iceberg REST catalog.
        let native = req.data_source_format == Some(DataSourceFormat::Iceberg);
        if native && iceberg_metadata.is_none() {
            return Err(Error::new(
                ErrorCode::UnsupportedTableFormat,
                "createTable cannot create native ICEBERG tables; use the Iceberg REST catalog \
                 endpoints instead.",
            ));
        }
        if !native && iceberg_metadata.is_some() {
            return Err(Error::internal(
                "createTableForIceberg requires ICEBERG format.",
            ));
        }
        validate_sql_object_name(&req.name)?;
        let mut req = req;
        validate_columns(&mut req.columns, req.data_source_format)?;
        let schema = self
            .resolve_schema(&req.catalog_name, &req.schema_name)
            .await?;
        let full_name = schema.child_full_name(&req.name);
        if self
            .table_row(&schema.schema_id, &req.name)
            .await?
            .is_some()
        {
            return Err(table_exists(&full_name));
        }

        // Upstream `TableRepository.persistTable`: an EXTERNAL table names
        // its own location; a MANAGED table takes over a staging table (and
        // its id) that the caller created.
        let mut staging_id = None;
        let (id, storage_location) = match req.table_type {
            TableType::External => {
                let raw = req.storage_location.as_deref().ok_or_else(|| {
                    Error::invalid_argument("Storage location is required for external table")
                })?;
                let url = NormalizedUrl::parse(raw)?;
                if url.is_cloud_storage_root() {
                    return Err(Error::invalid_argument(format!(
                        "External table storage location must include a non-empty path prefix: {raw}"
                    )));
                }
                self.validate_not_overlap_with_managed_storage(&url).await?;
                (Uuid::new_v4().to_string(), Some(url.into_string()))
            }
            TableType::Managed => {
                self.check_managed_enabled()?;
                if !matches!(
                    req.data_source_format,
                    Some(DataSourceFormat::Delta) | Some(DataSourceFormat::Iceberg)
                ) {
                    return Err(Error::invalid_argument(
                        "Managed table creation is only supported for Delta and Iceberg formats.",
                    ));
                }
                let raw = req.storage_location.as_deref().ok_or_else(|| {
                    Error::invalid_argument("Storage location is required for managed table")
                })?;
                let url = NormalizedUrl::parse(raw)?;
                let staging = self.require_owned_staging(&url, created_by).await?;
                if staging.stage_committed {
                    return Err(staging_committed(&url));
                }
                if req.data_source_format == Some(DataSourceFormat::Delta) {
                    validate_table_id_property(req.properties.as_ref(), &staging.id_string)?;
                }
                staging_id = Some(staging.id_string.clone());
                (staging.id_string, Some(url.into_string()))
            }
            TableType::MetricView | TableType::View => {
                let metric = req.table_type == TableType::MetricView;
                let label = if metric { "metric view" } else { "view" };
                self.validate_view_like(&req, label).await?;
                // A metric view's dependencies always come from the client.
                if metric && dependencies(&req).is_empty() {
                    return Err(Error::invalid_argument(
                        "view_dependencies must contain at least one entry for metric view",
                    ));
                }
                (Uuid::new_v4().to_string(), None)
            }
            TableType::StreamingTable => {
                return Err(Error::invalid_argument(
                    "STREAMING TABLE creation is not supported yet.",
                ));
            }
            TableType::MaterializedView => {
                return Err(Error::invalid_argument(
                    "MATERIALIZED VIEW creation is not supported yet.",
                ));
            }
        };
        // Only view-like tables keep their dependencies (upstream).
        let view_like = matches!(req.table_type, TableType::MetricView | TableType::View);

        let now = now_epoch_millis();
        let mut row = TableRow {
            id_string: id,
            name: req.name.clone(),
            schema_id: schema.schema_id.clone(),
            table_type: enum_str(&req.table_type),
            data_source_format: req.data_source_format.map(|f| enum_str(&f)),
            columns: Some(req.columns),
            storage_location,
            comment: req.comment,
            properties: req.properties,
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
            view_definition: req.view_definition,
            view_dependencies: req.view_dependencies.filter(|_| view_like),
            iceberg_metadata_location: iceberg_metadata.map(ToString::to_string),
            uniform_iceberg_converted_delta_version: None,
            uniform_iceberg_converted_delta_timestamp: None,
        };
        if let Some(uniform) = &uniform {
            row.set_uniform(uniform);
        }

        let Some(staging_id) = staging_id else {
            return self.insert_table(row, &full_name).await?.into_info(&schema);
        };
        // Commit the staging table and create the table in one transaction,
        // so a staging location is used at most once.
        let failed = db_err("create managed table");
        let tx = self.db.begin().await.map_err(&failed)?;
        let committed = tx
            .execute(
                "UPDATE staging_table SET stage_committed = $1, stage_committed_at = $2, \
                     accessed_at = $2 \
                 WHERE id_string = $3 AND stage_committed = $4",
                &[true.into(), now.into(), staging_id.into(), false.into()],
            )
            .await
            .map_err(&failed)?;
        if committed == 0 {
            let location = row.storage_location.clone().unwrap_or_default();
            return Err(staging_committed(&location));
        }
        insert(tx.as_ref(), &TBL, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::TableAlreadyExists,
                format!("Table already exists: {full_name}"),
            ))?;
        tx.commit().await.map_err(&failed)?;
        row.into_info(&schema)
    }
}

fn staging_committed(location: &dyn std::fmt::Display) -> Error {
    Error::failed_precondition(format!("Staging table already committed: {location}"))
}
