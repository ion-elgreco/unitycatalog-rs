// @generated — do not edit by hand.
#![allow(unused_mut)]
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
type BoxFut<'a, T> = ::futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxFut<'a, T> = ::futures::future::LocalBoxFuture<'a, T>;
use super::client::*;
use crate::codegen::error::Result;
use std::future::IntoFuture;
use uc_client::models::scim::v1::*;

/// Builder for users
pub struct ListUsersBuilder {
    client: ScimClient,
    request: ListUsersRequest,
}

impl ListUsersBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = ListUsersRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// SCIM filter (RFC 7644, section 3.4.2.2), such as `userName eq "a@example.com"`.
    pub fn with_filter(mut self, filter: impl Into<Option<String>>) -> Self {
        self.request.filter = filter.into();
        self
    }

    /// Index of the first user to return, from 1.
    pub fn with_start_index(mut self, start_index: impl Into<Option<i32>>) -> Self {
        self.request.start_index = start_index.into();
        self
    }

    /// The maximum number of users to return. The default is 50.
    pub fn with_count(mut self, count: impl Into<Option<i32>>) -> Self {
        self.request.count = count.into();
        self
    }
}

impl IntoFuture for ListUsersBuilder {
    type Output = Result<ListScimUsersResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_users(&request).await })
    }
}

/// Builder for user
pub struct CreateUserBuilder {
    client: ScimClient,
    request: ScimUser,
}

impl CreateUserBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = ScimUser {
            ..Default::default()
        };
        Self { client, request }
    }

    /// The identity the user has at the client's identity provider. The token exchange resolves an
    /// OAuth client (`azp`, `client_id`) to the user by it.
    pub fn with_external_id(mut self, external_id: impl Into<Option<String>>) -> Self {
        self.request.external_id = external_id.into();
        self
    }

    /// Name of the user.
    pub fn with_display_name(mut self, display_name: impl Into<Option<String>>) -> Self {
        self.request.display_name = display_name.into();
        self
    }

    /// Whether the user can sign in. A new user is always enabled; absent in a replace request:
    /// unchanged.
    pub fn with_active(mut self, active: impl Into<Option<bool>>) -> Self {
        self.request.active = active.into();
        self
    }

    /// Email addresses of the user. A new user needs one with `primary` set; a replace request
    /// does not change the email.
    pub fn with_emails<I>(mut self, emails: I) -> Self
    where
        I: IntoIterator<Item = ScimEmail>,
    {
        self.request.emails = emails.into_iter().collect();
        self
    }

    /// Pictures of the user. The server keeps the first; a replace request does not change it.
    pub fn with_photos<I>(mut self, photos: I) -> Self
    where
        I: IntoIterator<Item = ScimPhoto>,
    {
        self.request.photos = photos.into_iter().collect();
        self
    }
}

impl IntoFuture for CreateUserBuilder {
    type Output = Result<ScimUser>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_user(&request).await })
    }
}

/// Builder for user
pub struct GetUserBuilder {
    client: ScimClient,
    request: GetUserRequest,
}

impl GetUserBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient, id: impl Into<String>) -> Self {
        let request = GetUserRequest {
            id: id.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetUserBuilder {
    type Output = Result<ScimUser>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_user(&request).await })
    }
}

/// Builder for user
pub struct ReplaceUserBuilder {
    client: ScimClient,
    request: ScimUser,
}

