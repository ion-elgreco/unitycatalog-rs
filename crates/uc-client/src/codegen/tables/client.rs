// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::tables::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct TableServiceClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl TableServiceClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the tables of a schema that the caller can see.
    pub async fn list_tables(&self, request: &ListTablesRequest) -> Result<ListTablesResponse> {
        let mut url = self.base_url.join("api/2.1/unity-catalog/tables")?;
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

    /// Create a table.
    pub async fn create_table(&self, request: &CreateTable) -> Result<TableInfo> {
        let url = self.base_url.join("api/2.1/unity-catalog/tables")?;
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

    /// Get a table.
    pub async fn get_table(&self, request: &GetTableRequest) -> Result<TableInfo> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/tables/{}",
            encode_path_value(&request.full_name, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.read_streaming_table_as_managed {
            url.query_pairs_mut()
                .append_pair("read_streaming_table_as_managed", &value.to_string());
        }
        if let Some(ref value) = request.read_materialized_view_as_managed {
            url.query_pairs_mut()
                .append_pair("read_materialized_view_as_managed", &value.to_string());
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

    /// Delete a table.
    pub async fn delete_table(&self, request: &DeleteTableRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/tables/{}",
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
