// @generated — do not edit by hand.
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
use ::olai_http::CloudClient as Transport;

use super::super::encode_path_value;
use crate::codegen::error::Result;
#[cfg(target_arch = "wasm32")]
use ::olai_http_wasm::WasmClient as Transport;
use uc_client::models::recipients::v1::*;
use url::Url;

/// HTTP client for service operations
#[derive(Clone)]
pub struct ActivationClient {
    pub(crate) client: Transport,
    pub(crate) base_url: Url,
}

impl ActivationClient {
    /// Create a new client instance
    pub fn new(client: Transport, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Open the activation link of a token: the Delta Sharing profile with the bearer token. A link
    /// works once, and the request needs no sign-in: the code is the secret. The server keeps a hash
    /// of the token only after this.
    pub async fn activate_recipient_token(
        &self,
        request: &ActivateRecipientTokenRequest,
    ) -> Result<RecipientProfile> {
        let formatted_path = format!(
            "api/1.0/opensharing/activations/{}",
            encode_path_value(&request.code, false)
        );
        let url = self.base_url.join(&formatted_path)?;
        let response = self.client.get(url).send_unchecked().await?;
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
