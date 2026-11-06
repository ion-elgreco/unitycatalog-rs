use std::time::Instant;

use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::{StatusCode, request::Parts};
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value};

/// Who sent a request, as a [`Resolver`](crate::Resolver) found it and the
/// host's [`Admission`](crate::Admission) took it.
///
/// [`auth_layer`](crate::auth_layer) puts it into the request extensions.
/// Handlers take it as an extractor, or as `Option<Identity>` where the
/// caller may be anonymous.
#[derive(Debug, Clone, PartialEq)]
pub struct Identity {
    /// The provider that vouched for the caller: the name of a resolver's
    /// provider, such as the name of an OIDC issuer.
    pub provider: String,
    /// How the caller proved who it is, such as [`Identity::JWT`].
    pub method: &'static str,
    /// The caller. A resolver puts the provider's name for it (a JWT's
    /// `sub`, an ARN, a header value); the admission may replace it with
    /// the host's name for the user.
    pub subject: String,
    /// The names of the caller's groups that the provider vouches for.
    pub groups: Vec<String>,
    /// What else the provider says of the caller, such as a JWT's claims.
    pub claims: Map<String, Value>,
    /// Until when the credential is valid, if the resolver lets the
    /// [`Authenticator`](crate::Authenticator) keep the identity for it.
    pub keep_until: Option<Instant>,
}

impl Identity {
    /// The method of [`JwtResolver`](crate::JwtResolver).
    pub const JWT: &'static str = "jwt";
    /// The method of [`AwsResolver`](crate::AwsResolver).
    pub const AWS_SIGV4: &'static str = "aws-sigv4";
    /// The method of [`TrustedHeaders`](crate::TrustedHeaders).
    pub const TRUSTED_HEADERS: &'static str = "trusted-headers";

    pub fn new(
        provider: impl Into<String>,
        method: &'static str,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            method,
            subject: subject.into(),
            groups: Vec::new(),
            claims: Map::new(),
            keep_until: None,
        }
    }

    /// The non-blank string claim `name`.
    pub fn claim(&self, name: &str) -> Option<&str> {
        crate::jwt::claim(&self.claims, name)
    }
}

/// The rejection of the [`Identity`] extractor on a route without
/// [`auth_layer`](crate::auth_layer): a 401.
#[derive(Debug, Clone, Copy)]
pub struct MissingIdentity;

impl IntoResponse for MissingIdentity {
    fn into_response(self) -> Response {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Identity {
    type Rejection = MissingIdentity;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Identity>()
            .cloned()
            .ok_or(MissingIdentity)
    }
}

impl<S: Send + Sync> OptionalFromRequestParts<S> for Identity {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<Identity>().cloned())
    }
}
