// @generated — do not edit by hand.
#![allow(unused_mut)]
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
type BoxFut<'a, T> = ::futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxFut<'a, T> = ::futures::future::LocalBoxFuture<'a, T>;
use super::client::*;
use crate::codegen::error::Result;
use std::future::IntoFuture;
use uc_client::models::credentials::v1::*;

/// Builder for temporary table credentials
pub struct GenerateTemporaryTableCredentialsBuilder {
    client: TemporaryCredentialClient,
    request: GenerateTemporaryTableCredential,
}

impl GenerateTemporaryTableCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TemporaryCredentialClient`.
    pub(crate) fn new(
        client: TemporaryCredentialClient,
        table_id: impl Into<String>,
        operation: TableOperation,
    ) -> Self {
        let request = GenerateTemporaryTableCredential {
            table_id: table_id.into(),
            operation: buffa::EnumValue::Known(operation),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GenerateTemporaryTableCredentialsBuilder {
    type Output = Result<TemporaryCredentials>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.generate_temporary_table_credentials(&request).await })
    }
}

/// Builder for temporary volume credentials
pub struct GenerateTemporaryVolumeCredentialsBuilder {
    client: TemporaryCredentialClient,
    request: GenerateTemporaryVolumeCredential,
}

impl GenerateTemporaryVolumeCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TemporaryCredentialClient`.
    pub(crate) fn new(
        client: TemporaryCredentialClient,
        volume_id: impl Into<String>,
        operation: VolumeOperation,
    ) -> Self {
        let request = GenerateTemporaryVolumeCredential {
            volume_id: volume_id.into(),
            operation: buffa::EnumValue::Known(operation),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GenerateTemporaryVolumeCredentialsBuilder {
    type Output = Result<TemporaryCredentials>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.generate_temporary_volume_credentials(&request).await })
    }
}

/// Builder for temporary path credentials
pub struct GenerateTemporaryPathCredentialsBuilder {
    client: TemporaryCredentialClient,
    request: GenerateTemporaryPathCredential,
}

impl GenerateTemporaryPathCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TemporaryCredentialClient`.
    pub(crate) fn new(
        client: TemporaryCredentialClient,
        url: impl Into<String>,
        operation: PathOperation,
    ) -> Self {
        let request = GenerateTemporaryPathCredential {
            url: url.into(),
            operation: buffa::EnumValue::Known(operation),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GenerateTemporaryPathCredentialsBuilder {
    type Output = Result<TemporaryCredentials>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.generate_temporary_path_credentials(&request).await })
    }
}

/// Builder for temporary model version credentials
pub struct GenerateTemporaryModelVersionCredentialsBuilder {
    client: TemporaryCredentialClient,
    request: GenerateTemporaryModelVersionCredential,
}

impl GenerateTemporaryModelVersionCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TemporaryCredentialClient`.
    pub(crate) fn new(
        client: TemporaryCredentialClient,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        model_name: impl Into<String>,
        version: i64,
        operation: ModelVersionOperation,
    ) -> Self {
        let request = GenerateTemporaryModelVersionCredential {
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            model_name: model_name.into(),
            version,
            operation: buffa::EnumValue::Known(operation),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GenerateTemporaryModelVersionCredentialsBuilder {
    type Output = Result<TemporaryCredentials>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move {
            client
                .generate_temporary_model_version_credentials(&request)
                .await
        })
    }
}
