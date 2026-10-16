//! `unitycatalog._core.schemas`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_common::SecurablePropertiesMap as Properties;

use crate::utils::repr;

/// A schema: the second level of the three-level namespace.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.schemas"
)]
#[derive(Clone, PartialEq)]
pub struct SchemaInfo {
    /// The name of the schema, in its catalog.
    pub name: Option<String>,
    /// The name of the catalog.
    pub catalog_name: Option<String>,
    /// A description.
    pub comment: Option<String>,
    /// Key-value properties.
    pub properties: Option<Properties>,
    /// The full name: `catalog.schema`.
    pub full_name: Option<String>,
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
    pub schema_id: Option<String>,
    /// The root URL of the managed storage of the schema.
    pub storage_root: Option<String>,
    /// The full URL of the managed storage location of the schema.
    pub storage_location: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl SchemaInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "catalog_name",
            "comment",
            "properties",
            "full_name",
            "owner",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "schema_id",
            "storage_root",
            "storage_location",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::SchemaInfo> for SchemaInfo {
    fn from(s: uc_api::SchemaInfo) -> Self {
        Self {
            name: s.name,
            catalog_name: s.catalog_name,
            comment: s.comment,
            properties: s.properties,
            full_name: s.full_name,
            owner: s.owner,
            created_at: s.created_at,
            created_by: s.created_by,
            updated_at: s.updated_at,
            updated_by: s.updated_by,
            schema_id: s.schema_id,
            storage_root: s.storage_root,
            storage_location: s.storage_location,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.schemas",
    "The classes of the schema methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "schemas", [SchemaInfo])
}
