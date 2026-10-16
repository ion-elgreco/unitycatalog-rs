//! The acting `Principal` of a request: the one the auth middleware sets,
//! else "anonymous".

use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

use uc_common::Error;

#[derive(Debug, Clone)]
pub struct Principal {
    pub username: String,
    /// `false` for the anonymous fallback (no or unrecognised token).
    pub authenticated: bool,
}

impl Principal {
    pub fn anonymous() -> Self {
        Self {
            username: "anonymous".to_string(),
            authenticated: false,
        }
    }

    pub fn authenticated(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            authenticated: true,
        }
    }

    /// The caller recorded as `owner` / `created_by` / `updated_by`. Like
    /// upstream's `IdentityUtils.findPrincipalEmailAddress`, there is none
    /// when the request carries no identity.
    pub fn actor(&self) -> Option<&str> {
        self.authenticated.then_some(self.username.as_str())
    }

    /// The caller, for what takes a signed-in caller even with
    /// authorization off.
    pub fn signed_in(&self) -> Result<&str, Error> {
        self.actor()
            .ok_or_else(|| Error::unauthenticated("No authorization found."))
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Principal {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<Principal>()
            .cloned()
            .unwrap_or_else(Self::anonymous))
    }
}