impl ReplaceUserBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient, id: impl Into<String>) -> Self {
        let request = ScimUser {
            id: id.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The identity the user has at the client's identity provider. The token exchange resolves an
    /// OAuth client (`azp`, `client_id`) to the user by it.
    pub fn with_external_id(mut self, external_id: impl Into<Option<String>>) -> Self {
        self.request.external_id = external_id.into();
        self
    }

    /// Name of the user.
    pub fn with_display_name(mut self, display_name: impl Into<Option<String>>) -> Self {
        self.request.display_name = display_name.into();
        self
    }

    /// Whether the user can sign in. A new user is always enabled; absent in a replace request:
    /// unchanged.
    pub fn with_active(mut self, active: impl Into<Option<bool>>) -> Self {
        self.request.active = active.into();
        self
    }

    /// Email addresses of the user. A new user needs one with `primary` set; a replace request
    /// does not change the email.
    pub fn with_emails<I>(mut self, emails: I) -> Self
    where
        I: IntoIterator<Item = ScimEmail>,
    {
        self.request.emails = emails.into_iter().collect();
        self
    }

    /// Pictures of the user. The server keeps the first; a replace request does not change it.
    pub fn with_photos<I>(mut self, photos: I) -> Self
    where
        I: IntoIterator<Item = ScimPhoto>,
    {
        self.request.photos = photos.into_iter().collect();
        self
    }
}

impl IntoFuture for ReplaceUserBuilder {
    type Output = Result<ScimUser>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.replace_user(&request).await })
    }
}

/// Builder for user
pub struct PatchUserBuilder {
    client: ScimClient,
    request: ScimPatchOp,
}

impl PatchUserBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(
        client: ScimClient,
        id: impl Into<String>,
        operations: Vec<ScimPatchOperation>,
    ) -> Self {
        let request = ScimPatchOp {
            id: id.into(),
            operations,
            ..Default::default()
        };
        Self { client, request }
    }

    /// SCIM schema URIs of the request: `urn:ietf:params:scim:api:messages:2.0:PatchOp`.
    pub fn with_schemas<I>(mut self, schemas: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.schemas = schemas.into_iter().collect();
        self
    }
}

impl IntoFuture for PatchUserBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.patch_user(&request).await })
    }
}

/// Builder for user
pub struct DeleteUserBuilder {
    client: ScimClient,
    request: DeleteUserRequest,
}

impl DeleteUserBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient, id: impl Into<String>) -> Self {
        let request = DeleteUserRequest {
            id: id.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteUserBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_user(&request).await })
    }
}

/// Builder for me
pub struct GetMeBuilder {
    client: ScimClient,
    request: GetMeRequest,
}

impl GetMeBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = GetMeRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetMeBuilder {
    type Output = Result<ScimUser>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_me(&request).await })
    }
}

/// Builder for groups
pub struct ListGroupsBuilder {
    client: ScimClient,
    request: ListGroupsRequest,
}

impl ListGroupsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = ListGroupsRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// SCIM filter (RFC 7644, section 3.4.2.2), such as `displayName eq "admins"`.
    pub fn with_filter(mut self, filter: impl Into<Option<String>>) -> Self {
        self.request.filter = filter.into();
        self
    }

    /// Index of the first group to return, from 1.
    pub fn with_start_index(mut self, start_index: impl Into<Option<i32>>) -> Self {
        self.request.start_index = start_index.into();
        self
    }

    /// The maximum number of groups to return. The default is 50.
    pub fn with_count(mut self, count: impl Into<Option<i32>>) -> Self {
        self.request.count = count.into();
        self
    }
}

impl IntoFuture for ListGroupsBuilder {
    type Output = Result<ListScimGroupsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_groups(&request).await })
    }
}

/// Builder for group
pub struct CreateGroupBuilder {
    client: ScimClient,
    request: ScimGroup,
}

impl CreateGroupBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient, display_name: impl Into<String>) -> Self {
        let request = ScimGroup {
            display_name: display_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Group`. Requests
    /// send the key even when empty; the server fills it in.
    pub fn with_schemas<I>(mut self, schemas: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.schemas = schemas.into_iter().collect();
        self
    }

    /// Members of the group.
    pub fn with_members<I>(mut self, members: I) -> Self
    where
        I: IntoIterator<Item = ScimGroupMember>,
    {
        self.request.members = members.into_iter().collect();
        self
    }
}

impl IntoFuture for CreateGroupBuilder {
    type Output = Result<ScimGroup>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_group(&request).await })
    }
}

/// Builder for group
pub struct GetGroupBuilder {
    client: ScimClient,
    request: GetGroupRequest,
}

impl GetGroupBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient, id: impl Into<String>) -> Self {
        let request = GetGroupRequest {
            id: id.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetGroupBuilder {
    type Output = Result<ScimGroup>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_group(&request).await })
    }
}

/// Builder for group
pub struct ReplaceGroupBuilder {
    client: ScimClient,
    request: ScimGroup,
}

impl ReplaceGroupBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(
        client: ScimClient,
        id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        let request = ScimGroup {
            id: id.into(),
            display_name: display_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Group`. Requests
    /// send the key even when empty; the server fills it in.
    pub fn with_schemas<I>(mut self, schemas: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.schemas = schemas.into_iter().collect();
        self
    }

    /// Members of the group.
    pub fn with_members<I>(mut self, members: I) -> Self
    where
        I: IntoIterator<Item = ScimGroupMember>,
    {
        self.request.members = members.into_iter().collect();
        self
    }
}

impl IntoFuture for ReplaceGroupBuilder {
    type Output = Result<ScimGroup>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.replace_group(&request).await })
    }
}

