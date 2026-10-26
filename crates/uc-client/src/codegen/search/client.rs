// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::search::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct SearchClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl SearchClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Find the objects whose name or comment holds all the words of a query.
    pub async fn search_objects(
        &self,
        request: &SearchObjectsRequest,
    ) -> Result<SearchObjectsResponse> {
        let mut url = self.base_url.join("api/2.1/unity-catalog/search")?;
        url.query_pairs_mut().append_pair("query", &request.query);
        if let Some(ref value) = request.securable_type {
            url.query_pairs_mut()
                .append_pair("securable_type", &value.to_string());
        }
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
}
