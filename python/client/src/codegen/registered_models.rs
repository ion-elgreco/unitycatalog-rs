// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::RegisteredModelClient;
use uc_client::models::models::v1::*;
use uc_client::models::*;

/// The requests on one `RegisteredModelInfo`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyclass(name = "RegisteredModelClient", module = "unitycatalog._core.models")]
pub struct PyRegisteredModelClient {
    pub(crate) client: RegisteredModelClient,
}

#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyRegisteredModelClient {
    /// Get a registered model.
    ///
    /// Returns:
    ///     A registered model: a securable in the three-level namespace (catalog.schema.model) that
    ///     holds
    ///     model versions.
    pub fn get(&self, py: Python) -> PyUnityCatalogResult<PyRegisteredModelInfo> {
        let request = self.client.get();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRegisteredModelInfo::from(result))
        })
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRegisteredModelInfo>> {
        let request = self.client.get();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRegisteredModelInfo::from(result))
        })
    }

    /// Change a registered model.
    ///
    /// Args:
    ///     comment: User-provided free-form text description.
    ///     new_name: New name of the registered model, relative to its schema.
    ///
    /// Returns:
    ///     A registered model: a securable in the three-level namespace (catalog.schema.model) that
    ///     holds
    ///     model versions.
    #[pyo3(signature = (*, comment = None, new_name = None))]
    pub fn update(
        &self,
        py: Python,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> PyUnityCatalogResult<PyRegisteredModelInfo> {
        let mut request = self.client.update();
        request = request.with_comment(comment);
        request = request.with_new_name(new_name);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRegisteredModelInfo::from(result))
        })
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, comment = None, new_name = None))]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRegisteredModelInfo>> {
        let mut request = self.client.update();
        request = request.with_comment(comment);
        request = request.with_new_name(new_name);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRegisteredModelInfo::from(result))
        })
    }

    /// Delete a registered model. A model with versions needs `force`.
    ///
    /// Args:
    ///     force: Delete the model versions of the registered model too.
    #[pyo3(signature = (*, force = None))]
    pub fn delete(&self, py: Python, force: Option<bool>) -> PyUnityCatalogResult<()> {
        let mut request = self.client.delete();
        request = request.with_force(force);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, force = None))]
    pub fn delete_async<'py>(
        &self,
        py: Python<'py>,
        force: Option<bool>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let mut request = self.client.delete();
        request = request.with_force(force);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }
}

impl PyRegisteredModelClient {
    pub fn new(client: RegisteredModelClient) -> Self {
        Self { client }
    }
}
