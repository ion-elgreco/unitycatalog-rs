// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::iceberg::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct IcebergClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl IcebergClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Get the client configuration of a catalog.
    pub async fn get_iceberg_config(
        &self,
        request: &GetIcebergConfigRequest,
    ) -> Result<IcebergConfigResponse> {
        let mut url = self
            .base_url
            .join("api/2.1/unity-catalog/iceberg/v1/config")?;
        url.query_pairs_mut()
            .append_pair("warehouse", &request.warehouse);
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

    /// List the namespaces of a catalog: its schemas that the caller can read.
    pub async fn list_iceberg_namespaces(
        &self,
        request: &ListIcebergNamespacesRequest,
    ) -> Result<IcebergListNamespacesResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces",
            encode_path_value(&request.catalog, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.parent {
            url.query_pairs_mut()
                .append_pair("parent", &value.to_string());
        }
        if let Some(ref value) = request.page_token {
            url.query_pairs_mut()
                .append_pair("pageToken", &value.to_string());
        }
        if let Some(ref value) = request.page_size {
            url.query_pairs_mut()
                .append_pair("pageSize", &value.to_string());
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

    /// Create a namespace: a schema of the catalog.
    pub async fn create_iceberg_namespace(
        &self,
        request: &IcebergCreateNamespaceRequest,
    ) -> Result<IcebergNamespaceResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces",
            encode_path_value(&request.catalog, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
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

    /// Get a namespace and its properties.
    pub async fn load_iceberg_namespace(
        &self,
        request: &LoadIcebergNamespaceRequest,
    ) -> Result<IcebergNamespaceResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false)
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

    /// Check that a namespace exists. A namespace that does not exist is a not-found error.
    pub async fn iceberg_namespace_exists(
        &self,
        request: &IcebergNamespaceExistsRequest,
    ) -> Result<bool> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false)
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

    /// Drop a namespace. It must be empty.
    pub async fn drop_iceberg_namespace(
        &self,
        request: &DropIcebergNamespaceRequest,
    ) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Set and remove properties of a namespace.
    pub async fn update_iceberg_namespace_properties(
        &self,
        request: &IcebergUpdateNamespacePropertiesRequest,
    ) -> Result<IcebergUpdateNamespacePropertiesResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/properties",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.namespace = Default::default();
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

    /// List the tables of a namespace that have Iceberg metadata and that the caller can read.
    pub async fn list_iceberg_tables(
        &self,
        request: &ListIcebergTablesRequest,
    ) -> Result<IcebergListTablesResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.page_token {
            url.query_pairs_mut()
                .append_pair("pageToken", &value.to_string());
        }
        if let Some(ref value) = request.page_size {
            url.query_pairs_mut()
                .append_pair("pageSize", &value.to_string());
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

    /// Create a native Iceberg table, or stage its creation.
    pub async fn create_iceberg_table(
        &self,
        request: &IcebergCreateTableRequest,
    ) -> Result<IcebergLoadTableResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.namespace = Default::default();
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

    /// Get the metadata of a table, with credentials for its files.
    pub async fn load_iceberg_table(
        &self,
        request: &LoadIcebergTableRequest,
    ) -> Result<IcebergLoadTableResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false),
            encode_path_value(&request.table, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.snapshots {
            url.query_pairs_mut()
                .append_pair("snapshots", &value.to_string());
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

    /// Check that a table exists. A table that does not exist is a not-found error.
    pub async fn iceberg_table_exists(&self, request: &IcebergTableExistsRequest) -> Result<bool> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false),
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

    /// Commit changes to a native Iceberg table, or create a staged table.
    pub async fn commit_iceberg_table(
        &self,
        request: &IcebergCommitTableRequest,
    ) -> Result<IcebergLoadTableResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.namespace = Default::default();
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

    /// Drop a native Iceberg table. UniForm tables are dropped through the Unity Catalog API.
    pub async fn drop_iceberg_table(&self, request: &DropIcebergTableRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false),
            encode_path_value(&request.table, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.purge_requested {
            url.query_pairs_mut()
                .append_pair("purgeRequested", &value.to_string());
        }
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Get fresh credentials for the files of a table.
    pub async fn load_iceberg_table_credentials(
        &self,
        request: &LoadIcebergTableCredentialsRequest,
    ) -> Result<IcebergLoadCredentialsResponse> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables/{}/credentials",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false),
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

    /// Send a scan or commit report of a table. The server checks it and does not keep it.
    pub async fn report_iceberg_metrics(
        &self,
        request: &ReportIcebergMetricsRequest,
    ) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/tables/{}/metrics",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false),
            encode_path_value(&request.table, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        body.namespace = Default::default();
        body.table = Default::default();
        let response = self.client.post(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Get a view. Views are not served: the answer is always a not-found error.
    pub async fn load_iceberg_view(&self, request: &LoadIcebergViewRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/namespaces/{}/views/{}",
            encode_path_value(&request.catalog, false),
            encode_path_value(&request.namespace, false),
            encode_path_value(&request.view, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Rename a native Iceberg table, in its namespace. A move to another namespace is not
    /// supported.
    pub async fn rename_iceberg_table(&self, request: &IcebergRenameTableRequest) -> Result<()> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/iceberg/v1/catalogs/{}/tables/rename",
            encode_path_value(&request.catalog, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.catalog = Default::default();
        let response = self.client.post(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }
}
