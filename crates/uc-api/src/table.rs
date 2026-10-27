use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken, SecurablePropertiesMap, serialize_properties};

/// Names of the table-cell types Unity Catalog tracks. Mirrors upstream
/// `ColumnTypeName` enum verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ColumnTypeName {
    Boolean,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Date,
    Timestamp,
    TimestampNtz,
    String,
    Binary,
    Decimal,
    Interval,
    Array,
    Struct,
    Map,
    Variant,
    Char,
    Null,
    UserDefinedType,
    TableType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum DataSourceFormat {
    Delta,
    Iceberg,
    Csv,
    Json,
    Avro,
    Parquet,
    Orc,
    Text,
    Lance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TableType {
    Managed,
    External,
    StreamingTable,
    MaterializedView,
    MetricView,
    View,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ColumnInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_name: Option<ColumnTypeName>,
    /// Required for DecimalTypes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_precision: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_scale: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_interval_type: Option<String>,
    /// Ordinal position of column (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Defaults to true per upstream spec.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TableDependency {
    /// Full name of the dependent table: `<catalog>.<schema>.<table>`.
    pub table_full_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionDependency {
    /// Full name of the dependent function: `<catalog>.<schema>.<function>`.
    pub function_full_name: String,
}

/// Either a table or a function dependency; per upstream spec exactly one
/// of the two fields must be populated.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Dependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<TableDependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionDependency>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DependencyList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<Dependency>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TableInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_type: Option<TableType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source_format: Option<DataSourceFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ColumnInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(serialize_with = "serialize_properties")]
    pub properties: Option<SecurablePropertiesMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
    /// Definition text for view-like table types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_dependencies: Option<DependencyList>,
}

/// `PATCH /tables/{full_name}`: a new comment for the table, and new
/// comments for its columns. Upstream's API has no table update; the web
/// UI edits comments with this.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateTable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Columns that get a new comment; the others keep theirs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<UpdateColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateColumn {
    pub name: String,
    /// Empty clears the comment.
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTable {
    pub name: String,
    pub catalog_name: String,
    pub schema_name: String,
    pub table_type: TableType,
    /// Required by the spec, but the upstream server accepts it missing.
    #[serde(default)]
    pub columns: Vec<ColumnInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source_format: Option<DataSourceFormat>,
    /// Required for EXTERNAL tables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SecurablePropertiesMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_dependencies: Option<DependencyList>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListTablesResponse {
    pub tables: Vec<TableInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_wire_names;
    use serde_json::json;

    #[test]
    fn enums_match_upstream() {
        assert_wire_names(&[
            (ColumnTypeName::Boolean, "BOOLEAN"),
            (ColumnTypeName::Byte, "BYTE"),
            (ColumnTypeName::Short, "SHORT"),
            (ColumnTypeName::Int, "INT"),
            (ColumnTypeName::Long, "LONG"),
            (ColumnTypeName::Float, "FLOAT"),
            (ColumnTypeName::Double, "DOUBLE"),
            (ColumnTypeName::Date, "DATE"),
            (ColumnTypeName::Timestamp, "TIMESTAMP"),
            (ColumnTypeName::TimestampNtz, "TIMESTAMP_NTZ"),
            (ColumnTypeName::String, "STRING"),
            (ColumnTypeName::Binary, "BINARY"),
            (ColumnTypeName::Decimal, "DECIMAL"),
            (ColumnTypeName::Interval, "INTERVAL"),
            (ColumnTypeName::Array, "ARRAY"),
            (ColumnTypeName::Struct, "STRUCT"),
            (ColumnTypeName::Map, "MAP"),
            (ColumnTypeName::Variant, "VARIANT"),
            (ColumnTypeName::Char, "CHAR"),
            (ColumnTypeName::Null, "NULL"),
            (ColumnTypeName::UserDefinedType, "USER_DEFINED_TYPE"),
            (ColumnTypeName::TableType, "TABLE_TYPE"),
        ]);
        assert_wire_names(&[
            (DataSourceFormat::Delta, "DELTA"),
            (DataSourceFormat::Iceberg, "ICEBERG"),
            (DataSourceFormat::Csv, "CSV"),
            (DataSourceFormat::Json, "JSON"),
            (DataSourceFormat::Avro, "AVRO"),
            (DataSourceFormat::Parquet, "PARQUET"),
            (DataSourceFormat::Orc, "ORC"),
            (DataSourceFormat::Text, "TEXT"),
            (DataSourceFormat::Lance, "LANCE"),
        ]);
        assert_wire_names(&[
            (TableType::Managed, "MANAGED"),
            (TableType::External, "EXTERNAL"),
            (TableType::StreamingTable, "STREAMING_TABLE"),
            (TableType::MaterializedView, "MATERIALIZED_VIEW"),
            (TableType::MetricView, "METRIC_VIEW"),
            (TableType::View, "VIEW"),
        ]);
    }

    #[test]
    fn create_without_columns_has_none() {
        let create: CreateTable = serde_json::from_value(json!({
            "name": "t", "catalog_name": "main", "schema_name": "default", "table_type": "EXTERNAL",
        }))
        .unwrap();
        assert!(create.columns.is_empty());
    }
}
