//! `unitycatalog._core.volumes`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_api::VolumeType;

use crate::str_enum::StrEnum;
use crate::utils::repr;

/// A volume: a directory of files in a schema.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.volumes"
)]
#[derive(Clone, PartialEq)]
pub struct VolumeInfo {
    /// The name of the catalog.
    pub catalog_name: Option<String>,
    /// The name of the schema.
    pub schema_name: Option<String>,
    /// The name of the volume, in its schema.
    pub name: Option<String>,
    /// A description.
    pub comment: Option<String>,
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
    pub volume_id: Option<String>,
    /// The kind of volume.
    pub volume_type: Option<StrEnum<VolumeType>>,
    /// The URL of the files.
    pub storage_location: Option<String>,
    /// The full name: `catalog.schema.volume`.
    pub full_name: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl VolumeInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "catalog_name",
            "schema_name",
            "name",
            "comment",
            "owner",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "volume_id",
            "volume_type",
            "storage_location",
            "full_name",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::VolumeInfo> for VolumeInfo {
    fn from(v: uc_api::VolumeInfo) -> Self {
        Self {
            catalog_name: v.catalog_name,
            schema_name: v.schema_name,
            name: v.name,
            comment: v.comment,
            owner: v.owner,
            created_at: v.created_at,
            created_by: v.created_by,
            updated_at: v.updated_at,
            updated_by: v.updated_by,
            volume_id: v.volume_id,
            volume_type: v.volume_type.map(StrEnum),
            storage_location: v.storage_location,
            full_name: v.full_name,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.volumes",
    "The classes of the volume methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "volumes", [VolumeInfo])
}
