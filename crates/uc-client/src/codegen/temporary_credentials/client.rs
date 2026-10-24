// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::credentials::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct TemporaryCredentialClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl TemporaryCredentialClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Generate temporary credentials for the storage of a table.
    pub async fn generate_temporary_table_credentials(
        &self,
        request: &GenerateTemporaryTableCredential,
    ) -> Result<TemporaryCredentials> {
        let url = self
            .base_url
            .join("api/2.1/unity-catalog/temporary-table-credentials")?;
        let response = self.client.post(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Generate temporary credentials for the storage of a volume.
    pub async fn generate_temporary_volume_credentials(
        &self,
        request: &GenerateTemporaryVolumeCredential,
    ) -> Result<TemporaryCredentials> {
        let url = self
            .base_url
            .join("api/2.1/unity-catalog/temporary-volume-credentials")?;
        let response = self.client.post(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Generate temporary credentials for a storage path.
    pub async fn generate_temporary_path_credentials(
        &self,
        request: &GenerateTemporaryPathCredential,
    ) -> Result<TemporaryCredentials> {
        let url = self
            .base_url
            .join("api/2.1/unity-catalog/temporary-path-credentials")?;
        let response = self.client.post(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Generate temporary credentials for the storage of a model version.
    pub async fn generate_temporary_model_version_credentials(
        &self,
        request: &GenerateTemporaryModelVersionCredential,
    ) -> Result<TemporaryCredentials> {
        let url = self
            .base_url
            .join("api/2.1/unity-catalog/temporary-model-version-credentials")?;
        let response = self.client.post(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }
}
