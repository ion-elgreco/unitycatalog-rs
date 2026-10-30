// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::VolumeClient;
use uc_client::models::volumes::v1::*;
use uc_client::models::*;

/// The requests on one `VolumeInfo`.
#[pyclass(name = "VolumeClient", module = "unitycatalog._core.volumes")]
pub struct PyVolumeClient {
    pub(crate) client: VolumeClient,
}

#[pymethods]
impl PyVolumeClient {
    /// Get a volume.
    ///
    /// Returns:
    ///     A volume: files in the third layer of the three-level namespace, below a schema.
    pub fn get(&self, py: Python) -> PyUnityCatalogResult<PyVolumeInfo> {
        let request = self.client.get();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyVolumeInfo::from(result))
        })
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyVolumeInfo>> {
        let request = self.client.get();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyVolumeInfo::from(result))
        })
    }

    /// Change a volume.
    ///
    /// Args:
    ///     comment: User-provided free-form text description.
    ///     new_name: New name of the volume, relative to the parent schema.
    ///
    /// Returns:
    ///     A volume: files in the third layer of the three-level namespace, below a schema.
    #[pyo3(signature = (*, comment = None, new_name = None))]
    pub fn update(
        &self,
        py: Python,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> PyUnityCatalogResult<PyVolumeInfo> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_new_name(new_name);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyVolumeInfo::from(result))
        })
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, comment = None, new_name = None))]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyVolumeInfo>> {
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
            Ok(PyVolumeInfo::from(result))
        })
    }

    /// Delete a volume.
    pub fn delete(&self, py: Python) -> PyUnityCatalogResult<()> {
        let request = self.client.delete();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn delete_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.delete();
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

impl PyVolumeClient {
    pub fn new(client: VolumeClient) -> Self {
        Self { client }
    }
}
