// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::RecipientClient;
use uc_client::models::recipients::v1::*;
use uc_client::models::*;

/// The requests on one `Recipient`.
#[pyclass(name = "RecipientClient", module = "unitycatalog._core.recipients")]
pub struct PyRecipientClient {
    pub(crate) client: RecipientClient,
}

#[pymethods]
impl PyRecipientClient {
    /// Get a recipient by name.
    ///
    /// Returns:
    ///     The requested resource
    pub fn get(&self, py: Python) -> PyUnityCatalogResult<PyRecipient> {
        let request = self.client.get();
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyRecipient::from(result))
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipient>> {
        let request = self.client.get();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipient::from(result))
        })
    }

    /// Update a recipient.
    ///
    /// Args:
    ///     new_name: New name for the recipient
    ///     owner: Username of the recipient owner.
    ///     comment: Description about the recipient.
    ///     properties_kvpairs: Recipient properties as map of string key-value pairs. When provided
    ///                         in update request, the specified properties will override the existing
    ///                         properties. To add and remove properties, one would need to perform a read-
    ///                         modify-write.
    ///     expiration_time: Expiration timestamp of the token, in epoch milliseconds.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(
        signature = (
            *,
            new_name = None,
            owner = None,
            comment = None,
            properties_kvpairs = None,
            expiration_time = None
        )
    )]
    pub fn update(
        &self,
        py: Python,
        new_name: Option<String>,
        owner: Option<String>,
        comment: Option<String>,
        properties_kvpairs: ::core::option::Option<PySecurablePropertiesKvPairs>,
        expiration_time: Option<i64>,
    ) -> PyUnityCatalogResult<PyRecipient> {
        let request = self
            .client
            .update()
            .with_new_name(new_name)
            .with_owner(owner)
            .with_comment(comment)
            .with_properties_kvpairs(properties_kvpairs.map(::core::convert::Into::into))
            .with_expiration_time(expiration_time);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyRecipient::from(result))
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            *,
            new_name = None,
            owner = None,
            comment = None,
            properties_kvpairs = None,
            expiration_time = None
        )
    )]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        new_name: Option<String>,
        owner: Option<String>,
        comment: Option<String>,
        properties_kvpairs: ::core::option::Option<PySecurablePropertiesKvPairs>,
        expiration_time: Option<i64>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipient>> {
        let request = self
            .client
            .update()
            .with_new_name(new_name)
            .with_owner(owner)
            .with_comment(comment)
            .with_properties_kvpairs(properties_kvpairs.map(::core::convert::Into::into))
            .with_expiration_time(expiration_time);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipient::from(result))
        })
    }

    /// Delete a recipient.
    pub fn delete(&self, py: Python) -> PyUnityCatalogResult<()> {
        let request = self.client.delete();
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(result)
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

    /// Rotate a recipient's token: issue a new token (and activation link) and retire the existing one.
    ///
    /// Args:
    ///     existing_token_expire_in_seconds: Seconds until the existing token expires. It can only shorten
    ///                                       the existing token's lifetime, never extend it; 0 expires
    ///                                       it immediately.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(signature = (*, existing_token_expire_in_seconds = None))]
    pub fn rotate_recipient_token(
        &self,
        py: Python,
        existing_token_expire_in_seconds: Option<i64>,
    ) -> PyUnityCatalogResult<PyRecipient> {
        let request = self
            .client
            .rotate_recipient_token()
            .with_existing_token_expire_in_seconds(existing_token_expire_in_seconds);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyRecipient::from(result))
    }

    /// Like `rotate_recipient_token`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, existing_token_expire_in_seconds = None))]
    pub fn rotate_recipient_token_async<'py>(
        &self,
        py: Python<'py>,
        existing_token_expire_in_seconds: Option<i64>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipient>> {
        let request = self
            .client
            .rotate_recipient_token()
            .with_existing_token_expire_in_seconds(existing_token_expire_in_seconds);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipient::from(result))
        })
    }
}

impl PyRecipientClient {
    pub fn new(client: RecipientClient) -> Self {
        Self { client }
    }
}
