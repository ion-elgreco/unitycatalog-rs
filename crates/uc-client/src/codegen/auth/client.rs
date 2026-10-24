// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::error::Result;
use crate::codegen::transport::HttpClient;
use uc_client::models::auth::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct AuthClient {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl AuthClient {
    /// Create a new client instance
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Exchange a token of an identity provider for a Unity Catalog access token (OAuth 2.0 token
    /// exchange, RFC 8693). It needs no sign-in.
    pub async fn exchange_token(
        &self,
        request: &OAuthTokenExchangeForm,
    ) -> Result<OAuthTokenExchangeInfo> {
        let url = self.base_url.join("api/1.0/unity-control/auth/tokens")?;
        let response = self.client.post(url).form(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Get an access token for a personal access token: the OAuth 2.0 `client_credentials` grant
    /// (RFC 6749, section 4.4). It needs no sign-in.
    pub async fn request_access_token(&self, request: &TokenRequest) -> Result<TokenResponse> {
        let url = self.base_url.join("api/2.1/unity-catalog/oauth/token")?;
        let response = self.client.post(url).form(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        let result = response.bytes().await?;
        let options = ::buffa::json::JsonParseOptions::new().ignore_unknown_enum_values(true);
        Ok(::buffa::json::with_json_parse_options(&options, || {
            serde_json::from_slice(&result)
        })?)
    }

    /// Sign out: expire the `UC_TOKEN` cookie of the request, if any.
    pub async fn logout(&self, request: &LogoutRequest) -> Result<()> {
        let url = self.base_url.join("api/1.0/unity-control/auth/logout")?;
        let response = self.client.post(url).json(request).send().await?;
        if !response.status().is_success() {
            return Err(crate::codegen::error::parse_error_response(response).await);
        }
        Ok(())
    }

    /// Get how the web UI signs in. It needs no sign-in.
    pub async fn get_auth_mode(&self, _request: &GetAuthModeRequest) -> Result<AuthMode> {
        let url = self.base_url.join("auth/mode")?;
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
