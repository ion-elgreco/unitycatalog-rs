// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::audit::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct AuditClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl AuditClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the audit events, newest first.
    pub async fn list_audit_events(
        &self,
        request: &ListAuditEventsRequest,
    ) -> Result<ListAuditEventsResponse> {
        let mut url = self.base_url.join("api/2.1/unity-catalog/admin/audit")?;
        if let Some(ref value) = request.principal {
            url.query_pairs_mut()
                .append_pair("principal", &value.to_string());
        }
        if let Some(ref value) = request.method {
            url.query_pairs_mut()
                .append_pair("method", &value.to_string());
        }
        if let Some(ref value) = request.path_prefix {
            url.query_pairs_mut()
                .append_pair("path_prefix", &value.to_string());
        }
        if let Some(ref value) = request.since {
            url.query_pairs_mut()
                .append_pair("since", &value.to_string());
        }
        if let Some(ref value) = request.until {
            url.query_pairs_mut()
                .append_pair("until", &value.to_string());
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
