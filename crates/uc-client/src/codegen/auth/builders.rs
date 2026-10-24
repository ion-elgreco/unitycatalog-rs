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
use uc_client::models::auth::v1::*;

/// Builder for token
pub struct ExchangeTokenBuilder {
    client: AuthClient,
    request: OAuthTokenExchangeForm,
}

impl ExchangeTokenBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `AuthClient`.
    pub(crate) fn new(
        client: AuthClient,
        grant_type: impl Into<String>,
        requested_token_type: impl Into<String>,
        subject_token: impl Into<String>,
        subject_token_type: impl Into<String>,
    ) -> Self {
        let request = OAuthTokenExchangeForm {
            grant_type: grant_type.into(),
            requested_token_type: requested_token_type.into(),
            subject_token: subject_token.into(),
            subject_token_type: subject_token_type.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The scope of the token exchange.
    pub fn with_scope(mut self, scope: impl Into<Option<String>>) -> Self {
        self.request.scope = scope.into();
        self
    }

    /// The token of the acting party. Not supported.
    pub fn with_actor_token(mut self, actor_token: impl Into<Option<String>>) -> Self {
        self.request.actor_token = actor_token.into();
        self
    }

    /// The type of `actor_token`. Not supported: the server refuses a request with it.
    pub fn with_actor_token_type(mut self, actor_token_type: impl Into<Option<String>>) -> Self {
        self.request.actor_token_type = actor_token_type.into();
        self
    }
}

impl IntoFuture for ExchangeTokenBuilder {
    type Output = Result<OAuthTokenExchangeInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.exchange_token(&request).await })
    }
}

/// Builder for access token
pub struct RequestAccessTokenBuilder {
    client: AuthClient,
    request: TokenRequest,
}

impl RequestAccessTokenBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `AuthClient`.
    pub(crate) fn new(
        client: AuthClient,
        grant_type: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        let request = TokenRequest {
            grant_type: grant_type.into(),
            client_secret: client_secret.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The `id` of the personal access token. When given, it must name the token of
    /// `client_secret`, and `expires_in` is the time until that token expires.
    pub fn with_client_id(mut self, client_id: impl Into<Option<String>>) -> Self {
        self.request.client_id = client_id.into();
        self
    }

    /// The scope of the request. The answer repeats it.
    pub fn with_scope(mut self, scope: impl Into<Option<String>>) -> Self {
        self.request.scope = scope.into();
        self
    }
}

impl IntoFuture for RequestAccessTokenBuilder {
    type Output = Result<TokenResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.request_access_token(&request).await })
    }
}

/// Builder for logout
pub struct LogoutBuilder {
    client: AuthClient,
    request: LogoutRequest,
}

impl LogoutBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `AuthClient`.
    pub(crate) fn new(client: AuthClient) -> Self {
        let request = LogoutRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for LogoutBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.logout(&request).await })
    }
}

/// Builder for auth mode
pub struct GetAuthModeBuilder {
    client: AuthClient,
    request: GetAuthModeRequest,
}

impl GetAuthModeBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `AuthClient`.
    pub(crate) fn new(client: AuthClient) -> Self {
        let request = GetAuthModeRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetAuthModeBuilder {
    type Output = Result<AuthMode>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_auth_mode(&request).await })
    }
}
