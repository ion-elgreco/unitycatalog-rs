//! [`StrEnum`]: a `uc_api` enum as a Python `str`.

use std::collections::{HashMap, HashSet};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;
use pyo3_stub_gen::{PyStubType, TypeInfo};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A `uc_api` enum, which Python takes and gives as its name in the JSON
/// body: `"MANAGED"`, `"USE CATALOG"`. Another string raises `ValueError`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StrEnum<T>(pub T);

impl<'a, 'py, T: DeserializeOwned> FromPyObject<'a, 'py> for StrEnum<T> {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        let name: String = obj.extract()?;
        serde_json::from_value(serde_json::Value::String(name))
            .map(StrEnum)
            .map_err(|e| PyValueError::new_err(format!("invalid {}: {e}", type_name::<T>())))
    }
}

impl<'py, T: Serialize> IntoPyObject<'py> for StrEnum<T> {
    type Target = PyString;
    type Output = Bound<'py, PyString>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        match serde_json::to_value(&self.0) {
            Ok(serde_json::Value::String(name)) => Ok(PyString::new(py, &name)),
            _ => Err(PyValueError::new_err(format!(
                "{} has no name",
                type_name::<T>()
            ))),
        }
    }
}

/// In the stubs, the `Literal` type of the same name in
/// `unitycatalog/types.py`. The stubs import the module as
/// `from unitycatalog import types`, so the name is `types.TableType`.
impl<T> PyStubType for StrEnum<T> {
    fn type_output() -> TypeInfo {
        TypeInfo {
            name: format!("types.{}", type_name::<T>()),
            source_module: None,
            import: HashSet::from(["unitycatalog.types".into()]),
            type_refs: HashMap::new(),
        }
    }
}

/// The name of `T` without its module path: `TableType`.
fn type_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    name.rsplit("::").next().unwrap_or(name)
}
