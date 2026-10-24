// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::ShareClient;
use uc_client::models::shares::v1::*;
use uc_client::models::*;

/// The requests on one `ShareInfo`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyclass(name = "ShareClient", module = "unitycatalog._core.shares")]
pub struct PyShareClient {
    pub(crate) client: ShareClient,
}

#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyShareClient {
    /// Get a share.
    ///
    /// Returns:
    ///     A share: a named set of tables that Delta Sharing recipients can read.
    pub fn get(&self, py: Python) -> PyUnityCatalogResult<PyShareInfo> {
        let request = self.client.get();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyShareInfo::from(result))
        })
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyShareInfo>> {
        let request = self.client.get();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyShareInfo::from(result))
        })
    }

    /// Change a share: its comment, its name and its tables.
    ///
    /// Args:
    ///     comment: User-provided free-form text description.
    ///     new_name: New name of the share.
    ///     add_objects: Tables to add to the share. A table that the share holds already is skipped.
    ///     remove_objects: Tables to remove from the share, matched by `source_table_full_name`.
    ///
    /// Returns:
    ///     A share: a named set of tables that Delta Sharing recipients can read.
    #[pyo3(
        signature = (
            *,
            comment = None,
            new_name = None,
            add_objects = None,
            remove_objects = None
        )
    )]
    pub fn update(
        &self,
        py: Python,
        comment: Option<String>,
        new_name: Option<String>,
        add_objects: ::core::option::Option<::std::vec::Vec<PySharedDataObject>>,
        remove_objects: ::core::option::Option<::std::vec::Vec<PySharedDataObject>>,
    ) -> PyUnityCatalogResult<PyShareInfo> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_new_name(new_name)
            .with_add_objects(
                add_objects
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_remove_objects(
                remove_objects
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyShareInfo::from(result))
        })
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            *,
            comment = None,
            new_name = None,
            add_objects = None,
            remove_objects = None
        )
    )]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        new_name: Option<String>,
        add_objects: ::core::option::Option<::std::vec::Vec<PySharedDataObject>>,
        remove_objects: ::core::option::Option<::std::vec::Vec<PySharedDataObject>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyShareInfo>> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_new_name(new_name)
            .with_add_objects(
                add_objects
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_remove_objects(
                remove_objects
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyShareInfo::from(result))
        })
    }

    /// Delete a share.
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

impl PyShareClient {
    pub fn new(client: ShareClient) -> Self {
        Self { client }
    }
}
