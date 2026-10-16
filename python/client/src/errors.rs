//! `uc_common::Error` → `unitycatalog.exceptions.UcError`.
//!
//! `UcError` is a Python class (`unitycatalog/exceptions.py`), for its
//! `code` and `message` attributes.

use pyo3::PyErr;

pyo3::import_exception!(unitycatalog.exceptions, UcError);

/// `UcError(code, message)`, with the upstream code of the error
/// (`CATALOG_NOT_FOUND`): from the server, or from the client when the
/// request got no answer. It takes a `uc_common::Error`, or a
/// `uc_client::Error`.
pub fn to_py_err(e: impl Into<uc_common::Error>) -> PyErr {
    let e = e.into();
    UcError::new_err((e.code().as_str(), e.message().to_string()))
}
