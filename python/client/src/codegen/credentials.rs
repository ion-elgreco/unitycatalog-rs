// @generated — do not edit by hand.
use crate::codegen::error::{PyUnityCatalogError, PyUnityCatalogResult};
use futures::TryStreamExt;
use pyo3::prelude::*;
use std::collections::HashMap;
use uc_client::CredentialClient;
use uc_client::models::credentials::v1::*;
use uc_client::models::*;

/// The requests on one `CredentialInfo`.
#[pyclass(name = "CredentialClient", module = "unitycatalog._core.credentials")]
pub struct PyCredentialClient {
    pub(crate) client: CredentialClient,
}

#[pymethods]
impl PyCredentialClient {
    /// Get a credential.
    ///
    /// Returns:
    ///     A credential that gives access to storage or to external services.
    pub fn get(&self, py: Python) -> PyUnityCatalogResult<PyCredentialInfo> {
        let request = self.client.get();
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyCredentialInfo::from(result))
        })
    }

    /// Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    pub fn get_async<'py>(
        &self,
        py: Python<'py>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyCredentialInfo>> {
        let request = self.client.get();
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyCredentialInfo::from(result))
        })
    }

    /// Change a credential.
    ///
    /// Args:
    ///     comment: User-provided free-form text description.
    ///     aws_iam_role: New AWS IAM role of the credential. The role gets a new external ID. Give at
    ///                   most one of `aws_iam_role`, `azure_service_principal`, `azure_managed_identity`,
    ///                   `gcp_service_account` and `gcp_service_account_key`: it replaces the identity of
    ///                   the credential.
    ///     new_name: New name of the credential.
    ///     azure_service_principal: New Azure service principal of the credential.
    ///     azure_managed_identity: New Azure managed identity of the credential.
    ///     gcp_service_account: New GCP service account of the credential.
    ///     gcp_service_account_key: New GCP service account key of the credential.
    ///
    /// Returns:
    ///     A credential that gives access to storage or to external services.
    #[pyo3(
        signature = (
            *,
            comment = None,
            aws_iam_role = None,
            new_name = None,
            azure_service_principal = None,
            azure_managed_identity = None,
            gcp_service_account = None,
            gcp_service_account_key = None
        )
    )]
    pub fn update(
        &self,
        py: Python,
        comment: Option<String>,
        aws_iam_role: ::core::option::Option<PyAwsIamRoleRequest>,
        new_name: Option<String>,
        azure_service_principal: ::core::option::Option<PyAzureServicePrincipalRequest>,
        azure_managed_identity: ::core::option::Option<PyAzureManagedIdentity>,
        gcp_service_account: ::core::option::Option<PyGcpServiceAccount>,
        gcp_service_account_key: ::core::option::Option<PyGcpServiceAccountKeyRequest>,
    ) -> PyUnityCatalogResult<PyCredentialInfo> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_aws_iam_role(aws_iam_role.map(::core::convert::Into::into))
            .with_new_name(new_name)
            .with_azure_service_principal(azure_service_principal.map(::core::convert::Into::into))
            .with_azure_managed_identity(azure_managed_identity.map(::core::convert::Into::into))
            .with_gcp_service_account(gcp_service_account.map(::core::convert::Into::into))
            .with_gcp_service_account_key(gcp_service_account_key.map(::core::convert::Into::into));
        let runtime = crate::codegen::runtime::rt();
        py.detach(|| {
            #[allow(clippy::let_unit_value)]
            let result = runtime.block_on(request.into_future())?;
            Ok::<_, PyUnityCatalogError>(PyCredentialInfo::from(result))
        })
    }

    /// Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
    #[pyo3(
        signature = (
            *,
            comment = None,
            aws_iam_role = None,
            new_name = None,
            azure_service_principal = None,
            azure_managed_identity = None,
            gcp_service_account = None,
            gcp_service_account_key = None
        )
    )]
    pub fn update_async<'py>(
        &self,
        py: Python<'py>,
        comment: Option<String>,
        aws_iam_role: ::core::option::Option<PyAwsIamRoleRequest>,
        new_name: Option<String>,
        azure_service_principal: ::core::option::Option<PyAzureServicePrincipalRequest>,
        azure_managed_identity: ::core::option::Option<PyAzureManagedIdentity>,
        gcp_service_account: ::core::option::Option<PyGcpServiceAccount>,
        gcp_service_account_key: ::core::option::Option<PyGcpServiceAccountKeyRequest>,
    ) -> ::pyo3::PyResult<crate::codegen::py_future::PyFuture<'py, PyCredentialInfo>> {
        let request = self
            .client
            .update()
            .with_comment(comment)
            .with_aws_iam_role(aws_iam_role.map(::core::convert::Into::into))
            .with_new_name(new_name)
            .with_azure_service_principal(azure_service_principal.map(::core::convert::Into::into))
            .with_azure_managed_identity(azure_managed_identity.map(::core::convert::Into::into))
            .with_gcp_service_account(gcp_service_account.map(::core::convert::Into::into))
            .with_gcp_service_account_key(gcp_service_account_key.map(::core::convert::Into::into));
        crate::codegen::py_future::PyFuture::new(py, async move {
            #[allow(clippy::let_unit_value, unused_variables)]
            let result = request
                .into_future()
                .await
                .map_err(|e| ::pyo3::PyErr::from(PyUnityCatalogError::from(e)))?;
            Ok(PyCredentialInfo::from(result))
        })
    }

    /// Delete a credential. A credential that external locations use needs `force`.
    ///
    /// Args:
    ///     force: Delete the credential even when external locations use it.
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

impl PyCredentialClient {
    pub fn new(client: CredentialClient) -> Self {
        Self { client }
    }
}
