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
use uc_client::models::health::v1::*;

/// Builder for health
pub struct GetHealthBuilder {
    client: HealthClient,
    request: GetHealthRequest,
}

impl GetHealthBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `HealthClient`.
    pub(crate) fn new(client: HealthClient) -> Self {
        let request = GetHealthRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetHealthBuilder {
    type Output = Result<HealthResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_health(&request).await })
    }
}

/// Builder for version
pub struct GetVersionBuilder {
    client: HealthClient,
    request: GetVersionRequest,
}

impl GetVersionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `HealthClient`.
    pub(crate) fn new(client: HealthClient) -> Self {
        let request = GetVersionRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetVersionBuilder {
    type Output = Result<VersionResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_version(&request).await })
    }
}
