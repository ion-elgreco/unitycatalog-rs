// @generated — do not edit by hand.
#![allow(dead_code, unused_imports, clippy::too_many_arguments)]

#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod catalogs;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod credentials;
pub mod error;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod external_locations;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod functions;
pub mod py_future;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod recipients;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod registered_models;
pub mod runtime;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod schemas;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod shares;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod tables;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod tokens;
#[allow(dead_code, unused_imports, clippy::too_many_arguments)]
pub mod volumes;
use crate::codegen::catalogs::PyCatalogClient;
use crate::codegen::credentials::PyCredentialClient;
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use crate::codegen::external_locations::PyExternalLocationClient;
use crate::codegen::functions::PyFunctionClient;
use crate::codegen::recipients::PyRecipientClient;
use crate::codegen::registered_models::PyRegisteredModelClient;
use crate::codegen::schemas::PySchemaClient;
use crate::codegen::shares::PyShareClient;
use crate::codegen::tables::PyTableClient;
use crate::codegen::tokens::PyTokenClient;
use crate::codegen::volumes::PyVolumeClient;
use futures::stream::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::Client;
use uc_client::models::admin::v1::*;
use uc_client::models::audit::v1::*;
use uc_client::models::auth::v1::*;
use uc_client::models::catalogs::v1::*;
use uc_client::models::credentials::v1::*;
use uc_client::models::delta::v1::*;
use uc_client::models::external_locations::v1::*;
use uc_client::models::functions::v1::*;
use uc_client::models::health::v1::*;
use uc_client::models::iceberg::v1::*;
use uc_client::models::metastore::v1::*;
use uc_client::models::models::v1::*;
use uc_client::models::permissions::v1::*;
use uc_client::models::recipients::v1::*;
use uc_client::models::schemas::v1::*;
use uc_client::models::scim::v1::*;
use uc_client::models::shares::v1::*;
use uc_client::models::sharing::v1::*;
use uc_client::models::tables::v1::*;
use uc_client::models::tokens::v1::*;
use uc_client::models::volumes::v1::*;
use uc_client::models::*;

/// A client of the API at one base URL.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyclass(name = "Client", module = "unitycatalog._core")]
pub struct PyClient {
    pub(crate) client: Client,
}

