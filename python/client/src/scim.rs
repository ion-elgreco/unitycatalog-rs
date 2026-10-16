//! `unitycatalog._core.scim`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::utils::repr;

/// A user, as SCIM 2.0 shows it.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.scim"
)]
#[derive(Clone, PartialEq)]
pub struct ScimUser {
    /// The unique identifier.
    pub id: Option<String>,
    /// The user's email, as principals name the user.
    pub user_name: String,
    /// The user's name.
    pub display_name: Option<String>,
    /// The identity of the user at an identity provider. Token exchange
    /// finds the user of an OAuth client by it.
    pub external_id: Option<String>,
    /// Whether the user is enabled. A deleted user is disabled.
    pub active: Option<bool>,
    /// The emails of the user; the primary one first.
    pub emails: Vec<String>,
    /// The creation time, in RFC 3339.
    pub created: Option<String>,
    /// The time of the last change, in RFC 3339.
    pub last_modified: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ScimUser {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "id",
            "user_name",
            "display_name",
            "external_id",
            "active",
            "emails",
            "created",
            "last_modified",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::ScimUser> for ScimUser {
    fn from(u: uc_api::ScimUser) -> Self {
        let mut emails = u.emails.unwrap_or_default();
        // A stable sort: the primary email first, the others in order.
        emails.sort_by_key(|e| e.primary != Some(true));
        let (created, last_modified) = match u.meta {
            Some(meta) => (meta.created, meta.last_modified),
            None => (None, None),
        };
        Self {
            id: u.id,
            user_name: u.user_name,
            display_name: u.display_name,
            external_id: u.external_id,
            active: u.active,
            emails: emails.into_iter().map(|e| e.value).collect(),
            created,
            last_modified,
        }
    }
}

/// A group of users, as SCIM 2.0 shows it.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.scim"
)]
#[derive(Clone, PartialEq)]
pub struct ScimGroup {
    /// The unique identifier.
    pub id: Option<String>,
    /// The name of the group.
    pub display_name: String,
    /// The ids of the users in the group.
    pub member_ids: Vec<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ScimGroup {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["id", "display_name", "member_ids"])
    }
}

impl From<uc_api::ScimGroup> for ScimGroup {
    fn from(g: uc_api::ScimGroup) -> Self {
        let members = g.members.unwrap_or_default();
        Self {
            id: g.id,
            display_name: g.display_name,
            member_ids: members.into_iter().map(|m| m.value).collect(),
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.scim",
    "The classes of the SCIM user and group methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "scim", [ScimUser, ScimGroup])
}
