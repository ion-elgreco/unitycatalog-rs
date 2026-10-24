// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::delta::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct DeltaCommitClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl DeltaCommitClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Ratify a staged commit, report backfilled commits, or both. Another commit at the same
    /// version is a conflict (status 409).
    pub async fn commit_delta(&self, request: &DeltaCommit) -> Result<()> {
        let url = self
            .base_url
            .join("api/2.1/unity-catalog/delta/preview/commits")?;
        let response = self.client.post(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Get the commits of a table that are not backfilled yet. The server takes the request as the
    /// JSON body of a `GET`.
    pub async fn get_delta_commits(
        &self,
        request: &DeltaGetCommits,
    ) -> Result<DeltaGetCommitsResponse> {
        let url = self
            .base_url
            .join("api/2.1/unity-catalog/delta/preview/commits")?;
        let response = self.client.get(url).json(request).send().await?;
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