#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyClient {
    /// A client of the API at `base_url`. `token`, when set, goes in every
    /// request as the bearer token.
    #[new]
    #[pyo3(signature = (base_url, token = None))]
    pub fn new(base_url: String, token: Option<String>) -> PyResult<Self> {
        let base_url = base_url.parse().map_err(PyUnityCatalogError::from)?;
        let client = match token {
            Some(token) => Client::new_with_token(base_url, token),
            None => Client::new_unauthenticated(base_url),
        };
        Ok(Self { client })
    }

    /// Get a snapshot of the whole catalog, in one answer.
    ///
    /// Returns:
    ///     A snapshot of the whole catalog: the rows of each table of the store. The audit log is not
    ///     in
    ///     it.
    pub fn get_backup(&self, py: Python) -> PyUnityCatalogResult<PyBackupResponse> {
        let request = self.client.get_backup();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyBackupResponse::from(result))
        })
    }

    /// Like `get_backup`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_backup_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyBackupResponse>> {
        let request = self.client.get_backup();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyBackupResponse::from(result))
        })
    }

    /// Replace the whole catalog with a snapshot, in one transaction: every table of the store is emptied,
    /// then filled with the rows of the snapshot.
    ///
    /// Args:
    ///     version: Version of the snapshot format. A restore takes only version 1.
    ///     tables: The rows of each table, by table name. A row is a JSON object of its columns, as the
    ///             table of the store has them.
    ///
    /// Returns:
    ///     What a restore imported.
    #[pyo3(signature = (version, tables))]
    pub fn restore_backup(
        &self,
        py: Python,
        version: i64,
        tables: ::pyo3::Bound<'_, ::pyo3::PyAny>,
    ) -> PyUnityCatalogResult<PyRestoreResponse> {
        let request = self.client.restore_backup(version, {
            let items: ::std::collections::HashMap<
                ::std::string::String,
                ::pyo3::Bound<'_, ::pyo3::PyAny>,
            > = ::pyo3::types::PyAnyMethods::extract(&tables)?;
            items
                .iter()
                .map(|(key, item)| {
                    ::core::result::Result::Ok((key.clone(), json_list_from_py(item)?))
                })
                .collect::<::pyo3::PyResult<::std::vec::Vec<_>>>()?
        });
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRestoreResponse::from(result))
        })
    }

    /// Like `restore_backup`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (version, tables))]
    pub fn restore_backup_async<'py>(
        &self,
        py: Python<'py>,
        version: i64,
        tables: ::pyo3::Bound<'_, ::pyo3::PyAny>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRestoreResponse>> {
        let request = self.client.restore_backup(version, {
            let items: ::std::collections::HashMap<
                ::std::string::String,
                ::pyo3::Bound<'_, ::pyo3::PyAny>,
            > = ::pyo3::types::PyAnyMethods::extract(&tables)?;
            items
                .iter()
                .map(|(key, item)| {
                    ::core::result::Result::Ok((key.clone(), json_list_from_py(item)?))
                })
                .collect::<::pyo3::PyResult<::std::vec::Vec<_>>>()?
        });
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRestoreResponse::from(result))
        })
    }

    /// List the audit events, newest first.
    ///
    /// Args:
    ///     principal: Return only the events of this principal.
    ///     method: Return only the events of this HTTP method, such as `DELETE`.
    ///     path_prefix: Return only the events whose path starts with this prefix.
    ///     since: Return only the events at or after this time, in epoch milliseconds.
    ///     until: Return only the events at or before this time, in epoch milliseconds.
    ///     max_results: The maximum number of events to return.
    ///     page_token: Opaque token of the page to return: the `next_page_token` of the page before.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(
        signature = (
            *,
            principal = None,
            method = None,
            path_prefix = None,
            since = None,
            until = None,
            max_results = None,
            page_token = None
        )
    )]
    pub fn list_audit_events(
        &self,
        py: Python,
        principal: Option<String>,
        method: Option<String>,
        path_prefix: Option<String>,
        since: Option<i64>,
        until: Option<i64>,
        max_results: Option<i32>,
        page_token: Option<String>,
    ) -> PyUnityCatalogResult<PyListAuditEventsResponse> {
        let request = self
            .client
            .list_audit_events()
            .with_principal(principal)
            .with_method(method)
            .with_path_prefix(path_prefix)
            .with_since(since)
            .with_until(until)
            .with_max_results(max_results)
            .with_page_token(page_token);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyListAuditEventsResponse::from(result))
        })
    }

    /// Like `list_audit_events`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            *,
            principal = None,
            method = None,
            path_prefix = None,
            since = None,
            until = None,
            max_results = None,
            page_token = None
        )
    )]
    pub fn list_audit_events_async<'py>(
        &self,
        py: Python<'py>,
        principal: Option<String>,
        method: Option<String>,
        path_prefix: Option<String>,
        since: Option<i64>,
        until: Option<i64>,
        max_results: Option<i32>,
        page_token: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyListAuditEventsResponse>> {
        let request = self
            .client
            .list_audit_events()
            .with_principal(principal)
            .with_method(method)
            .with_path_prefix(path_prefix)
            .with_since(since)
            .with_until(until)
            .with_max_results(max_results)
            .with_page_token(page_token);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyListAuditEventsResponse::from(result))
        })
    }

    /// Exchange a token of an identity provider for a Unity Catalog access token (OAuth 2.0 token exchange,
    /// RFC 8693). It needs no sign-in.
    ///
    /// Args:
    ///     grant_type: The grant type: `urn:ietf:params:oauth:grant-type:token-exchange`.
    ///     requested_token_type: The type of the token to issue: `urn:ietf:params:oauth:token-
    ///                           type:access_token`.
    ///     subject_token: The token of the identity provider that names the user, such as an ID token.
    ///     subject_token_type: The type of `subject_token`, such as `urn:ietf:params:oauth:token-
    ///                         type:id_token`.
    ///     scope: The scope of the token exchange.
    ///     actor_token: The token of the acting party. Not supported.
    ///     actor_token_type: The type of `actor_token`. Not supported: the server refuses a request with
    ///                       it.
    ///
    /// Returns:
    ///     An OAuth 2.0 token exchange response (RFC 8693): a Unity Catalog access token.
    #[pyo3(
        signature = (
            grant_type,
            requested_token_type,
            subject_token,
            subject_token_type,
            *,
            scope = None,
            actor_token = None,
            actor_token_type = None
        )
    )]
    pub fn exchange_token(
        &self,
        py: Python,
        grant_type: String,
        requested_token_type: String,
        subject_token: String,
        subject_token_type: String,
        scope: Option<String>,
        actor_token: Option<String>,
        actor_token_type: Option<String>,
    ) -> PyUnityCatalogResult<PyOAuthTokenExchangeInfo> {
        let request = self
            .client
            .exchange_token(
                grant_type,
                requested_token_type,
                subject_token,
                subject_token_type,
            )
            .with_scope(scope)
            .with_actor_token(actor_token)
            .with_actor_token_type(actor_token_type);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyOAuthTokenExchangeInfo::from(result))
        })
    }

    /// Like `exchange_token`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            grant_type,
            requested_token_type,
            subject_token,
            subject_token_type,
            *,
            scope = None,
            actor_token = None,
            actor_token_type = None
        )
    )]
    pub fn exchange_token_async<'py>(
        &self,
        py: Python<'py>,
        grant_type: String,
        requested_token_type: String,
        subject_token: String,
        subject_token_type: String,
        scope: Option<String>,
        actor_token: Option<String>,
        actor_token_type: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyOAuthTokenExchangeInfo>> {
        let request = self
            .client
            .exchange_token(
                grant_type,
                requested_token_type,
                subject_token,
                subject_token_type,
            )
            .with_scope(scope)
            .with_actor_token(actor_token)
            .with_actor_token_type(actor_token_type);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyOAuthTokenExchangeInfo::from(result))
        })
    }

    /// Get an access token for a personal access token: the OAuth 2.0 `client_credentials` grant (RFC 6749,
    /// section 4.4). It needs no sign-in.
    ///
    /// Args:
    ///     grant_type: The grant type: `client_credentials`.
    ///     client_secret: The personal access token.
    ///     client_id: The `id` of the personal access token. When given, it must name the token of
    ///                `client_secret`, and `expires_in` is the time until that token expires.
    ///     scope: The scope of the request. The answer repeats it.
    ///
    /// Returns:
    ///     An OAuth 2.0 token response (RFC 6749, section 5.1). The access token is the personal access
    ///     token itself.
    #[pyo3(signature = (grant_type, client_secret, *, client_id = None, scope = None))]
    pub fn request_access_token(
        &self,
        py: Python,
        grant_type: String,
        client_secret: String,
        client_id: Option<String>,
        scope: Option<String>,
    ) -> PyUnityCatalogResult<PyTokenResponse> {
        let request = self
            .client
            .request_access_token(grant_type, client_secret)
            .with_client_id(client_id)
            .with_scope(scope);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTokenResponse::from(result))
        })
    }

    /// Like `request_access_token`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (grant_type, client_secret, *, client_id = None, scope = None))]
    pub fn request_access_token_async<'py>(
        &self,
        py: Python<'py>,
        grant_type: String,
        client_secret: String,
        client_id: Option<String>,
        scope: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTokenResponse>> {
        let request = self
            .client
            .request_access_token(grant_type, client_secret)
            .with_client_id(client_id)
            .with_scope(scope);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyTokenResponse::from(result))
        })
    }

    /// Sign out: expire the `UC_TOKEN` cookie of the request, if any.
    pub fn logout(&self, py: Python) -> PyUnityCatalogResult<()> {
        let request = self.client.logout();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `logout`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn logout_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.logout();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Get how the web UI signs in. It needs no sign-in.
    ///
    /// Returns:
    ///     How the web UI signs in, and where its "Log out" goes.
    pub fn get_auth_mode(&self, py: Python) -> PyUnityCatalogResult<PyAuthMode> {
        let request = self.client.get_auth_mode();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyAuthMode::from(result))
        })
    }

    /// Like `get_auth_mode`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_auth_mode_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyAuthMode>> {
        let request = self.client.get_auth_mode();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyAuthMode::from(result))
        })
    }

    /// List the catalogs that the caller can see.
    ///
    /// Args:
    ///     max_results: The maximum number of catalogs to return.
    ///
    /// Returns:
    ///     The catalogs.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_catalogs(
        &self,
        py: Python,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyCatalogInfo>> {
        let request = self.client.list_catalogs().with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyCatalogInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_catalogs`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_catalogs_async<'py>(
        &self,
        py: Python<'py>,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyCatalogInfo>>>
    {
        let request = self.client.list_catalogs().with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyCatalogInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a catalog.
    ///
    /// Args:
    ///     name: Name of the catalog.
    ///     comment: User-provided free-form text description.
    ///     properties: Key-value properties of the catalog.
    ///     storage_root: Storage root URL for the managed storage of the catalog.
    ///
    /// Returns:
    ///     A catalog: the first layer of the three-level namespace. It holds schemas.
    #[pyo3(
        signature = (name, *, comment = None, properties = None, storage_root = None)
    )]
    pub fn create_catalog(
        &self,
        py: Python,
        name: String,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        storage_root: Option<String>,
    ) -> PyUnityCatalogResult<PyCatalogInfo> {
        let request = self
            .client
            .create_catalog(name)
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_storage_root(storage_root);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyCatalogInfo::from(result))
        })
    }

    /// Like `create_catalog`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (name, *, comment = None, properties = None, storage_root = None)
    )]
    pub fn create_catalog_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        storage_root: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyCatalogInfo>> {
        let request = self
            .client
            .create_catalog(name)
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_storage_root(storage_root);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyCatalogInfo::from(result))
        })
    }

    /// List the credentials that the caller can see.
    ///
    /// Args:
    ///     max_results: The maximum number of credentials to return.
    ///     purpose: Return only the credentials with this purpose.
    ///
    /// Returns:
    ///     The credentials.
    #[pyo3(signature = (*, max_results = None, purpose = None))]
    pub fn list_credentials(
        &self,
        py: Python,
        max_results: Option<i32>,
        purpose: ::core::option::Option<PyCredentialPurpose>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyCredentialInfo>> {
        let request = self
            .client
            .list_credentials()
            .with_max_results(max_results)
            .with_purpose(purpose.map(::core::convert::Into::into));
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyCredentialInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, max_results = None, purpose = None))]
    pub fn list_credentials_async<'py>(
        &self,
        py: Python<'py>,
        max_results: Option<i32>,
        purpose: ::core::option::Option<PyCredentialPurpose>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyCredentialInfo>>>
    {
        let request = self
            .client
            .list_credentials()
            .with_max_results(max_results)
            .with_purpose(purpose.map(::core::convert::Into::into));
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyCredentialInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a credential.
    ///
    /// Args:
    ///     name: Name of the credential.
    ///     aws_iam_role: The AWS IAM role of the credential: an AWS IAM role is the only kind of
    ///                   credential.
    ///     comment: User-provided free-form text description.
    ///     purpose: What the credential is used for.
    ///
    /// Returns:
    ///     A credential that gives access to storage or to external services.
    #[pyo3(signature = (name, aws_iam_role, *, comment = None, purpose = None))]
    pub fn create_credential(
        &self,
        py: Python,
        name: String,
        aws_iam_role: PyAwsIamRoleRequest,
        comment: Option<String>,
        purpose: ::core::option::Option<PyCredentialPurpose>,
    ) -> PyUnityCatalogResult<PyCredentialInfo> {
        let request = self
            .client
            .create_credential(name, aws_iam_role.into())
            .with_comment(comment)
            .with_purpose(purpose.map(::core::convert::Into::into));
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyCredentialInfo::from(result))
        })
    }

    /// Like `create_credential`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (name, aws_iam_role, *, comment = None, purpose = None))]
    pub fn create_credential_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        aws_iam_role: PyAwsIamRoleRequest,
        comment: Option<String>,
        purpose: ::core::option::Option<PyCredentialPurpose>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyCredentialInfo>> {
        let request = self
            .client
            .create_credential(name, aws_iam_role.into())
            .with_comment(comment)
            .with_purpose(purpose.map(::core::convert::Into::into));
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyCredentialInfo::from(result))
        })
    }

    /// Get the configuration of the Delta API for a catalog: the endpoints that the server supports and the
    /// protocol version.
    ///
    /// Args:
    ///     catalog: Name of the catalog.
    ///     protocol_versions: The highest protocol versions that the client supports per major version,
    ///                        comma-separated, such as `1.1,2.3`. The server accepts it and ignores it:
    ///                        `1.0` is the only version.
    ///
    /// Returns:
    ///     The configuration of the Delta API for a catalog.
    #[pyo3(signature = (catalog, *, protocol_versions = None))]
    pub fn get_delta_config(
        &self,
        py: Python,
        catalog: String,
        protocol_versions: Option<String>,
    ) -> PyUnityCatalogResult<PyDeltaCatalogConfig> {
        let request = self
            .client
            .get_delta_config(catalog)
            .with_protocol_versions(protocol_versions);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaCatalogConfig::from(result))
        })
    }

    /// Like `get_delta_config`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, *, protocol_versions = None))]
    pub fn get_delta_config_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        protocol_versions: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaCatalogConfig>> {
        let request = self
            .client
            .get_delta_config(catalog)
            .with_protocol_versions(protocol_versions);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaCatalogConfig::from(result))
        })
    }

    /// Create a staging table: reserve a table id and a storage location for a new MANAGED table, with
    /// credentials to write its first commit.
    ///
    /// Args:
    ///     name: Name of the table.
    ///
    /// Returns:
    ///     A staging table: the table id and storage location that the server reserves for a new
    ///     MANAGED
    ///     table, and what the first commit must contain.
    #[pyo3(signature = (catalog, schema, name))]
    pub fn create_delta_staging_table(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        name: String,
    ) -> PyUnityCatalogResult<PyDeltaStagingTableResponse> {
        let request = self
            .client
            .create_delta_staging_table(catalog, schema, name);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaStagingTableResponse::from(result))
        })
    }

    /// Like `create_delta_staging_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, name))]
    pub fn create_delta_staging_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        name: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaStagingTableResponse>>
    {
        let request = self
            .client
            .create_delta_staging_table(catalog, schema, name);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaStagingTableResponse::from(result))
        })
    }

    /// Create a table. A MANAGED table takes over the staging table at its location.
    ///
    /// Args:
    ///     name: Name of the table.
    ///     location: Storage location of the table. For a MANAGED table, the location of its staging table.
    ///     table_type: Type of the table.
    ///     columns: The columns of the table. They must hold at least one field.
    ///     protocol: The Delta protocol of the table.
    ///     last_commit_timestamp_ms: Timestamp of version 0, the commit that the client wrote before the
    ///                               call, in epoch milliseconds.
    ///     comment: User-provided free-form text description.
    ///     partition_columns: Names of the partition columns.
    ///     properties: The Delta table properties.
    ///     domain_metadata: The domain metadata of the table.
    ///     uniform: UniForm conversion metadata. With it, the table is UniForm-enabled; the client writes
    ///              the Iceberg metadata file before the call.
    ///
    /// Returns:
    ///     A Delta table: its metadata and, for a MANAGED table, its commits that are not backfilled
    ///     yet.
    #[pyo3(
        signature = (
            catalog,
            schema,
            name,
            location,
            table_type,
            columns,
            protocol,
            last_commit_timestamp_ms,
            *,
            comment = None,
            partition_columns = None,
            properties = None,
            domain_metadata = None,
            uniform = None
        )
    )]
    pub fn create_delta_table(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        name: String,
        location: String,
        table_type: PyDeltaTableType,
        columns: PyDeltaStructType,
        protocol: PyDeltaProtocol,
        last_commit_timestamp_ms: i64,
        comment: Option<String>,
        partition_columns: Option<Vec<String>>,
        properties: Option<HashMap<String, String>>,
        domain_metadata: ::core::option::Option<PyDeltaDomainMetadataUpdates>,
        uniform: ::core::option::Option<PyDeltaUniformMetadata>,
    ) -> PyUnityCatalogResult<PyDeltaLoadTableResponse> {
        let request = self
            .client
            .create_delta_table(
                catalog,
                schema,
                name,
                location,
                table_type.into(),
                columns.into(),
                protocol.into(),
                last_commit_timestamp_ms,
            )
            .with_comment(comment)
            .with_partition_columns(partition_columns.unwrap_or_default())
            .with_properties(properties.unwrap_or_default())
            .with_domain_metadata(domain_metadata.map(::core::convert::Into::into))
            .with_uniform(uniform.map(::core::convert::Into::into));
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaLoadTableResponse::from(result))
        })
    }

    /// Like `create_delta_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            catalog,
            schema,
            name,
            location,
            table_type,
            columns,
            protocol,
            last_commit_timestamp_ms,
            *,
            comment = None,
            partition_columns = None,
            properties = None,
            domain_metadata = None,
            uniform = None
        )
    )]
    pub fn create_delta_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        name: String,
        location: String,
        table_type: PyDeltaTableType,
        columns: PyDeltaStructType,
        protocol: PyDeltaProtocol,
        last_commit_timestamp_ms: i64,
        comment: Option<String>,
        partition_columns: Option<Vec<String>>,
        properties: Option<HashMap<String, String>>,
        domain_metadata: ::core::option::Option<PyDeltaDomainMetadataUpdates>,
        uniform: ::core::option::Option<PyDeltaUniformMetadata>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaLoadTableResponse>> {
        let request = self
            .client
            .create_delta_table(
                catalog,
                schema,
                name,
                location,
                table_type.into(),
                columns.into(),
                protocol.into(),
                last_commit_timestamp_ms,
            )
            .with_comment(comment)
            .with_partition_columns(partition_columns.unwrap_or_default())
            .with_properties(properties.unwrap_or_default())
            .with_domain_metadata(domain_metadata.map(::core::convert::Into::into))
            .with_uniform(uniform.map(::core::convert::Into::into));
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaLoadTableResponse::from(result))
        })
    }

    /// Load a table: its metadata and, for a MANAGED table, its commits that are not backfilled yet. It
    /// holds no credentials.
    ///
    /// Returns:
    ///     A Delta table: its metadata and, for a MANAGED table, its commits that are not backfilled
    ///     yet.
    #[pyo3(signature = (catalog, schema, table))]
    pub fn load_delta_table(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        table: String,
    ) -> PyUnityCatalogResult<PyDeltaLoadTableResponse> {
        let request = self.client.load_delta_table(catalog, schema, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaLoadTableResponse::from(result))
        })
    }

    /// Like `load_delta_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, table))]
    pub fn load_delta_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        table: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaLoadTableResponse>> {
        let request = self.client.load_delta_table(catalog, schema, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaLoadTableResponse::from(result))
        })
    }

    /// Update a table or commit to it, then load it. A replay of an accepted `add-commit` is a no-op. A
    /// version or requirement conflict is status 409; too many commits that are not backfilled is status
    /// 429.
    ///
    /// Args:
    ///     requirements: Preconditions of the update. An `assert-table-uuid` requirement is always
    ///                   required; at most one of each type.
    ///     updates: The updates to apply. At least one; at most one of each action.
    ///
    /// Returns:
    ///     A Delta table: its metadata and, for a MANAGED table, its commits that are not backfilled
    ///     yet.
    #[pyo3(signature = (catalog, schema, table, requirements, updates))]
    pub fn update_delta_table(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        table: String,
        requirements: ::std::vec::Vec<PyDeltaTableRequirement>,
        updates: ::std::vec::Vec<PyDeltaTableUpdate>,
    ) -> PyUnityCatalogResult<PyDeltaLoadTableResponse> {
        let request = self.client.update_delta_table(
            catalog,
            schema,
            table,
            requirements
                .into_iter()
                .map(::core::convert::Into::into)
                .collect(),
            updates
                .into_iter()
                .map(::core::convert::Into::into)
                .collect(),
        );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaLoadTableResponse::from(result))
        })
    }

    /// Like `update_delta_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, table, requirements, updates))]
    pub fn update_delta_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        table: String,
        requirements: ::std::vec::Vec<PyDeltaTableRequirement>,
        updates: ::std::vec::Vec<PyDeltaTableUpdate>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaLoadTableResponse>> {
        let request = self.client.update_delta_table(
            catalog,
            schema,
            table,
            requirements
                .into_iter()
                .map(::core::convert::Into::into)
                .collect(),
            updates
                .into_iter()
                .map(::core::convert::Into::into)
                .collect(),
        );
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaLoadTableResponse::from(result))
        })
    }

    /// Check that a table exists. A missing table is status 404.
    #[pyo3(signature = (catalog, schema, table))]
    pub fn delta_table_exists(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        table: String,
    ) -> PyUnityCatalogResult<bool> {
        let request = self.client.delta_table_exists(catalog, schema, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `delta_table_exists`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, table))]
    pub fn delta_table_exists_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        table: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, bool>> {
        let request = self.client.delta_table_exists(catalog, schema, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result)
        })
    }

    /// Delete a table.
    #[pyo3(signature = (catalog, schema, table))]
    pub fn delete_delta_table(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        table: String,
    ) -> PyUnityCatalogResult<()> {
        let request = self.client.delete_delta_table(catalog, schema, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `delete_delta_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, table))]
    pub fn delete_delta_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        table: String,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.delete_delta_table(catalog, schema, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Rename a table in its schema.
    ///
    /// Args:
    ///     new_name: New name of the table, in the same schema.
    #[pyo3(signature = (catalog, schema, table, new_name))]
    pub fn rename_delta_table(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        table: String,
        new_name: String,
    ) -> PyUnityCatalogResult<()> {
        let request = self
            .client
            .rename_delta_table(catalog, schema, table, new_name);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `rename_delta_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, table, new_name))]
    pub fn rename_delta_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        table: String,
        new_name: String,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self
            .client
            .rename_delta_table(catalog, schema, table, new_name);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Report the metrics of a commit. The server checks the table id and does not store the metrics.
    ///
    /// Args:
    ///     table_id: Unique identifier of the table. It must be the id of the table in the path.
    ///     report: The metrics. The server accepts them and does not store them.
    #[pyo3(signature = (catalog, schema, table, table_id, *, report = None))]
    pub fn report_delta_table_metrics(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        table: String,
        table_id: String,
        report: ::core::option::Option<PyDeltaMetricsReport>,
    ) -> PyUnityCatalogResult<()> {
        let request = self
            .client
            .report_delta_table_metrics(catalog, schema, table, table_id)
            .with_report(report.map(::core::convert::Into::into));
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `report_delta_table_metrics`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, table, table_id, *, report = None))]
    pub fn report_delta_table_metrics_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        table: String,
        table_id: String,
        report: ::core::option::Option<PyDeltaMetricsReport>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self
            .client
            .report_delta_table_metrics(catalog, schema, table, table_id)
            .with_report(report.map(::core::convert::Into::into));
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Get temporary credentials for the data of a table.
    ///
    /// Args:
    ///     operation: The access level of the credentials.
    ///
    /// Returns:
    ///     Temporary credentials for the data of a table.
    #[pyo3(signature = (catalog, schema, table, operation))]
    pub fn get_delta_table_credentials(
        &self,
        py: Python,
        catalog: String,
        schema: String,
        table: String,
        operation: PyDeltaCredentialOperation,
    ) -> PyUnityCatalogResult<PyDeltaCredentialsResponse> {
        let request =
            self.client
                .get_delta_table_credentials(catalog, schema, table, operation.into());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaCredentialsResponse::from(result))
        })
    }

    /// Like `get_delta_table_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, schema, table, operation))]
    pub fn get_delta_table_credentials_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        schema: String,
        table: String,
        operation: PyDeltaCredentialOperation,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaCredentialsResponse>>
    {
        let request =
            self.client
                .get_delta_table_credentials(catalog, schema, table, operation.into());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaCredentialsResponse::from(result))
        })
    }

    /// Get temporary credentials for the data of a staging table. They are always `READ_WRITE`, for the
    /// first commit. Only the owner of the staging table gets them.
    ///
    /// Returns:
    ///     Temporary credentials for the data of a table.
    #[pyo3(signature = (table_id))]
    pub fn get_delta_staging_table_credentials(
        &self,
        py: Python,
        table_id: String,
    ) -> PyUnityCatalogResult<PyDeltaCredentialsResponse> {
        let request = self.client.get_delta_staging_table_credentials(table_id);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaCredentialsResponse::from(result))
        })
    }

    /// Like `get_delta_staging_table_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (table_id))]
    pub fn get_delta_staging_table_credentials_async<'py>(
        &self,
        py: Python<'py>,
        table_id: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaCredentialsResponse>>
    {
        let request = self.client.get_delta_staging_table_credentials(table_id);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaCredentialsResponse::from(result))
        })
    }

    /// Ratify a staged commit, report backfilled commits, or both. Another commit at the same version is a
    /// conflict (status 409).
    ///
    /// Args:
    ///     table_id: Unique identifier of the table to commit to.
    ///     table_uri: Storage location of the table. It must be the location of the table.
    ///     commit_info: The commit file to ratify.
    ///     latest_backfilled_version: The commits up to this version are backfilled into `_delta_log`, so
    ///                                the server stops tracking them.
    ///     metadata: A metadata change to apply with the commit. Not with a backfill report alone.
    ///     uniform: UniForm metadata to commit with the commit.
    #[pyo3(
        signature = (
            table_id,
            table_uri,
            *,
            commit_info = None,
            latest_backfilled_version = None,
            metadata = None,
            uniform = None
        )
    )]
    pub fn commit_delta(
        &self,
        py: Python,
        table_id: String,
        table_uri: String,
        commit_info: ::core::option::Option<PyDeltaCommitInfo>,
        latest_backfilled_version: Option<i64>,
        metadata: ::core::option::Option<PyDeltaMetadata>,
        uniform: ::core::option::Option<PyDeltaUniform>,
    ) -> PyUnityCatalogResult<()> {
        let request = self
            .client
            .commit_delta(table_id, table_uri)
            .with_commit_info(commit_info.map(::core::convert::Into::into))
            .with_latest_backfilled_version(latest_backfilled_version)
            .with_metadata(metadata.map(::core::convert::Into::into))
            .with_uniform(uniform.map(::core::convert::Into::into));
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `commit_delta`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            table_id,
            table_uri,
            *,
            commit_info = None,
            latest_backfilled_version = None,
            metadata = None,
            uniform = None
        )
    )]
    pub fn commit_delta_async<'py>(
        &self,
        py: Python<'py>,
        table_id: String,
        table_uri: String,
        commit_info: ::core::option::Option<PyDeltaCommitInfo>,
        latest_backfilled_version: Option<i64>,
        metadata: ::core::option::Option<PyDeltaMetadata>,
        uniform: ::core::option::Option<PyDeltaUniform>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self
            .client
            .commit_delta(table_id, table_uri)
            .with_commit_info(commit_info.map(::core::convert::Into::into))
            .with_latest_backfilled_version(latest_backfilled_version)
            .with_metadata(metadata.map(::core::convert::Into::into))
            .with_uniform(uniform.map(::core::convert::Into::into));
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Get the commits of a table that are not backfilled yet. The server takes the request as the JSON
    /// body of a `GET`.
    ///
    /// Args:
    ///     table_id: Unique identifier of the table.
    ///     start_version: First version to return (inclusive). It must be 0 or more.
    ///     table_uri: Storage location of the table.
    ///     end_version: Last version to return (inclusive). It does not change `latest_table_version`.
    ///
    /// Returns:
    ///     The commits that the server tracks for a table.
    #[pyo3(
        signature = (table_id, start_version, *, table_uri = None, end_version = None)
    )]
    pub fn get_delta_commits(
        &self,
        py: Python,
        table_id: String,
        start_version: i64,
        table_uri: Option<String>,
        end_version: Option<i64>,
    ) -> PyUnityCatalogResult<PyDeltaGetCommitsResponse> {
        let request = self
            .client
            .get_delta_commits(table_id, start_version)
            .with_table_uri(table_uri)
            .with_end_version(end_version);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyDeltaGetCommitsResponse::from(result))
        })
    }

    /// Like `get_delta_commits`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (table_id, start_version, *, table_uri = None, end_version = None)
    )]
    pub fn get_delta_commits_async<'py>(
        &self,
        py: Python<'py>,
        table_id: String,
        start_version: i64,
        table_uri: Option<String>,
        end_version: Option<i64>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyDeltaGetCommitsResponse>> {
        let request = self
            .client
            .get_delta_commits(table_id, start_version)
            .with_table_uri(table_uri)
            .with_end_version(end_version);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyDeltaGetCommitsResponse::from(result))
        })
    }

    /// List the external locations that the caller can see.
    ///
    /// Args:
    ///     max_results: The maximum number of external locations to return.
    ///
    /// Returns:
    ///     The external locations.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_external_locations(
        &self,
        py: Python,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyExternalLocationInfo>> {
        let request = self
            .client
            .list_external_locations()
            .with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyExternalLocationInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_external_locations`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_external_locations_async<'py>(
        &self,
        py: Python<'py>,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyExternalLocationInfo>>,
    > {
        let request = self
            .client
            .list_external_locations()
            .with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyExternalLocationInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create an external location.
    ///
    /// Args:
    ///     name: Name of the external location.
    ///     url: URL of the storage path, such as `s3://bucket/path`.
    ///     credential_name: Name of the credential that gives access to the path.
    ///     comment: User-provided free-form text description.
    ///
    /// Returns:
    ///     An external location: a storage path and the credential that gives access to it.
    #[pyo3(signature = (name, url, credential_name, *, comment = None))]
    pub fn create_external_location(
        &self,
        py: Python,
        name: String,
        url: String,
        credential_name: String,
        comment: Option<String>,
    ) -> PyUnityCatalogResult<PyExternalLocationInfo> {
        let request = self
            .client
            .create_external_location(name, url, credential_name)
            .with_comment(comment);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyExternalLocationInfo::from(result))
        })
    }

    /// Like `create_external_location`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (name, url, credential_name, *, comment = None))]
    pub fn create_external_location_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        url: String,
        credential_name: String,
        comment: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyExternalLocationInfo>> {
        let request = self
            .client
            .create_external_location(name, url, credential_name)
            .with_comment(comment);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyExternalLocationInfo::from(result))
        })
    }

    /// List the functions of a schema that the caller can see.
    ///
    /// Args:
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     max_results: The maximum number of functions to return.
    ///
    /// Returns:
    ///     The functions.
    #[pyo3(signature = (catalog_name, schema_name, *, max_results = None))]
    pub fn list_functions(
        &self,
        py: Python,
        catalog_name: String,
        schema_name: String,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyFunctionInfo>> {
        let request = self
            .client
            .list_functions(catalog_name, schema_name)
            .with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyFunctionInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_functions`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog_name, schema_name, *, max_results = None))]
    pub fn list_functions_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyFunctionInfo>>>
    {
        let request = self
            .client
            .list_functions(catalog_name, schema_name)
            .with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyFunctionInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a function.
    ///
    /// Args:
    ///     function_info: The function to create.
    ///
    /// Returns:
    ///     A function: a user-defined function in the third layer of the three-level namespace, below
    ///     a schema.
    #[pyo3(signature = (function_info))]
    pub fn create_function(
        &self,
        py: Python,
        function_info: PyCreateFunction,
    ) -> PyUnityCatalogResult<PyFunctionInfo> {
        let request = self.client.create_function(function_info.into());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyFunctionInfo::from(result))
        })
    }

    /// Like `create_function`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (function_info))]
    pub fn create_function_async<'py>(
        &self,
        py: Python<'py>,
        function_info: PyCreateFunction,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyFunctionInfo>> {
        let request = self.client.create_function(function_info.into());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyFunctionInfo::from(result))
        })
    }

    /// Check that the server answers. The database is not asked.
    ///
    /// Returns:
    ///     The state of the server.
    pub fn get_health(&self, py: Python) -> PyUnityCatalogResult<PyHealthResponse> {
        let request = self.client.get_health();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyHealthResponse::from(result))
        })
    }

    /// Like `get_health`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_health_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyHealthResponse>> {
        let request = self.client.get_health();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyHealthResponse::from(result))
        })
    }

    /// Get the version of the server.
    ///
    /// Returns:
    ///     The server and its versions.
    pub fn get_version(&self, py: Python) -> PyUnityCatalogResult<PyVersionResponse> {
        let request = self.client.get_version();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyVersionResponse::from(result))
        })
    }

    /// Like `get_version`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_version_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyVersionResponse>> {
        let request = self.client.get_version();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyVersionResponse::from(result))
        })
    }

    /// Get the client configuration of a catalog.
    ///
    /// Args:
    ///     warehouse: Name of the catalog: the Iceberg warehouse.
    ///
    /// Returns:
    ///     The client configuration of a catalog.
    #[pyo3(signature = (warehouse))]
    pub fn get_iceberg_config(
        &self,
        py: Python,
        warehouse: String,
    ) -> PyUnityCatalogResult<PyIcebergConfigResponse> {
        let request = self.client.get_iceberg_config(warehouse);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergConfigResponse::from(result))
        })
    }

    /// Like `get_iceberg_config`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (warehouse))]
    pub fn get_iceberg_config_async<'py>(
        &self,
        py: Python<'py>,
        warehouse: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergConfigResponse>> {
        let request = self.client.get_iceberg_config(warehouse);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergConfigResponse::from(result))
        })
    }

    /// List the namespaces of a catalog: its schemas that the caller can read.
    ///
    /// Args:
    ///     parent: Parent namespace. Namespaces are not nested: a parent that exists has no children, a
    ///             parent that does not exist is not found. An empty value is no parent.
    ///     page_token: Opaque token of the page to return: the `next-page-token` of the page before.
    ///                 An empty token asks for the first page. Without a token the answer is the whole
    ///                 listing.
    ///     page_size: The maximum number of namespaces in a page, at least 1; a larger value than 100
    ///                counts as 100. Read only with a page token.
    ///
    /// Returns:
    ///     A page of namespaces.
    #[pyo3(signature = (catalog, *, parent = None, page_token = None, page_size = None))]
    pub fn list_iceberg_namespaces(
        &self,
        py: Python,
        catalog: String,
        parent: Option<String>,
        page_token: Option<String>,
        page_size: Option<i32>,
    ) -> PyUnityCatalogResult<PyIcebergListNamespacesResponse> {
        let request = self
            .client
            .list_iceberg_namespaces(catalog)
            .with_parent(parent)
            .with_page_token(page_token)
            .with_page_size(page_size);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergListNamespacesResponse::from(result))
        })
    }

    /// Like `list_iceberg_namespaces`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, *, parent = None, page_token = None, page_size = None))]
    pub fn list_iceberg_namespaces_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        parent: Option<String>,
        page_token: Option<String>,
        page_size: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergListNamespacesResponse>>
    {
        let request = self
            .client
            .list_iceberg_namespaces(catalog)
            .with_parent(parent)
            .with_page_token(page_token)
            .with_page_size(page_size);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergListNamespacesResponse::from(result))
        })
    }

    /// Create a namespace: a schema of the catalog.
    ///
    /// Args:
    ///     namespace: The levels of the namespace. Exactly one level: the name of the schema to create.
    ///     properties: Properties of the namespace.
    ///
    /// Returns:
    ///     A namespace and its properties.
    #[pyo3(signature = (catalog, namespace, *, properties = None))]
    pub fn create_iceberg_namespace(
        &self,
        py: Python,
        catalog: String,
        namespace: Vec<String>,
        properties: Option<HashMap<String, String>>,
    ) -> PyUnityCatalogResult<PyIcebergNamespaceResponse> {
        let request = self
            .client
            .create_iceberg_namespace(catalog, namespace)
            .with_properties(properties.unwrap_or_default());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergNamespaceResponse::from(result))
        })
    }

    /// Like `create_iceberg_namespace`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, *, properties = None))]
    pub fn create_iceberg_namespace_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: Vec<String>,
        properties: Option<HashMap<String, String>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergNamespaceResponse>>
    {
        let request = self
            .client
            .create_iceberg_namespace(catalog, namespace)
            .with_properties(properties.unwrap_or_default());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergNamespaceResponse::from(result))
        })
    }

    /// Get a namespace and its properties.
    ///
    /// Returns:
    ///     A namespace and its properties.
    #[pyo3(signature = (catalog, namespace))]
    pub fn load_iceberg_namespace(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
    ) -> PyUnityCatalogResult<PyIcebergNamespaceResponse> {
        let request = self.client.load_iceberg_namespace(catalog, namespace);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergNamespaceResponse::from(result))
        })
    }

    /// Like `load_iceberg_namespace`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace))]
    pub fn load_iceberg_namespace_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergNamespaceResponse>>
    {
        let request = self.client.load_iceberg_namespace(catalog, namespace);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergNamespaceResponse::from(result))
        })
    }

    /// Check that a namespace exists. A namespace that does not exist is a not-found error.
    #[pyo3(signature = (catalog, namespace))]
    pub fn iceberg_namespace_exists(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
    ) -> PyUnityCatalogResult<bool> {
        let request = self.client.iceberg_namespace_exists(catalog, namespace);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `iceberg_namespace_exists`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace))]
    pub fn iceberg_namespace_exists_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, bool>> {
        let request = self.client.iceberg_namespace_exists(catalog, namespace);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result)
        })
    }

    /// Drop a namespace. It must be empty.
    #[pyo3(signature = (catalog, namespace))]
    pub fn drop_iceberg_namespace(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
    ) -> PyUnityCatalogResult<()> {
        let request = self.client.drop_iceberg_namespace(catalog, namespace);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `drop_iceberg_namespace`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace))]
    pub fn drop_iceberg_namespace_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.drop_iceberg_namespace(catalog, namespace);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Set and remove properties of a namespace.
    ///
    /// Args:
    ///     removals: Keys of the properties to remove. A key cannot be both removed and set.
    ///     updates: Properties to set.
    ///
    /// Returns:
    ///     The changes of a namespace properties update.
    #[pyo3(signature = (catalog, namespace, *, removals = None, updates = None))]
    pub fn update_iceberg_namespace_properties(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        removals: Option<Vec<String>>,
        updates: Option<HashMap<String, String>>,
    ) -> PyUnityCatalogResult<PyIcebergUpdateNamespacePropertiesResponse> {
        let request = self
            .client
            .update_iceberg_namespace_properties(catalog, namespace)
            .with_removals(removals.unwrap_or_default())
            .with_updates(updates.unwrap_or_default());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergUpdateNamespacePropertiesResponse::from(result))
        })
    }

    /// Like `update_iceberg_namespace_properties`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, *, removals = None, updates = None))]
    pub fn update_iceberg_namespace_properties_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        removals: Option<Vec<String>>,
        updates: Option<HashMap<String, String>>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, PyIcebergUpdateNamespacePropertiesResponse>,
    > {
        let request = self
            .client
            .update_iceberg_namespace_properties(catalog, namespace)
            .with_removals(removals.unwrap_or_default())
            .with_updates(updates.unwrap_or_default());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergUpdateNamespacePropertiesResponse::from(result))
        })
    }

    /// List the tables of a namespace that have Iceberg metadata and that the caller can read.
    ///
    /// Args:
    ///     page_token: Opaque token of the page to return: the `next-page-token` of the page before.
    ///                 An empty token asks for the first page. Without a token the answer is the whole
    ///                 listing.
    ///     page_size: The maximum number of tables in a page, at least 1; a larger value than 100 counts as
    ///                100. Read only with a page token.
    ///
    /// Returns:
    ///     A page of tables: the tables of a namespace that have Iceberg metadata.
    #[pyo3(signature = (catalog, namespace, *, page_token = None, page_size = None))]
    pub fn list_iceberg_tables(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        page_token: Option<String>,
        page_size: Option<i32>,
    ) -> PyUnityCatalogResult<PyIcebergListTablesResponse> {
        let request = self
            .client
            .list_iceberg_tables(catalog, namespace)
            .with_page_token(page_token)
            .with_page_size(page_size);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergListTablesResponse::from(result))
        })
    }

    /// Like `list_iceberg_tables`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, *, page_token = None, page_size = None))]
    pub fn list_iceberg_tables_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        page_token: Option<String>,
        page_size: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergListTablesResponse>>
    {
        let request = self
            .client
            .list_iceberg_tables(catalog, namespace)
            .with_page_token(page_token)
            .with_page_size(page_size);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergListTablesResponse::from(result))
        })
    }

    /// Create a native Iceberg table, or stage its creation.
    ///
    /// Args:
    ///     name: Name of the table.
    ///     schema: Schema of the table. The server assigns fresh ids.
    ///     location: Location of the table. With a location the table is EXTERNAL; without one it is
    ///               MANAGED, at a location that the server picks.
    ///     partition_spec: Partition spec of the table. Unpartitioned when absent.
    ///     write_order: Sort order of the table. Unsorted when absent.
    ///     stage_create: Return the metadata of the table without creating it. A commit with an `assert-
    ///                   create` requirement creates it.
    ///     properties: Properties of the table. `format-version` picks the format version (2 by default)
    ///                 and is not kept.
    ///
    /// Returns:
    ///     The metadata of a table, with the properties a client needs to read and write its files.
    #[pyo3(
        signature = (
            catalog,
            namespace,
            name,
            schema,
            *,
            location = None,
            partition_spec = None,
            write_order = None,
            stage_create = None,
            properties = None
        )
    )]
    pub fn create_iceberg_table(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        name: String,
        schema: PyIcebergSchema,
        location: Option<String>,
        partition_spec: ::core::option::Option<PyIcebergPartitionSpec>,
        write_order: ::core::option::Option<PyIcebergSortOrder>,
        stage_create: Option<bool>,
        properties: Option<HashMap<String, String>>,
    ) -> PyUnityCatalogResult<PyIcebergLoadTableResponse> {
        let request = self
            .client
            .create_iceberg_table(catalog, namespace, name, schema.into())
            .with_location(location)
            .with_partition_spec(partition_spec.map(::core::convert::Into::into))
            .with_write_order(write_order.map(::core::convert::Into::into))
            .with_stage_create(stage_create)
            .with_properties(properties.unwrap_or_default());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergLoadTableResponse::from(result))
        })
    }

    /// Like `create_iceberg_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            catalog,
            namespace,
            name,
            schema,
            *,
            location = None,
            partition_spec = None,
            write_order = None,
            stage_create = None,
            properties = None
        )
    )]
    pub fn create_iceberg_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        name: String,
        schema: PyIcebergSchema,
        location: Option<String>,
        partition_spec: ::core::option::Option<PyIcebergPartitionSpec>,
        write_order: ::core::option::Option<PyIcebergSortOrder>,
        stage_create: Option<bool>,
        properties: Option<HashMap<String, String>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergLoadTableResponse>>
    {
        let request = self
            .client
            .create_iceberg_table(catalog, namespace, name, schema.into())
            .with_location(location)
            .with_partition_spec(partition_spec.map(::core::convert::Into::into))
            .with_write_order(write_order.map(::core::convert::Into::into))
            .with_stage_create(stage_create)
            .with_properties(properties.unwrap_or_default());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergLoadTableResponse::from(result))
        })
    }

    /// Get the metadata of a table, with credentials for its files.
    ///
    /// Args:
    ///     snapshots: The snapshots to return: `all` (the default) or `refs`, the snapshots that a branch
    ///                or tag points at.
    ///
    /// Returns:
    ///     The metadata of a table, with the properties a client needs to read and write its files.
    #[pyo3(signature = (catalog, namespace, table, *, snapshots = None))]
    pub fn load_iceberg_table(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        table: String,
        snapshots: Option<String>,
    ) -> PyUnityCatalogResult<PyIcebergLoadTableResponse> {
        let request = self
            .client
            .load_iceberg_table(catalog, namespace, table)
            .with_snapshots(snapshots);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergLoadTableResponse::from(result))
        })
    }

    /// Like `load_iceberg_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, table, *, snapshots = None))]
    pub fn load_iceberg_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        table: String,
        snapshots: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergLoadTableResponse>>
    {
        let request = self
            .client
            .load_iceberg_table(catalog, namespace, table)
            .with_snapshots(snapshots);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergLoadTableResponse::from(result))
        })
    }

    /// Check that a table exists. A table that does not exist is a not-found error.
    #[pyo3(signature = (catalog, namespace, table))]
    pub fn iceberg_table_exists(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        table: String,
    ) -> PyUnityCatalogResult<bool> {
        let request = self.client.iceberg_table_exists(catalog, namespace, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `iceberg_table_exists`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, table))]
    pub fn iceberg_table_exists_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        table: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, bool>> {
        let request = self.client.iceberg_table_exists(catalog, namespace, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result)
        })
    }

    /// Commit changes to a native Iceberg table, or create a staged table.
    ///
    /// Args:
    ///     identifier: The table. The server does not read it: the path names the table.
    ///     requirements: Conditions that the table must meet for the commit to apply. With `assert-create`,
    ///                   the commit creates a staged table.
    ///     updates: Changes to the metadata, applied in order.
    ///
    /// Returns:
    ///     The metadata of a table, with the properties a client needs to read and write its files.
    #[pyo3(
        signature = (
            catalog,
            namespace,
            table,
            *,
            identifier = None,
            requirements = None,
            updates = None
        )
    )]
    pub fn commit_iceberg_table(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        table: String,
        identifier: ::core::option::Option<PyIcebergTableIdentifier>,
        requirements: ::core::option::Option<::std::vec::Vec<PyIcebergTableRequirement>>,
        updates: ::core::option::Option<::std::vec::Vec<PyIcebergTableUpdate>>,
    ) -> PyUnityCatalogResult<PyIcebergLoadTableResponse> {
        let request = self
            .client
            .commit_iceberg_table(catalog, namespace, table)
            .with_identifier(identifier.map(::core::convert::Into::into))
            .with_requirements(
                requirements
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_updates(
                updates
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergLoadTableResponse::from(result))
        })
    }

    /// Like `commit_iceberg_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            catalog,
            namespace,
            table,
            *,
            identifier = None,
            requirements = None,
            updates = None
        )
    )]
    pub fn commit_iceberg_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        table: String,
        identifier: ::core::option::Option<PyIcebergTableIdentifier>,
        requirements: ::core::option::Option<::std::vec::Vec<PyIcebergTableRequirement>>,
        updates: ::core::option::Option<::std::vec::Vec<PyIcebergTableUpdate>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergLoadTableResponse>>
    {
        let request = self
            .client
            .commit_iceberg_table(catalog, namespace, table)
            .with_identifier(identifier.map(::core::convert::Into::into))
            .with_requirements(
                requirements
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_updates(
                updates
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
            Ok(PyIcebergLoadTableResponse::from(result))
        })
    }

    /// Drop a native Iceberg table. UniForm tables are dropped through the Unity Catalog API.
    ///
    /// Args:
    ///     purge_requested: Whether to delete the data too. Checked, but without effect: the files of a
    ///                      MANAGED table are always removed, those of an EXTERNAL table never.
    #[pyo3(signature = (catalog, namespace, table, *, purge_requested = None))]
    pub fn drop_iceberg_table(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        table: String,
        purge_requested: Option<bool>,
    ) -> PyUnityCatalogResult<()> {
        let request = self
            .client
            .drop_iceberg_table(catalog, namespace, table)
            .with_purge_requested(purge_requested);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `drop_iceberg_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, table, *, purge_requested = None))]
    pub fn drop_iceberg_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        table: String,
        purge_requested: Option<bool>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self
            .client
            .drop_iceberg_table(catalog, namespace, table)
            .with_purge_requested(purge_requested);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Get fresh credentials for the files of a table.
    ///
    /// Returns:
    ///     Fresh credentials for the files of a table.
    #[pyo3(signature = (catalog, namespace, table))]
    pub fn load_iceberg_table_credentials(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        table: String,
    ) -> PyUnityCatalogResult<PyIcebergLoadCredentialsResponse> {
        let request = self
            .client
            .load_iceberg_table_credentials(catalog, namespace, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyIcebergLoadCredentialsResponse::from(result))
        })
    }

    /// Like `load_iceberg_table_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, table))]
    pub fn load_iceberg_table_credentials_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        table: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyIcebergLoadCredentialsResponse>>
    {
        let request = self
            .client
            .load_iceberg_table_credentials(catalog, namespace, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyIcebergLoadCredentialsResponse::from(result))
        })
    }

    /// Send a scan or commit report of a table. The server checks it and does not keep it.
    ///
    /// Args:
    ///     report_type: The type of report: `scan-report` or `commit-report`.
    ///     table_name: Full name of the table.
    ///     snapshot_id: Id of the snapshot that was scanned or committed.
    ///     filter: `scan-report`: the filter of the scan, an Iceberg expression (`true`, `false`, or an
    ///             expression object).
    ///     schema_id: `scan-report`: id of the schema of the scan.
    ///     projected_field_ids: `scan-report`: ids of the projected fields. Always sent.
    ///     projected_field_names: `scan-report`: names of the projected fields. Always sent.
    ///     sequence_number: `commit-report`: sequence number of the commit.
    ///     operation: `commit-report`: the operation of the commit, such as `append`.
    ///     metrics: The metrics, by name: counters (`unit`, `value`) and timers (`time-unit`, `count`,
    ///              `total- duration`).
    ///     metadata: Other properties of the report.
    #[pyo3(
        signature = (
            catalog,
            namespace,
            table,
            report_type,
            *,
            table_name = None,
            snapshot_id = None,
            filter = None,
            schema_id = None,
            projected_field_ids = None,
            projected_field_names = None,
            sequence_number = None,
            operation = None,
            metrics = None,
            metadata = None
        )
    )]
    pub fn report_iceberg_metrics(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        table: String,
        report_type: String,
        table_name: Option<String>,
        snapshot_id: Option<i64>,
        filter: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
        schema_id: Option<i32>,
        projected_field_ids: Option<Vec<i32>>,
        projected_field_names: Option<Vec<String>>,
        sequence_number: Option<i64>,
        operation: Option<String>,
        metrics: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
        metadata: Option<HashMap<String, String>>,
    ) -> PyUnityCatalogResult<()> {
        let request = self
            .client
            .report_iceberg_metrics(catalog, namespace, table, report_type)
            .with_table_name(table_name)
            .with_snapshot_id(snapshot_id)
            .with_filter(filter.as_ref().map(json_value_from_py).transpose()?)
            .with_schema_id(schema_id)
            .with_projected_field_ids(projected_field_ids.unwrap_or_default())
            .with_projected_field_names(projected_field_names.unwrap_or_default())
            .with_sequence_number(sequence_number)
            .with_operation(operation)
            .with_metrics(metrics.as_ref().map(json_struct_from_py).transpose()?)
            .with_metadata(metadata.unwrap_or_default());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `report_iceberg_metrics`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            catalog,
            namespace,
            table,
            report_type,
            *,
            table_name = None,
            snapshot_id = None,
            filter = None,
            schema_id = None,
            projected_field_ids = None,
            projected_field_names = None,
            sequence_number = None,
            operation = None,
            metrics = None,
            metadata = None
        )
    )]
    pub fn report_iceberg_metrics_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        table: String,
        report_type: String,
        table_name: Option<String>,
        snapshot_id: Option<i64>,
        filter: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
        schema_id: Option<i32>,
        projected_field_ids: Option<Vec<i32>>,
        projected_field_names: Option<Vec<String>>,
        sequence_number: Option<i64>,
        operation: Option<String>,
        metrics: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
        metadata: Option<HashMap<String, String>>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self
            .client
            .report_iceberg_metrics(catalog, namespace, table, report_type)
            .with_table_name(table_name)
            .with_snapshot_id(snapshot_id)
            .with_filter(filter.as_ref().map(json_value_from_py).transpose()?)
            .with_schema_id(schema_id)
            .with_projected_field_ids(projected_field_ids.unwrap_or_default())
            .with_projected_field_names(projected_field_names.unwrap_or_default())
            .with_sequence_number(sequence_number)
            .with_operation(operation)
            .with_metrics(metrics.as_ref().map(json_struct_from_py).transpose()?)
            .with_metadata(metadata.unwrap_or_default());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Get a view. Views are not served: the answer is always a not-found error.
    #[pyo3(signature = (catalog, namespace, view))]
    pub fn load_iceberg_view(
        &self,
        py: Python,
        catalog: String,
        namespace: String,
        view: String,
    ) -> PyUnityCatalogResult<()> {
        let request = self.client.load_iceberg_view(catalog, namespace, view);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `load_iceberg_view`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, namespace, view))]
    pub fn load_iceberg_view_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        namespace: String,
        view: String,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.load_iceberg_view(catalog, namespace, view);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Rename a native Iceberg table, in its namespace. A move to another namespace is not supported.
    ///
    /// Args:
    ///     source: The table to rename.
    ///     destination: The new name of the table, in the same namespace.
    #[pyo3(signature = (catalog, source, destination))]
    pub fn rename_iceberg_table(
        &self,
        py: Python,
        catalog: String,
        source: PyIcebergTableIdentifier,
        destination: PyIcebergTableIdentifier,
    ) -> PyUnityCatalogResult<()> {
        let request = self
            .client
            .rename_iceberg_table(catalog, source.into(), destination.into());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `rename_iceberg_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog, source, destination))]
    pub fn rename_iceberg_table_async<'py>(
        &self,
        py: Python<'py>,
        catalog: String,
        source: PyIcebergTableIdentifier,
        destination: PyIcebergTableIdentifier,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self
            .client
            .rename_iceberg_table(catalog, source.into(), destination.into());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Get a summary of the metastore.
    ///
    /// Returns:
    ///     A summary of the metastore of the server.
    pub fn get_metastore_summary(
        &self,
        py: Python,
    ) -> PyUnityCatalogResult<PyGetMetastoreSummaryResponse> {
        let request = self.client.get_metastore_summary();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyGetMetastoreSummaryResponse::from(result))
        })
    }

    /// Like `get_metastore_summary`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_metastore_summary_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyGetMetastoreSummaryResponse>>
    {
        let request = self.client.get_metastore_summary();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyGetMetastoreSummaryResponse::from(result))
        })
    }

    /// List the versions of a registered model.
    ///
    /// Args:
    ///     max_results: The maximum number of model versions to return.
    ///
    /// Returns:
    ///     The model versions.
    #[pyo3(signature = (full_name, *, max_results = None))]
    pub fn list_model_versions(
        &self,
        py: Python,
        full_name: String,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyModelVersionInfo>> {
        let request = self
            .client
            .list_model_versions(full_name)
            .with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyModelVersionInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_model_versions`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (full_name, *, max_results = None))]
    pub fn list_model_versions_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyModelVersionInfo>>,
    > {
        let request = self
            .client
            .list_model_versions(full_name)
            .with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyModelVersionInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a model version with the status PENDING_REGISTRATION. The server gives it the next version
    /// number and a storage location.
    ///
    /// Args:
    ///     model_name: Name of the parent registered model, relative to its schema.
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     source: URI of the source files the model version is created from, such as an MLflow run
    ///             artifact path.
    ///     run_id: ID of the run of the ML package that made the model.
    ///     comment: User-provided free-form text description.
    ///
    /// Returns:
    ///     A version of a registered model. The server gives each new version the next number.
    #[pyo3(
        signature = (
            model_name,
            catalog_name,
            schema_name,
            source,
            *,
            run_id = None,
            comment = None
        )
    )]
    pub fn create_model_version(
        &self,
        py: Python,
        model_name: String,
        catalog_name: String,
        schema_name: String,
        source: String,
        run_id: Option<String>,
        comment: Option<String>,
    ) -> PyUnityCatalogResult<PyModelVersionInfo> {
        let request = self
            .client
            .create_model_version(model_name, catalog_name, schema_name, source)
            .with_run_id(run_id)
            .with_comment(comment);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyModelVersionInfo::from(result))
        })
    }

    /// Like `create_model_version`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            model_name,
            catalog_name,
            schema_name,
            source,
            *,
            run_id = None,
            comment = None
        )
    )]
    pub fn create_model_version_async<'py>(
        &self,
        py: Python<'py>,
        model_name: String,
        catalog_name: String,
        schema_name: String,
        source: String,
        run_id: Option<String>,
        comment: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyModelVersionInfo>> {
        let request = self
            .client
            .create_model_version(model_name, catalog_name, schema_name, source)
            .with_run_id(run_id)
            .with_comment(comment);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyModelVersionInfo::from(result))
        })
    }

    /// Get a model version.
    ///
    /// Returns:
    ///     A version of a registered model. The server gives each new version the next number.
    #[pyo3(signature = (full_name, version))]
    pub fn get_model_version(
        &self,
        py: Python,
        full_name: String,
        version: i64,
    ) -> PyUnityCatalogResult<PyModelVersionInfo> {
        let request = self.client.get_model_version(full_name, version);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyModelVersionInfo::from(result))
        })
    }

    /// Like `get_model_version`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (full_name, version))]
    pub fn get_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyModelVersionInfo>> {
        let request = self.client.get_model_version(full_name, version);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyModelVersionInfo::from(result))
        })
    }

    /// Change a model version.
    ///
    /// Args:
    ///     comment: User-provided free-form text description. The comment is the only field that can
    ///              change; an empty one clears it.
    ///
    /// Returns:
    ///     A version of a registered model. The server gives each new version the next number.
    #[pyo3(signature = (full_name, version, comment))]
    pub fn update_model_version(
        &self,
        py: Python,
        full_name: String,
        version: i64,
        comment: String,
    ) -> PyUnityCatalogResult<PyModelVersionInfo> {
        let request = self
            .client
            .update_model_version(full_name, version, comment);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyModelVersionInfo::from(result))
        })
    }

    /// Like `update_model_version`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (full_name, version, comment))]
    pub fn update_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
        comment: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyModelVersionInfo>> {
        let request = self
            .client
            .update_model_version(full_name, version, comment);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyModelVersionInfo::from(result))
        })
    }

    /// Delete a model version.
    #[pyo3(signature = (full_name, version))]
    pub fn delete_model_version(
        &self,
        py: Python,
        full_name: String,
        version: i64,
    ) -> PyUnityCatalogResult<()> {
        let request = self.client.delete_model_version(full_name, version);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `delete_model_version`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (full_name, version))]
    pub fn delete_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.delete_model_version(full_name, version);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Finalize a model version: its status changes from PENDING_REGISTRATION to READY after the client
    /// wrote all its files.
    ///
    /// Returns:
    ///     A version of a registered model. The server gives each new version the next number.
    #[pyo3(signature = (full_name, version))]
    pub fn finalize_model_version(
        &self,
        py: Python,
        full_name: String,
        version: i64,
    ) -> PyUnityCatalogResult<PyModelVersionInfo> {
        let request = self.client.finalize_model_version(full_name, version);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyModelVersionInfo::from(result))
        })
    }

    /// Like `finalize_model_version`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (full_name, version))]
    pub fn finalize_model_version_async<'py>(
        &self,
        py: Python<'py>,
        full_name: String,
        version: i64,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyModelVersionInfo>> {
        let request = self.client.finalize_model_version(full_name, version);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyModelVersionInfo::from(result))
        })
    }

    /// Get the privileges on a securable.
    ///
    /// Args:
    ///     principal: Return only the privileges of this principal.
    ///
    /// Returns:
    ///     The privileges on a securable, per principal.
    #[pyo3(signature = (securable_type, full_name, *, principal = None))]
    pub fn get_permissions(
        &self,
        py: Python,
        securable_type: String,
        full_name: String,
        principal: Option<String>,
    ) -> PyUnityCatalogResult<PyPermissionsList> {
        let request = self
            .client
            .get_permissions(securable_type, full_name)
            .with_principal(principal);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyPermissionsList::from(result))
        })
    }

    /// Like `get_permissions`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (securable_type, full_name, *, principal = None))]
    pub fn get_permissions_async<'py>(
        &self,
        py: Python<'py>,
        securable_type: String,
        full_name: String,
        principal: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyPermissionsList>> {
        let request = self
            .client
            .get_permissions(securable_type, full_name)
            .with_principal(principal);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyPermissionsList::from(result))
        })
    }

    /// Grant and revoke privileges on a securable. The response lists the privileges of the principals of
    /// the request.
    ///
    /// Args:
    ///     changes: The changes to make, one per principal.
    ///
    /// Returns:
    ///     The privileges on a securable, per principal.
    #[pyo3(signature = (securable_type, full_name, changes))]
    pub fn update_permissions(
        &self,
        py: Python,
        securable_type: String,
        full_name: String,
        changes: ::std::vec::Vec<PyPermissionsChange>,
    ) -> PyUnityCatalogResult<PyPermissionsList> {
        let request = self.client.update_permissions(
            securable_type,
            full_name,
            changes
                .into_iter()
                .map(::core::convert::Into::into)
                .collect(),
        );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyPermissionsList::from(result))
        })
    }

    /// Like `update_permissions`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (securable_type, full_name, changes))]
    pub fn update_permissions_async<'py>(
        &self,
        py: Python<'py>,
        securable_type: String,
        full_name: String,
        changes: ::std::vec::Vec<PyPermissionsChange>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyPermissionsList>> {
        let request = self.client.update_permissions(
            securable_type,
            full_name,
            changes
                .into_iter()
                .map(::core::convert::Into::into)
                .collect(),
        );
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyPermissionsList::from(result))
        })
    }

    /// List the recipients.
    ///
    /// Args:
    ///     max_results: The maximum number of recipients to return.
    ///
    /// Returns:
    ///     The recipients.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_recipients(
        &self,
        py: Python,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyRecipientInfo>> {
        let request = self.client.list_recipients().with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyRecipientInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_recipients`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_recipients_async<'py>(
        &self,
        py: Python<'py>,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyRecipientInfo>>>
    {
        let request = self.client.list_recipients().with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyRecipientInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a recipient. The response holds its bearer token, which no later response shows.
    ///
    /// Args:
    ///     name: Name of the recipient.
    ///     authentication_type: How the recipient signs in. Default: `TOKEN`.
    ///     comment: User-provided free-form text description.
    ///
    /// Returns:
    ///     A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares
    ///     with
    ///     a bearer token. The server keeps a hash of the token only.
    #[pyo3(signature = (name, *, authentication_type = None, comment = None))]
    pub fn create_recipient(
        &self,
        py: Python,
        name: String,
        authentication_type: ::core::option::Option<PyAuthenticationType>,
        comment: Option<String>,
    ) -> PyUnityCatalogResult<PyRecipientInfo> {
        let request = self
            .client
            .create_recipient(name)
            .with_authentication_type(authentication_type.map(::core::convert::Into::into))
            .with_comment(comment);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRecipientInfo::from(result))
        })
    }

    /// Like `create_recipient`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (name, *, authentication_type = None, comment = None))]
    pub fn create_recipient_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        authentication_type: ::core::option::Option<PyAuthenticationType>,
        comment: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRecipientInfo>> {
        let request = self
            .client
            .create_recipient(name)
            .with_authentication_type(authentication_type.map(::core::convert::Into::into))
            .with_comment(comment);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRecipientInfo::from(result))
        })
    }

    /// List the registered models that the caller can see, of a catalog or schema when given.
    ///
    /// Args:
    ///     catalog_name: Name of the catalog whose registered models to return.
    ///     schema_name: Name of the schema whose registered models to return.
    ///     max_results: The maximum number of registered models to return.
    ///
    /// Returns:
    ///     The registered models.
    #[pyo3(signature = (*, catalog_name = None, schema_name = None, max_results = None))]
    pub fn list_registered_models(
        &self,
        py: Python,
        catalog_name: Option<String>,
        schema_name: Option<String>,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyRegisteredModelInfo>> {
        let request = self
            .client
            .list_registered_models()
            .with_catalog_name(catalog_name)
            .with_schema_name(schema_name)
            .with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyRegisteredModelInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_registered_models`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, catalog_name = None, schema_name = None, max_results = None))]
    pub fn list_registered_models_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: Option<String>,
        schema_name: Option<String>,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyRegisteredModelInfo>>,
    > {
        let request = self
            .client
            .list_registered_models()
            .with_catalog_name(catalog_name)
            .with_schema_name(schema_name)
            .with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyRegisteredModelInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a registered model.
    ///
    /// Args:
    ///     name: Name of the registered model, relative to its schema.
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     comment: User-provided free-form text description.
    ///
    /// Returns:
    ///     A registered model: a securable in the three-level namespace (catalog.schema.model) that
    ///     holds
    ///     model versions.
    #[pyo3(signature = (name, catalog_name, schema_name, *, comment = None))]
    pub fn create_registered_model(
        &self,
        py: Python,
        name: String,
        catalog_name: String,
        schema_name: String,
        comment: Option<String>,
    ) -> PyUnityCatalogResult<PyRegisteredModelInfo> {
        let request = self
            .client
            .create_registered_model(name, catalog_name, schema_name)
            .with_comment(comment);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyRegisteredModelInfo::from(result))
        })
    }

    /// Like `create_registered_model`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (name, catalog_name, schema_name, *, comment = None))]
    pub fn create_registered_model_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
        comment: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyRegisteredModelInfo>> {
        let request = self
            .client
            .create_registered_model(name, catalog_name, schema_name)
            .with_comment(comment);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyRegisteredModelInfo::from(result))
        })
    }

    /// List the schemas of a catalog that the caller can see.
    ///
    /// Args:
    ///     catalog_name: Name of the parent catalog.
    ///     max_results: The maximum number of schemas to return.
    ///
    /// Returns:
    ///     The schemas.
    #[pyo3(signature = (catalog_name, *, max_results = None))]
    pub fn list_schemas(
        &self,
        py: Python,
        catalog_name: String,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PySchemaInfo>> {
        let request = self
            .client
            .list_schemas(catalog_name)
            .with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PySchemaInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_schemas`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog_name, *, max_results = None))]
    pub fn list_schemas_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PySchemaInfo>>>
    {
        let request = self
            .client
            .list_schemas(catalog_name)
            .with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PySchemaInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a schema.
    ///
    /// Args:
    ///     name: Name of the schema, relative to the parent catalog.
    ///     catalog_name: Name of the parent catalog.
    ///     comment: User-provided free-form text description.
    ///     properties: Key-value properties of the schema.
    ///     storage_root: Storage root URL for the managed storage of the schema.
    ///
    /// Returns:
    ///     A schema: the second layer of the three-level namespace. It holds tables, volumes, functions
    ///     and models.
    #[pyo3(
        signature = (
            name,
            catalog_name,
            *,
            comment = None,
            properties = None,
            storage_root = None
        )
    )]
    pub fn create_schema(
        &self,
        py: Python,
        name: String,
        catalog_name: String,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        storage_root: Option<String>,
    ) -> PyUnityCatalogResult<PySchemaInfo> {
        let request = self
            .client
            .create_schema(name, catalog_name)
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_storage_root(storage_root);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PySchemaInfo::from(result))
        })
    }

    /// Like `create_schema`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            name,
            catalog_name,
            *,
            comment = None,
            properties = None,
            storage_root = None
        )
    )]
    pub fn create_schema_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        storage_root: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySchemaInfo>> {
        let request = self
            .client
            .create_schema(name, catalog_name)
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_storage_root(storage_root);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySchemaInfo::from(result))
        })
    }

    /// List the users. Only the metastore admin can.
    ///
    /// Args:
    ///     filter: SCIM filter (RFC 7644, section 3.4.2.2), such as `userName eq "a@example.com"`.
    ///     start_index: Index of the first user to return, from 1.
    ///     count: The maximum number of users to return. The default is 50.
    ///
    /// Returns:
    ///     A page of SCIM users.
    #[pyo3(signature = (*, filter = None, start_index = None, count = None))]
    pub fn list_users(
        &self,
        py: Python,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> PyUnityCatalogResult<PyListScimUsersResponse> {
        let request = self
            .client
            .list_users()
            .with_filter(filter)
            .with_start_index(start_index)
            .with_count(count);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyListScimUsersResponse::from(result))
        })
    }

    /// Like `list_users`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, filter = None, start_index = None, count = None))]
    pub fn list_users_async<'py>(
        &self,
        py: Python<'py>,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyListScimUsersResponse>> {
        let request = self
            .client
            .list_users()
            .with_filter(filter)
            .with_start_index(start_index)
            .with_count(count);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyListScimUsersResponse::from(result))
        })
    }

    /// Create a user, named by its primary email. Only the metastore admin can.
    ///
    /// Args:
    ///     external_id: The identity the user has at the client's identity provider. The token exchange
    ///                  resolves an OAuth client (`azp`, `client_id`) to the user by it.
    ///     display_name: Name of the user.
    ///     active: Whether the user can sign in. A new user is always enabled; absent in a replace request:
    ///             unchanged.
    ///     emails: Email addresses of the user. A new user needs one with `primary` set; a replace request
    ///             does not change the email.
    ///     photos: Pictures of the user. The server keeps the first; a replace request does not change it.
    ///
    /// Returns:
    ///     A SCIM 2.0 user. The email of the user is its user name and its one primary email.
    #[pyo3(
        signature = (
            *,
            external_id = None,
            display_name = None,
            active = None,
            emails = None,
            photos = None
        )
    )]
    pub fn create_user(
        &self,
        py: Python,
        external_id: Option<String>,
        display_name: Option<String>,
        active: Option<bool>,
        emails: ::core::option::Option<::std::vec::Vec<PyScimEmail>>,
        photos: ::core::option::Option<::std::vec::Vec<PyScimPhoto>>,
    ) -> PyUnityCatalogResult<PyScimUser> {
        let request = self
            .client
            .create_user()
            .with_external_id(external_id)
            .with_display_name(display_name)
            .with_active(active)
            .with_emails(
                emails
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_photos(
                photos
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimUser::from(result))
        })
    }

    /// Like `create_user`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            *,
            external_id = None,
            display_name = None,
            active = None,
            emails = None,
            photos = None
        )
    )]
    pub fn create_user_async<'py>(
        &self,
        py: Python<'py>,
        external_id: Option<String>,
        display_name: Option<String>,
        active: Option<bool>,
        emails: ::core::option::Option<::std::vec::Vec<PyScimEmail>>,
        photos: ::core::option::Option<::std::vec::Vec<PyScimPhoto>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimUser>> {
        let request = self
            .client
            .create_user()
            .with_external_id(external_id)
            .with_display_name(display_name)
            .with_active(active)
            .with_emails(
                emails
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_photos(
                photos
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
            Ok(PyScimUser::from(result))
        })
    }

    /// Get a user. Only the metastore admin can.
    ///
    /// Returns:
    ///     A SCIM 2.0 user. The email of the user is its user name and its one primary email.
    #[pyo3(signature = (id))]
    pub fn get_user(&self, py: Python, id: String) -> PyUnityCatalogResult<PyScimUser> {
        let request = self.client.get_user(id);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimUser::from(result))
        })
    }

    /// Like `get_user`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (id))]
    pub fn get_user_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimUser>> {
        let request = self.client.get_user(id);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyScimUser::from(result))
        })
    }

    /// Replace a user: the name, state and external id change when given; the email and picture stay. Only
    /// the metastore admin can.
    ///
    /// Args:
    ///     external_id: The identity the user has at the client's identity provider. The token exchange
    ///                  resolves an OAuth client (`azp`, `client_id`) to the user by it.
    ///     display_name: Name of the user.
    ///     active: Whether the user can sign in. A new user is always enabled; absent in a replace request:
    ///             unchanged.
    ///     emails: Email addresses of the user. A new user needs one with `primary` set; a replace request
    ///             does not change the email.
    ///     photos: Pictures of the user. The server keeps the first; a replace request does not change it.
    ///
    /// Returns:
    ///     A SCIM 2.0 user. The email of the user is its user name and its one primary email.
    #[pyo3(
        signature = (
            id,
            *,
            external_id = None,
            display_name = None,
            active = None,
            emails = None,
            photos = None
        )
    )]
    pub fn replace_user(
        &self,
        py: Python,
        id: String,
        external_id: Option<String>,
        display_name: Option<String>,
        active: Option<bool>,
        emails: ::core::option::Option<::std::vec::Vec<PyScimEmail>>,
        photos: ::core::option::Option<::std::vec::Vec<PyScimPhoto>>,
    ) -> PyUnityCatalogResult<PyScimUser> {
        let request = self
            .client
            .replace_user(id)
            .with_external_id(external_id)
            .with_display_name(display_name)
            .with_active(active)
            .with_emails(
                emails
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_photos(
                photos
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimUser::from(result))
        })
    }

    /// Like `replace_user`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            id,
            *,
            external_id = None,
            display_name = None,
            active = None,
            emails = None,
            photos = None
        )
    )]
    pub fn replace_user_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
        external_id: Option<String>,
        display_name: Option<String>,
        active: Option<bool>,
        emails: ::core::option::Option<::std::vec::Vec<PyScimEmail>>,
        photos: ::core::option::Option<::std::vec::Vec<PyScimPhoto>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimUser>> {
        let request = self
            .client
            .replace_user(id)
            .with_external_id(external_id)
            .with_display_name(display_name)
            .with_active(active)
            .with_emails(
                emails
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_photos(
                photos
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
            Ok(PyScimUser::from(result))
        })
    }

    /// Enable or disable a user. The server takes the first `replace` whose `path` is `active` or absent;
    /// without `path`, the first attribute of `value` is the state (`{"active": false}`). With no such
    /// operation the server answers 501. Only the metastore admin can.
    ///
    /// Args:
    ///     operations: The operations, in order.
    ///     schemas: SCIM schema URIs of the request: `urn:ietf:params:scim:api:messages:2.0:PatchOp`.
    #[pyo3(signature = (id, operations, *, schemas = None))]
    pub fn patch_user(
        &self,
        py: Python,
        id: String,
        operations: ::std::vec::Vec<PyScimPatchOperation>,
        schemas: Option<Vec<String>>,
    ) -> PyUnityCatalogResult<()> {
        let request = self
            .client
            .patch_user(
                id,
                operations
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect(),
            )
            .with_schemas(schemas.unwrap_or_default());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `patch_user`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (id, operations, *, schemas = None))]
    pub fn patch_user_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
        operations: ::std::vec::Vec<PyScimPatchOperation>,
        schemas: Option<Vec<String>>,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self
            .client
            .patch_user(
                id,
                operations
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect(),
            )
            .with_schemas(schemas.unwrap_or_default());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Disable a user and remove its grants. The user stays, disabled. Only the metastore admin can.
    #[pyo3(signature = (id))]
    pub fn delete_user(&self, py: Python, id: String) -> PyUnityCatalogResult<()> {
        let request = self.client.delete_user(id);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `delete_user`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (id))]
    pub fn delete_user_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.delete_user(id);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Get the signed-in user. The role `admin` names a caller that can do what the metastore admin does.
    ///
    /// Returns:
    ///     A SCIM 2.0 user. The email of the user is its user name and its one primary email.
    pub fn get_me(&self, py: Python) -> PyUnityCatalogResult<PyScimUser> {
        let request = self.client.get_me();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimUser::from(result))
        })
    }

    /// Like `get_me`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_me_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimUser>> {
        let request = self.client.get_me();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyScimUser::from(result))
        })
    }

    /// List the groups. Only the metastore admin can.
    ///
    /// Args:
    ///     filter: SCIM filter (RFC 7644, section 3.4.2.2), such as `displayName eq "admins"`.
    ///     start_index: Index of the first group to return, from 1.
    ///     count: The maximum number of groups to return. The default is 50.
    ///
    /// Returns:
    ///     A page of SCIM groups.
    #[pyo3(signature = (*, filter = None, start_index = None, count = None))]
    pub fn list_groups(
        &self,
        py: Python,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> PyUnityCatalogResult<PyListScimGroupsResponse> {
        let request = self
            .client
            .list_groups()
            .with_filter(filter)
            .with_start_index(start_index)
            .with_count(count);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyListScimGroupsResponse::from(result))
        })
    }

    /// Like `list_groups`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, filter = None, start_index = None, count = None))]
    pub fn list_groups_async<'py>(
        &self,
        py: Python<'py>,
        filter: Option<String>,
        start_index: Option<i32>,
        count: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyListScimGroupsResponse>> {
        let request = self
            .client
            .list_groups()
            .with_filter(filter)
            .with_start_index(start_index)
            .with_count(count);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyListScimGroupsResponse::from(result))
        })
    }

    /// Create a group. Only the metastore admin can.
    ///
    /// Args:
    ///     display_name: Name of the group.
    ///     schemas: SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Group`.
    ///              Requests send the key even when empty; the server fills it in.
    ///     members: Members of the group.
    ///
    /// Returns:
    ///     A SCIM 2.0 group of users.
    #[pyo3(signature = (display_name, *, schemas = None, members = None))]
    pub fn create_group(
        &self,
        py: Python,
        display_name: String,
        schemas: Option<Vec<String>>,
        members: ::core::option::Option<::std::vec::Vec<PyScimGroupMember>>,
    ) -> PyUnityCatalogResult<PyScimGroup> {
        let request = self
            .client
            .create_group(display_name)
            .with_schemas(schemas.unwrap_or_default())
            .with_members(
                members
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimGroup::from(result))
        })
    }

    /// Like `create_group`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (display_name, *, schemas = None, members = None))]
    pub fn create_group_async<'py>(
        &self,
        py: Python<'py>,
        display_name: String,
        schemas: Option<Vec<String>>,
        members: ::core::option::Option<::std::vec::Vec<PyScimGroupMember>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimGroup>> {
        let request = self
            .client
            .create_group(display_name)
            .with_schemas(schemas.unwrap_or_default())
            .with_members(
                members
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
            Ok(PyScimGroup::from(result))
        })
    }

    /// Get a group. Only the metastore admin can.
    ///
    /// Returns:
    ///     A SCIM 2.0 group of users.
    #[pyo3(signature = (id))]
    pub fn get_group(&self, py: Python, id: String) -> PyUnityCatalogResult<PyScimGroup> {
        let request = self.client.get_group(id);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimGroup::from(result))
        })
    }

    /// Like `get_group`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (id))]
    pub fn get_group_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimGroup>> {
        let request = self.client.get_group(id);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyScimGroup::from(result))
        })
    }

    /// Replace a group: its name and members. Only the metastore admin can.
    ///
    /// Args:
    ///     display_name: Name of the group.
    ///     schemas: SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Group`.
    ///              Requests send the key even when empty; the server fills it in.
    ///     members: Members of the group.
    ///
    /// Returns:
    ///     A SCIM 2.0 group of users.
    #[pyo3(signature = (id, display_name, *, schemas = None, members = None))]
    pub fn replace_group(
        &self,
        py: Python,
        id: String,
        display_name: String,
        schemas: Option<Vec<String>>,
        members: ::core::option::Option<::std::vec::Vec<PyScimGroupMember>>,
    ) -> PyUnityCatalogResult<PyScimGroup> {
        let request = self
            .client
            .replace_group(id, display_name)
            .with_schemas(schemas.unwrap_or_default())
            .with_members(
                members
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimGroup::from(result))
        })
    }

    /// Like `replace_group`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (id, display_name, *, schemas = None, members = None))]
    pub fn replace_group_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
        display_name: String,
        schemas: Option<Vec<String>>,
        members: ::core::option::Option<::std::vec::Vec<PyScimGroupMember>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimGroup>> {
        let request = self
            .client
            .replace_group(id, display_name)
            .with_schemas(schemas.unwrap_or_default())
            .with_members(
                members
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
            Ok(PyScimGroup::from(result))
        })
    }

    /// Change a group: `add` to `members`, `remove` of `members` or `members[value eq "<id>"]`, `replace`
    /// of `displayName`, or `replace` without `path` with a whole group. Other operations are ignored. Only
    /// the metastore admin can.
    ///
    /// Args:
    ///     operations: The operations, in order.
    ///     schemas: SCIM schema URIs of the request: `urn:ietf:params:scim:api:messages:2.0:PatchOp`.
    ///
    /// Returns:
    ///     A SCIM 2.0 group of users.
    #[pyo3(signature = (id, operations, *, schemas = None))]
    pub fn patch_group(
        &self,
        py: Python,
        id: String,
        operations: ::std::vec::Vec<PyScimPatchOperation>,
        schemas: Option<Vec<String>>,
    ) -> PyUnityCatalogResult<PyScimGroup> {
        let request = self
            .client
            .patch_group(
                id,
                operations
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect(),
            )
            .with_schemas(schemas.unwrap_or_default());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimGroup::from(result))
        })
    }

    /// Like `patch_group`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (id, operations, *, schemas = None))]
    pub fn patch_group_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
        operations: ::std::vec::Vec<PyScimPatchOperation>,
        schemas: Option<Vec<String>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimGroup>> {
        let request = self
            .client
            .patch_group(
                id,
                operations
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect(),
            )
            .with_schemas(schemas.unwrap_or_default());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyScimGroup::from(result))
        })
    }

    /// Delete a group. Only the metastore admin can.
    #[pyo3(signature = (id))]
    pub fn delete_group(&self, py: Python, id: String) -> PyUnityCatalogResult<()> {
        let request = self.client.delete_group(id);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(result)
        })
    }

    /// Like `delete_group`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (id))]
    pub fn delete_group_async<'py>(
        &self,
        py: Python<'py>,
        id: String,
    ) -> ::pyo3::PyResult<
        crate::codegen::py_future::PyFuture<'py, crate::codegen::py_future::NoResult>,
    > {
        let request = self.client.delete_group(id);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(crate::codegen::py_future::NoResult)
        })
    }

    /// Run operations on users and groups in one request. Each operation has its own status. Only the
    /// metastore admin can.
    ///
    /// Args:
    ///     schemas: SCIM schema URIs of the request: `urn:ietf:params:scim:api:messages:2.0:BulkRequest`.
    ///     fail_on_errors: Stop after this many operations failed. Absent: run every operation.
    ///     operations: The operations, in order. At most 100.
    ///
    /// Returns:
    ///     The answer to a SCIM bulk request: one result per operation that ran, in order.
    #[pyo3(signature = (*, schemas = None, fail_on_errors = None, operations = None))]
    pub fn scim_bulk(
        &self,
        py: Python,
        schemas: Option<Vec<String>>,
        fail_on_errors: Option<i32>,
        operations: ::core::option::Option<::std::vec::Vec<PyScimBulkOp>>,
    ) -> PyUnityCatalogResult<PyScimBulkResponse> {
        let request = self
            .client
            .scim_bulk()
            .with_schemas(schemas.unwrap_or_default())
            .with_fail_on_errors(fail_on_errors)
            .with_operations(
                operations
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimBulkResponse::from(result))
        })
    }

    /// Like `scim_bulk`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, schemas = None, fail_on_errors = None, operations = None))]
    pub fn scim_bulk_async<'py>(
        &self,
        py: Python<'py>,
        schemas: Option<Vec<String>>,
        fail_on_errors: Option<i32>,
        operations: ::core::option::Option<::std::vec::Vec<PyScimBulkOp>>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimBulkResponse>> {
        let request = self
            .client
            .scim_bulk()
            .with_schemas(schemas.unwrap_or_default())
            .with_fail_on_errors(fail_on_errors)
            .with_operations(
                operations
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
            Ok(PyScimBulkResponse::from(result))
        })
    }

    /// List the SCIM schemas of the server: User and Group. Any signed-in caller can.
    ///
    /// Returns:
    ///     The SCIM schemas of the server, User and Group, as a SCIM list (RFC 7644, section 4).
    pub fn list_scim_schemas(&self, py: Python) -> PyUnityCatalogResult<PyListScimSchemasResponse> {
        let request = self.client.list_scim_schemas();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyListScimSchemasResponse::from(result))
        })
    }

    /// Like `list_scim_schemas`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn list_scim_schemas_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyListScimSchemasResponse>> {
        let request = self.client.list_scim_schemas();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyListScimSchemasResponse::from(result))
        })
    }

    /// List the SCIM resource types of the server: User and Group. Any signed-in caller can.
    ///
    /// Returns:
    ///     The SCIM resource types of the server, User and Group, as a SCIM list (RFC 7644, section 4).
    pub fn list_scim_resource_types(
        &self,
        py: Python,
    ) -> PyUnityCatalogResult<PyListScimResourceTypesResponse> {
        let request = self.client.list_scim_resource_types();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyListScimResourceTypesResponse::from(result))
        })
    }

    /// Like `list_scim_resource_types`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn list_scim_resource_types_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyListScimResourceTypesResponse>>
    {
        let request = self.client.list_scim_resource_types();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyListScimResourceTypesResponse::from(result))
        })
    }

    /// Get the SCIM features that the server supports. Any signed-in caller can.
    ///
    /// Returns:
    ///     The SCIM features that the server supports (RFC 7643, section 5).
    pub fn get_service_provider_config(
        &self,
        py: Python,
    ) -> PyUnityCatalogResult<PyScimServiceProviderConfig> {
        let request = self.client.get_service_provider_config();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyScimServiceProviderConfig::from(result))
        })
    }

    /// Like `get_service_provider_config`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_service_provider_config_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyScimServiceProviderConfig>>
    {
        let request = self.client.get_service_provider_config();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyScimServiceProviderConfig::from(result))
        })
    }

    /// List the shares.
    ///
    /// Args:
    ///     max_results: The maximum number of shares to return.
    ///
    /// Returns:
    ///     The shares.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_shares(
        &self,
        py: Python,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyShareInfo>> {
        let request = self.client.list_shares().with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyShareInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_shares`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_shares_async<'py>(
        &self,
        py: Python<'py>,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyShareInfo>>>
    {
        let request = self.client.list_shares().with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyShareInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a share. It holds no tables: add them with `UpdateShare`.
    ///
    /// Args:
    ///     name: Name of the share.
    ///     comment: User-provided free-form text description.
    ///
    /// Returns:
    ///     A share: a named set of tables that Delta Sharing recipients can read.
    #[pyo3(signature = (name, *, comment = None))]
    pub fn create_share(
        &self,
        py: Python,
        name: String,
        comment: Option<String>,
    ) -> PyUnityCatalogResult<PyShareInfo> {
        let request = self.client.create_share(name).with_comment(comment);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyShareInfo::from(result))
        })
    }

    /// Like `create_share`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (name, *, comment = None))]
    pub fn create_share_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        comment: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyShareInfo>> {
        let request = self.client.create_share(name).with_comment(comment);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyShareInfo::from(result))
        })
    }

    /// List the shares that the recipient can read.
    ///
    /// Returns:
    ///     The requested resource
    pub fn sharing_list_shares(
        &self,
        py: Python,
    ) -> PyUnityCatalogResult<PySharingListSharesResponse> {
        let request = self.client.sharing_list_shares();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PySharingListSharesResponse::from(result))
        })
    }

    /// Like `sharing_list_shares`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn sharing_list_shares_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySharingListSharesResponse>>
    {
        let request = self.client.sharing_list_shares();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySharingListSharesResponse::from(result))
        })
    }

    /// List the schemas of a share.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(signature = (share))]
    pub fn sharing_list_schemas(
        &self,
        py: Python,
        share: String,
    ) -> PyUnityCatalogResult<PySharingListSchemasResponse> {
        let request = self.client.sharing_list_schemas(share);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PySharingListSchemasResponse::from(result))
        })
    }

    /// Like `sharing_list_schemas`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (share))]
    pub fn sharing_list_schemas_async<'py>(
        &self,
        py: Python<'py>,
        share: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySharingListSchemasResponse>>
    {
        let request = self.client.sharing_list_schemas(share);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySharingListSchemasResponse::from(result))
        })
    }

    /// List the tables of a schema of a share.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(signature = (share, schema))]
    pub fn sharing_list_tables(
        &self,
        py: Python,
        share: String,
        schema: String,
    ) -> PyUnityCatalogResult<PySharingListTablesResponse> {
        let request = self.client.sharing_list_tables(share, schema);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PySharingListTablesResponse::from(result))
        })
    }

    /// Like `sharing_list_tables`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (share, schema))]
    pub fn sharing_list_tables_async<'py>(
        &self,
        py: Python<'py>,
        share: String,
        schema: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySharingListTablesResponse>>
    {
        let request = self.client.sharing_list_tables(share, schema);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySharingListTablesResponse::from(result))
        })
    }

    /// List the tables of all schemas of a share.
    ///
    /// Returns:
    ///     The requested resource
    #[pyo3(signature = (share))]
    pub fn sharing_list_all_tables(
        &self,
        py: Python,
        share: String,
    ) -> PyUnityCatalogResult<PySharingListTablesResponse> {
        let request = self.client.sharing_list_all_tables(share);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PySharingListTablesResponse::from(result))
        })
    }

    /// Like `sharing_list_all_tables`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (share))]
    pub fn sharing_list_all_tables_async<'py>(
        &self,
        py: Python<'py>,
        share: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySharingListTablesResponse>>
    {
        let request = self.client.sharing_list_all_tables(share);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySharingListTablesResponse::from(result))
        })
    }

    /// Get the version of a table.
    ///
    /// Returns:
    ///     The version of a table. The response has no body.
    #[pyo3(signature = (share, schema, table))]
    pub fn sharing_get_table_version(
        &self,
        py: Python,
        share: String,
        schema: String,
        table: String,
    ) -> PyUnityCatalogResult<PySharingGetTableVersionResponse> {
        let request = self.client.sharing_get_table_version(share, schema, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PySharingGetTableVersionResponse::from(result))
        })
    }

    /// Like `sharing_get_table_version`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (share, schema, table))]
    pub fn sharing_get_table_version_async<'py>(
        &self,
        py: Python<'py>,
        share: String,
        schema: String,
        table: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PySharingGetTableVersionResponse>>
    {
        let request = self.client.sharing_get_table_version(share, schema, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PySharingGetTableVersionResponse::from(result))
        })
    }

    /// Get the protocol and the metadata of a table. The response also has the version of the table in the
    /// `Delta-Table-Version` header (always 0).
    ///
    /// Returns:
    ///     One line of the NDJSON answer of `SharingGetTableMetadata` and `SharingQueryTable`: an
    ///     object
    ///     with one key. The first line holds `protocol`, the second `metaData`.
    #[pyo3(signature = (share, schema, table))]
    pub fn sharing_get_table_metadata(
        &self,
        py: Python,
        share: String,
        schema: String,
        table: String,
    ) -> PyUnityCatalogResult<Vec<PySharingTableLine>> {
        let request = self.client.sharing_get_table_metadata(share, schema, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PySharingTableLine::from)
                    .collect::<Vec<_>>(),
            )
        })
    }

    /// Like `sharing_get_table_metadata`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (share, schema, table))]
    pub fn sharing_get_table_metadata_async<'py>(
        &self,
        py: Python<'py>,
        share: String,
        schema: String,
        table: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, Vec<PySharingTableLine>>> {
        let request = self.client.sharing_get_table_metadata(share, schema, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PySharingTableLine::from)
                .collect::<Vec<_>>())
        })
    }

    /// Read a table. The server does not read the Delta log: the answer is that of
    /// `SharingGetTableMetadata`, with no data files. The server ignores the request body.
    ///
    /// Returns:
    ///     One line of the NDJSON answer of `SharingGetTableMetadata` and `SharingQueryTable`: an
    ///     object
    ///     with one key. The first line holds `protocol`, the second `metaData`.
    #[pyo3(signature = (share, schema, table))]
    pub fn sharing_query_table(
        &self,
        py: Python,
        share: String,
        schema: String,
        table: String,
    ) -> PyUnityCatalogResult<Vec<PySharingTableLine>> {
        let request = self.client.sharing_query_table(share, schema, table);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PySharingTableLine::from)
                    .collect::<Vec<_>>(),
            )
        })
    }

    /// Like `sharing_query_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (share, schema, table))]
    pub fn sharing_query_table_async<'py>(
        &self,
        py: Python<'py>,
        share: String,
        schema: String,
        table: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, Vec<PySharingTableLine>>> {
        let request = self.client.sharing_query_table(share, schema, table);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PySharingTableLine::from)
                .collect::<Vec<_>>())
        })
    }

    /// Create a staging table: reserve a table id and a managed storage location for a MANAGED table.
    ///
    /// Args:
    ///     name: Name of the table, relative to the parent schema.
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///
    /// Returns:
    ///     A staging table: a table id and managed storage location reserved for a MANAGED table that a
    ///     client is about to create.
    #[pyo3(signature = (name, catalog_name, schema_name))]
    pub fn create_staging_table(
        &self,
        py: Python,
        name: String,
        catalog_name: String,
        schema_name: String,
    ) -> PyUnityCatalogResult<PyStagingTableInfo> {
        let request = self
            .client
            .create_staging_table(name, catalog_name, schema_name);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyStagingTableInfo::from(result))
        })
    }

    /// Like `create_staging_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (name, catalog_name, schema_name))]
    pub fn create_staging_table_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyStagingTableInfo>> {
        let request = self
            .client
            .create_staging_table(name, catalog_name, schema_name);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyStagingTableInfo::from(result))
        })
    }

    /// List the tables of a schema that the caller can see.
    ///
    /// Args:
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     max_results: The maximum number of tables to return.
    ///
    /// Returns:
    ///     The tables.
    #[pyo3(signature = (catalog_name, schema_name, *, max_results = None))]
    pub fn list_tables(
        &self,
        py: Python,
        catalog_name: String,
        schema_name: String,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyTableInfo>> {
        let request = self
            .client
            .list_tables(catalog_name, schema_name)
            .with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyTableInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_tables`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog_name, schema_name, *, max_results = None))]
    pub fn list_tables_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyTableInfo>>>
    {
        let request = self
            .client
            .list_tables(catalog_name, schema_name)
            .with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyTableInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a table.
    ///
    /// Args:
    ///     name: Name of the table, relative to the parent schema.
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     table_type: Type of the table.
    ///     columns: The columns of the table. Views and metric views need at least one.
    ///     data_source_format: File format of the data of the table. MANAGED tables are DELTA; ICEBERG
    ///                         tables come only from the Iceberg REST catalog.
    ///     storage_location: Storage location URL of the table. Required for EXTERNAL tables; for MANAGED
    ///                       tables, the `staging_location` of a staging table that the caller created.
    ///     comment: User-provided free-form text description.
    ///     properties: Key-value properties of the table.
    ///     view_definition: Definition of a view-like table: SQL for views, YAML for metric views.
    ///     view_dependencies: The tables and functions that a view-like table reads. Required for metric
    ///                        views.
    ///
    /// Returns:
    ///     A table: data in the third layer of the three-level namespace, below a schema.
    #[pyo3(
        signature = (
            name,
            catalog_name,
            schema_name,
            table_type,
            *,
            columns = None,
            data_source_format = None,
            storage_location = None,
            comment = None,
            properties = None,
            view_definition = None,
            view_dependencies = None
        )
    )]
    pub fn create_table(
        &self,
        py: Python,
        name: String,
        catalog_name: String,
        schema_name: String,
        table_type: PyTableType,
        columns: ::core::option::Option<::std::vec::Vec<PyColumnInfo>>,
        data_source_format: ::core::option::Option<PyDataSourceFormat>,
        storage_location: Option<String>,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        view_definition: Option<String>,
        view_dependencies: ::core::option::Option<PyDependencyList>,
    ) -> PyUnityCatalogResult<PyTableInfo> {
        let request = self
            .client
            .create_table(name, catalog_name, schema_name, table_type.into())
            .with_columns(
                columns
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_data_source_format(data_source_format.map(::core::convert::Into::into))
            .with_storage_location(storage_location)
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_view_definition(view_definition)
            .with_view_dependencies(view_dependencies.map(::core::convert::Into::into));
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTableInfo::from(result))
        })
    }

    /// Like `create_table`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            name,
            catalog_name,
            schema_name,
            table_type,
            *,
            columns = None,
            data_source_format = None,
            storage_location = None,
            comment = None,
            properties = None,
            view_definition = None,
            view_dependencies = None
        )
    )]
    pub fn create_table_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        catalog_name: String,
        schema_name: String,
        table_type: PyTableType,
        columns: ::core::option::Option<::std::vec::Vec<PyColumnInfo>>,
        data_source_format: ::core::option::Option<PyDataSourceFormat>,
        storage_location: Option<String>,
        comment: Option<String>,
        properties: Option<HashMap<String, String>>,
        view_definition: Option<String>,
        view_dependencies: ::core::option::Option<PyDependencyList>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTableInfo>> {
        let request = self
            .client
            .create_table(name, catalog_name, schema_name, table_type.into())
            .with_columns(
                columns
                    .unwrap_or_default()
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<_>>(),
            )
            .with_data_source_format(data_source_format.map(::core::convert::Into::into))
            .with_storage_location(storage_location)
            .with_comment(comment)
            .with_properties(properties.unwrap_or_default())
            .with_view_definition(view_definition)
            .with_view_dependencies(view_dependencies.map(::core::convert::Into::into));
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyTableInfo::from(result))
        })
    }

    /// Generate temporary credentials for the storage of a table.
    ///
    /// Args:
    ///     table_id: Unique identifier of the table.
    ///     operation: The operation on the table.
    ///
    /// Returns:
    ///     Temporary credentials for the cloud of a storage location. Empty for a local `file:`
    ///     location.
    #[pyo3(signature = (table_id, operation))]
    pub fn generate_temporary_table_credentials(
        &self,
        py: Python,
        table_id: String,
        operation: PyTableOperation,
    ) -> PyUnityCatalogResult<PyTemporaryCredentials> {
        let request = self
            .client
            .generate_temporary_table_credentials(table_id, operation.into());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTemporaryCredentials::from(result))
        })
    }

    /// Like `generate_temporary_table_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (table_id, operation))]
    pub fn generate_temporary_table_credentials_async<'py>(
        &self,
        py: Python<'py>,
        table_id: String,
        operation: PyTableOperation,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTemporaryCredentials>> {
        let request = self
            .client
            .generate_temporary_table_credentials(table_id, operation.into());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyTemporaryCredentials::from(result))
        })
    }

    /// Generate temporary credentials for the storage of a volume.
    ///
    /// Args:
    ///     volume_id: Unique identifier of the volume.
    ///     operation: The operation on the volume.
    ///
    /// Returns:
    ///     Temporary credentials for the cloud of a storage location. Empty for a local `file:`
    ///     location.
    #[pyo3(signature = (volume_id, operation))]
    pub fn generate_temporary_volume_credentials(
        &self,
        py: Python,
        volume_id: String,
        operation: PyVolumeOperation,
    ) -> PyUnityCatalogResult<PyTemporaryCredentials> {
        let request = self
            .client
            .generate_temporary_volume_credentials(volume_id, operation.into());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTemporaryCredentials::from(result))
        })
    }

    /// Like `generate_temporary_volume_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (volume_id, operation))]
    pub fn generate_temporary_volume_credentials_async<'py>(
        &self,
        py: Python<'py>,
        volume_id: String,
        operation: PyVolumeOperation,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTemporaryCredentials>> {
        let request = self
            .client
            .generate_temporary_volume_credentials(volume_id, operation.into());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyTemporaryCredentials::from(result))
        })
    }

    /// Generate temporary credentials for a storage path.
    ///
    /// Args:
    ///     url: URL of the storage path.
    ///     operation: The operation on the path.
    ///
    /// Returns:
    ///     Temporary credentials for the cloud of a storage location. Empty for a local `file:`
    ///     location.
    #[pyo3(signature = (url, operation))]
    pub fn generate_temporary_path_credentials(
        &self,
        py: Python,
        url: String,
        operation: PyPathOperation,
    ) -> PyUnityCatalogResult<PyTemporaryCredentials> {
        let request = self
            .client
            .generate_temporary_path_credentials(url, operation.into());
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTemporaryCredentials::from(result))
        })
    }

    /// Like `generate_temporary_path_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (url, operation))]
    pub fn generate_temporary_path_credentials_async<'py>(
        &self,
        py: Python<'py>,
        url: String,
        operation: PyPathOperation,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTemporaryCredentials>> {
        let request = self
            .client
            .generate_temporary_path_credentials(url, operation.into());
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyTemporaryCredentials::from(result))
        })
    }

    /// Generate temporary credentials for the storage of a model version.
    ///
    /// Args:
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     model_name: Name of the registered model, relative to its schema.
    ///     version: Number of the model version.
    ///     operation: The operation on the model version.
    ///
    /// Returns:
    ///     Temporary credentials for the cloud of a storage location. Empty for a local `file:`
    ///     location.
    #[pyo3(signature = (catalog_name, schema_name, model_name, version, operation))]
    pub fn generate_temporary_model_version_credentials(
        &self,
        py: Python,
        catalog_name: String,
        schema_name: String,
        model_name: String,
        version: i64,
        operation: PyModelVersionOperation,
    ) -> PyUnityCatalogResult<PyTemporaryCredentials> {
        let request = self.client.generate_temporary_model_version_credentials(
            catalog_name,
            schema_name,
            model_name,
            version,
            operation.into(),
        );
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyTemporaryCredentials::from(result))
        })
    }

    /// Like `generate_temporary_model_version_credentials`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog_name, schema_name, model_name, version, operation))]
    pub fn generate_temporary_model_version_credentials_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
        model_name: String,
        version: i64,
        operation: PyModelVersionOperation,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyTemporaryCredentials>> {
        let request = self.client.generate_temporary_model_version_credentials(
            catalog_name,
            schema_name,
            model_name,
            version,
            operation.into(),
        );
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyTemporaryCredentials::from(result))
        })
    }

    /// List the personal access tokens of the caller.
    ///
    /// Args:
    ///     max_results: The maximum number of tokens to return.
    ///
    /// Returns:
    ///     The tokens.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_tokens(
        &self,
        py: Python,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyPatInfo>> {
        let request = self.client.list_tokens().with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyPatInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_tokens`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (*, max_results = None))]
    pub fn list_tokens_async<'py>(
        &self,
        py: Python<'py>,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyPatInfo>>>
    {
        let request = self.client.list_tokens().with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyPatInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a personal access token for the caller.
    ///
    /// Args:
    ///     name: Name of the token.
    ///     expires_at: Time at which the token expires, in epoch milliseconds. Without it the token does
    ///                 not expire.
    ///
    /// Returns:
    ///     A personal access token of a user. The server keeps a hash of the token only: the token
    ///     itself
    ///     is in the response of `CreateToken` and nowhere else.
    #[pyo3(signature = (name, *, expires_at = None))]
    pub fn create_token(
        &self,
        py: Python,
        name: String,
        expires_at: Option<i64>,
    ) -> PyUnityCatalogResult<PyPatInfo> {
        let request = self.client.create_token(name).with_expires_at(expires_at);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyPatInfo::from(result))
        })
    }

    /// Like `create_token`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (name, *, expires_at = None))]
    pub fn create_token_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        expires_at: Option<i64>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyPatInfo>> {
        let request = self.client.create_token(name).with_expires_at(expires_at);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyPatInfo::from(result))
        })
    }

    /// List the volumes of a schema that the caller can see.
    ///
    /// Args:
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     max_results: The maximum number of volumes to return.
    ///
    /// Returns:
    ///     The volumes.
    #[pyo3(signature = (catalog_name, schema_name, *, max_results = None))]
    pub fn list_volumes(
        &self,
        py: Python,
        catalog_name: String,
        schema_name: String,
        max_results: Option<i32>,
    ) -> PyUnityCatalogResult<::std::vec::Vec<PyVolumeInfo>> {
        let request = self
            .client
            .list_volumes(catalog_name, schema_name)
            .with_max_results(max_results);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            })?;
            Ok::<_, PyUnityCatalogError>(
                result
                    .into_iter()
                    .map(PyVolumeInfo::from)
                    .collect::<::std::vec::Vec<_>>(),
            )
        })
    }

    /// Like `list_volumes`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(signature = (catalog_name, schema_name, *, max_results = None))]
    pub fn list_volumes_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
        max_results: Option<i32>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, ::std::vec::Vec<PyVolumeInfo>>>
    {
        let request = self
            .client
            .list_volumes(catalog_name, schema_name)
            .with_max_results(max_results);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = async move {
                request
                    .into_stream()
                    .try_collect::<::std::vec::Vec<_>>()
                    .await
            }
            .await
            .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(result
                .into_iter()
                .map(PyVolumeInfo::from)
                .collect::<::std::vec::Vec<_>>())
        })
    }

    /// Create a volume.
    ///
    /// Args:
    ///     catalog_name: Name of the parent catalog.
    ///     schema_name: Name of the parent schema.
    ///     name: Name of the volume, relative to the parent schema.
    ///     volume_type: Type of the volume.
    ///     comment: User-provided free-form text description.
    ///     storage_location: Storage location URL of the volume. Required for EXTERNAL volumes; not allowed
    ///                       for MANAGED volumes.
    ///
    /// Returns:
    ///     A volume: files in the third layer of the three-level namespace, below a schema.
    #[pyo3(
        signature = (
            catalog_name,
            schema_name,
            name,
            volume_type,
            *,
            comment = None,
            storage_location = None
        )
    )]
    pub fn create_volume(
        &self,
        py: Python,
        catalog_name: String,
        schema_name: String,
        name: String,
        volume_type: PyVolumeType,
        comment: Option<String>,
        storage_location: Option<String>,
    ) -> PyUnityCatalogResult<PyVolumeInfo> {
        let request = self
            .client
            .create_volume(catalog_name, schema_name, name, volume_type.into())
            .with_comment(comment)
            .with_storage_location(storage_location);
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyVolumeInfo::from(result))
        })
    }

    /// Like `create_volume`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            catalog_name,
            schema_name,
            name,
            volume_type,
            *,
            comment = None,
            storage_location = None
        )
    )]
    pub fn create_volume_async<'py>(
        &self,
        py: Python<'py>,
        catalog_name: String,
        schema_name: String,
        name: String,
        volume_type: PyVolumeType,
        comment: Option<String>,
        storage_location: Option<String>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyVolumeInfo>> {
        let request = self
            .client
            .create_volume(catalog_name, schema_name, name, volume_type.into())
            .with_comment(comment)
            .with_storage_location(storage_location);
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyVolumeInfo::from(result))
        })
    }

    /// The requests on one `CatalogInfo`. It sends no request itself.
    pub fn catalog(&self, catalog_name: String) -> PyCatalogClient {
        PyCatalogClient {
            client: self.client.catalog(catalog_name),
        }
    }

    /// The requests on one `CredentialInfo`. It sends no request itself.
    pub fn credential(&self, credential_name: String) -> PyCredentialClient {
        PyCredentialClient {
            client: self.client.credential(credential_name),
        }
    }

    /// The requests on one `ExternalLocationInfo`. It sends no request itself.
    pub fn external_location(&self, external_location_name: String) -> PyExternalLocationClient {
        PyExternalLocationClient {
            client: self.client.external_location(external_location_name),
        }
    }

    /// The requests on one `FunctionInfo`. It sends no request itself.
    pub fn function(&self, function_name: String) -> PyFunctionClient {
        PyFunctionClient {
            client: self.client.function(function_name),
        }
    }

    /// The requests on one `RecipientInfo`. It sends no request itself.
    pub fn recipient(&self, recipient_name: String) -> PyRecipientClient {
        PyRecipientClient {
            client: self.client.recipient(recipient_name),
        }
    }

    /// The requests on one `RegisteredModelInfo`. It sends no request itself.
    pub fn registered_model(&self, registered_model_name: String) -> PyRegisteredModelClient {
        PyRegisteredModelClient {
            client: self.client.registered_model(registered_model_name),
        }
    }

    /// The requests on one `SchemaInfo`. It sends no request itself.
    pub fn schema(&self, schema_name: String) -> PySchemaClient {
        PySchemaClient {
            client: self.client.schema(schema_name),
        }
    }

    /// The requests on one `ShareInfo`. It sends no request itself.
    pub fn share(&self, share_name: String) -> PyShareClient {
        PyShareClient {
            client: self.client.share(share_name),
        }
    }

    /// The requests on one `TableInfo`. It sends no request itself.
    pub fn table(&self, table_name: String) -> PyTableClient {
        PyTableClient {
            client: self.client.table(table_name),
        }
    }

    /// The requests on one `PatInfo`. It sends no request itself.
    pub fn token(&self, token_name: String) -> PyTokenClient {
        PyTokenClient {
            client: self.client.token(token_name),
        }
    }

    /// The requests on one `VolumeInfo`. It sends no request itself.
    pub fn volume(&self, volume_name: String) -> PyVolumeClient {
        PyVolumeClient {
            client: self.client.volume(volume_name),
        }
    }
}

/// The extension module, as Python imports it.
pub const CORE_MODULE: &str = "unitycatalog._core";

/// Register the submodule `<CORE_MODULE>.<name>` with its classes. It also goes in
/// `sys.modules`, so `import <CORE_MODULE>.<name>` finds it.
///
/// Usage:
/// ```ignore
/// register_submodule!(module, "tables", [PyColumnInfo, PyTableInfo])?;
/// ```
macro_rules! register_submodule {
    ($parent:expr, $mod_name:expr, [$($type:ty),* $(,)?]) => {{
        let py = $parent.py();
        let child = ::pyo3::types::PyModule::new(py, $mod_name)?;
        $(
            child.add_class::<$type>()?;
        )*
        $parent.add_submodule(&child)?;
        py.import("sys")?
            .getattr("modules")?
            .set_item(format!("{}.{}", CORE_MODULE, $mod_name), child)
    }};
}

/// Add the generated classes to the extension module `module`.
pub fn register(module: &::pyo3::Bound<'_, ::pyo3::types::PyModule>) -> ::pyo3::PyResult<()> {
    crate::codegen::runtime::init_async_runtime();
    crate::codegen::error::add_exceptions(module)?;
    module.add_class::<PyClient>()?;
    register_submodule!(module, "admin", [PyBackupResponse, PyRestoreResponse])?;
    register_submodule!(module, "audit", [PyAuditEvent, PyListAuditEventsResponse])?;
    register_submodule!(
        module,
        "auth",
        [PyAuthMode, PyOAuthTokenExchangeInfo, PyTokenResponse]
    )?;
    register_submodule!(
        module,
        "catalogs",
        [PyCatalogInfo, PyListCatalogsResponse, PyCatalogClient]
    )?;
    register_submodule!(
        module,
        "credentials",
        [
            PyAwsCredentials,
            PyAwsIamRoleRequest,
            PyAwsIamRoleResponse,
            PyAzureUserDelegationSAS,
            PyCredentialInfo,
            PyGcpOauthToken,
            PyListCredentialsResponse,
            PyTemporaryCredentials,
            PyCredentialClient
        ]
    )?;
    register_submodule!(
        module,
        "delta",
        [
            PyColumnInfos,
            PyDeltaApiCommit,
            PyDeltaCatalogConfig,
            PyDeltaClusteringDomainMetadata,
            PyDeltaCommitInfo,
            PyDeltaCommitMetadataProperties,
            PyDeltaCommitReport,
            PyDeltaCredentialsResponse,
            PyDeltaDomainMetadataUpdates,
            PyDeltaFileSizeHistogram,
            PyDeltaGetCommitsResponse,
            PyDeltaLoadTableResponse,
            PyDeltaMetadata,
            PyDeltaMetricsReport,
            PyDeltaProtocol,
            PyDeltaRowTrackingDomainMetadata,
            PyDeltaStagingTableResponse,
            PyDeltaStorageCredential,
            PyDeltaStorageCredentialConfig,
            PyDeltaStructField,
            PyDeltaStructType,
            PyDeltaSuggestedProtocol,
            PyDeltaTableMetadata,
            PyDeltaTableRequirement,
            PyDeltaTableUpdate,
            PyDeltaUniform,
            PyDeltaUniformIceberg,
            PyDeltaUniformMetadata,
            PyDeltaUniformMetadataIceberg
        ]
    )?;
    register_submodule!(
        module,
        "external_locations",
        [
            PyExternalLocationInfo,
            PyListExternalLocationsResponse,
            PyExternalLocationClient
        ]
    )?;
    register_submodule!(
        module,
        "functions",
        [
            PyCreateFunction,
            PyFunctionInfo,
            PyFunctionParameterInfo,
            PyFunctionParameterInfos,
            PyListFunctionsResponse,
            PyFunctionClient
        ]
    )?;
    register_submodule!(module, "health", [PyHealthResponse, PyVersionResponse])?;
    register_submodule!(
        module,
        "iceberg",
        [
            PyIcebergBlobMetadata,
            PyIcebergConfigResponse,
            PyIcebergEncryptedKey,
            PyIcebergListNamespacesResponse,
            PyIcebergListTablesResponse,
            PyIcebergLoadCredentialsResponse,
            PyIcebergLoadTableResponse,
            PyIcebergMetadataLogEntry,
            PyIcebergNamespaceResponse,
            PyIcebergPartitionField,
            PyIcebergPartitionSpec,
            PyIcebergPartitionStatisticsFile,
            PyIcebergSchema,
            PyIcebergSnapshot,
            PyIcebergSnapshotLogEntry,
            PyIcebergSnapshotReference,
            PyIcebergSortField,
            PyIcebergSortOrder,
            PyIcebergStatisticsFile,
            PyIcebergStorageCredential,
            PyIcebergStructField,
            PyIcebergTableIdentifier,
            PyIcebergTableMetadata,
            PyIcebergTableRequirement,
            PyIcebergTableUpdate,
            PyIcebergUpdateNamespacePropertiesResponse
        ]
    )?;
    register_submodule!(module, "metastore", [PyGetMetastoreSummaryResponse])?;
    register_submodule!(
        module,
        "models",
        [
            PyListModelVersionsResponse,
            PyListRegisteredModelsResponse,
            PyModelVersionInfo,
            PyRegisteredModelInfo,
            PyRegisteredModelClient
        ]
    )?;
    register_submodule!(
        module,
        "permissions",
        [
            PyPermissionsChange,
            PyPermissionsList,
            PyPrivilegeAssignment
        ]
    )?;
    register_submodule!(
        module,
        "recipients",
        [PyListRecipientsResponse, PyRecipientInfo, PyRecipientClient]
    )?;
    register_submodule!(
        module,
        "schemas",
        [PyListSchemasResponse, PySchemaInfo, PySchemaClient]
    )?;
    register_submodule!(
        module,
        "scim",
        [
            PyListScimGroupsResponse,
            PyListScimResourceTypesResponse,
            PyListScimSchemasResponse,
            PyListScimUsersResponse,
            PyScimAuthenticationScheme,
            PyScimBulkConfig,
            PyScimBulkOp,
            PyScimBulkOpResult,
            PyScimBulkResponse,
            PyScimEmail,
            PyScimFilterConfig,
            PyScimGroup,
            PyScimGroupMember,
            PyScimMeta,
            PyScimPatchOperation,
            PyScimPhoto,
            PyScimResourceType,
            PyScimRole,
            PyScimSchema,
            PyScimSchemaAttribute,
            PyScimServiceProviderConfig,
            PyScimSupported,
            PyScimUser
        ]
    )?;
    register_submodule!(
        module,
        "shares",
        [
            PyListSharesResponse,
            PyShareInfo,
            PySharedDataObject,
            PyShareClient
        ]
    )?;
    register_submodule!(
        module,
        "sharing",
        [
            PyRecipientProfile,
            PySharingFormat,
            PySharingGetTableVersionResponse,
            PySharingListSchemasResponse,
            PySharingListSharesResponse,
            PySharingListTablesResponse,
            PySharingMetadata,
            PySharingProtocol,
            PySharingSchema,
            PySharingShare,
            PySharingTable,
            PySharingTableLine
        ]
    )?;
    register_submodule!(
        module,
        "tables",
        [
            PyColumnInfo,
            PyDependency,
            PyDependencyList,
            PyFunctionDependency,
            PyListTablesResponse,
            PyStagingTableInfo,
            PyTableDependency,
            PyTableInfo,
            PyTableClient
        ]
    )?;
    register_submodule!(
        module,
        "tokens",
        [PyListPatsResponse, PyPatInfo, PyTokenClient]
    )?;
    register_submodule!(
        module,
        "volumes",
        [PyListVolumesResponseContent, PyVolumeInfo, PyVolumeClient]
    )?;
    Ok(())
}
