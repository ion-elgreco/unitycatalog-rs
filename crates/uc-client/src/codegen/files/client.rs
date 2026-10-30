// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::files::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct FileClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl FileClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the files and directories of a directory. Needs `READ VOLUME` on a volume, or `SELECT` on
    /// a table.
    pub async fn list_directory_contents(
        &self,
        request: &ListDirectoryContentsRequest,
    ) -> Result<ListDirectoryContentsResponse> {
        let formatted_path = format!(
            "api/2.0/fs/directories/{}",
            encode_path_value(&request.directory_path, true)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.page_size {
            url.query_pairs_mut()
                .append_pair("page_size", &value.to_string());
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

    /// Check that a directory exists. A missing directory is status 404.
    pub async fn get_directory_metadata(
        &self,
        request: &GetDirectoryMetadataRequest,
    ) -> Result<bool> {
        let formatted_path = format!(
            "api/2.0/fs/directories/{}",
            encode_path_value(&request.directory_path, true)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.head(url).send().await?;
        if response.status().as_u16() == 404 {
            return Ok(false);
        }
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(true)
    }

    /// Create a directory and its parents. In object storage, a directory exists only through its
    /// files, so this only checks the path. Only in volumes; needs ownership of the volume.
    pub async fn create_directory(&self, request: &CreateDirectoryRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.0/fs/directories/{}",
            encode_path_value(&request.directory_path, true)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.put(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Delete a directory: an empty one, or with `recursive`, one and what it holds. Only in volumes;
    /// needs ownership of the volume.
    pub async fn delete_directory(&self, request: &DeleteDirectoryRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.0/fs/directories/{}",
            encode_path_value(&request.directory_path, true)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.recursive {
            url.query_pairs_mut()
                .append_pair("recursive", &value.to_string());
        }
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Read a file, or a range of its bytes. Needs `READ VOLUME` on a volume, or `SELECT` on a table.
    pub async fn download_file(
        &self,
        request: &DownloadFileRequest,
    ) -> Result<DownloadFileResponse> {
        let formatted_path = format!(
            "api/2.0/fs/files/{}",
            encode_path_value(&request.file_path, true)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.range {
            url.query_pairs_mut()
                .append_pair("range", &value.to_string());
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

    /// Get the metadata of a file. Needs `READ VOLUME` on a volume, or `SELECT` on a table.
    pub async fn get_file_metadata(
        &self,
        request: &GetFileMetadataRequest,
    ) -> Result<GetFileMetadataResponse> {
        let formatted_path = format!(
            "api/2.0/fs/files/{}",
            encode_path_value(&request.file_path, true)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.head(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Write a file, and create its directories. Only in volumes; needs ownership of the volume.
    pub async fn upload_file(&self, request: &UploadFileRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.0/fs/files/{}",
            encode_path_value(&request.file_path, true)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.overwrite {
            url.query_pairs_mut()
                .append_pair("overwrite", &value.to_string());
        }
        let mut body = request.clone();
        body.file_path = Default::default();
        let response = self.client.put(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Delete a file. Only in volumes; needs ownership of the volume.
    pub async fn delete_file(&self, request: &DeleteFileRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.0/fs/files/{}",
            encode_path_value(&request.file_path, true)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }
}
