// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::recipients::v1::*;
use uc_client::models::sharing::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct RecipientServiceClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl RecipientServiceClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the recipients.
    pub async fn list_recipients(
        &self,
        request: &ListRecipientsRequest,
    ) -> Result<ListRecipientsResponse> {
        let mut url = self.base_url.join("api/2.1/unity-catalog/recipients")?;
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

    /// Create a recipient. The response holds its bearer token, which no later response shows.
    pub async fn create_recipient(&self, request: &CreateRecipient) -> Result<RecipientInfo> {
        let url = self.base_url.join("api/2.1/unity-catalog/recipients")?;
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

    /// Get a recipient. The response does not hold the bearer token.
    pub async fn get_recipient(&self, request: &GetRecipientRequest) -> Result<RecipientInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/recipients/{}",
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

    /// Change a recipient: its comment and the shares it can read.
    pub async fn update_recipient(&self, request: &UpdateRecipient) -> Result<RecipientInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/recipients/{}",
            encode_path_value(&request.name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.patch(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Delete a recipient.
    pub async fn delete_recipient(&self, request: &DeleteRecipientRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/recipients/{}",
            encode_path_value(&request.name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Replace the bearer token of a recipient. The old token stops working. The response holds the
    /// new token.
    pub async fn rotate_recipient_token(
        &self,
        request: &RotateRecipientTokenRequest,
    ) -> Result<RecipientInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/recipients/{}/rotate-token",
            encode_path_value(&request.name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
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

    /// Replace the bearer token of a recipient and return a Delta Sharing profile with the new token.
    /// The old token stops working.
    pub async fn create_recipient_profile(
        &self,
        request: &CreateRecipientProfileRequest,
    ) -> Result<RecipientProfile> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/recipients/{}/profile",
            encode_path_value(&request.name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
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
