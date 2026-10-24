// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::permissions::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct PermissionClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl PermissionClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Get the privileges on a securable.
    pub async fn get_permissions(
        &self,
        request: &GetPermissionsRequest,
    ) -> Result<PermissionsList> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/permissions/{}/{}",
            encode_path_value(&request.securable_type, false),
            encode_path_value(&request.full_name, false)
        );
        let mut url = self.base_url.join(&formatted_path)?;
        if let Some(ref value) = request.principal {
            url.query_pairs_mut()
                .append_pair("principal", &value.to_string());
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

    /// Grant and revoke privileges on a securable. The response lists the privileges of the
    /// principals of the request.
    pub async fn update_permissions(&self, request: &UpdatePermissions) -> Result<PermissionsList> {
        let formatted_path = format!(
            "api/2.1/unity-catalog/permissions/{}/{}",
            encode_path_value(&request.securable_type, false),
            encode_path_value(&request.full_name, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.securable_type = Default::default();
        body.full_name = Default::default();
        let response = self.client.patch(url).json(&body).send().await?;
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
