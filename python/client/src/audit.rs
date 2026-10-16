//! `unitycatalog._core.audit`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::utils::repr;

/// A request that changed the catalog, as the audit log has it.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.audit"
)]
#[derive(Clone, PartialEq)]
pub struct AuditEvent {
    /// The unique identifier.
    pub id: Option<String>,
    /// The username of the caller, or `"anonymous"`.
    pub principal: String,
    /// The HTTP method, such as `"POST"`.
    pub method: String,
    /// The path of the request, with its query.
    pub path: String,
    /// The HTTP status of the response.
    pub status: u16,
    /// The time of the response, in epoch milliseconds.
    pub timestamp: i64,
}

#[gen_stub_pymethods]
#[pymethods]
impl AuditEvent {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = ["id", "principal", "method", "path", "status", "timestamp"];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::AuditEvent> for AuditEvent {
    fn from(e: uc_api::AuditEvent) -> Self {
        Self {
            id: e.id,
            principal: e.principal,
            method: e.method,
            path: e.path,
            status: e.status,
            timestamp: e.timestamp,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.audit",
    "The classes of the audit log method."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "audit", [AuditEvent])
}
