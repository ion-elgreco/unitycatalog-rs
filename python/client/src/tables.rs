//! `unitycatalog._core.tables`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_api::{ColumnTypeName, DataSourceFormat, TableType};
use uc_common::SecurablePropertiesMap as Properties;

use crate::str_enum::StrEnum;
use crate::utils::repr;

/// A column of a table.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    from_py_object,
    module = "unitycatalog._core.tables"
)]
#[derive(Clone, PartialEq)]
pub struct ColumnInfo {
    /// The name of the column.
    pub name: Option<String>,
    /// The type as SQL text, such as `"bigint"`.
    pub type_text: Option<String>,
    /// The type as JSON, as Spark writes it.
    pub type_json: Option<String>,
    /// The name of the type.
    pub type_name: Option<StrEnum<ColumnTypeName>>,
    /// The precision of a `DECIMAL` type.
    pub type_precision: Option<i32>,
    /// The scale of a `DECIMAL` type.
    pub type_scale: Option<i32>,
    /// The format of an `INTERVAL` type.
    pub type_interval_type: Option<String>,
    /// The position of the column, from 0.
    pub position: Option<i32>,
    /// A description.
    pub comment: Option<String>,
    /// Whether the column can be null.
    pub nullable: Option<bool>,
    /// The position of the column in the partition key, from 0.
    pub partition_index: Option<i32>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ColumnInfo {
    #[new]
    #[pyo3(signature = (
        name=None, *, type_text=None, type_json=None, type_name=None, type_precision=None,
        type_scale=None, type_interval_type=None, position=None, comment=None, nullable=None,
        partition_index=None
    ))]
    fn new(
        name: Option<String>,
        type_text: Option<String>,
        type_json: Option<String>,
        type_name: Option<StrEnum<ColumnTypeName>>,
        type_precision: Option<i32>,
        type_scale: Option<i32>,
        type_interval_type: Option<String>,
        position: Option<i32>,
        comment: Option<String>,
        nullable: Option<bool>,
        partition_index: Option<i32>,
    ) -> Self {
        Self {
            name,
            type_text,
            type_json,
            type_name,
            type_precision,
            type_scale,
            type_interval_type,
            position,
            comment,
            nullable,
            partition_index,
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "type_text",
            "type_json",
            "type_name",
            "type_precision",
            "type_scale",
            "type_interval_type",
            "position",
            "comment",
            "nullable",
            "partition_index",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::ColumnInfo> for ColumnInfo {
    fn from(c: uc_api::ColumnInfo) -> Self {
        Self {
            name: c.name,
            type_text: c.type_text,
            type_json: c.type_json,
            type_name: c.type_name.map(StrEnum),
            type_precision: c.type_precision,
            type_scale: c.type_scale,
            type_interval_type: c.type_interval_type,
            position: c.position,
            comment: c.comment,
            nullable: c.nullable,
            partition_index: c.partition_index,
        }
    }
}

impl From<ColumnInfo> for uc_api::ColumnInfo {
    fn from(c: ColumnInfo) -> Self {
        Self {
            name: c.name,
            type_text: c.type_text,
            type_json: c.type_json,
            type_name: c.type_name.map(|t| t.0),
            type_precision: c.type_precision,
            type_scale: c.type_scale,
            type_interval_type: c.type_interval_type,
            position: c.position,
            comment: c.comment,
            nullable: c.nullable,
            partition_index: c.partition_index,
        }
    }
}

/// A table or a function that a view or a function depends on: one of the
/// two full names is set.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    from_py_object,
    module = "unitycatalog._core.tables"
)]
#[derive(Clone, PartialEq)]
pub struct Dependency {
    /// The full name of the table: `catalog.schema.table`.
    pub table_full_name: Option<String>,
    /// The full name of the function: `catalog.schema.function`.
    pub function_full_name: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Dependency {
    #[new]
    #[pyo3(signature = (*, table_full_name=None, function_full_name=None))]
    fn new(table_full_name: Option<String>, function_full_name: Option<String>) -> Self {
        Self {
            table_full_name,
            function_full_name,
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["table_full_name", "function_full_name"])
    }
}

impl From<uc_api::Dependency> for Dependency {
    fn from(d: uc_api::Dependency) -> Self {
        Self {
            table_full_name: d.table.map(|t| t.table_full_name),
            function_full_name: d.function.map(|f| f.function_full_name),
        }
    }
}

impl From<Dependency> for uc_api::Dependency {
    fn from(d: Dependency) -> Self {
        Self {
            table: d
                .table_full_name
                .map(|table_full_name| uc_api::TableDependency { table_full_name }),
            function: d
                .function_full_name
                .map(|function_full_name| uc_api::FunctionDependency { function_full_name }),
        }
    }
}

/// The dependencies in the API's `{"dependencies": [...]}`.
pub fn dependencies_from_api(list: uc_api::DependencyList) -> Vec<Dependency> {
    let dependencies = list.dependencies.unwrap_or_default();
    dependencies.into_iter().map(Dependency::from).collect()
}

/// `dependencies` as the API's `{"dependencies": [...]}`.
pub fn dependencies_to_api(dependencies: Vec<Dependency>) -> uc_api::DependencyList {
    uc_api::DependencyList {
        dependencies: Some(dependencies.into_iter().map(Into::into).collect()),
    }
}

/// A table, or a view.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.tables"
)]
#[derive(Clone, PartialEq)]
pub struct TableInfo {
    /// The name of the table, in its schema.
    pub name: Option<String>,
    /// The name of the catalog.
    pub catalog_name: Option<String>,
    /// The name of the schema.
    pub schema_name: Option<String>,
    /// The kind of table.
    pub table_type: Option<StrEnum<TableType>>,
    /// The format of the data files.
    pub data_source_format: Option<StrEnum<DataSourceFormat>>,
    /// The columns.
    pub columns: Option<Vec<ColumnInfo>>,
    /// The URL of the data.
    pub storage_location: Option<String>,
    /// A description.
    pub comment: Option<String>,
    /// Key-value properties.
    pub properties: Option<Properties>,
    /// The username of the owner.
    pub owner: Option<String>,
    /// The creation time, in epoch milliseconds.
    pub created_at: Option<i64>,
    /// The username of the creator.
    pub created_by: Option<String>,
    /// The time of the last update, in epoch milliseconds.
    pub updated_at: Option<i64>,
    /// The username of the last updater.
    pub updated_by: Option<String>,
    /// The unique identifier.
    pub table_id: Option<String>,
    /// The query of a view.
    pub view_definition: Option<String>,
    /// The tables and functions that a view reads.
    pub view_dependencies: Option<Vec<Dependency>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl TableInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "catalog_name",
            "schema_name",
            "table_type",
            "data_source_format",
            "columns",
            "storage_location",
            "comment",
            "properties",
            "owner",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "table_id",
            "view_definition",
            "view_dependencies",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::TableInfo> for TableInfo {
    fn from(t: uc_api::TableInfo) -> Self {
        Self {
            name: t.name,
            catalog_name: t.catalog_name,
            schema_name: t.schema_name,
            table_type: t.table_type.map(StrEnum),
            data_source_format: t.data_source_format.map(StrEnum),
            columns: t
                .columns
                .map(|columns| columns.into_iter().map(ColumnInfo::from).collect()),
            storage_location: t.storage_location,
            comment: t.comment,
            properties: t.properties,
            owner: t.owner,
            created_at: t.created_at,
            created_by: t.created_by,
            updated_at: t.updated_at,
            updated_by: t.updated_by,
            table_id: t.table_id,
            view_definition: t.view_definition,
            view_dependencies: t.view_dependencies.map(dependencies_from_api),
        }
    }
}

/// A staging table: the id and the location kept for a new managed table.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.tables"
)]
#[derive(Clone, PartialEq)]
pub struct StagingTableInfo {
    /// The name of the table to create.
    pub name: Option<String>,
    /// The name of the catalog.
    pub catalog_name: Option<String>,
    /// The name of the schema.
    pub schema_name: Option<String>,
    /// The id of the table to create: give it as the table property
    /// `io.unitycatalog.tableId`.
    pub id: Option<String>,
    /// The location of the table to create.
    pub staging_location: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl StagingTableInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "catalog_name",
            "schema_name",
            "id",
            "staging_location",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::StagingTableInfo> for StagingTableInfo {
    fn from(s: uc_api::StagingTableInfo) -> Self {
        Self {
            name: s.name,
            catalog_name: s.catalog_name,
            schema_name: s.schema_name,
            id: s.id,
            staging_location: s.staging_location,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.tables",
    "The classes of the table methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(
        parent,
        "tables",
        [TableInfo, ColumnInfo, Dependency, StagingTableInfo]
    )
}
