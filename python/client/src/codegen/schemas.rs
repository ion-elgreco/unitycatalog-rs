// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::SchemaClient;
use uc_client::models::schemas::v1::*;
use uc_client::models::*;

/// The requests on one `SchemaInfo`.
#[pyclass(name = "SchemaClient", module = "unitycatalog._core.schemas")]
pub struct PySchemaClient {
    pub(crate) client: SchemaClient,
}

#[pymethods]
impl PySchemaClient {
    /// Get a schema.
    ///
    /// Args:
    ///     include_browse: Also answer a caller who sees the schema through the `BROWSE` privilege only,
    ///                     with its discovery fields and `browse_only` set.
    ///
    /// Returns:
    ///     A schema: the second layer of the three-level namespace. It holds tables, volumes, functions
    ///     and models.
    #[pyo3(signature = (*, include_browse = None))]
    pub fn get(
        &self,
        py: Python,
        include_browse: Option<bool>,
    ) -> PyUnityCatalogResult<PySchemaInfo> {
        let request = self.client.get().with_include_browse(include_browse);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PySchemaInfo::from(result))
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, include_browse = None))]
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
        include_browse: Option<bool>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySchemaInfo>> {
        let request = self.client.get().with_include_browse(include_browse);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySchemaInfo::from(result))
        })
    }

    /// Change a schema.
    ///
    /// Args:
    ///     comment: User-provided free-form text description.
    ///     properties: Key-value properties of the schema. They replace the properties of the schema.
    ///     new_name: New name of the schema, relative to the parent catalog.
    ///
    /// Returns:
    ///     A schema: the second layer of the three-level namespace. It holds tables, volumes, functions
    ///     and models.
    #[pyo3(signature = (*, comment = None, properties = None, new_name = None))]
    pub fn update(
        &self,
        py: Python,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        new_name: Option<String>,
    ) -> PyUnityCatalogResult<PySchemaInfo> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_new_name(new_name);
        #[allow(clippy::let_unit_value)]
        let result = crate::codegen::runtime::block_on(py, request.into_future())??;
        Ok(PySchemaInfo::from(result))
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, comment = None, properties = None, new_name = None))]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        new_name: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySchemaInfo>> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_new_name(new_name);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySchemaInfo::from(result))
        })
    }

    /// Delete a schema.
    ///
    /// Args:
    ///     force: Delete the tables, volumes, functions and models of the schema too.
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

impl PySchemaClient {
    pub fn new(client: SchemaClient) -> Self {
        Self { client }
    }
}
