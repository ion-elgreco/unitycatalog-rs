// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::functions::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct FunctionServiceClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl FunctionServiceClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the functions of a schema that the caller can see.
    pub async fn list_functions(
        &self,
        request: &ListFunctionsRequest,
    ) -> Result<ListFunctionsResponse> {
        let mut url = self.base_url.join("api/2.1/unity-catalog/functions")?;
        url.query_pairs_mut()
            .append_pair("catalog_name", &request.catalog_name);
        url.query_pairs_mut()
            .append_pair("schema_name", &request.schema_name);
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

    /// Create a function.
    pub async fn create_function(&self, request: &CreateFunctionRequest) -> Result<FunctionInfo> {
        let url = self.base_url.join("api/2.1/unity-catalog/functions")?;
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

    /// Get a function.
    pub async fn get_function(&self, request: &GetFunctionRequest) -> Result<FunctionInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/functions/{}",
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

    /// Delete a function.
    pub async fn delete_function(&self, request: &DeleteFunctionRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/functions/{}",
            encode_path_value(&request.full_name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }
}
