// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::tables::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct StagingTableClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl StagingTableClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Create a staging table: reserve a table id and a managed storage location for a MANAGED
    /// table.
    pub async fn create_staging_table(
        &self,
        request: &CreateStagingTable,
    ) -> Result<StagingTableInfo> {
        let url = self.base_url.join("api/2.1/unity-catalog/staging-tables")?;
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
