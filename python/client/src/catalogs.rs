//! `unitycatalog._core.catalogs`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_common::SecurablePropertiesMap as Properties;

use crate::utils::repr;

/// A catalog: the first level of the three-level namespace.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.catalogs"
)]
#[derive(Clone, PartialEq)]
pub struct CatalogInfo {
    /// The name of the catalog.
    pub name: Option<String>,
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
    pub id: Option<String>,
    /// The root URL of the managed storage of the catalog.
    pub storage_root: Option<String>,
    /// The full URL of the managed storage location of the catalog.
    pub storage_location: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl CatalogInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "comment",
            "properties",
            "owner",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "id",
            "storage_root",
            "storage_location",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::CatalogInfo> for CatalogInfo {
    fn from(c: uc_api::CatalogInfo) -> Self {
        Self {
            name: c.name,
            comment: c.comment,
            properties: c.properties,
            owner: c.owner,
            created_at: c.created_at,
            created_by: c.created_by,
            updated_at: c.updated_at,
            updated_by: c.updated_by,
            id: c.id,
            storage_root: c.storage_root,
            storage_location: c.storage_location,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.catalogs",
    "The classes of the catalog methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "catalogs", [CatalogInfo])
}
