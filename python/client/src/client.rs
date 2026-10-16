//! `Client`: `uc_client::Client` for Python.
//!
//! Each method comes twice: `name` waits for the response with the GIL
//! released, and `name_async` returns an `asyncio.Future` of it, from
//! `future_into_py` ([`PyFuture`]). Both send the request on the shared
//! Tokio runtime.

use std::collections::HashMap;
use std::sync::Arc;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use pyo3_stub_gen::{PyStubType, TypeInfo};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uc_api::{
    ColumnTypeName, CredentialPurpose, DataSourceFormat, ParameterStyle, PathOperation,
    RoutineBody, SecurityType, SqlDataAccess, TableOperation, TableType, VolumeOperation,
    VolumeType,
};
use uc_client::Page;
use uc_common::{SecurablePropertiesMap as Properties, SecurableType};

use crate::admin::RestoreResponse;
use crate::audit::AuditEvent;
use crate::catalogs::CatalogInfo;
use crate::credentials::{CredentialInfo, TemporaryCredentials};
use crate::errors::to_py_err;
use crate::external_locations::ExternalLocationInfo;
use crate::functions::{FunctionInfo, FunctionParameterInfo, parameters_to_api};
use crate::iceberg_rest::{IcebergConfig, IcebergTableIdentifier};
use crate::metastore::MetastoreSummary;
use crate::models::{ModelVersionInfo, RegisteredModelInfo};
use crate::permissions::{PermissionsChange, PrivilegeAssignment};
use crate::py_future::{NoResult, PyFuture};
use crate::runtime::rt;
use crate::schemas::SchemaInfo;
use crate::scim::{ScimGroup, ScimUser};
use crate::str_enum::StrEnum;
use crate::tables::{ColumnInfo, Dependency, StagingTableInfo, TableInfo, dependencies_to_api};
use crate::tokens::PatInfo;
use crate::volumes::VolumeInfo;

/// A client of a Unity Catalog server.
///
/// Each method comes twice: `name` waits for the response, and
/// `name_async` returns an `asyncio.Future` of it; call it in a running
/// event loop. An error raises `unitycatalog.UcError`.
///
/// The list methods return the first page of results.
#[gen_stub_pyclass]
#[pyclass(frozen, module = "unitycatalog._core")]
pub struct Client {
    inner: Arc<uc_client::Client>,
    base: String,
}

#[gen_stub_pymethods]
#[pymethods]
impl Client {
    /// Make a client.
    ///
    /// Args:
    ///     base: The base URL of the API, such as
    ///         `"http://localhost:8080/api/2.1/unity-catalog/"`.
    #[new]
    fn new(base: String) -> PyResult<Self> {
        let inner = uc_client::Client::new(&base).map_err(to_py_err)?;
        Ok(Self {
            inner: Arc::new(inner),
            base,
        })
    }

    /// A copy of the client that sends `token` as its bearer token: a
    /// personal access token, or an access token of the server.
    fn with_bearer(&self, token: &str) -> Self {
        Self {
            inner: Arc::new((*self.inner).clone().with_bearer(token)),
            base: self.base.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Client({:?})", self.base)
    }

    /// Create a catalog.
    ///
    /// Args:
    ///     name: The name.
    ///     comment: A description.
    ///     properties: Key-value properties.
    ///     storage_root: The root URL of the managed storage of the catalog,
    ///         such as `"s3://bucket/root"`.
    #[pyo3(signature = (name, *, comment=None, properties=None, storage_root=None))]
    fn create_catalog(
        &self,
        py: Python<'_>,
        name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        storage_root: Option<String>,
    ) -> PyResult<CatalogInfo> {
        let request = uc_api::CreateCatalog {
            name,
            comment,
            properties,
            storage_root,
        };
        py.detach(|| rt().block_on(self.inner.create_catalog(&request)))
            .map(CatalogInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_catalog`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, comment=None, properties=None, storage_root=None))]
    fn create_catalog_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        storage_root: Option<String>,
    ) -> PyResult<PyFuture<'py, CatalogInfo>> {
        let request = uc_api::CreateCatalog {
            name,
            comment,
            properties,
            storage_root,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_catalog(&request)
                .await
                .map(CatalogInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get a catalog.
    ///
    /// Args:
    ///     name: The name.
    fn get_catalog(&self, py: Python<'_>, name: String) -> PyResult<CatalogInfo> {
        py.detach(|| rt().block_on(self.inner.get_catalog(&name)))
            .map(CatalogInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_catalog`, but returns an `asyncio.Future`.
    fn get_catalog_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
    ) -> PyResult<PyFuture<'py, CatalogInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_catalog(&name)
                .await
                .map(CatalogInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the catalogs.
    fn list_catalogs(&self, py: Python<'_>) -> PyResult<Vec<CatalogInfo>> {
        let response = py
            .detach(|| rt().block_on(self.inner.list_catalogs(&Page::default())))
            .map_err(to_py_err)?;
        Ok(response
            .catalogs
            .into_iter()
            .map(CatalogInfo::from)
            .collect())
    }

    /// Like `list_catalogs`, but returns an `asyncio.Future`.
    fn list_catalogs_async<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyFuture<'py, Vec<CatalogInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_catalogs(&Page::default())
                .await
                .map_err(to_py_err)?;
            Ok(response
                .catalogs
                .into_iter()
                .map(CatalogInfo::from)
                .collect())
        })
    }

    /// Update a catalog. Only the arguments that are set change.
    ///
    /// Args:
    ///     name: The name.
    ///     comment: A new description.
    ///     properties: Key-value properties, to replace the current ones.
    ///     new_name: A new name.
    #[pyo3(signature = (name, *, comment=None, properties=None, new_name=None))]
    fn update_catalog(
        &self,
        py: Python<'_>,
        name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        new_name: Option<String>,
    ) -> PyResult<CatalogInfo> {
        let request = uc_api::UpdateCatalog {
            comment,
            properties,
            new_name,
        };
        py.detach(|| rt().block_on(self.inner.update_catalog(&name, &request)))
            .map(CatalogInfo::from)
            .map_err(to_py_err)
    }

