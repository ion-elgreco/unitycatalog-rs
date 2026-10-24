// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::delta::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct DeltaApiClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl DeltaApiClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Get the configuration of the Delta API for a catalog: the endpoints that the server supports
    /// and the protocol version.
    pub async fn get_delta_config(
        &self,
        request: &GetDeltaConfigRequest,
    ) -> Result<DeltaCatalogConfig> {
        let mut url = self
            .base_url
            .join("api/2.1/unity-catalog/delta/v1/config")?;
        url.query_pairs_mut()
            .append_pair("catalog", &request.catalog);
        url.query_pairs_mut()
            .append_pair("protocol-versions", &request.protocol_versions);
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

    /// Create a staging table: reserve a table id and a storage location for a new MANAGED table,
    /// with credentials to write its first commit.
    pub async fn create_delta_staging_table(
        &self,
        request: &DeltaCreateStagingTableRequest,
    ) -> Result<DeltaStagingTableResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/staging-tables",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.schema = Default::default();
        let response = self.client.post(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Create a table. A MANAGED table takes over the staging table at its location.
    pub async fn create_delta_table(
        &self,
        request: &DeltaCreateTableRequest,
    ) -> Result<DeltaLoadTableResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.schema = Default::default();
        let response = self.client.post(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Load a table: its metadata and, for a MANAGED table, its commits that are not backfilled yet.
    /// It holds no credentials.
    pub async fn load_delta_table(
        &self,
        request: &LoadDeltaTableRequest,
    ) -> Result<DeltaLoadTableResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables/{}",
            encode_path_value(&request.catalog, false),
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
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Update a table or commit to it, then load it. A replay of an accepted `add-commit` is a no-op.
    /// A version or requirement conflict is status 409; too many commits that are not backfilled
    /// is status 429.
    pub async fn update_delta_table(
        &self,
        request: &DeltaUpdateTableRequest,
    ) -> Result<DeltaLoadTableResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.schema = Default::default();
        body.table = Default::default();
        let response = self.client.post(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Check that a table exists. A missing table is status 404.
    pub async fn delta_table_exists(&self, request: &DeltaTableExistsRequest) -> Result<bool> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
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

    /// Delete a table.
    pub async fn delete_delta_table(&self, request: &DeleteDeltaTableRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Rename a table in its schema.
    pub async fn rename_delta_table(&self, request: &DeltaRenameTableRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables/{}/rename",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.schema = Default::default();
        body.table = Default::default();
        let response = self.client.post(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Report the metrics of a commit. The server checks the table id and does not store the
    /// metrics.
    pub async fn report_delta_table_metrics(
        &self,
        request: &DeltaReportMetricsRequest,
    ) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables/{}/metrics",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.schema = Default::default();
        body.table = Default::default();
        let response = self.client.post(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Get temporary credentials for the data of a table.
    pub async fn get_delta_table_credentials(
        &self,
        request: &GetDeltaTableCredentialsRequest,
    ) -> Result<DeltaCredentialsResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/catalogs/{}/schemas/{}/tables/{}/credentials",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.schema, false),
            encode_path_value(&request.table, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(known) = request.operation.as_known() {
            use buffa::Enumeration as _;
            url.query_pairs_mut()
                .append_pair("operation", known.proto_name());
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

    /// Get temporary credentials for the data of a staging table. They are always `READ_WRITE`, for
    /// the first commit. Only the owner of the staging table gets them.
    pub async fn get_delta_staging_table_credentials(
        &self,
        request: &GetDeltaStagingTableCredentialsRequest,
    ) -> Result<DeltaCredentialsResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/delta/v1/staging-tables/{}/credentials",
            encode_path_value(&request.table_id, false)
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
}
