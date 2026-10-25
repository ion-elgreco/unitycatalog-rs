// @generated — do not edit by hand.
#![allow(unused_mut)]
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
type BoxFut<'a, T> = ::futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxFut<'a, T> = ::futures::future::LocalBoxFuture<'a, T>;
#[cfg(not(target_arch = "wasm32"))]
type BoxStr<'a, T> = ::futures::stream::BoxStream<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxStr<'a, T> = ::futures::stream::LocalBoxStream<'a, T>;
use super::super::stream_paginated;
use super::client::*;
use crate::codegen::error::Result;
use futures::{StreamExt, TryStreamExt};
use std::future::IntoFuture;
use uc_client::models::credentials::v1::*;

/// Builder for listing credentials
pub struct ListCredentialsBuilder {
    client: CredentialServiceClient,
    request: ListCredentialsRequest,
}

impl ListCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `CredentialServiceClient`.
    pub(crate) fn new(client: CredentialServiceClient) -> Self {
        let request = ListCredentialsRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of credentials to return.
    pub fn with_max_results(mut self, max_results: impl Into<Option<i32>>) -> Self {
        self.request.max_results = max_results.into();
        self
    }

    /// Opaque token of the page to return: the `next_page_token` of the page before.
    pub fn with_page_token(mut self, page_token: impl Into<Option<String>>) -> Self {
        self.request.page_token = page_token.into();
        self
    }

    /// Return only the credentials with this purpose.
    pub fn with_purpose(mut self, purpose: impl Into<Option<CredentialPurpose>>) -> Self {
        self.request.purpose = purpose.into().map(buffa::EnumValue::Known);
        self
    }

    /// Convert paginated request into stream of results
    pub fn into_stream(self) -> BoxStr<'static, Result<CredentialInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_credentials(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.credentials.truncate((*rem).max(0) as usize);
                    *rem -= res.credentials.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.credentials.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListCredentialsBuilder {
    type Output = Result<ListCredentialsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_credentials(&request).await })
    }
}

/// Builder for creating a credential
pub struct CreateCredentialBuilder {
    client: CredentialServiceClient,
    request: CreateCredentialRequest,
}

impl CreateCredentialBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `CredentialServiceClient`.
    pub(crate) fn new(client: CredentialServiceClient, name: impl Into<String>) -> Self {
        let request = CreateCredentialRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// The AWS IAM role of the credential. A credential has exactly one of `aws_iam_role`,
    /// `azure_service_principal`, `azure_managed_identity`, `gcp_service_account` and
    /// `gcp_service_account_key`.
    pub fn with_aws_iam_role(mut self, aws_iam_role: impl Into<Option<AwsIamRoleRequest>>) -> Self {
        self.request.aws_iam_role = {
            let aws_iam_role: ::core::option::Option<_> = aws_iam_role.into();
            buffa::MessageField::from(aws_iam_role)
        };
        self
    }

    /// What the credential is used for.
    pub fn with_purpose(mut self, purpose: impl Into<Option<CredentialPurpose>>) -> Self {
        self.request.purpose = purpose.into().map(buffa::EnumValue::Known);
        self
    }

    /// The Azure service principal of the credential.
    pub fn with_azure_service_principal(
        mut self,
        azure_service_principal: impl Into<Option<AzureServicePrincipalRequest>>,
    ) -> Self {
        self.request.azure_service_principal = {
            let azure_service_principal: ::core::option::Option<_> = azure_service_principal.into();
            buffa::MessageField::from(azure_service_principal)
        };
        self
    }

    /// The Azure managed identity of the credential.
    pub fn with_azure_managed_identity(
        mut self,
        azure_managed_identity: impl Into<Option<AzureManagedIdentity>>,
    ) -> Self {
        self.request.azure_managed_identity = {
            let azure_managed_identity: ::core::option::Option<_> = azure_managed_identity.into();
            buffa::MessageField::from(azure_managed_identity)
        };
        self
    }

    /// The GCP service account of the credential.
    pub fn with_gcp_service_account(
        mut self,
        gcp_service_account: impl Into<Option<GcpServiceAccount>>,
    ) -> Self {
        self.request.gcp_service_account = {
            let gcp_service_account: ::core::option::Option<_> = gcp_service_account.into();
            buffa::MessageField::from(gcp_service_account)
        };
        self
    }

    /// The GCP service account key of the credential.
    pub fn with_gcp_service_account_key(
        mut self,
        gcp_service_account_key: impl Into<Option<GcpServiceAccountKeyRequest>>,
    ) -> Self {
        self.request.gcp_service_account_key = {
            let gcp_service_account_key: ::core::option::Option<_> = gcp_service_account_key.into();
            buffa::MessageField::from(gcp_service_account_key)
        };
        self
    }
}

