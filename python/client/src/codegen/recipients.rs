// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::RecipientClient;
use uc_client::models::recipients::v1::*;
use uc_client::models::*;

/// The requests on one `RecipientInfo`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyclass(name = "RecipientClient", module = "unitycatalog._core.recipients")]
pub struct PyRecipientClient {
    pub(crate) client: RecipientClient,
}

#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyRecipientClient {
    /// Get a recipient. The response does not hold the bearer token.
    ///
    /// Returns:
    ///     A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares
    ///     with
    ///     a bearer token. The server keeps a hash of the token only.
    pub fn get(&self, py: Python) -> PyUnityCatalogResult<PyRecipientInfo> {
        let request = self.client.get();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRecipientInfo::from(result))
        })
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipientInfo>> {
        let request = self.client.get();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipientInfo::from(result))
        })
    }

    /// Change a recipient: its comment and the shares it can read.
    ///
    /// Args:
    ///     comment: User-provided free-form text description.
    ///     add_shares: Names of the shares to give the recipient access to.
    ///     remove_shares: Names of the shares to take the recipient's access away from.
    ///
    /// Returns:
    ///     A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares
    ///     with
    ///     a bearer token. The server keeps a hash of the token only.
    #[pyo3(signature = (*, comment = None, add_shares = None, remove_shares = None))]
    pub fn update(
        &self,
        py: Python,
        comment: Option<String>,
        add_shares: Option<Vec<String>>,
        remove_shares: Option<Vec<String>>,
    ) -> PyUnityCatalogResult<PyRecipientInfo> {
        let mut request = self.client.update();
        request = request.with_comment(comment);
        if let Some(add_shares) = add_shares {
            request = request.with_add_shares(add_shares);
        }
        if let Some(remove_shares) = remove_shares {
            request = request.with_remove_shares(remove_shares);
        }
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRecipientInfo::from(result))
        })
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, comment = None, add_shares = None, remove_shares = None))]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        add_shares: Option<Vec<String>>,
        remove_shares: Option<Vec<String>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipientInfo>> {
        let mut request = self.client.update();
        request = request.with_comment(comment);
        if let Some(add_shares) = add_shares {
            request = request.with_add_shares(add_shares);
        }
        if let Some(remove_shares) = remove_shares {
            request = request.with_remove_shares(remove_shares);
        }
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipientInfo::from(result))
        })
    }

    /// Delete a recipient.
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

    /// Replace the bearer token of a recipient. The old token stops working. The response holds the new
    /// token.
    ///
    /// Returns:
    ///     A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares
    ///     with
    ///     a bearer token. The server keeps a hash of the token only.
    pub fn rotate_recipient_token(&self, py: Python) -> PyUnityCatalogResult<PyRecipientInfo> {
        let request = self.client.rotate_recipient_token();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRecipientInfo::from(result))
        })
    }

    /// Like `rotate_recipient_token`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn rotate_recipient_token_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipientInfo>> {
        let request = self.client.rotate_recipient_token();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipientInfo::from(result))
        })
    }

    /// Replace the bearer token of a recipient and return a Delta Sharing profile with the new token. The
    /// old token stops working.
    ///
    /// Returns:
    ///     A Delta Sharing profile file (`profile.json`): what a Delta Sharing client needs to connect
    ///     to
    ///     the server as a recipient.
    pub fn create_recipient_profile(&self, py: Python) -> PyUnityCatalogResult<PyRecipientProfile> {
        let request = self.client.create_recipient_profile();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRecipientProfile::from(result))
        })
    }

    /// Like `create_recipient_profile`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn create_recipient_profile_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipientProfile>> {
        let request = self.client.create_recipient_profile();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipientProfile::from(result))
        })
    }
}

impl PyRecipientClient {
    pub fn new(client: RecipientClient) -> Self {
        Self { client }
    }
}
