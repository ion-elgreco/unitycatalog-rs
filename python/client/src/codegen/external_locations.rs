// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::ExternalLocationClient;
use uc_client::models::external_locations::v1::*;
use uc_client::models::*;

/// The requests on one `ExternalLocationInfo`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyclass(
    name = "ExternalLocationClient",
    module = "unitycatalog._core.external_locations"
)]
pub struct PyExternalLocationClient {
    pub(crate) client: ExternalLocationClient,
}

#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyExternalLocationClient {
    /// Get an external location.
    ///
    /// Returns:
    ///     An external location: a storage path and the credential that gives access to it.
    pub fn get(&self, py: Python) -> PyUnityCatalogResult<PyExternalLocationInfo> {
        let request = self.client.get();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyExternalLocationInfo::from(result))
        })
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyExternalLocationInfo>> {
        let request = self.client.get();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyExternalLocationInfo::from(result))
        })
    }

    /// Change an external location.
    ///
    /// Args:
    ///     url: New URL of the storage path.
    ///     new_name: New name of the external location.
    ///     comment: User-provided free-form text description.
    ///     credential_name: Name of the new credential that gives access to the path.
    ///
    /// Returns:
    ///     An external location: a storage path and the credential that gives access to it.
    #[pyo3(
        signature = (
            *,
            url = None,
            new_name = None,
            comment = None,
            credential_name = None
        )
    )]
    pub fn update(
        &self,
        py: Python,
        url: Option<String>,
        new_name: Option<String>,
        comment: Option<String>,
        credential_name: Option<String>,
    ) -> PyUnityCatalogResult<PyExternalLocationInfo> {
        let request = self
            .client
            .update()
            .with_url(url)
            .with_new_name(new_name)
            .with_comment(comment)
            .with_credential_name(credential_name);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyExternalLocationInfo::from(result))
        })
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            *,
            url = None,
            new_name = None,
            comment = None,
            credential_name = None
        )
    )]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        url: Option<String>,
        new_name: Option<String>,
        comment: Option<String>,
        credential_name: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyExternalLocationInfo>> {
        let request = self
            .client
            .update()
            .with_url(url)
            .with_new_name(new_name)
            .with_comment(comment)
            .with_credential_name(credential_name);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyExternalLocationInfo::from(result))
        })
    }

    /// Delete an external location.
    ///
    /// Args:
    ///     force: Delete the external location even when tables, volumes or models use its path.
    #[pyo3(signature = (*, force = None))]
    pub fn delete(&self, py: Python, force: Option<bool>) -> PyUnityCatalogResult<()> {
        let request = self.client.delete().with_force(force);
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

impl PyExternalLocationClient {
    pub fn new(client: ExternalLocationClient) -> Self {
        Self { client }
    }
}
