//! `unitycatalog._core.metastore`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::utils::repr;

/// The summary of the metastore, the top of the namespace.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.metastore"
)]
#[derive(Clone, PartialEq)]
pub struct MetastoreSummary {
    /// The unique identifier of the metastore.
    pub metastore_id: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MetastoreSummary {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["metastore_id"])
    }
}

impl From<uc_api::GetMetastoreSummaryResponse> for MetastoreSummary {
    fn from(s: uc_api::GetMetastoreSummaryResponse) -> Self {
        Self {
            metastore_id: s.metastore_id,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.metastore",
    "The classes of the metastore methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "metastore", [MetastoreSummary])
}
