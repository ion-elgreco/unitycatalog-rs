// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::TableClient;
use uc_client::models::tables::v1::*;
use uc_client::models::*;

/// The requests on one `TableInfo`.
#[pyclass(name = "TableClient", module = "unitycatalog._core.tables")]
pub struct PyTableClient {
    pub(crate) client: TableClient,
}

#[pymethods]
impl PyTableClient {
    /// Get a table.
    ///
    /// Args:
    ///     read_streaming_table_as_managed: Show a streaming table as a MANAGED table, as upstream's server
    ///                                      can. This server does not read it.
    ///     read_materialized_view_as_managed: Show a materialized view as a MANAGED table, as upstream's
    ///                                        server can. This server does not read it.
    ///
    /// Returns:
    ///     A table: data in the third layer of the three-level namespace, below a schema.
    #[pyo3(
        signature = (
            *,
            read_streaming_table_as_managed = None,
            read_materialized_view_as_managed = None
        )
    )]
    pub fn get(
        &self,
        py: Python,
        read_streaming_table_as_managed: Option<bool>,
        read_materialized_view_as_managed: Option<bool>,
    ) -> PyUnityCatalogResult<PyTableInfo> {
        let request = self
            .client
            .get()
            .with_read_streaming_table_as_managed(read_streaming_table_as_managed)
            .with_read_materialized_view_as_managed(read_materialized_view_as_managed);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTableInfo::from(result))
        })
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            *,
            read_streaming_table_as_managed = None,
            read_materialized_view_as_managed = None
        )
    )]
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
        read_streaming_table_as_managed: Option<bool>,
        read_materialized_view_as_managed: Option<bool>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTableInfo>> {
        let request = self
            .client
            .get()
            .with_read_streaming_table_as_managed(read_streaming_table_as_managed)
            .with_read_materialized_view_as_managed(read_materialized_view_as_managed);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyTableInfo::from(result))
        })
    }

    /// Change the comments of a table and its columns.
    ///
    /// Args:
    ///     comment: User-provided free-form text description.
    ///     columns: Columns that get a new comment; the others keep theirs.
    ///
    /// Returns:
    ///     A table: data in the third layer of the three-level namespace, below a schema.
    #[pyo3(signature = (*, comment = None, columns = None))]
    pub fn update(
        &self,
        py: Python,
        comment: Option<String>,
        columns: ::core::option::Option<::std::vec::Vec<PyUpdateColumn>>,
    ) -> PyUnityCatalogResult<PyTableInfo> {
        let request = self.client.update().with_comment(comment).with_columns(
            columns
                .unwrap_or_default()
                .into_iter()
                .map(::core::convert::Into::into)
                .collect::<::std::vec::Vec<_>>(),
        );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTableInfo::from(result))
        })
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, comment = None, columns = None))]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        columns: ::core::option::Option<::std::vec::Vec<PyUpdateColumn>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTableInfo>> {
        let request = self.client.update().with_comment(comment).with_columns(
            columns
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
            Ok(PyTableInfo::from(result))
        })
    }

    /// Delete a table.
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

impl PyTableClient {
    pub fn new(client: TableClient) -> Self {
        Self { client }
    }
}