impl IntoFuture for CreateCredentialBuilder {
    type Output = Result<CredentialInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_credential(&request).await })
    }
}

/// Builder for getting a credential
pub struct GetCredentialBuilder {
    client: CredentialServiceClient,
    request: GetCredentialRequest,
}

impl GetCredentialBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `CredentialServiceClient`.
    pub(crate) fn new(client: CredentialServiceClient, name: impl Into<String>) -> Self {
        let request = GetCredentialRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetCredentialBuilder {
    type Output = Result<CredentialInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_credential(&request).await })
    }
}

/// Builder for updating a credential
pub struct UpdateCredentialBuilder {
    client: CredentialServiceClient,
    request: UpdateCredentialRequest,
}

impl UpdateCredentialBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `CredentialServiceClient`.
    pub(crate) fn new(client: CredentialServiceClient, name: impl Into<String>) -> Self {
        let request = UpdateCredentialRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// New AWS IAM role of the credential. The role gets a new external ID. Give at most one of
    /// `aws_iam_role`, `azure_service_principal`, `azure_managed_identity`, `gcp_service_account`
    /// and `gcp_service_account_key`: it replaces the identity of the credential.
    pub fn with_aws_iam_role(mut self, aws_iam_role: impl Into<Option<AwsIamRoleRequest>>) -> Self {
        self.request.aws_iam_role = {
            let aws_iam_role: ::core::option::Option<_> = aws_iam_role.into();
            buffa::MessageField::from(aws_iam_role)
        };
        self
    }

    /// New name of the credential.
    pub fn with_new_name(mut self, new_name: impl Into<Option<String>>) -> Self {
        self.request.new_name = new_name.into();
        self
    }

    /// New Azure service principal of the credential.
    pub fn with_azure_service_principal(
        mut self,
        azure_service_principal: impl Into<Option<AzureServicePrincipalRequest>>,
    ) -> Self {
        self.request.azure_service_principal = {
            let azure_service_principal: ::core::option::Option<_> = azure_service_principal.into();
            buffa::MessageField::from(azure_service_principal)
        };
        self
    }

    /// New Azure managed identity of the credential.
    pub fn with_azure_managed_identity(
        mut self,
        azure_managed_identity: impl Into<Option<AzureManagedIdentity>>,
    ) -> Self {
        self.request.azure_managed_identity = {
            let azure_managed_identity: ::core::option::Option<_> = azure_managed_identity.into();
            buffa::MessageField::from(azure_managed_identity)
        };
        self
    }

    /// New GCP service account of the credential.
    pub fn with_gcp_service_account(
        mut self,
        gcp_service_account: impl Into<Option<GcpServiceAccount>>,
    ) -> Self {
        self.request.gcp_service_account = {
            let gcp_service_account: ::core::option::Option<_> = gcp_service_account.into();
            buffa::MessageField::from(gcp_service_account)
        };
        self
    }

    /// New GCP service account key of the credential.
    pub fn with_gcp_service_account_key(
        mut self,
        gcp_service_account_key: impl Into<Option<GcpServiceAccountKeyRequest>>,
    ) -> Self {
        self.request.gcp_service_account_key = {
            let gcp_service_account_key: ::core::option::Option<_> = gcp_service_account_key.into();
            buffa::MessageField::from(gcp_service_account_key)
        };
        self
    }
}

impl IntoFuture for UpdateCredentialBuilder {
    type Output = Result<CredentialInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_credential(&request).await })
    }
}

/// Builder for deleting a credential
pub struct DeleteCredentialBuilder {
    client: CredentialServiceClient,
    request: DeleteCredentialRequest,
}

impl DeleteCredentialBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `CredentialServiceClient`.
    pub(crate) fn new(client: CredentialServiceClient, name: impl Into<String>) -> Self {
        let request = DeleteCredentialRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Delete the credential even when external locations use it.
    pub fn with_force(mut self, force: impl Into<Option<bool>>) -> Self {
        self.request.force = force.into();
        self
    }
}

impl IntoFuture for DeleteCredentialBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_credential(&request).await })
    }
}
