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
use uc_client::models::protection::v1::*;

/// Builder for protection
pub struct GetProtectionBuilder {
    client: ProtectionClient,
    request: GetProtectionRequest,
}

impl GetProtectionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ProtectionClient`.
    pub(crate) fn new(
        client: ProtectionClient,
        securable_type: impl Into<String>,
        full_name: impl Into<String>,
    ) -> Self {
        let request = GetProtectionRequest {
            securable_type: securable_type.into(),
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetProtectionBuilder {
    type Output = Result<Protection>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_protection(&request).await })
    }
}

/// Builder for protection
pub struct UpdateProtectionBuilder {
    client: ProtectionClient,
    request: UpdateProtection,
}

impl UpdateProtectionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ProtectionClient`.
    pub(crate) fn new(
        client: ProtectionClient,
        securable_type: impl Into<String>,
        full_name: impl Into<String>,
        protected: bool,
    ) -> Self {
        let request = UpdateProtection {
            securable_type: securable_type.into(),
            full_name: full_name.into(),
            protected,
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for UpdateProtectionBuilder {
    type Output = Result<Protection>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_protection(&request).await })
    }
}
