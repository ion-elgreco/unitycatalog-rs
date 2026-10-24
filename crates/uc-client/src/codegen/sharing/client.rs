// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::sharing::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct SharingClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl SharingClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the shares that the recipient can read.
    pub async fn sharing_list_shares(
        &self,
        _request: &SharingListSharesRequest,
    ) -> Result<SharingListSharesResponse> {
        let url = self
            .base_url
            .join("api/2.1/unity-catalog/delta-sharing/shares")?;
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

    /// List the schemas of a share.
    pub async fn sharing_list_schemas(
        &self,
        request: &SharingListSchemasRequest,
    ) -> Result<SharingListSchemasResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta-sharing/shares/{}/schemas",
            encode_path_value(&request.share, false)
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

    /// List the tables of a schema of a share.
    pub async fn sharing_list_tables(
        &self,
        request: &SharingListTablesRequest,
    ) -> Result<SharingListTablesResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta-sharing/shares/{}/schemas/{}/tables",
            encode_path_value(&request.share, false),
            encode_path_value(&request.schema, false)
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

    /// List the tables of all schemas of a share.
    pub async fn sharing_list_all_tables(
        &self,
        request: &SharingListAllTablesRequest,
    ) -> Result<SharingListTablesResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta-sharing/shares/{}/all-tables",
            encode_path_value(&request.share, false)
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

    /// Get the version of a table.
    pub async fn sharing_get_table_version(
        &self,
        request: &SharingGetTableVersionRequest,
    ) -> Result<SharingGetTableVersionResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta-sharing/shares/{}/schemas/{}/tables/{}/version",
            encode_path_value(&request.share, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let headers = response.headers().clone();
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        let mut output: SharingGetTableVersionResponse = if result.trim_ascii().is_empty() {
            Default::default()
        } else {
            ::buffa::json::with_json_parse_options(&options, || serde_json::from_slice(&result))?
        };
        if let Some(value) = headers
            .get("Delta-Table-Version")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
        {
            output.version = value;
        }
        Ok(output)
    }

    /// Get the protocol and the metadata of a table. The response also has the version of the table
    /// in the `Delta-Table-Version` header (always 0).
    pub async fn sharing_get_table_metadata(
        &self,
        request: &SharingGetTableMetadataRequest,
    ) -> Result<Vec<SharingTableLine>> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta-sharing/shares/{}/schemas/{}/tables/{}/metadata",
            encode_path_value(&request.share, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        result
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.trim_ascii().is_empty())
            .map(|line| -> Result<SharingTableLine> {
                Ok(::buffa::json::with_json_parse_options(&options, || {
                    serde_json::from_slice(line)
                })?)
            })
            .collect()
    }

    /// Read a table. The server does not read the Delta log: the answer is that of
    /// `SharingGetTableMetadata`, with no data files. The server ignores the request body.
    pub async fn sharing_query_table(
        &self,
        request: &SharingQueryTableRequest,
    ) -> Result<Vec<SharingTableLine>> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta-sharing/shares/{}/schemas/{}/tables/{}/query",
            encode_path_value(&request.share, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.post(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        result
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.trim_ascii().is_empty())
            .map(|line| -> Result<SharingTableLine> {
                Ok(::buffa::json::with_json_parse_options(&options, || {
                    serde_json::from_slice(line)
                })?)
            })
            .collect()
    }
}
