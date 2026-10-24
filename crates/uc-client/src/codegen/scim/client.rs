// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::super::encode_path_value;
use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::scim::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct ScimClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl ScimClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// List the users. Only the metastore admin can.
    pub async fn list_users(&self, request: &ListUsersRequest) -> Result<ListScimUsersResponse> {
        let mut url = self.base_url.join("api/1.0/unity-control/scim2/Users")?;
        if let Some(ref value) = request.filter {
            url.query_pairs_mut()
                .append_pair("filter", &value.to_string());
        }
        if let Some(ref value) = request.start_index {
            url.query_pairs_mut()
                .append_pair("startIndex", &value.to_string());
        }
        if let Some(ref value) = request.count {
            url.query_pairs_mut()
                .append_pair("count", &value.to_string());
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

    /// Create a user, named by its primary email. Only the metastore admin can.
    pub async fn create_user(&self, request: &ScimUser) -> Result<ScimUser> {
        let url = self.base_url.join("api/1.0/unity-control/scim2/Users")?;
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

    /// Get a user. Only the metastore admin can.
    pub async fn get_user(&self, request: &GetUserRequest) -> Result<ScimUser> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Users/{}",
            encode_path_value(&request.id, false)
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

    /// Replace a user: the name, state and external id change when given; the email and picture
    /// stay. Only the metastore admin can.
    pub async fn replace_user(&self, request: &ScimUser) -> Result<ScimUser> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Users/{}",
            encode_path_value(&request.id, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.put(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Enable or disable a user. The server takes the first `replace` whose `path` is `active` or
    /// absent; without `path`, the first attribute of `value` is the state (`{"active": false}`).
    /// With no such operation the server answers 501. Only the metastore admin can.
    pub async fn patch_user(&self, request: &ScimPatchOp) -> Result<()> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Users/{}",
            encode_path_value(&request.id, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.id = Default::default();
        let response = self.client.patch(url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Disable a user and remove its grants. The user stays, disabled. Only the metastore admin can.
    pub async fn delete_user(&self, request: &DeleteUserRequest) -> Result<()> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Users/{}",
            encode_path_value(&request.id, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Get the signed-in user. The role `admin` names a caller that can do what the metastore admin
    /// does.
    pub async fn get_me(&self, _request: &GetMeRequest) -> Result<ScimUser> {
        let url = self.base_url.join("api/1.0/unity-control/scim2/Me")?;
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

    /// List the groups. Only the metastore admin can.
    pub async fn list_groups(&self, request: &ListGroupsRequest) -> Result<ListScimGroupsResponse> {
        let mut url = self.base_url.join("api/1.0/unity-control/scim2/Groups")?;
        if let Some(ref value) = request.filter {
            url.query_pairs_mut()
                .append_pair("filter", &value.to_string());
        }
        if let Some(ref value) = request.start_index {
            url.query_pairs_mut()
                .append_pair("startIndex", &value.to_string());
        }
        if let Some(ref value) = request.count {
            url.query_pairs_mut()
                .append_pair("count", &value.to_string());
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

    /// Create a group. Only the metastore admin can.
    pub async fn create_group(&self, request: &ScimGroup) -> Result<ScimGroup> {
        let url = self.base_url.join("api/1.0/unity-control/scim2/Groups")?;
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

    /// Get a group. Only the metastore admin can.
    pub async fn get_group(&self, request: &GetGroupRequest) -> Result<ScimGroup> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Groups/{}",
            encode_path_value(&request.id, false)
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

    /// Replace a group: its name and members. Only the metastore admin can.
    pub async fn replace_group(&self, request: &ScimGroup) -> Result<ScimGroup> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Groups/{}",
            encode_path_value(&request.id, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.put(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Change a group: `add` to `members`, `remove` of `members` or `members[value eq "<id>"]`,
    /// `replace` of `displayName`, or `replace` without `path` with a whole group. Other operations
    /// are ignored. Only the metastore admin can.
    pub async fn patch_group(&self, request: &ScimPatchOp) -> Result<ScimGroup> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Groups/{}",
            encode_path_value(&request.id, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let mut body = request.clone();
        body.id = Default::default();
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

    /// Delete a group. Only the metastore admin can.
    pub async fn delete_group(&self, request: &DeleteGroupRequest) -> Result<()> {
        let formatted_path = format!(
            "api/1.0/unity-control/scim2/Groups/{}",
            encode_path_value(&request.id, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.delete(url).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Run operations on users and groups in one request. Each operation has its own status. Only
    /// the metastore admin can.
    pub async fn scim_bulk(&self, request: &ScimBulkRequest) -> Result<ScimBulkResponse> {
        let url = self.base_url.join("api/1.0/unity-control/scim2/Bulk")?;
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

    /// List the SCIM schemas of the server: User and Group. Any signed-in caller can.
    pub async fn list_scim_schemas(
        &self,
        _request: &ListScimSchemasRequest,
    ) -> Result<ListScimSchemasResponse> {
        let url = self.base_url.join("api/1.0/unity-control/scim2/Schemas")?;
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

    /// List the SCIM resource types of the server: User and Group. Any signed-in caller can.
    pub async fn list_scim_resource_types(
        &self,
        _request: &ListScimResourceTypesRequest,
    ) -> Result<ListScimResourceTypesResponse> {
        let url = self
            .base_url
            .join("api/1.0/unity-control/scim2/ResourceTypes")?;
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

    /// Get the SCIM features that the server supports. Any signed-in caller can.
    pub async fn get_service_provider_config(
        &self,
        _request: &GetServiceProviderConfigRequest,
    ) -> Result<ScimServiceProviderConfig> {
        let url = self
            .base_url
            .join("api/1.0/unity-control/scim2/ServiceProviderConfig")?;
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
