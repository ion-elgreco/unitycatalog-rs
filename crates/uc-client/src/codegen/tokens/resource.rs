// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::TokenServiceClient;
use uc_client::models::tokens::v1::*;

/// A client scoped to a single `token`.
#[derive(Clone)]
pub struct TokenClient {
    pub(crate) token_name: String,
    pub(crate) client: TokenServiceClient,
}

impl TokenClient {
    /// Create a client bound to the resource's name components.
    pub fn new(token_name: impl Into<String>, client: TokenServiceClient) -> Self {
        Self {
            token_name: token_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.token_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.token_name.clone()
    }

    /// Revoke a personal access token. Only its owner or an admin can revoke it.
    pub fn delete(&self) -> DeleteTokenBuilder {
        DeleteTokenBuilder::new(self.client.clone(), &self.token_name)
    }
}
