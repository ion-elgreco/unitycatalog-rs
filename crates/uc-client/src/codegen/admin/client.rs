// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::admin::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct AdminClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl AdminClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Get a snapshot of the whole catalog, in one answer.
    pub async fn get_backup(&self, _request: &GetBackupRequest) -> Result<BackupResponse> {
        let url = self.base_url.join("api/2.1/unity-catalog/admin/backup")?;
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

    /// Replace the whole catalog with a snapshot, in one transaction: every table of the store is
    /// emptied, then filled with the rows of the snapshot.
    pub async fn restore_backup(&self, request: &BackupResponse) -> Result<RestoreResponse> {
        let url = self.base_url.join("api/2.1/unity-catalog/admin/restore")?;
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
