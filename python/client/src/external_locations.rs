//! `unitycatalog._core.external_locations`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::utils::repr;

/// An external location: a cloud storage path and the credential for it.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.external_locations"
)]
#[derive(Clone, PartialEq)]
pub struct ExternalLocationInfo {
    /// The name of the external location.
    pub name: Option<String>,
    /// The URL of the path, such as `s3://bucket/prefix`.
    pub url: Option<String>,
    /// The name of the credential for the path.
    pub credential_name: Option<String>,
    /// The username of the owner.
    pub owner: Option<String>,
    /// A description.
    pub comment: Option<String>,
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
    /// The id of the credential.
    pub credential_id: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ExternalLocationInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "url",
            "credential_name",
            "owner",
            "comment",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "id",
            "credential_id",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::ExternalLocationInfo> for ExternalLocationInfo {
    fn from(e: uc_api::ExternalLocationInfo) -> Self {
        Self {
            name: e.name,
            url: e.url,
            credential_name: e.credential_name,
            owner: e.owner,
            comment: e.comment,
            created_at: e.created_at,
            created_by: e.created_by,
            updated_at: e.updated_at,
            updated_by: e.updated_by,
            id: e.id,
            credential_id: e.credential_id,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.external_locations",
    "The classes of the external location methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "external_locations", [ExternalLocationInfo])
}
