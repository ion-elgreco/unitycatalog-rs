//! `unitycatalog._core.models`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_api::ModelVersionStatus;

use crate::str_enum::StrEnum;
use crate::utils::repr;

/// A registered model: the versions of an ML model, in a schema.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.models"
)]
#[derive(Clone, PartialEq)]
pub struct RegisteredModelInfo {
    /// The name of the model, in its schema.
    pub name: Option<String>,
    /// The name of the catalog.
    pub catalog_name: Option<String>,
    /// The name of the schema.
    pub schema_name: Option<String>,
    /// The URL of the model's files.
    pub storage_location: Option<String>,
    /// The full name: `catalog.schema.model`.
    pub full_name: Option<String>,
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
    pub id: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl RegisteredModelInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "catalog_name",
            "schema_name",
            "storage_location",
            "full_name",
            "comment",
            "owner",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "id",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::RegisteredModelInfo> for RegisteredModelInfo {
    fn from(m: uc_api::RegisteredModelInfo) -> Self {
        Self {
            name: m.name,
            catalog_name: m.catalog_name,
            schema_name: m.schema_name,
            storage_location: m.storage_location,
            full_name: m.full_name,
            comment: m.comment,
            owner: m.owner,
            created_at: m.created_at,
            created_by: m.created_by,
            updated_at: m.updated_at,
            updated_by: m.updated_by,
            id: m.id,
        }
    }
}

/// A version of a registered model.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.models"
)]
#[derive(Clone, PartialEq)]
pub struct ModelVersionInfo {
    /// The name of the registered model.
    pub model_name: Option<String>,
    /// The name of the catalog.
    pub catalog_name: Option<String>,
    /// The name of the schema.
    pub schema_name: Option<String>,
    /// The version number, from 1.
    pub version: Option<i64>,
    /// The URL that the model files come from.
    pub source: Option<String>,
    /// The id of the run that made the model.
    pub run_id: Option<String>,
    /// The status of the version.
    pub status: Option<StrEnum<ModelVersionStatus>>,
    /// The URL of the version's files.
    pub storage_location: Option<String>,
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
}

#[gen_stub_pymethods]
#[pymethods]
impl ModelVersionInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "model_name",
            "catalog_name",
            "schema_name",
            "version",
            "source",
            "run_id",
            "status",
            "storage_location",
            "comment",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "id",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::ModelVersionInfo> for ModelVersionInfo {
    fn from(v: uc_api::ModelVersionInfo) -> Self {
        Self {
            model_name: v.model_name,
            catalog_name: v.catalog_name,
            schema_name: v.schema_name,
            version: v.version,
            source: v.source,
            run_id: v.run_id,
            status: v.status.map(StrEnum),
            storage_location: v.storage_location,
            comment: v.comment,
            created_at: v.created_at,
            created_by: v.created_by,
            updated_at: v.updated_at,
            updated_by: v.updated_by,
            id: v.id,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.models",
    "The classes of the registered model methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "models", [RegisteredModelInfo, ModelVersionInfo])
}
