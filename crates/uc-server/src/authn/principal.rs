//! The acting `Principal` of a request: the one the auth middleware sets,
//! else "anonymous".

use axum::{extract::FromRequestParts, http::request::Parts};

use uc_auth::authz::AuthContext;
use uc_common::Error;

#[derive(Debug, Clone)]
pub struct Principal {
    pub username: String,
    /// `false` for the anonymous fallback (no or unrecognised token).
    pub authenticated: bool,
    /// The groups that the caller's token names (`groups_claim` of its
    /// issuer). They count when a group of that name exists in the server;
    /// the memberships of the server's own groups are read with the grants.
    pub groups: Vec<String>,
    /// The context of the request for the authorizer: how the caller
    /// signed in, its address, the time.
    pub context: AuthContext,
}

impl Principal {
    pub fn anonymous() -> Self {
        Self {
            username: "anonymous".to_string(),
            authenticated: false,
            groups: Vec::new(),
            context: AuthContext::default(),
        }
    }

    pub fn authenticated(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            authenticated: true,
            groups: Vec::new(),
            context: AuthContext::default(),
        }
    }

    /// With the context of the request.
    pub fn with_context(mut self, context: AuthContext) -> Self {
        self.context = context;
        self
    }

    /// With the groups of the caller's token.
    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
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
