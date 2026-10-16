//! `unitycatalog._core.admin`.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::utils::repr;

/// What `restore` imported.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.admin"
)]
#[derive(Clone, PartialEq)]
pub struct RestoreResponse {
    /// The number of rows imported, per table of the store.
    pub imported: HashMap<String, u64>,
    /// The tables of the snapshot that the store does not have.
    pub skipped: Vec<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl RestoreResponse {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["imported", "skipped"])
    }
}

impl From<uc_api::RestoreResponse> for RestoreResponse {
    fn from(r: uc_api::RestoreResponse) -> Self {
        Self {
            imported: r
                .imported
                .into_iter()
                .map(|(table, rows)| (table, rows as u64))
                .collect(),
            skipped: r.skipped,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.admin",
    "The classes of the backup and restore methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "admin", [RestoreResponse])
}
