//! `unitycatalog._core.permissions`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_api::Privilege;

use crate::str_enum::StrEnum;
use crate::utils::repr;

/// The privileges of a principal on a securable.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.permissions"
)]
#[derive(Clone, PartialEq)]
pub struct PrivilegeAssignment {
    /// The user or group.
    pub principal: String,
    /// The privileges granted to it.
    pub privileges: Vec<StrEnum<Privilege>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PrivilegeAssignment {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["principal", "privileges"])
    }
}

impl From<uc_api::PrivilegeAssignment> for PrivilegeAssignment {
    fn from(a: uc_api::PrivilegeAssignment) -> Self {
        Self {
            principal: a.principal,
            privileges: a.privileges.into_iter().map(StrEnum).collect(),
        }
    }
}

/// A change to the privileges of a principal: those to grant and to
/// revoke.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    from_py_object,
    module = "unitycatalog._core.permissions"
)]
#[derive(Clone, PartialEq)]
pub struct PermissionsChange {
    /// The user or group.
    pub principal: String,
    /// The privileges to grant.
    pub add: Vec<StrEnum<Privilege>>,
    /// The privileges to revoke.
    pub remove: Vec<StrEnum<Privilege>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PermissionsChange {
    #[new]
    #[pyo3(signature = (principal, *, add=None, remove=None))]
    fn new(
        principal: String,
        add: Option<Vec<StrEnum<Privilege>>>,
        remove: Option<Vec<StrEnum<Privilege>>>,
    ) -> Self {
        Self {
            principal,
            add: add.unwrap_or_default(),
            remove: remove.unwrap_or_default(),
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &["principal", "add", "remove"])
    }
}

impl From<PermissionsChange> for uc_api::PermissionsChange {
    fn from(c: PermissionsChange) -> Self {
        Self {
            principal: c.principal,
            add: c.add.into_iter().map(|p| p.0).collect(),
            remove: c.remove.into_iter().map(|p| p.0).collect(),
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.permissions",
    "The classes of the permission methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(
        parent,
        "permissions",
        [PrivilegeAssignment, PermissionsChange]
    )
}
