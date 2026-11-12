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
use uc_client::models::recipients::v1::*;

/// Builder for recipient token
pub struct ActivateRecipientTokenBuilder {
    client: ActivationClient,
    request: ActivateRecipientTokenRequest,
}

impl ActivateRecipientTokenBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ActivationClient`.
    pub(crate) fn new(client: ActivationClient, code: impl Into<String>) -> Self {
        let request = ActivateRecipientTokenRequest {
            code: code.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for ActivateRecipientTokenBuilder {
    type Output = Result<RecipientProfile>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.activate_recipient_token(&request).await })
    }
}