    /// Like `update_catalog`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, comment=None, properties=None, new_name=None))]
    fn update_catalog_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        new_name: Option<String>,
    ) -> PyResult<PyFuture<'py, CatalogInfo>> {
        let request = uc_api::UpdateCatalog {
            comment,
            properties,
            new_name,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_catalog(&name, &request)
                .await
                .map(CatalogInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Delete a catalog.
    ///
    /// Args:
    ///     name: The name.
    ///     force: Also delete its schemas and their contents. Without it, a
    ///         catalog that has schemas is not deleted.
    #[pyo3(signature = (name, *, force=false))]
    fn delete_catalog(&self, py: Python<'_>, name: String, force: bool) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_catalog(&name, force)))
            .map_err(to_py_err)
    }

    /// Like `delete_catalog`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, force=false))]
    fn delete_catalog_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        force: bool,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_catalog(&name, force)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Create a schema.
    ///
    /// Args:
    ///     name: The name, in the catalog.
    ///     catalog_name: The name of the catalog.
    ///     comment: A description.
    ///     properties: Key-value properties.
    ///     storage_root: The root URL of the managed storage of the schema.
    #[pyo3(signature = (name, catalog_name, *, comment=None, properties=None, storage_root=None))]
    fn create_schema(
        &self,
        py: Python<'_>,
        name: String,
        catalog_name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        storage_root: Option<String>,
    ) -> PyResult<SchemaInfo> {
        let request = uc_api::CreateSchema {
            name,
            catalog_name,
            comment,
            properties,
            storage_root,
        };
        py.detach(|| rt().block_on(self.inner.create_schema(&request)))
            .map(SchemaInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_schema`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, catalog_name, *, comment=None, properties=None, storage_root=None))]
    fn create_schema_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        storage_root: Option<String>,
    ) -> PyResult<PyFuture<'py, SchemaInfo>> {
        let request = uc_api::CreateSchema {
            name,
            catalog_name,
            comment,
            properties,
            storage_root,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_schema(&request)
                .await
                .map(SchemaInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get a schema.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema`.
    fn get_schema(&self, py: Python<'_>, full_name: String) -> PyResult<SchemaInfo> {
        py.detach(|| rt().block_on(self.inner.get_schema(&full_name)))
            .map(SchemaInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_schema`, but returns an `asyncio.Future`.
    fn get_schema_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, SchemaInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_schema(&full_name)
                .await
                .map(SchemaInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the schemas of a catalog.
    ///
    /// Args:
    ///     catalog_name: The name of the catalog.
    fn list_schemas(&self, py: Python<'_>, catalog_name: String) -> PyResult<Vec<SchemaInfo>> {
        let response = py
            .detach(|| rt().block_on(self.inner.list_schemas(&catalog_name, &Page::default())))
            .map_err(to_py_err)?;
        Ok(response.schemas.into_iter().map(SchemaInfo::from).collect())
    }

    /// Like `list_schemas`, but returns an `asyncio.Future`.
    fn list_schemas_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
    ) -> PyResult<PyFuture<'py, Vec<SchemaInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_schemas(&catalog_name, &Page::default())
                .await
                .map_err(to_py_err)?;
            Ok(response.schemas.into_iter().map(SchemaInfo::from).collect())
        })
    }

    /// Update a schema. Only the arguments that are set change.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema`.
    ///     comment: A new description.
    ///     properties: Key-value properties, to replace the current ones.
    ///     new_name: A new name, in the catalog.
    #[pyo3(signature = (full_name, *, comment=None, properties=None, new_name=None))]
    fn update_schema(
        &self,
        py: Python<'_>,
        full_name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        new_name: Option<String>,
    ) -> PyResult<SchemaInfo> {
        let request = uc_api::UpdateSchema {
            comment,
            properties,
            new_name,
        };
        py.detach(|| rt().block_on(self.inner.update_schema(&full_name, &request)))
            .map(SchemaInfo::from)
            .map_err(to_py_err)
    }

    /// Like `update_schema`, but returns an `asyncio.Future`.
    #[pyo3(signature = (full_name, *, comment=None, properties=None, new_name=None))]
    fn update_schema_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        comment: Option<String>,
        properties: Option<Properties>,
        new_name: Option<String>,
    ) -> PyResult<PyFuture<'py, SchemaInfo>> {
        let request = uc_api::UpdateSchema {
            comment,
            properties,
            new_name,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_schema(&full_name, &request)
                .await
                .map(SchemaInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Delete a schema.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema`.
    ///     force: Also delete its tables, volumes, functions and models.
    ///         Without it, a schema that has them is not deleted.
    #[pyo3(signature = (full_name, *, force=false))]
    fn delete_schema(&self, py: Python<'_>, full_name: String, force: bool) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_schema(&full_name, force)))
            .map_err(to_py_err)
    }

    /// Like `delete_schema`, but returns an `asyncio.Future`.
    #[pyo3(signature = (full_name, *, force=false))]
    fn delete_schema_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        force: bool,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_schema(&full_name, force)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Create a table or a view.
    ///
    /// A managed table needs a staging table first (`create_staging_table`):
    /// give its `staging_location` as `storage_location`, and its `id` as
    /// the property `io.unitycatalog.tableId`.
    ///
    /// Args:
    ///     name: The name, in the schema.
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    ///     table_type: The kind of table.
    ///     columns: The columns.
    ///     data_source_format: The format of the data files, for a table.
    ///     storage_location: The URL of the data, for a table.
    ///     comment: A description.
    ///     properties: Key-value properties.
    ///     view_definition: The query of a view.
    ///     view_dependencies: The tables and functions that a view reads.
    #[pyo3(signature = (
        name, catalog_name, schema_name, table_type, *, columns=None,
        data_source_format=None, storage_location=None, comment=None, properties=None,
        view_definition=None, view_dependencies=None
    ))]
    fn create_table(
        &self,
        py: Python<'_>,
        name: String,
        catalog_name: String,
        schema_name: String,
        table_type: StrEnum<TableType>,
        columns: Option<Vec<ColumnInfo>>,
        data_source_format: Option<StrEnum<DataSourceFormat>>,
        storage_location: Option<String>,
        comment: Option<String>,
        properties: Option<Properties>,
        view_definition: Option<String>,
        view_dependencies: Option<Vec<Dependency>>,
    ) -> PyResult<TableInfo> {
        let request = uc_api::CreateTable {
            name,
            catalog_name,
            schema_name,
            table_type: table_type.0,
            columns: columns
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            data_source_format: data_source_format.map(|f| f.0),
            storage_location,
            comment,
            properties,
            view_definition,
            view_dependencies: view_dependencies.map(dependencies_to_api),
        };
        py.detach(|| rt().block_on(self.inner.create_table(&request)))
            .map(TableInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_table`, but returns an `asyncio.Future`.
    #[pyo3(signature = (
        name, catalog_name, schema_name, table_type, *, columns=None,
        data_source_format=None, storage_location=None, comment=None, properties=None,
        view_definition=None, view_dependencies=None
    ))]
    fn create_table_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
        table_type: StrEnum<TableType>,
        columns: Option<Vec<ColumnInfo>>,
        data_source_format: Option<StrEnum<DataSourceFormat>>,
        storage_location: Option<String>,
        comment: Option<String>,
        properties: Option<Properties>,
        view_definition: Option<String>,
        view_dependencies: Option<Vec<Dependency>>,
    ) -> PyResult<PyFuture<'py, TableInfo>> {
        let request = uc_api::CreateTable {
            name,
            catalog_name,
            schema_name,
            table_type: table_type.0,
            columns: columns
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            data_source_format: data_source_format.map(|f| f.0),
            storage_location,
            comment,
            properties,
            view_definition,
            view_dependencies: view_dependencies.map(dependencies_to_api),
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_table(&request)
                .await
                .map(TableInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get a table.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.table`.
    fn get_table(&self, py: Python<'_>, full_name: String) -> PyResult<TableInfo> {
        py.detach(|| rt().block_on(self.inner.get_table(&full_name, false, false)))
            .map(TableInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_table`, but returns an `asyncio.Future`.
    fn get_table_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, TableInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_table(&full_name, false, false)
                .await
                .map(TableInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the tables of a schema.
    ///
    /// Args:
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    fn list_tables(
        &self,
        py: Python<'_>,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<Vec<TableInfo>> {
        let response = py
            .detach(|| {
                rt().block_on(
                    self.inner
                        .list_tables(&catalog_name, &schema_name, &Page::default()),
                )
            })
            .map_err(to_py_err)?;
        Ok(response.tables.into_iter().map(TableInfo::from).collect())
    }

    /// Like `list_tables`, but returns an `asyncio.Future`.
    fn list_tables_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<PyFuture<'py, Vec<TableInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_tables(&catalog_name, &schema_name, &Page::default())
                .await
                .map_err(to_py_err)?;
            Ok(response.tables.into_iter().map(TableInfo::from).collect())
        })
    }

    /// Delete a table.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.table`.
    fn delete_table(&self, py: Python<'_>, full_name: String) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_table(&full_name)))
            .map_err(to_py_err)
    }

    /// Like `delete_table`, but returns an `asyncio.Future`.
    fn delete_table_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_table(&full_name)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Reserve the id and the location of a new managed table. Then create
    /// the table with `create_table`.
    ///
    /// Args:
    ///     name: The name of the table, in the schema.
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    fn create_staging_table(
        &self,
        py: Python<'_>,
        name: String,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<StagingTableInfo> {
        let request = uc_api::CreateStagingTable {
            name,
            catalog_name,
            schema_name,
        };
        py.detach(|| rt().block_on(self.inner.create_staging_table(&request)))
            .map(StagingTableInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_staging_table`, but returns an `asyncio.Future`.
    fn create_staging_table_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<PyFuture<'py, StagingTableInfo>> {
        let request = uc_api::CreateStagingTable {
            name,
            catalog_name,
            schema_name,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_staging_table(&request)
                .await
                .map(StagingTableInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Create a volume.
    ///
    /// Args:
    ///     name: The name, in the schema.
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    ///     volume_type: The kind of volume.
    ///     comment: A description.
    ///     storage_location: The URL of the files, for an `EXTERNAL` volume.
    #[pyo3(signature = (
        name, catalog_name, schema_name, volume_type, *, comment=None, storage_location=None
    ))]
    fn create_volume(
        &self,
        py: Python<'_>,
        name: String,
        catalog_name: String,
        schema_name: String,
        volume_type: StrEnum<VolumeType>,
        comment: Option<String>,
        storage_location: Option<String>,
    ) -> PyResult<VolumeInfo> {
        let request = uc_api::CreateVolumeRequestContent {
            catalog_name,
            schema_name,
            name,
            volume_type: volume_type.0,
            comment,
            storage_location,
        };
        py.detach(|| rt().block_on(self.inner.create_volume(&request)))
            .map(VolumeInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_volume`, but returns an `asyncio.Future`.
    #[pyo3(signature = (
        name, catalog_name, schema_name, volume_type, *, comment=None, storage_location=None
    ))]
    fn create_volume_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
        volume_type: StrEnum<VolumeType>,
        comment: Option<String>,
        storage_location: Option<String>,
    ) -> PyResult<PyFuture<'py, VolumeInfo>> {
        let request = uc_api::CreateVolumeRequestContent {
            catalog_name,
            schema_name,
            name,
            volume_type: volume_type.0,
            comment,
            storage_location,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_volume(&request)
                .await
                .map(VolumeInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get a volume.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.volume`.
    fn get_volume(&self, py: Python<'_>, full_name: String) -> PyResult<VolumeInfo> {
        py.detach(|| rt().block_on(self.inner.get_volume(&full_name)))
            .map(VolumeInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_volume`, but returns an `asyncio.Future`.
    fn get_volume_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, VolumeInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_volume(&full_name)
                .await
                .map(VolumeInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the volumes of a schema.
    ///
    /// Args:
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    fn list_volumes(
        &self,
        py: Python<'_>,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<Vec<VolumeInfo>> {
        let response = py
            .detach(|| {
                rt().block_on(self.inner.list_volumes(
                    &catalog_name,
                    &schema_name,
                    &Page::default(),
                ))
            })
            .map_err(to_py_err)?;
        Ok(response.volumes.into_iter().map(VolumeInfo::from).collect())
    }

    /// Like `list_volumes`, but returns an `asyncio.Future`.
    fn list_volumes_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<PyFuture<'py, Vec<VolumeInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_volumes(&catalog_name, &schema_name, &Page::default())
                .await
                .map_err(to_py_err)?;
            Ok(response.volumes.into_iter().map(VolumeInfo::from).collect())
        })
    }

    /// Update a volume. Only the arguments that are set change.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.volume`.
    ///     comment: A new description.
    ///     new_name: A new name, in the schema.
    #[pyo3(signature = (full_name, *, comment=None, new_name=None))]
    fn update_volume(
        &self,
        py: Python<'_>,
        full_name: String,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> PyResult<VolumeInfo> {
        let request = uc_api::UpdateVolumeRequestContent { comment, new_name };
        py.detach(|| rt().block_on(self.inner.update_volume(&full_name, &request)))
            .map(VolumeInfo::from)
            .map_err(to_py_err)
    }

    /// Like `update_volume`, but returns an `asyncio.Future`.
    #[pyo3(signature = (full_name, *, comment=None, new_name=None))]
    fn update_volume_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> PyResult<PyFuture<'py, VolumeInfo>> {
        let request = uc_api::UpdateVolumeRequestContent { comment, new_name };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_volume(&full_name, &request)
                .await
                .map(VolumeInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Delete a volume.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.volume`.
    fn delete_volume(&self, py: Python<'_>, full_name: String) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_volume(&full_name)))
            .map_err(to_py_err)
    }

    /// Like `delete_volume`, but returns an `asyncio.Future`.
    fn delete_volume_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_volume(&full_name)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Create a function.
    ///
    /// Args:
    ///     name: The name, in the schema.
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    ///     routine_body: The language of the body: `"SQL"`, or `"EXTERNAL"`
    ///         with `external_language`.
    ///     input_params: The parameters.
    ///     data_type: The type of the result.
    ///     full_data_type: The type of the result as SQL text.
    ///     return_params: The columns of the returned table, for a table
    ///         function.
    ///     routine_definition: The body.
    ///     routine_dependencies: The tables and functions that the body uses.
    ///     parameter_style: The parameter style; unset is `"S"`, the only one.
    ///     is_deterministic: Whether the function gives the same result for
    ///         the same arguments.
    ///     sql_data_access: How the function accesses SQL data.
    ///     is_null_call: Whether the function returns null when an argument
    ///         is null.
    ///     security_type: Whose rights the function runs with; unset is
    ///         `"DEFINER"`, the only one.
    ///     specific_name: The specific name of the function.
    ///     comment: A description.
    ///     properties: Key-value properties, as a JSON object in a string.
    ///     external_language: The language of an `EXTERNAL` body, such as
    ///         `"python"`.
    #[pyo3(signature = (
        name, catalog_name, schema_name, routine_body, *, input_params=None, data_type=None,
        full_data_type=None, return_params=None, routine_definition=None,
        routine_dependencies=None, parameter_style=None,
        is_deterministic=None, sql_data_access=None, is_null_call=None,
        security_type=None, specific_name=None, comment=None,
        properties=None, external_language=None
    ))]
    fn create_function(
        &self,
        py: Python<'_>,
        name: String,
        catalog_name: String,
        schema_name: String,
        routine_body: StrEnum<RoutineBody>,
        input_params: Option<Vec<FunctionParameterInfo>>,
        data_type: Option<StrEnum<ColumnTypeName>>,
        full_data_type: Option<String>,
        return_params: Option<Vec<FunctionParameterInfo>>,
        routine_definition: Option<String>,
        routine_dependencies: Option<Vec<Dependency>>,
        parameter_style: Option<StrEnum<ParameterStyle>>,
        is_deterministic: Option<bool>,
        sql_data_access: Option<StrEnum<SqlDataAccess>>,
        is_null_call: Option<bool>,
        security_type: Option<StrEnum<SecurityType>>,
        specific_name: Option<String>,
        comment: Option<String>,
        properties: Option<String>,
        external_language: Option<String>,
    ) -> PyResult<FunctionInfo> {
        let request = uc_api::CreateFunctionRequest {
            function_info: uc_api::CreateFunction {
                name: Some(name),
                catalog_name: Some(catalog_name),
                schema_name: Some(schema_name),
                input_params: input_params.map(parameters_to_api),
                data_type: data_type.map(|t| t.0),
                full_data_type,
                return_params: return_params.map(parameters_to_api),
                routine_body: routine_body.0,
                routine_definition,
                routine_dependencies: routine_dependencies.map(dependencies_to_api),
                parameter_style: parameter_style.map_or(ParameterStyle::S, |s| s.0),
                is_deterministic,
                sql_data_access: sql_data_access.map(|a| a.0),
                is_null_call,
                security_type: security_type.map_or(SecurityType::Definer, |s| s.0),
                specific_name,
                comment,
                properties,
                external_language,
            },
        };
        py.detach(|| rt().block_on(self.inner.create_function(&request)))
            .map(FunctionInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_function`, but returns an `asyncio.Future`.
    #[pyo3(signature = (
        name, catalog_name, schema_name, routine_body, *, input_params=None, data_type=None,
        full_data_type=None, return_params=None, routine_definition=None,
        routine_dependencies=None, parameter_style=None,
        is_deterministic=None, sql_data_access=None, is_null_call=None,
        security_type=None, specific_name=None, comment=None,
        properties=None, external_language=None
    ))]
    fn create_function_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
        routine_body: StrEnum<RoutineBody>,
        input_params: Option<Vec<FunctionParameterInfo>>,
        data_type: Option<StrEnum<ColumnTypeName>>,
        full_data_type: Option<String>,
        return_params: Option<Vec<FunctionParameterInfo>>,
        routine_definition: Option<String>,
        routine_dependencies: Option<Vec<Dependency>>,
        parameter_style: Option<StrEnum<ParameterStyle>>,
        is_deterministic: Option<bool>,
        sql_data_access: Option<StrEnum<SqlDataAccess>>,
        is_null_call: Option<bool>,
        security_type: Option<StrEnum<SecurityType>>,
        specific_name: Option<String>,
        comment: Option<String>,
        properties: Option<String>,
        external_language: Option<String>,
    ) -> PyResult<PyFuture<'py, FunctionInfo>> {
        let request = uc_api::CreateFunctionRequest {
            function_info: uc_api::CreateFunction {
                name: Some(name),
                catalog_name: Some(catalog_name),
                schema_name: Some(schema_name),
                input_params: input_params.map(parameters_to_api),
                data_type: data_type.map(|t| t.0),
                full_data_type,
                return_params: return_params.map(parameters_to_api),
                routine_body: routine_body.0,
                routine_definition,
                routine_dependencies: routine_dependencies.map(dependencies_to_api),
                parameter_style: parameter_style.map_or(ParameterStyle::S, |s| s.0),
                is_deterministic,
                sql_data_access: sql_data_access.map(|a| a.0),
                is_null_call,
                security_type: security_type.map_or(SecurityType::Definer, |s| s.0),
                specific_name,
                comment,
                properties,
                external_language,
            },
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_function(&request)
                .await
                .map(FunctionInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get a function.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.function`.
    fn get_function(&self, py: Python<'_>, full_name: String) -> PyResult<FunctionInfo> {
        py.detach(|| rt().block_on(self.inner.get_function(&full_name)))
            .map(FunctionInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_function`, but returns an `asyncio.Future`.
    fn get_function_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, FunctionInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_function(&full_name)
                .await
                .map(FunctionInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the functions of a schema.
    ///
    /// Args:
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    fn list_functions(
        &self,
        py: Python<'_>,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<Vec<FunctionInfo>> {
        let response = py
            .detach(|| {
                rt().block_on(self.inner.list_functions(
                    &catalog_name,
                    &schema_name,
                    &Page::default(),
                ))
            })
            .map_err(to_py_err)?;
        Ok(response
            .functions
            .into_iter()
            .map(FunctionInfo::from)
            .collect())
    }

    /// Like `list_functions`, but returns an `asyncio.Future`.
    fn list_functions_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
    ) -> PyResult<PyFuture<'py, Vec<FunctionInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_functions(&catalog_name, &schema_name, &Page::default())
                .await
                .map_err(to_py_err)?;
            Ok(response
                .functions
                .into_iter()
                .map(FunctionInfo::from)
                .collect())
        })
    }

    /// Delete a function.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.function`.
    fn delete_function(&self, py: Python<'_>, full_name: String) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_function(&full_name)))
            .map_err(to_py_err)
    }

    /// Like `delete_function`, but returns an `asyncio.Future`.
    fn delete_function_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_function(&full_name)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Create a registered model.
    ///
    /// Args:
    ///     name: The name, in the schema.
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    ///     comment: A description.
    #[pyo3(signature = (name, catalog_name, schema_name, *, comment=None))]
    fn create_registered_model(
        &self,
        py: Python<'_>,
        name: String,
        catalog_name: String,
        schema_name: String,
        comment: Option<String>,
    ) -> PyResult<RegisteredModelInfo> {
        let request = uc_api::CreateRegisteredModel {
            name,
            catalog_name,
            schema_name,
            comment,
        };
        py.detach(|| rt().block_on(self.inner.create_registered_model(&request)))
            .map(RegisteredModelInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_registered_model`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, catalog_name, schema_name, *, comment=None))]
    fn create_registered_model_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
        comment: Option<String>,
    ) -> PyResult<PyFuture<'py, RegisteredModelInfo>> {
        let request = uc_api::CreateRegisteredModel {
            name,
            catalog_name,
            schema_name,
            comment,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_registered_model(&request)
                .await
                .map(RegisteredModelInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get a registered model.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.model`.
    fn get_registered_model(
        &self,
        py: Python<'_>,
        full_name: String,
    ) -> PyResult<RegisteredModelInfo> {
        py.detach(|| rt().block_on(self.inner.get_registered_model(&full_name)))
            .map(RegisteredModelInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_registered_model`, but returns an `asyncio.Future`.
    fn get_registered_model_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, RegisteredModelInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_registered_model(&full_name)
                .await
                .map(RegisteredModelInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the registered models: all of them, or those of a catalog or
    /// a schema.
    ///
    /// Args:
    ///     catalog_name: Only the models of this catalog.
    ///     schema_name: Only the models of this schema, with `catalog_name`.
    #[pyo3(signature = (*, catalog_name=None, schema_name=None))]
    fn list_registered_models(
        &self,
        py: Python<'_>,
        catalog_name: Option<String>,
        schema_name: Option<String>,
    ) -> PyResult<Vec<RegisteredModelInfo>> {
        let response = py
            .detach(|| {
                let page = Page::default();
                let list = self.inner.list_registered_models(
                    catalog_name.as_deref(),
                    schema_name.as_deref(),
                    &page,
                );
                rt().block_on(list)
            })
            .map_err(to_py_err)?;
        let models = response.registered_models.into_iter();
        Ok(models.map(RegisteredModelInfo::from).collect())
    }

    /// Like `list_registered_models`, but returns an `asyncio.Future`.
    #[pyo3(signature = (*, catalog_name=None, schema_name=None))]
    fn list_registered_models_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: Option<String>,
        schema_name: Option<String>,
    ) -> PyResult<PyFuture<'py, Vec<RegisteredModelInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_registered_models(
                    catalog_name.as_deref(),
                    schema_name.as_deref(),
                    &Page::default(),
                )
                .await
                .map_err(to_py_err)?;
            let models = response.registered_models.into_iter();
            Ok(models.map(RegisteredModelInfo::from).collect())
        })
    }

    /// Update a registered model. Only the arguments that are set change.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.model`.
    ///     comment: A new description.
    ///     new_name: A new name, in the schema.
    #[pyo3(signature = (full_name, *, comment=None, new_name=None))]
    fn update_registered_model(
        &self,
        py: Python<'_>,
        full_name: String,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> PyResult<RegisteredModelInfo> {
        let request = uc_api::UpdateRegisteredModel { comment, new_name };
        py.detach(|| rt().block_on(self.inner.update_registered_model(&full_name, &request)))
            .map(RegisteredModelInfo::from)
            .map_err(to_py_err)
    }

    /// Like `update_registered_model`, but returns an `asyncio.Future`.
    #[pyo3(signature = (full_name, *, comment=None, new_name=None))]
    fn update_registered_model_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        comment: Option<String>,
        new_name: Option<String>,
    ) -> PyResult<PyFuture<'py, RegisteredModelInfo>> {
        let request = uc_api::UpdateRegisteredModel { comment, new_name };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_registered_model(&full_name, &request)
                .await
                .map(RegisteredModelInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Delete a registered model.
    ///
    /// Args:
    ///     full_name: The full name: `catalog.schema.model`.
    ///     force: Also delete its versions. Without it, a model that has
    ///         versions is not deleted.
    #[pyo3(signature = (full_name, *, force=false))]
    fn delete_registered_model(
        &self,
        py: Python<'_>,
        full_name: String,
        force: bool,
    ) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_registered_model(&full_name, force)))
            .map_err(to_py_err)
    }

    /// Like `delete_registered_model`, but returns an `asyncio.Future`.
    #[pyo3(signature = (full_name, *, force=false))]
    fn delete_registered_model_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        force: bool,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_registered_model(&full_name, force)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Create a version of a registered model.
    ///
    /// Args:
    ///     model_name: The name of the registered model.
    ///     catalog_name: The name of the catalog.
    ///     schema_name: The name of the schema.
    ///     source: The URL that the model files come from.
    ///     run_id: The id of the run that made the model.
    ///     comment: A description.
    #[pyo3(signature = (model_name, catalog_name, schema_name, source, *, run_id=None, comment=None))]
    fn create_model_version(
        &self,
        py: Python<'_>,
        model_name: String,
        catalog_name: String,
        schema_name: String,
        source: String,
        run_id: Option<String>,
        comment: Option<String>,
    ) -> PyResult<ModelVersionInfo> {
        let request = uc_api::CreateModelVersion {
            model_name,
            catalog_name,
            schema_name,
            source,
            run_id,
            comment,
        };
        py.detach(|| rt().block_on(self.inner.create_model_version(&request)))
            .map(ModelVersionInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_model_version`, but returns an `asyncio.Future`.
    #[pyo3(signature = (model_name, catalog_name, schema_name, source, *, run_id=None, comment=None))]
    fn create_model_version_async<'py>(
        &self,
        py: Python<'py>,
        model_name: String,
        catalog_name: String,
        schema_name: String,
        source: String,
        run_id: Option<String>,
        comment: Option<String>,
    ) -> PyResult<PyFuture<'py, ModelVersionInfo>> {
        let request = uc_api::CreateModelVersion {
            model_name,
            catalog_name,
            schema_name,
            source,
            run_id,
            comment,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_model_version(&request)
                .await
                .map(ModelVersionInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the versions of a registered model.
    ///
    /// Args:
    ///     full_name: The full name of the model: `catalog.schema.model`.
    fn list_model_versions(
        &self,
        py: Python<'_>,
        full_name: String,
    ) -> PyResult<Vec<ModelVersionInfo>> {
        let response = py
            .detach(|| rt().block_on(self.inner.list_model_versions(&full_name, &Page::default())))
            .map_err(to_py_err)?;
        let versions = response.model_versions.into_iter();
        Ok(versions.map(ModelVersionInfo::from).collect())
    }

    /// Like `list_model_versions`, but returns an `asyncio.Future`.
    fn list_model_versions_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
    ) -> PyResult<PyFuture<'py, Vec<ModelVersionInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_model_versions(&full_name, &Page::default())
                .await
                .map_err(to_py_err)?;
            let versions = response.model_versions.into_iter();
            Ok(versions.map(ModelVersionInfo::from).collect())
        })
    }

    /// Get a version of a registered model.
    ///
    /// Args:
    ///     full_name: The full name of the model: `catalog.schema.model`.
    ///     version: The version number.
    fn get_model_version(
        &self,
        py: Python<'_>,
        full_name: String,
        version: i64,
    ) -> PyResult<ModelVersionInfo> {
        py.detach(|| rt().block_on(self.inner.get_model_version(&full_name, version)))
            .map(ModelVersionInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_model_version`, but returns an `asyncio.Future`.
    fn get_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
    ) -> PyResult<PyFuture<'py, ModelVersionInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_model_version(&full_name, version)
                .await
                .map(ModelVersionInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Update a version of a registered model. Only the arguments that are
    /// set change.
    ///
    /// Args:
    ///     full_name: The full name of the model: `catalog.schema.model`.
    ///     version: The version number.
    ///     comment: A new description.
    #[pyo3(signature = (full_name, version, *, comment=None))]
    fn update_model_version(
        &self,
        py: Python<'_>,
        full_name: String,
        version: i64,
        comment: Option<String>,
    ) -> PyResult<ModelVersionInfo> {
        let request = uc_api::UpdateModelVersion { comment };
        py.detach(|| {
            let update = self
                .inner
                .update_model_version(&full_name, version, &request);
            rt().block_on(update)
        })
        .map(ModelVersionInfo::from)
        .map_err(to_py_err)
    }

    /// Like `update_model_version`, but returns an `asyncio.Future`.
    #[pyo3(signature = (full_name, version, *, comment=None))]
    fn update_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
        comment: Option<String>,
    ) -> PyResult<PyFuture<'py, ModelVersionInfo>> {
        let request = uc_api::UpdateModelVersion { comment };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_model_version(&full_name, version, &request)
                .await
                .map(ModelVersionInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Delete a version of a registered model.
    ///
    /// Args:
    ///     full_name: The full name of the model: `catalog.schema.model`.
    ///     version: The version number.
    fn delete_model_version(
        &self,
        py: Python<'_>,
        full_name: String,
        version: i64,
    ) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_model_version(&full_name, version)))
            .map_err(to_py_err)
    }

    /// Like `delete_model_version`, but returns an `asyncio.Future`.
    fn delete_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_model_version(&full_name, version)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Mark a version of a registered model `READY`, when its files are
    /// written.
    ///
    /// Args:
    ///     full_name: The full name of the model: `catalog.schema.model`.
    ///     version: The version number.
    fn finalize_model_version(
        &self,
        py: Python<'_>,
        full_name: String,
        version: i64,
    ) -> PyResult<ModelVersionInfo> {
        py.detach(|| rt().block_on(self.inner.finalize_model_version(&full_name, version)))
            .map(ModelVersionInfo::from)
            .map_err(to_py_err)
    }

    /// Like `finalize_model_version`, but returns an `asyncio.Future`.
    fn finalize_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
    ) -> PyResult<PyFuture<'py, ModelVersionInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .finalize_model_version(&full_name, version)
                .await
                .map(ModelVersionInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Create a credential.
    ///
    /// Args:
    ///     name: The name.
    ///     purpose: What the credential is for.
    ///     aws_iam_role_arn: The ARN of an AWS IAM role, for the server to
    ///         assume.
    ///     comment: A description.
    #[pyo3(signature = (name, *, purpose=None, aws_iam_role_arn=None, comment=None))]
    fn create_credential(
        &self,
        py: Python<'_>,
        name: String,
        purpose: Option<StrEnum<CredentialPurpose>>,
        aws_iam_role_arn: Option<String>,
        comment: Option<String>,
    ) -> PyResult<CredentialInfo> {
        let request = uc_api::CreateCredentialRequest {
            name,
            comment,
            aws_iam_role: aws_iam_role_arn.map(|role_arn| uc_api::AwsIamRoleRequest { role_arn }),
            purpose: purpose.map(|p| p.0),
        };
        py.detach(|| rt().block_on(self.inner.create_credential(&request)))
            .map(CredentialInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_credential`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, purpose=None, aws_iam_role_arn=None, comment=None))]
    fn create_credential_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        purpose: Option<StrEnum<CredentialPurpose>>,
        aws_iam_role_arn: Option<String>,
        comment: Option<String>,
    ) -> PyResult<PyFuture<'py, CredentialInfo>> {
        let request = uc_api::CreateCredentialRequest {
            name,
            comment,
            aws_iam_role: aws_iam_role_arn.map(|role_arn| uc_api::AwsIamRoleRequest { role_arn }),
            purpose: purpose.map(|p| p.0),
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_credential(&request)
                .await
                .map(CredentialInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get a credential.
    ///
    /// Args:
    ///     name: The name.
    fn get_credential(&self, py: Python<'_>, name: String) -> PyResult<CredentialInfo> {
        py.detach(|| rt().block_on(self.inner.get_credential(&name)))
            .map(CredentialInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_credential`, but returns an `asyncio.Future`.
    fn get_credential_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
    ) -> PyResult<PyFuture<'py, CredentialInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_credential(&name)
                .await
                .map(CredentialInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the credentials.
    fn list_credentials(&self, py: Python<'_>) -> PyResult<Vec<CredentialInfo>> {
        let response = py
            .detach(|| rt().block_on(self.inner.list_credentials(&Page::default())))
            .map_err(to_py_err)?;
        let credentials = response.credentials.into_iter();
        Ok(credentials.map(CredentialInfo::from).collect())
    }

    /// Like `list_credentials`, but returns an `asyncio.Future`.
    fn list_credentials_async<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyFuture<'py, Vec<CredentialInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_credentials(&Page::default())
                .await
                .map_err(to_py_err)?;
            let credentials = response.credentials.into_iter();
            Ok(credentials.map(CredentialInfo::from).collect())
        })
    }

    /// Update a credential. Only the arguments that are set change.
    ///
    /// Args:
    ///     name: The name.
    ///     comment: A new description.
    ///     aws_iam_role_arn: The ARN of a new AWS IAM role. The credential
    ///         gets a new external id too.
    ///     new_name: A new name.
    #[pyo3(signature = (name, *, comment=None, aws_iam_role_arn=None, new_name=None))]
    fn update_credential(
        &self,
        py: Python<'_>,
        name: String,
        comment: Option<String>,
        aws_iam_role_arn: Option<String>,
        new_name: Option<String>,
    ) -> PyResult<CredentialInfo> {
        // No `owner`: the server ignores it, as upstream does.
        let request = uc_api::UpdateCredentialRequest {
            comment,
            owner: None,
            aws_iam_role: aws_iam_role_arn.map(|role_arn| uc_api::AwsIamRoleRequest { role_arn }),
            new_name,
        };
        py.detach(|| rt().block_on(self.inner.update_credential(&name, &request)))
            .map(CredentialInfo::from)
            .map_err(to_py_err)
    }

    /// Like `update_credential`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, comment=None, aws_iam_role_arn=None, new_name=None))]
    fn update_credential_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        comment: Option<String>,
        aws_iam_role_arn: Option<String>,
        new_name: Option<String>,
    ) -> PyResult<PyFuture<'py, CredentialInfo>> {
        let request = uc_api::UpdateCredentialRequest {
            comment,
            owner: None,
            aws_iam_role: aws_iam_role_arn.map(|role_arn| uc_api::AwsIamRoleRequest { role_arn }),
            new_name,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_credential(&name, &request)
                .await
                .map(CredentialInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Delete a credential.
    ///
    /// Args:
    ///     name: The name.
    ///     force: Delete it also when external locations use it.
    #[pyo3(signature = (name, *, force=false))]
    fn delete_credential(&self, py: Python<'_>, name: String, force: bool) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_credential(&name, force)))
            .map_err(to_py_err)
    }

    /// Like `delete_credential`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, force=false))]
    fn delete_credential_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        force: bool,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_credential(&name, force)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Get temporary credentials for the storage of a table.
    ///
    /// Args:
    ///     table_id: The id of the table.
    ///     operation: What the credentials are for: `"READ"`, or
    ///         `"READ_WRITE"`.
    fn generate_temporary_table_credentials(
        &self,
        py: Python<'_>,
        table_id: String,
        operation: StrEnum<TableOperation>,
    ) -> PyResult<TemporaryCredentials> {
        let request = uc_api::GenerateTemporaryTableCredential {
            table_id,
            operation: operation.0,
        };
        py.detach(|| rt().block_on(self.inner.generate_temporary_table_credentials(&request)))
            .map(TemporaryCredentials::from)
            .map_err(to_py_err)
    }

    /// Like `generate_temporary_table_credentials`, but returns an
    /// `asyncio.Future`.
    fn generate_temporary_table_credentials_async<'py>(
        &self,
        py: Python<'py>,
        table_id: String,
        operation: StrEnum<TableOperation>,
    ) -> PyResult<PyFuture<'py, TemporaryCredentials>> {
        let request = uc_api::GenerateTemporaryTableCredential {
            table_id,
            operation: operation.0,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .generate_temporary_table_credentials(&request)
                .await
                .map(TemporaryCredentials::from)
                .map_err(to_py_err)
        })
    }

    /// Get temporary credentials for the storage of a volume.
    ///
    /// Args:
    ///     volume_id: The id of the volume.
    ///     operation: What the credentials are for: `"READ_VOLUME"`, or
    ///         `"WRITE_VOLUME"`.
    fn generate_temporary_volume_credentials(
        &self,
        py: Python<'_>,
        volume_id: String,
        operation: StrEnum<VolumeOperation>,
    ) -> PyResult<TemporaryCredentials> {
        let request = uc_api::GenerateTemporaryVolumeCredential {
            volume_id,
            operation: operation.0,
        };
        py.detach(|| rt().block_on(self.inner.generate_temporary_volume_credentials(&request)))
            .map(TemporaryCredentials::from)
            .map_err(to_py_err)
    }

    /// Like `generate_temporary_volume_credentials`, but returns an
    /// `asyncio.Future`.
    fn generate_temporary_volume_credentials_async<'py>(
        &self,
        py: Python<'py>,
        volume_id: String,
        operation: StrEnum<VolumeOperation>,
    ) -> PyResult<PyFuture<'py, TemporaryCredentials>> {
        let request = uc_api::GenerateTemporaryVolumeCredential {
            volume_id,
            operation: operation.0,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .generate_temporary_volume_credentials(&request)
                .await
                .map(TemporaryCredentials::from)
                .map_err(to_py_err)
        })
    }

    /// Get temporary credentials for a storage path.
    ///
    /// Args:
    ///     url: The URL of the path, such as `"s3://bucket/prefix"`.
    ///     operation: What the credentials are for: `"PATH_READ"`,
    ///         `"PATH_READ_WRITE"`, or `"PATH_CREATE_TABLE"`.
    fn generate_temporary_path_credentials(
        &self,
        py: Python<'_>,
        url: String,
        operation: StrEnum<PathOperation>,
    ) -> PyResult<TemporaryCredentials> {
        let request = uc_api::GenerateTemporaryPathCredential {
            url,
            operation: operation.0,
        };
        py.detach(|| rt().block_on(self.inner.generate_temporary_path_credentials(&request)))
            .map(TemporaryCredentials::from)
            .map_err(to_py_err)
    }

    /// Like `generate_temporary_path_credentials`, but returns an
    /// `asyncio.Future`.
    fn generate_temporary_path_credentials_async<'py>(
        &self,
        py: Python<'py>,
        url: String,
        operation: StrEnum<PathOperation>,
    ) -> PyResult<PyFuture<'py, TemporaryCredentials>> {
        let request = uc_api::GenerateTemporaryPathCredential {
            url,
            operation: operation.0,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .generate_temporary_path_credentials(&request)
                .await
                .map(TemporaryCredentials::from)
                .map_err(to_py_err)
        })
    }

    /// Create an external location.
    ///
    /// Args:
    ///     name: The name.
    ///     url: The URL of the path, such as `"s3://bucket/prefix"`.
    ///     credential_name: The name of the credential for the path.
    ///     comment: A description.
    #[pyo3(signature = (name, url, credential_name, *, comment=None))]
    fn create_external_location(
        &self,
        py: Python<'_>,
        name: String,
        url: String,
        credential_name: String,
        comment: Option<String>,
    ) -> PyResult<ExternalLocationInfo> {
        let request = uc_api::CreateExternalLocation {
            name,
            url,
            credential_name,
            comment,
        };
        py.detach(|| rt().block_on(self.inner.create_external_location(&request)))
            .map(ExternalLocationInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_external_location`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, url, credential_name, *, comment=None))]
    fn create_external_location_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        url: String,
        credential_name: String,
        comment: Option<String>,
    ) -> PyResult<PyFuture<'py, ExternalLocationInfo>> {
        let request = uc_api::CreateExternalLocation {
            name,
            url,
            credential_name,
            comment,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_external_location(&request)
                .await
                .map(ExternalLocationInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Get an external location.
    ///
    /// Args:
    ///     name: The name.
    fn get_external_location(
        &self,
        py: Python<'_>,
        name: String,
    ) -> PyResult<ExternalLocationInfo> {
        py.detach(|| rt().block_on(self.inner.get_external_location(&name)))
            .map(ExternalLocationInfo::from)
            .map_err(to_py_err)
    }

    /// Like `get_external_location`, but returns an `asyncio.Future`.
    fn get_external_location_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
    ) -> PyResult<PyFuture<'py, ExternalLocationInfo>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_external_location(&name)
                .await
                .map(ExternalLocationInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the external locations.
    fn list_external_locations(&self, py: Python<'_>) -> PyResult<Vec<ExternalLocationInfo>> {
        let response = py
            .detach(|| rt().block_on(self.inner.list_external_locations(&Page::default())))
            .map_err(to_py_err)?;
        let locations = response.external_locations.into_iter();
        Ok(locations.map(ExternalLocationInfo::from).collect())
    }

    /// Like `list_external_locations`, but returns an `asyncio.Future`.
    fn list_external_locations_async<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyFuture<'py, Vec<ExternalLocationInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_external_locations(&Page::default())
                .await
                .map_err(to_py_err)?;
            let locations = response.external_locations.into_iter();
            Ok(locations.map(ExternalLocationInfo::from).collect())
        })
    }

    /// Update an external location. Only the arguments that are set change.
    ///
    /// Args:
    ///     name: The name.
    ///     url: A new URL.
    ///     new_name: A new name.
    ///     comment: A new description.
    ///     credential_name: The name of a new credential.
    #[pyo3(signature = (name, *, url=None, new_name=None, comment=None, credential_name=None))]
    fn update_external_location(
        &self,
        py: Python<'_>,
        name: String,
        url: Option<String>,
        new_name: Option<String>,
        comment: Option<String>,
        credential_name: Option<String>,
    ) -> PyResult<ExternalLocationInfo> {
        // No `owner`: the server ignores it, as upstream does.
        let request = uc_api::UpdateExternalLocation {
            url,
            new_name,
            comment,
            credential_name,
            owner: None,
        };
        py.detach(|| rt().block_on(self.inner.update_external_location(&name, &request)))
            .map(ExternalLocationInfo::from)
            .map_err(to_py_err)
    }

    /// Like `update_external_location`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, url=None, new_name=None, comment=None, credential_name=None))]
    fn update_external_location_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        url: Option<String>,
        new_name: Option<String>,
        comment: Option<String>,
        credential_name: Option<String>,
    ) -> PyResult<PyFuture<'py, ExternalLocationInfo>> {
        let request = uc_api::UpdateExternalLocation {
            url,
            new_name,
            comment,
            credential_name,
            owner: None,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_external_location(&name, &request)
                .await
                .map(ExternalLocationInfo::from)
                .map_err(to_py_err)
        })
    }

    /// Delete an external location.
    ///
    /// Args:
    ///     name: The name.
    ///     force: Delete it also when tables, volumes or models are in it.
    #[pyo3(signature = (name, *, force=false))]
    fn delete_external_location(&self, py: Python<'_>, name: String, force: bool) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_external_location(&name, force)))
            .map_err(to_py_err)
    }

    /// Like `delete_external_location`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, force=false))]
    fn delete_external_location_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        force: bool,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_external_location(&name, force)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Get the privileges that are granted on a securable.
    ///
    /// Args:
    ///     securable_type: The kind of securable.
    ///     full_name: The full name of the securable.
    ///     principal: Only the privileges of this user or group.
    ///
    /// Returns:
    ///     The privileges of each principal.
    #[pyo3(signature = (securable_type, full_name, *, principal=None))]
    fn get_permissions(
        &self,
        py: Python<'_>,
        securable_type: StrEnum<SecurableType>,
        full_name: String,
        principal: Option<String>,
    ) -> PyResult<Vec<PrivilegeAssignment>> {
        let response = py
            .detach(|| {
                let get =
                    self.inner
                        .get_permissions(securable_type.0, &full_name, principal.as_deref());
                rt().block_on(get)
            })
            .map_err(to_py_err)?;
        let assignments = response.privilege_assignments.into_iter();
        Ok(assignments.map(PrivilegeAssignment::from).collect())
    }

    /// Like `get_permissions`, but returns an `asyncio.Future`.
    #[pyo3(signature = (securable_type, full_name, *, principal=None))]
    fn get_permissions_async<'py>(
        &self,
        py: Python<'py>,
        securable_type: StrEnum<SecurableType>,
        full_name: String,
        principal: Option<String>,
    ) -> PyResult<PyFuture<'py, Vec<PrivilegeAssignment>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .get_permissions(securable_type.0, &full_name, principal.as_deref())
                .await
                .map_err(to_py_err)?;
            let assignments = response.privilege_assignments.into_iter();
            Ok(assignments.map(PrivilegeAssignment::from).collect())
        })
    }

    /// Grant and revoke privileges on a securable.
    ///
    /// Args:
    ///     securable_type: The kind of securable.
    ///     full_name: The full name of the securable.
    ///     changes: The privileges to grant and to revoke, per principal.
    ///
    /// Returns:
    ///     The privileges of each principal, after the changes.
    fn update_permissions(
        &self,
        py: Python<'_>,
        securable_type: StrEnum<SecurableType>,
        full_name: String,
        changes: Vec<PermissionsChange>,
    ) -> PyResult<Vec<PrivilegeAssignment>> {
        let request = uc_api::UpdatePermissions {
            changes: changes.into_iter().map(Into::into).collect(),
        };
        let response = py
            .detach(|| {
                let update = self
                    .inner
                    .update_permissions(securable_type.0, &full_name, &request);
                rt().block_on(update)
            })
            .map_err(to_py_err)?;
        let assignments = response.privilege_assignments.into_iter();
        Ok(assignments.map(PrivilegeAssignment::from).collect())
    }

    /// Like `update_permissions`, but returns an `asyncio.Future`.
    fn update_permissions_async<'py>(
        &self,
        py: Python<'py>,
        securable_type: StrEnum<SecurableType>,
        full_name: String,
        changes: Vec<PermissionsChange>,
    ) -> PyResult<PyFuture<'py, Vec<PrivilegeAssignment>>> {
        let request = uc_api::UpdatePermissions {
            changes: changes.into_iter().map(Into::into).collect(),
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .update_permissions(securable_type.0, &full_name, &request)
                .await
                .map_err(to_py_err)?;
            let assignments = response.privilege_assignments.into_iter();
            Ok(assignments.map(PrivilegeAssignment::from).collect())
        })
    }

    /// Get the summary of the metastore.
    fn metastore_summary(&self, py: Python<'_>) -> PyResult<MetastoreSummary> {
        py.detach(|| rt().block_on(self.inner.metastore_summary()))
            .map(MetastoreSummary::from)
            .map_err(to_py_err)
    }

    /// Like `metastore_summary`, but returns an `asyncio.Future`.
    fn metastore_summary_async<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyFuture<'py, MetastoreSummary>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .metastore_summary()
                .await
                .map(MetastoreSummary::from)
                .map_err(to_py_err)
        })
    }

    /// Create a personal access token for the caller. The result has the
    /// token, `token_value`; no other method returns it.
    ///
    /// Args:
    ///     name: The name.
    ///     expires_at: The expiry time, in epoch milliseconds. No expiry
    ///         when unset.
    #[pyo3(signature = (name, *, expires_at=None))]
    fn create_pat(
        &self,
        py: Python<'_>,
        name: String,
        expires_at: Option<i64>,
    ) -> PyResult<PatInfo> {
        let request = uc_api::CreatePat { name, expires_at };
        py.detach(|| rt().block_on(self.inner.create_pat(&request)))
            .map(PatInfo::from)
            .map_err(to_py_err)
    }

    /// Like `create_pat`, but returns an `asyncio.Future`.
    #[pyo3(signature = (name, *, expires_at=None))]
    fn create_pat_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        expires_at: Option<i64>,
    ) -> PyResult<PyFuture<'py, PatInfo>> {
        let request = uc_api::CreatePat { name, expires_at };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_pat(&request)
                .await
                .map(PatInfo::from)
                .map_err(to_py_err)
        })
    }

    /// List the personal access tokens of the caller.
    fn list_pats(&self, py: Python<'_>) -> PyResult<Vec<PatInfo>> {
        let response = py
            .detach(|| rt().block_on(self.inner.list_pats(&Page::default())))
            .map_err(to_py_err)?;
        Ok(response.tokens.into_iter().map(PatInfo::from).collect())
    }

    /// Like `list_pats`, but returns an `asyncio.Future`.
    fn list_pats_async<'py>(&self, py: Python<'py>) -> PyResult<PyFuture<'py, Vec<PatInfo>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_pats(&Page::default())
                .await
                .map_err(to_py_err)?;
            Ok(response.tokens.into_iter().map(PatInfo::from).collect())
        })
    }

    /// Revoke a personal access token.
    ///
    /// Args:
    ///     id: The id of the token.
    fn revoke_pat(&self, py: Python<'_>, id: String) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.revoke_pat(&id)))
            .map_err(to_py_err)
    }

    /// Like `revoke_pat`, but returns an `asyncio.Future`.
    fn revoke_pat_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .revoke_pat(&id)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Create a user. The email names the user, as principals name it.
    ///
    /// Args:
    ///     email: The user's email: its name and its primary email.
    ///     display_name: The user's name.
    ///     external_id: The identity of the user at an identity provider,
    ///         such as the client id of an OAuth client.
    #[pyo3(signature = (email, *, display_name=None, external_id=None))]
    fn create_user(
        &self,
        py: Python<'_>,
        email: String,
        display_name: Option<String>,
        external_id: Option<String>,
    ) -> PyResult<ScimUser> {
        let user = new_scim_user(email, display_name, external_id);
        py.detach(|| rt().block_on(self.inner.create_user(&user)))
            .map(ScimUser::from)
            .map_err(to_py_err)
    }

    /// Like `create_user`, but returns an `asyncio.Future`.
    #[pyo3(signature = (email, *, display_name=None, external_id=None))]
    fn create_user_async<'py>(
        &self,
        py: Python<'py>,
        email: String,
        display_name: Option<String>,
        external_id: Option<String>,
    ) -> PyResult<PyFuture<'py, ScimUser>> {
        let user = new_scim_user(email, display_name, external_id);
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_user(&user)
                .await
                .map(ScimUser::from)
                .map_err(to_py_err)
        })
    }

    /// Get a user.
    ///
    /// Args:
    ///     id: The id of the user.
    fn get_user(&self, py: Python<'_>, id: String) -> PyResult<ScimUser> {
        py.detach(|| rt().block_on(self.inner.get_user(&id)))
            .map(ScimUser::from)
            .map_err(to_py_err)
    }

    /// Like `get_user`, but returns an `asyncio.Future`.
    fn get_user_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> PyResult<PyFuture<'py, ScimUser>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_user(&id)
                .await
                .map(ScimUser::from)
                .map_err(to_py_err)
        })
    }

    /// List the users.
    ///
    /// Args:
    ///     filter: A SCIM filter, such as `'userName eq "ada@example.com"'`.
    ///     start_index: The position of the first user to return, from 1.
    ///     count: The number of users to return.
    #[pyo3(signature = (*, filter=None, start_index=None, count=None))]
    fn list_users(
        &self,
        py: Python<'_>,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> PyResult<Vec<ScimUser>> {
        let page = uc_client::ScimPage {
            filter,
            start_index,
            count,
        };
        let response = py
            .detach(|| rt().block_on(self.inner.list_users(&page)))
            .map_err(to_py_err)?;
        Ok(response.resources.into_iter().map(ScimUser::from).collect())
    }

    /// Like `list_users`, but returns an `asyncio.Future`.
    #[pyo3(signature = (*, filter=None, start_index=None, count=None))]
    fn list_users_async<'py>(
        &self,
        py: Python<'py>,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> PyResult<PyFuture<'py, Vec<ScimUser>>> {
        let page = uc_client::ScimPage {
            filter,
            start_index,
            count,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client.list_users(&page).await.map_err(to_py_err)?;
            Ok(response.resources.into_iter().map(ScimUser::from).collect())
        })
    }

    /// Update a user. Only the arguments that are set change; the email
    /// does not change.
    ///
    /// Args:
    ///     id: The id of the user.
    ///     display_name: A new name.
    ///     external_id: A new identity at an identity provider.
    ///     active: Enable or disable the user.
    #[pyo3(signature = (id, *, display_name=None, external_id=None, active=None))]
    fn update_user(
        &self,
        py: Python<'_>,
        id: String,
        display_name: Option<String>,
        external_id: Option<String>,
        active: Option<bool>,
    ) -> PyResult<ScimUser> {
        let user = uc_api::ScimUser {
            schemas: vec![uc_api::schema_uri::USER.to_string()],
            id: Some(id.clone()),
            display_name,
            external_id,
            active,
            ..Default::default()
        };
        py.detach(|| rt().block_on(self.inner.update_user(&id, &user)))
            .map(ScimUser::from)
            .map_err(to_py_err)
    }

    /// Like `update_user`, but returns an `asyncio.Future`.
    #[pyo3(signature = (id, *, display_name=None, external_id=None, active=None))]
    fn update_user_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
        display_name: Option<String>,
        external_id: Option<String>,
        active: Option<bool>,
    ) -> PyResult<PyFuture<'py, ScimUser>> {
        let user = uc_api::ScimUser {
            schemas: vec![uc_api::schema_uri::USER.to_string()],
            id: Some(id.clone()),
            display_name,
            external_id,
            active,
            ..Default::default()
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .update_user(&id, &user)
                .await
                .map(ScimUser::from)
                .map_err(to_py_err)
        })
    }

    /// Delete a user. As in upstream, the user stays, disabled, and loses
    /// its privileges.
    ///
    /// Args:
    ///     id: The id of the user.
    fn delete_user(&self, py: Python<'_>, id: String) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_user(&id)))
            .map_err(to_py_err)
    }

    /// Like `delete_user`, but returns an `asyncio.Future`.
    fn delete_user_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_user(&id)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Create a group, without members.
    ///
    /// Args:
    ///     display_name: The name of the group.
    fn create_group(&self, py: Python<'_>, display_name: String) -> PyResult<ScimGroup> {
        let group = uc_api::ScimGroup {
            schemas: vec![uc_api::schema_uri::GROUP.to_string()],
            display_name,
            ..Default::default()
        };
        py.detach(|| rt().block_on(self.inner.create_group(&group)))
            .map(ScimGroup::from)
            .map_err(to_py_err)
    }

    /// Like `create_group`, but returns an `asyncio.Future`.
    fn create_group_async<'py>(
        &self,
        py: Python<'py>,
        display_name: String,
    ) -> PyResult<PyFuture<'py, ScimGroup>> {
        let group = uc_api::ScimGroup {
            schemas: vec![uc_api::schema_uri::GROUP.to_string()],
            display_name,
            ..Default::default()
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .create_group(&group)
                .await
                .map(ScimGroup::from)
                .map_err(to_py_err)
        })
    }

    /// Get a group.
    ///
    /// Args:
    ///     id: The id of the group.
    fn get_group(&self, py: Python<'_>, id: String) -> PyResult<ScimGroup> {
        py.detach(|| rt().block_on(self.inner.get_group(&id)))
            .map(ScimGroup::from)
            .map_err(to_py_err)
    }

    /// Like `get_group`, but returns an `asyncio.Future`.
    fn get_group_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> PyResult<PyFuture<'py, ScimGroup>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .get_group(&id)
                .await
                .map(ScimGroup::from)
                .map_err(to_py_err)
        })
    }

    /// List the groups.
    ///
    /// Args:
    ///     filter: A SCIM filter, such as `'displayName eq "admins"'`.
    ///     start_index: The position of the first group to return, from 1.
    ///     count: The number of groups to return.
    #[pyo3(signature = (*, filter=None, start_index=None, count=None))]
    fn list_groups(
        &self,
        py: Python<'_>,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> PyResult<Vec<ScimGroup>> {
        let page = uc_client::ScimPage {
            filter,
            start_index,
            count,
        };
        let response = py
            .detach(|| rt().block_on(self.inner.list_groups(&page)))
            .map_err(to_py_err)?;
        Ok(response
            .resources
            .into_iter()
            .map(ScimGroup::from)
            .collect())
    }

    /// Like `list_groups`, but returns an `asyncio.Future`.
    #[pyo3(signature = (*, filter=None, start_index=None, count=None))]
    fn list_groups_async<'py>(
        &self,
        py: Python<'py>,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> PyResult<PyFuture<'py, Vec<ScimGroup>>> {
        let page = uc_client::ScimPage {
            filter,
            start_index,
            count,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client.list_groups(&page).await.map_err(to_py_err)?;
            Ok(response
                .resources
                .into_iter()
                .map(ScimGroup::from)
                .collect())
        })
    }

    /// Delete a group.
    ///
    /// Args:
    ///     id: The id of the group.
    fn delete_group(&self, py: Python<'_>, id: String) -> PyResult<()> {
        py.detach(|| rt().block_on(self.inner.delete_group(&id)))
            .map_err(to_py_err)
    }

    /// Like `delete_group`, but returns an `asyncio.Future`.
    fn delete_group_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .delete_group(&id)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Get a snapshot of the whole catalog, for `restore`. It takes an
    /// admin, signed in.
    ///
    /// Returns:
    ///     The snapshot: `{"version": …, "tables": {…}}`, the rows of each
    ///     table of the store. It has credentials and token hashes: keep it
    ///     secret.
    fn backup(&self, py: Python<'_>) -> PyResult<JsonObject> {
        let snapshot = py
            .detach(|| rt().block_on(self.inner.backup()))
            .map_err(to_py_err)?;
        JsonObject::new(&snapshot)
    }

    /// Like `backup`, but returns an `asyncio.Future`.
    fn backup_async<'py>(&self, py: Python<'py>) -> PyResult<PyFuture<'py, JsonObject>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let snapshot = client.backup().await.map_err(to_py_err)?;
            JsonObject::new(&snapshot)
        })
    }

    /// Replace the whole catalog with a snapshot of `backup`. It takes an
    /// admin, signed in.
    ///
    /// Args:
    ///     snapshot: The snapshot, as `backup` returns it.
    fn restore(&self, py: Python<'_>, snapshot: JsonObject) -> PyResult<RestoreResponse> {
        let snapshot: uc_api::BackupResponse = snapshot.parse("snapshot")?;
        py.detach(|| rt().block_on(self.inner.restore(&snapshot)))
            .map(RestoreResponse::from)
            .map_err(to_py_err)
    }

    /// Like `restore`, but returns an `asyncio.Future`.
    fn restore_async<'py>(
        &self,
        py: Python<'py>,
        snapshot: JsonObject,
    ) -> PyResult<PyFuture<'py, RestoreResponse>> {
        let snapshot: uc_api::BackupResponse = snapshot.parse("snapshot")?;
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .restore(&snapshot)
                .await
                .map(RestoreResponse::from)
                .map_err(to_py_err)
        })
    }

    /// List the requests that changed the catalog, from the audit log. It
    /// takes an admin, signed in.
    ///
    /// Args:
    ///     principal: Only the requests of this user.
    ///     method: Only the requests of this HTTP method, such as
    ///         `"DELETE"`.
    ///     path_prefix: Only the requests with a path that starts with it.
    ///     max_results: The number of events to return.
    #[pyo3(signature = (*, principal=None, method=None, path_prefix=None, max_results=None))]
    fn list_audit_events(
        &self,
        py: Python<'_>,
        principal: Option<String>,
        method: Option<String>,
        path_prefix: Option<String>,
        max_results: Option<i32>,
    ) -> PyResult<Vec<AuditEvent>> {
        let filter = uc_client::AuditFilter {
            principal,
            method,
            path_prefix,
            max_results,
        };
        let response = py
            .detach(|| rt().block_on(self.inner.list_audit_events(&filter)))
            .map_err(to_py_err)?;
        Ok(response.events.into_iter().map(AuditEvent::from).collect())
    }

    /// Like `list_audit_events`, but returns an `asyncio.Future`.
    #[pyo3(signature = (*, principal=None, method=None, path_prefix=None, max_results=None))]
    fn list_audit_events_async<'py>(
        &self,
        py: Python<'py>,
        principal: Option<String>,
        method: Option<String>,
        path_prefix: Option<String>,
        max_results: Option<i32>,
    ) -> PyResult<PyFuture<'py, Vec<AuditEvent>>> {
        let filter = uc_client::AuditFilter {
            principal,
            method,
            path_prefix,
            max_results,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client.list_audit_events(&filter).await.map_err(to_py_err)?;
            Ok(response.events.into_iter().map(AuditEvent::from).collect())
        })
    }

    /// Get the configuration of an Iceberg REST client for a catalog.
    ///
    /// Args:
    ///     catalog: The name of the catalog: the Iceberg warehouse.
    fn iceberg_config(&self, py: Python<'_>, catalog: String) -> PyResult<IcebergConfig> {
        py.detach(|| rt().block_on(self.inner.iceberg_config(&catalog)))
            .map(IcebergConfig::from)
            .map_err(to_py_err)
    }

    /// Like `iceberg_config`, but returns an `asyncio.Future`.
    fn iceberg_config_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
    ) -> PyResult<PyFuture<'py, IcebergConfig>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .iceberg_config(&catalog)
                .await
                .map(IcebergConfig::from)
                .map_err(to_py_err)
        })
    }

    /// List the Iceberg namespaces of a catalog: its schemas.
    ///
    /// Args:
    ///     catalog: The name of the catalog.
    ///
    /// Returns:
    ///     The levels of each namespace, such as `[["default"]]`.
    fn list_iceberg_namespaces(
        &self,
        py: Python<'_>,
        catalog: String,
    ) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            let page = Page::default();
            rt().block_on(self.inner.list_iceberg_namespaces(&catalog, &page))
        })
        .map(|response| response.namespaces)
        .map_err(to_py_err)
    }

    /// Like `list_iceberg_namespaces`, but returns an `asyncio.Future`.
    fn list_iceberg_namespaces_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
    ) -> PyResult<PyFuture<'py, Vec<Vec<String>>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .list_iceberg_namespaces(&catalog, &Page::default())
                .await
                .map(|response| response.namespaces)
                .map_err(to_py_err)
        })
    }

    /// List the Iceberg tables of a namespace: the tables of a schema that
    /// have Iceberg metadata.
    ///
    /// Args:
    ///     catalog: The name of the catalog.
    ///     namespace: The namespace: the name of the schema.
    fn list_iceberg_tables(
        &self,
        py: Python<'_>,
        catalog: String,
        namespace: String,
    ) -> PyResult<Vec<IcebergTableIdentifier>> {
        let response = py
            .detach(|| {
                let page = Page::default();
                rt().block_on(self.inner.list_iceberg_tables(&catalog, &namespace, &page))
            })
            .map_err(to_py_err)?;
        let identifiers = response.identifiers.into_iter();
        Ok(identifiers.map(IcebergTableIdentifier::from).collect())
    }

    /// Like `list_iceberg_tables`, but returns an `asyncio.Future`.
    fn list_iceberg_tables_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
    ) -> PyResult<PyFuture<'py, Vec<IcebergTableIdentifier>>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .list_iceberg_tables(&catalog, &namespace, &Page::default())
                .await
                .map_err(to_py_err)?;
            let identifiers = response.identifiers.into_iter();
            Ok(identifiers.map(IcebergTableIdentifier::from).collect())
        })
    }

    /// Load an Iceberg table.
    ///
    /// Args:
    ///     catalog: The name of the catalog.
    ///     namespace: The namespace: the name of the schema.
    ///     table: The name of the table.
    ///
    /// Returns:
    ///     The Iceberg REST response: `metadata-location`, the table
    ///     `metadata` and the client `config`.
    fn load_iceberg_table(
        &self,
        py: Python<'_>,
        catalog: String,
        namespace: String,
        table: String,
    ) -> PyResult<JsonObject> {
        let response = py
            .detach(|| rt().block_on(self.inner.load_iceberg_table(&catalog, &namespace, &table)))
            .map_err(to_py_err)?;
        JsonObject::new(&response)
    }

    /// Like `load_iceberg_table`, but returns an `asyncio.Future`.
    fn load_iceberg_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        table: String,
    ) -> PyResult<PyFuture<'py, JsonObject>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .load_iceberg_table(&catalog, &namespace, &table)
                .await
                .map_err(to_py_err)?;
            JsonObject::new(&response)
        })
    }

    /// Get the commits that the server holds for a catalog-managed Delta
    /// table: those not yet backfilled into `_delta_log`.
    ///
    /// Args:
    ///     table_id: The id of the table.
    ///     table_uri: The storage location of the table.
    ///     start_version: The first version to return.
    ///     end_version: The last version to return; unset for the latest.
    ///
    /// Returns:
    ///     `{"commits": [...], "latest_table_version": …}`: the commits,
    ///     newest first, and the latest version of the table.
    #[pyo3(signature = (table_id, table_uri, start_version, *, end_version=None))]
    fn get_delta_commits(
        &self,
        py: Python<'_>,
        table_id: String,
        table_uri: String,
        start_version: i64,
        end_version: Option<i64>,
    ) -> PyResult<JsonObject> {
        let request = uc_api::DeltaGetCommits {
            table_id: Some(table_id),
            table_uri: Some(table_uri),
            start_version: Some(start_version),
            end_version,
        };
        let response = py
            .detach(|| rt().block_on(self.inner.get_delta_commits(&request)))
            .map_err(to_py_err)?;
        JsonObject::new(&response)
    }

    /// Like `get_delta_commits`, but returns an `asyncio.Future`.
    #[pyo3(signature = (table_id, table_uri, start_version, *, end_version=None))]
    fn get_delta_commits_async<'py>(
        &self,
        py: Python<'py>,
        table_id: String,
        table_uri: String,
        start_version: i64,
        end_version: Option<i64>,
    ) -> PyResult<PyFuture<'py, JsonObject>> {
        let request = uc_api::DeltaGetCommits {
            table_id: Some(table_id),
            table_uri: Some(table_uri),
            start_version: Some(start_version),
            end_version,
        };
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            let response = client
                .get_delta_commits(&request)
                .await
                .map_err(to_py_err)?;
            JsonObject::new(&response)
        })
    }

    /// Ratify a staged commit of a catalog-managed Delta table, or report
    /// a backfill. Another commit of the same version raises `UcError`.
    ///
    /// Args:
    ///     commit: The commit, as the Delta commit coordinator API has it:
    ///         `table_id`, `table_uri`, and `commit_info` (`version`,
    ///         `timestamp`, `file_name`, `file_size`,
    ///         `file_modification_timestamp`) or `latest_backfilled_version`,
    ///         or both; `metadata` and `uniform` when they change.
    fn commit_delta(&self, py: Python<'_>, commit: JsonObject) -> PyResult<()> {
        let commit: uc_api::DeltaCommit = commit.parse("commit")?;
        py.detach(|| rt().block_on(self.inner.commit_delta(&commit)))
            .map_err(to_py_err)
    }

    /// Like `commit_delta`, but returns an `asyncio.Future`.
    fn commit_delta_async<'py>(
        &self,
        py: Python<'py>,
        commit: JsonObject,
    ) -> PyResult<PyFuture<'py, NoResult>> {
        let commit: uc_api::DeltaCommit = commit.parse("commit")?;
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .commit_delta(&commit)
                .await
                .map(|()| NoResult)
                .map_err(to_py_err)
        })
    }

    /// Call an endpoint of the server that has no method here.
    ///
    /// Args:
    ///     method: The HTTP method, such as `"GET"`.
    ///     path: The path: relative to the base URL (`"catalogs"`), or from
    ///         the root of the server (`"/api/1.0/unity-control/scim2/Users"`).
    ///     query: The query parameters, such as `{"max_results": "10"}`.
    ///     body: The body, as a value that `json.dumps` takes.
    ///
    /// Returns:
    ///     The JSON response, as `json.loads` gives it; `None` for an empty
    ///     one.
    #[pyo3(signature = (method, path, *, query=None, body=None))]
    fn raw_json(
        &self,
        py: Python<'_>,
        method: HttpMethod,
        path: String,
        query: Option<HashMap<String, String>>,
        body: Option<Json>,
    ) -> PyResult<Json> {
        let query = query_pairs(query.as_ref());
        let body = body.map(|b| b.0);
        py.detach(|| rt().block_on(self.inner.raw_json(method.0, &path, &query, body.as_ref())))
            .map(Json)
            .map_err(to_py_err)
    }

    /// Like `raw_json`, but returns an `asyncio.Future`.
    #[pyo3(signature = (method, path, *, query=None, body=None))]
    fn raw_json_async<'py>(
        &self,
        py: Python<'py>,
        method: HttpMethod,
        path: String,
        query: Option<HashMap<String, String>>,
        body: Option<Json>,
    ) -> PyResult<PyFuture<'py, Json>> {
        let body = body.map(|b| b.0);
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .raw_json(method.0, &path, &query_pairs(query.as_ref()), body.as_ref())
                .await
                .map(Json)
                .map_err(to_py_err)
        })
    }

    /// Post a form, as the OAuth token exchange
    /// (`/api/1.0/unity-control/auth/tokens`) takes it.
    ///
    /// Args:
    ///     path: The path, as `raw_json` takes it.
    ///     form: The fields of the form.
    ///
    /// Returns:
    ///     The JSON response, as `json.loads` gives it.
    fn post_form(
        &self,
        py: Python<'_>,
        path: String,
        form: HashMap<String, String>,
    ) -> PyResult<Json> {
        py.detach(|| rt().block_on(self.inner.post_form(&path, &form_pairs(&form))))
            .map(Json)
            .map_err(to_py_err)
    }

    /// Like `post_form`, but returns an `asyncio.Future`.
    fn post_form_async<'py>(
        &self,
        py: Python<'py>,
        path: String,
        form: HashMap<String, String>,
    ) -> PyResult<PyFuture<'py, Json>> {
        let client = self.inner.clone();
        PyFuture::new(py, async move {
            client
                .post_form(&path, &form_pairs(&form))
                .await
                .map(Json)
                .map_err(to_py_err)
        })
    }
}

/// A new SCIM user: the email is its name and its primary email, as
/// upstream takes it.
fn new_scim_user(
    email: String,
    display_name: Option<String>,
    external_id: Option<String>,
) -> uc_api::ScimUser {
    uc_api::ScimUser {
        schemas: vec![uc_api::schema_uri::USER.to_string()],
        external_id,
        user_name: email.clone(),
        display_name,
        emails: Some(vec![uc_api::ScimEmail {
            value: email,
            primary: Some(true),
            r#type: None,
        }]),
        ..Default::default()
    }
}

/// The pairs of `raw_json`'s query.
fn query_pairs(query: Option<&HashMap<String, String>>) -> Vec<(&str, String)> {
    query
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect()
}

/// The pairs of `post_form`'s form.
fn form_pairs(form: &HashMap<String, String>) -> Vec<(&str, &str)> {
    form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

/// An HTTP method, in any case: `"get"` is `GET`.
pub struct HttpMethod(uc_client::Method);

impl<'a, 'py> FromPyObject<'a, 'py> for HttpMethod {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        let name: String = obj.extract()?;
        uc_client::Method::from_bytes(name.to_ascii_uppercase().as_bytes())
            .map(HttpMethod)
            .map_err(|_| PyValueError::new_err(format!("invalid HTTP method: {name:?}")))
    }
}

impl PyStubType for HttpMethod {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("str")
    }
}

/// A JSON value: Python gives it as `json.dumps` takes it, and gets it as
/// `json.loads` returns it.
pub struct Json(serde_json::Value);

impl<'a, 'py> FromPyObject<'a, 'py> for Json {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        let py = obj.py();
        let options = [("allow_nan", false)].into_py_dict(py)?;
        let text: String = py
            .import("json")?
            .getattr("dumps")?
            .call((obj,), Some(&options))?
            .extract()?;
        serde_json::from_str(&text)
            .map(Json)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

impl<'py> IntoPyObject<'py> for Json {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        let text =
            serde_json::to_string(&self.0).map_err(|e| PyValueError::new_err(e.to_string()))?;
        py.import("json")?.call_method1("loads", (text,))
    }
}

impl PyStubType for Json {
    fn type_output() -> TypeInfo {
        TypeInfo::any()
    }
}

/// A JSON object: Python gives and gets it as a `dict`, as `json.dumps`
/// takes it and `json.loads` returns it.
pub struct JsonObject(serde_json::Map<String, serde_json::Value>);

impl JsonObject {
    /// `value`, which serializes to an object.
    fn new(value: &impl Serialize) -> PyResult<Self> {
        match serde_json::to_value(value) {
            Ok(serde_json::Value::Object(object)) => Ok(Self(object)),
            Ok(_) => Err(PyValueError::new_err("not a JSON object")),
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        }
    }

    /// The object as a `T`; a `ValueError` that names it `what` if it is
    /// not one.
    fn parse<T: DeserializeOwned>(self, what: &str) -> PyResult<T> {
        serde_json::from_value(serde_json::Value::Object(self.0))
            .map_err(|e| PyValueError::new_err(format!("invalid {what}: {e}")))
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for JsonObject {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        obj.cast::<PyDict>()?;
        match obj.extract::<Json>()?.0 {
            serde_json::Value::Object(object) => Ok(Self(object)),
            _ => Err(PyTypeError::new_err("not a JSON object")),
        }
    }
}

impl<'py> IntoPyObject<'py> for JsonObject {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        Json(serde_json::Value::Object(self.0)).into_pyobject(py)
    }
}

impl PyStubType for JsonObject {
    fn type_output() -> TypeInfo {
        TypeInfo::dict_of::<String, Json>()
    }
}
