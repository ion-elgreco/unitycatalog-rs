// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::schemas::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct SchemaServiceClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl SchemaServiceClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the schemas of a catalog that the caller can see.
    pub async fn list_schemas(&self, request: &ListSchemasRequest) -> Result<ListSchemasResponse> {
        let mut url = self.base_url.join("api/2.1/unity-catalog/schemas")?;
        url.query_pairs_mut()
            .append_pair("catalog_name", &request.catalog_name);
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

    /// Create a schema.
    pub async fn create_schema(&self, request: &CreateSchema) -> Result<SchemaInfo> {
        let url = self.base_url.join("api/2.1/unity-catalog/schemas")?;
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

    /// Get a schema.
    pub async fn get_schema(&self, request: &GetSchemaRequest) -> Result<SchemaInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/schemas/{}",
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

    /// Change a schema.
    pub async fn update_schema(&self, request: &UpdateSchema) -> Result<SchemaInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/schemas/{}",
            encode_path_value(&request.full_name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.full_name = Default::default();
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

    /// Delete a schema.
    pub async fn delete_schema(&self, request: &DeleteSchemaRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/schemas/{}",
            encode_path_value(&request.full_name, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.force {
            url.query_pairs_mut()
                .append_pair("force", &value.to_string());
        }
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }
}
