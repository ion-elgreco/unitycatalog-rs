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
use uc_client::models::permissions::v1::*;

/// Builder for permissions
pub struct GetPermissionsBuilder {
    client: PermissionClient,
    request: GetPermissionsRequest,
}

impl GetPermissionsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `PermissionClient`.
    pub(crate) fn new(
        client: PermissionClient,
        securable_type: impl Into<String>,
        full_name: impl Into<String>,
    ) -> Self {
        let request = GetPermissionsRequest {
            securable_type: securable_type.into(),
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Return only the privileges of this principal.
    pub fn with_principal(mut self, principal: impl Into<Option<String>>) -> Self {
        self.request.principal = principal.into();
        self
    }
}

impl IntoFuture for GetPermissionsBuilder {
    type Output = Result<PermissionsList>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_permissions(&request).await })
    }
}

/// Builder for permissions
pub struct UpdatePermissionsBuilder {
    client: PermissionClient,
    request: UpdatePermissions,
}

impl UpdatePermissionsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `PermissionClient`.
    pub(crate) fn new(
        client: PermissionClient,
        securable_type: impl Into<String>,
        full_name: impl Into<String>,
        changes: Vec<PermissionsChange>,
    ) -> Self {
        let request = UpdatePermissions {
            securable_type: securable_type.into(),
            full_name: full_name.into(),
            changes,
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for UpdatePermissionsBuilder {
    type Output = Result<PermissionsList>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_permissions(&request).await })
    }
}
