// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::shares::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct ShareServiceClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl ShareServiceClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the shares.
    pub async fn list_shares(&self, request: &ListSharesRequest) -> Result<ListSharesResponse> {
        let mut url = self.base_url.join("api/2.1/unity-catalog/shares")?;
        if let Some(ref value) = request.max_results {
            url.query_pairs_mut()
                .append_pair("max_results", &value.to_string());
        }
        if let Some(ref value) = request.page_token {
            url.query_pairs_mut()
                .append_pair("page_token", &value.to_string());
        }
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

    /// Create a share. It holds no tables: add them with `UpdateShare`.
    pub async fn create_share(&self, request: &CreateShare) -> Result<ShareInfo> {
        let url = self.base_url.join("api/2.1/unity-catalog/shares")?;
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

    /// Get a share.
    pub async fn get_share(&self, request: &GetShareRequest) -> Result<ShareInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/shares/{}",
            encode_path_value(&request.name, false)
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

    /// Change a share: its comment, its name and its tables.
    pub async fn update_share(&self, request: &UpdateShare) -> Result<ShareInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/shares/{}",
            encode_path_value(&request.name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.name = Default::default();
        let response = self.client.patch(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Delete a share.
    pub async fn delete_share(&self, request: &DeleteShareRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/shares/{}",
            encode_path_value(&request.name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }
}
