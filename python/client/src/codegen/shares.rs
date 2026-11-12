// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::ShareClient;
use uc_client::models::shares::v1::*;
use uc_client::models::*;

/// The requests on one `Share`.
#[pyclass(name = "ShareClient", module = "unitycatalog._core.shares")]
pub struct PyShareClient {
    pub(crate) client: ShareClient,
}

#[pymethods]
impl PyShareClient {
    /// Get a share by name.
    ///
    /// Args:
    ///     include_shared_data: Query for data to include in the share.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(signature = (*, include_shared_data = None))]
    pub fn get(
        &self,
        py: Python,
        include_shared_data: Option<bool>,
    ) -> PyUnityCatalogResult<PyShare> {
        let request = self
            .client
            .get()
            .with_include_shared_data(include_shared_data);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyShare::from(result))
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, include_shared_data = None))]
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
        include_shared_data: Option<bool>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyShare>> {
        let request = self
            .client
            .get()
            .with_include_shared_data(include_shared_data);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyShare::from(result))
        })
    }

    /// Update a share.
    ///
    /// Args:
    ///     updates: Array of shared data object updates. At most 100 data objects may be updated per
    ///              request.
    ///     new_name: A new name for the share.
    ///     owner: Owner of the share.
    ///     comment: User-provided free-form text description.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(
        signature = (*, updates = None, new_name = None, owner = None, comment = None)
    )]
    pub fn update(
        &self,
        py: Python,
        updates: ::core::option::Option<::std::vec::Vec<PyDataObjectUpdate>>,
        new_name: Option<String>,
        owner: Option<String>,
        comment: Option<String>,
    ) -> PyUnityCatalogResult<PyShare> {
        let request = self
            .client
            .update()
            .with_updates(
                updates
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_new_name(new_name)
            .with_owner(owner)
            .with_comment(comment);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyShare::from(result))
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (*, updates = None, new_name = None, owner = None, comment = None)
    )]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        updates: ::core::option::Option<::std::vec::Vec<PyDataObjectUpdate>>,
        new_name: Option<String>,
        owner: Option<String>,
        comment: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyShare>> {
        let request = self
            .client
            .update()
            .with_updates(
                updates
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_new_name(new_name)
            .with_owner(owner)
            .with_comment(comment);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyShare::from(result))
        })
    }

    /// Deletes a share.
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

    /// Gets the permissions for a data share from the metastore.
    ///
    /// Args:
    ///     max_results: The maximum number of results per page that should be returned. If unset or 0, the
    ///                  server's default page size is used. Negative values are rejected.
    ///     page_token: Opaque pagination token to go to next page based on previous query.
    ///
    /// Returns:
    ///     Response to list shares.
    #[pyo3(signature = (*, max_results = None, page_token = None))]
    pub fn get_share_permissions(
        &self,
        py: Python,
        max_results: Option<i32>,
        page_token: Option<String>,
    ) -> PyUnityCatalogResult<PyGetSharePermissionsResponse> {
        let request = self
            .client
            .get_share_permissions()
            .with_max_results(max_results)
            .with_page_token(page_token);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyGetSharePermissionsResponse::from(result))
    }

    /// Like `get_share_permissions`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, max_results = None, page_token = None))]
    pub fn get_share_permissions_async<'py>(
        &self,
        py: Python<'py>,
        max_results: Option<i32>,
        page_token: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyGetSharePermissionsResponse>>
    {
        let request = self
            .client
            .get_share_permissions()
            .with_max_results(max_results)
            .with_page_token(page_token);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyGetSharePermissionsResponse::from(result))
        })
    }

    /// Updates the permissions for a data share in the metastore.
    ///
    /// Args:
    ///     changes: Array of permissions change objects.
    ///     omit_permissions_list: Whether to return the latest permissions list of the share in the
    ///                            response.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(signature = (*, changes = None, omit_permissions_list = None))]
    pub fn update_share_permissions(
        &self,
        py: Python,
        changes: ::core::option::Option<::std::vec::Vec<PySharePermissionsChange>>,
        omit_permissions_list: Option<bool>,
    ) -> PyUnityCatalogResult<PyUpdateSharePermissionsResponse> {
        let request = self
            .client
            .update_share_permissions()
            .with_changes(
                changes
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_omit_permissions_list(omit_permissions_list);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PyUpdateSharePermissionsResponse::from(result))
    }

    /// Like `update_share_permissions`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, changes = None, omit_permissions_list = None))]
    pub fn update_share_permissions_async<'py>(
        &self,
        py: Python<'py>,
        changes: ::core::option::Option<::std::vec::Vec<PySharePermissionsChange>>,
        omit_permissions_list: Option<bool>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyUpdateSharePermissionsResponse>>
    {
        let request = self
            .client
            .update_share_permissions()
            .with_changes(
                changes
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_omit_permissions_list(omit_permissions_list);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyUpdateSharePermissionsResponse::from(result))
        })
    }
}

impl PyShareClient {
    pub fn new(client: ShareClient) -> Self {
        Self { client }
    }
}
