// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::RegisteredModelClient;
use uc_client::models::models::v1::*;
use uc_client::models::*;

/// The requests on one `RegisteredModelInfo`.
#[pyclass(name = "RegisteredModelClient", module = "unitycatalog._core.models")]
pub struct PyRegisteredModelClient {
    pub(crate) client: RegisteredModelClient,
}

#[pymethods]
impl PyRegisteredModelClient {
    /// Get a registered model.
    ///
    /// Args:
    ///     include_browse: Also answer a caller who sees the registered model through the `BROWSE`
    ///                     privilege only, with its discovery fields and `browse_only` set.
    ///
    /// Returns:
    ///     A registered model: a securable in the three-level namespace (catalog.schema.model) that
    ///     holds
    ///     model versions.
    #[pyo3(signature = (*, include_browse = None))]
    pub fn get(
        &self,
        py: Python,
        include_browse: Option<bool>,
    ) -> PyUnityCatalogResult<PyRegisteredModelInfo> {
        let request = self.client.get().with_include_browse(include_browse);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyRegisteredModelInfo::from(result))
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, include_browse = None))]
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
        include_browse: Option<bool>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRegisteredModelInfo>> {
        let request = self.client.get().with_include_browse(include_browse);
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
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_new_name(new_name);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyRegisteredModelInfo::from(result))
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, comment = None, new_name = None))]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRegisteredModelInfo>> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_new_name(new_name);
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
        let request = self.client.delete().with_force(force);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(result)
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
        let request = self.client.delete().with_force(force);
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
