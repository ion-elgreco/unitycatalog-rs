//! `unitycatalog._core.iceberg_rest`.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::utils::repr;

/// The configuration of an Iceberg REST client for a catalog.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.iceberg_rest"
)]
#[derive(Clone, PartialEq)]
pub struct IcebergConfig {
    /// The properties that the client's own configuration overrides.
    pub defaults: HashMap<String, String>,
    /// The properties that override the client's own configuration, such
    /// as `prefix`.
    pub overrides: HashMap<String, String>,
    /// The endpoints that the server has, such as
    /// `"GET /v1/{prefix}/namespaces"`.
    pub endpoints: Option<Vec<String>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl IcebergConfig {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["defaults", "overrides", "endpoints"])
    }
}

impl From<uc_api::iceberg_rest::IcebergConfigResponse> for IcebergConfig {
    fn from(c: uc_api::iceberg_rest::IcebergConfigResponse) -> Self {
        Self {
            defaults: c.defaults.into_iter().collect(),
            overrides: c.overrides.into_iter().collect(),
            endpoints: c.endpoints,
        }
    }
}

/// The name of an Iceberg table: its namespace, and its name in it.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.iceberg_rest"
)]
#[derive(Clone, PartialEq)]
pub struct IcebergTableIdentifier {
    /// The levels of the namespace: the schema, as `["schema"]`.
    pub namespace: Vec<String>,
    /// The name of the table.
    pub name: String,
}

#[gen_stub_pymethods]
#[pymethods]
impl IcebergTableIdentifier {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["namespace", "name"])
    }
}

impl From<uc_api::iceberg_rest::IcebergTableIdentifier> for IcebergTableIdentifier {
    fn from(t: uc_api::iceberg_rest::IcebergTableIdentifier) -> Self {
        Self {
            namespace: t.namespace,
            name: t.name,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.iceberg_rest",
    "The classes of the Iceberg REST catalog methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(
        parent,
        "iceberg_rest",
        [IcebergConfig, IcebergTableIdentifier]
    )
}
