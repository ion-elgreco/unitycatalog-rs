//! `unitycatalog._core.tokens`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::utils::repr;

/// A personal access token. Only `create_pat` returns its secret,
/// `token_value`; the `repr` leaves it out.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.tokens"
)]
#[derive(Clone, PartialEq)]
pub struct PatInfo {
    /// The unique identifier.
    pub id: Option<String>,
    /// The name of the token.
    pub name: Option<String>,
    /// The first characters of the token, to recognize it.
    pub token_prefix: Option<String>,
    /// The username of the owner.
    pub owner: Option<String>,
    /// The creation time, in epoch milliseconds.
    pub created_at: Option<i64>,
    /// The expiry time, in epoch milliseconds; `None` for no expiry.
    pub expires_at: Option<i64>,
    /// The token: only on the result of `create_pat`.
    pub token_value: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PatInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        // Not `token_value`: a repr ends up in logs.
        let fields = [
            "id",
            "name",
            "token_prefix",
            "owner",
            "created_at",
            "expires_at",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::PatInfo> for PatInfo {
    fn from(p: uc_api::PatInfo) -> Self {
        Self {
            id: p.id,
            name: p.name,
            token_prefix: p.token_prefix,
            owner: p.owner,
            created_at: p.created_at,
            expires_at: p.expires_at,
            token_value: p.token_value,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.tokens",
    "The classes of the personal access token methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "tokens", [PatInfo])
}
