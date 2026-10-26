// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::protection::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct ProtectionClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl ProtectionClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Get the protection of a securable.
    pub async fn get_protection(&self, request: &GetProtectionRequest) -> Result<Protection> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/protection/{}/{}",
            encode_path_value(&request.securable_type, false),
            encode_path_value(&request.full_name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Protect a securable, or remove its protection. Who can delete the securable can do this.
    pub async fn update_protection(&self, request: &UpdateProtection) -> Result<Protection> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/protection/{}/{}",
            encode_path_value(&request.securable_type, false),
            encode_path_value(&request.full_name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.securable_type = Default::default();
        body.full_name = Default::default();
        let response = self.client.put(url).json(&body).send().await?;
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