/// Builder for group
pub struct PatchGroupBuilder {
    client: ScimClient,
    request: ScimPatchOp,
}

impl PatchGroupBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(
        client: ScimClient,
        id: impl Into<String>,
        operations: Vec<ScimPatchOperation>,
    ) -> Self {
        let request = ScimPatchOp {
            id: id.into(),
            operations,
            ..Default::default()
        };
        Self { client, request }
    }

    /// SCIM schema URIs of the request: `urn:ietf:params:scim:api:messages:2.0:PatchOp`.
    pub fn with_schemas<I>(mut self, schemas: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.schemas = schemas.into_iter().collect();
        self
    }
}

impl IntoFuture for PatchGroupBuilder {
    type Output = Result<ScimGroup>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.patch_group(&request).await })
    }
}

/// Builder for group
pub struct DeleteGroupBuilder {
    client: ScimClient,
    request: DeleteGroupRequest,
}

impl DeleteGroupBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient, id: impl Into<String>) -> Self {
        let request = DeleteGroupRequest {
            id: id.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteGroupBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_group(&request).await })
    }
}

/// Builder for bulk
pub struct ScimBulkBuilder {
    client: ScimClient,
    request: ScimBulkRequest,
}

impl ScimBulkBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = ScimBulkRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// SCIM schema URIs of the request: `urn:ietf:params:scim:api:messages:2.0:BulkRequest`.
    pub fn with_schemas<I>(mut self, schemas: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.schemas = schemas.into_iter().collect();
        self
    }

    /// Stop after this many operations failed. Absent: run every operation.
    pub fn with_fail_on_errors(mut self, fail_on_errors: impl Into<Option<i32>>) -> Self {
        self.request.fail_on_errors = fail_on_errors.into();
        self
    }

    /// The operations, in order. At most 100.
    pub fn with_operations<I>(mut self, operations: I) -> Self
    where
        I: IntoIterator<Item = ScimBulkOp>,
    {
        self.request.operations = operations.into_iter().collect();
        self
    }
}

impl IntoFuture for ScimBulkBuilder {
    type Output = Result<ScimBulkResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.scim_bulk(&request).await })
    }
}

/// Builder for scim schemas
pub struct ListScimSchemasBuilder {
    client: ScimClient,
    request: ListScimSchemasRequest,
}

impl ListScimSchemasBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = ListScimSchemasRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for ListScimSchemasBuilder {
    type Output = Result<ListScimSchemasResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_scim_schemas(&request).await })
    }
}

/// Builder for scim resource types
pub struct ListScimResourceTypesBuilder {
    client: ScimClient,
    request: ListScimResourceTypesRequest,
}

impl ListScimResourceTypesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = ListScimResourceTypesRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for ListScimResourceTypesBuilder {
    type Output = Result<ListScimResourceTypesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_scim_resource_types(&request).await })
    }
}

/// Builder for service provider config
pub struct GetServiceProviderConfigBuilder {
    client: ScimClient,
    request: GetServiceProviderConfigRequest,
}

impl GetServiceProviderConfigBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ScimClient`.
    pub(crate) fn new(client: ScimClient) -> Self {
        let request = GetServiceProviderConfigRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetServiceProviderConfigBuilder {
    type Output = Result<ScimServiceProviderConfig>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_service_provider_config(&request).await })
    }
}
