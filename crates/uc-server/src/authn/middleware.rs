//! Sign-in, after upstream `AuthDecorator`.
//!
//! A request carries its token as `Authorization: Bearer <token>` or in the
//! `UC_TOKEN` cookie. Tokens are UC access tokens (RS512 JWTs this server
//! signed, see [`crate::authn::security`]) or personal access tokens. Delta
//! Sharing recipients sign in to the protocol, outside the API, with tokens
//! of their own (`crate::routes::sharing`).
//!
//! With authorization on, every request to the API and the control plane
//! must be signed in, except those that sign in (token exchange, client
//! credentials), and the user must exist and be active. With it off, a
//! valid token names the caller and any other request is anonymous:
//! upstream ignores tokens then.
//!
//! Behind an auth proxy (`UC_AUTH_MODE=forward`, `crate::authn::sign_in`), a
//! request from one of its addresses that names a user in its headers is
//! that user, whatever token it carries; other requests sign in with their
//! tokens.

use std::net::SocketAddr;
use std::time::Duration;

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, Request, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use uc_common::{API_BASE, CONTROL_BASE, Error, ErrorCode, Result};

use crate::{
    authn::{principal::Principal, security::ADMIN},
    http::error::ApiError,
    server::state::AppState,
};

/// The cookie upstream signs in with (`ext=cookie`).
pub const UC_TOKEN_COOKIE: &str = "UC_TOKEN";

pub async fn auth_layer(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let required = state.authorization_enabled && requires_sign_in(&path);
    let caller = match forwarded_user(&state, &req) {
        // The auth proxy's user: an active user, as with tokens, but never
        // the bootstrap admin, whom only the server's own tokens name.
        Some(user) if user == ADMIN => Some(Err(Error::new(
            ErrorCode::PermissionDenied,
            format!("User not allowed: {user}"),
        ))),
        Some(user) => Some(
            require_active_user(&state, &user)
                .await
                .map(Principal::authenticated),
        ),
        None => {
            match bearer_token(req.headers()).or_else(|| cookie(req.headers(), UC_TOKEN_COOKIE)) {
                Some(token) => Some(authenticate(&state, &token, required).await),
                None => None,
            }
        }
    };
    match (caller, required) {
        (Some(Ok(principal)), _) => {
            req.extensions_mut().insert(principal);
            next.run(req).await
        }
        (Some(Err(e)), true) => ApiError(e).into_response(),
        (None, true) => ApiError(Error::new(
            ErrorCode::Unauthenticated,
            "No authorization found.",
        ))
        .into_response(),
        // Without a sign-in required, a caller that does not check out is
        // anonymous.
        (Some(Err(_)) | None, false) => next.run(req).await,
    }
}

/// The user that the auth proxy names (`UC_AUTH_MODE=forward`): only for a
/// connection from one of its addresses, as anyone else could send the
/// headers.
fn forwarded_user(state: &AppState, req: &Request<Body>) -> Option<String> {
    let forward = state.auth.sign_in.as_ref()?.forward()?;
    let peer = req.extensions().get::<ConnectInfo<SocketAddr>>()?.0.ip();
    if !forward.trusts(peer) {
        return None;
    }
    forward.user(req.headers())
}

/// Upstream guards the API and control-plane paths, except token exchange;
/// client credentials sign in too.
fn requires_sign_in(path: &str) -> bool {
    let under = |base: &str| {
        path == base
            || path
                .strip_prefix(base)
                .is_some_and(|rest| rest.starts_with('/'))
    };
    let signs_in = path.strip_prefix(CONTROL_BASE) == Some("/auth/tokens")
        || path.strip_prefix(API_BASE) == Some("/oauth/token");
    (under(API_BASE) || under(CONTROL_BASE)) && !signs_in
}

pub(crate) fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?.trim();
    (!token.is_empty()).then(|| token.to_string())
}

/// The value of cookie `name`.
pub fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, value)| *key == name && !value.is_empty())
        .map(|(_, value)| value.to_string())
}

/// A `Set-Cookie` for the UC token: upstream's cookie on `/`, `Secure`
/// when `secure` ([`AuthSettings::secure_cookies`](crate::authn::idp::AuthSettings::secure_cookies)).
pub(crate) fn token_cookie(value: &str, max_age: Duration, secure: bool) -> Result<HeaderValue> {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{UC_TOKEN_COOKIE}={value}; Max-Age={}; Path=/; HttpOnly; SameSite=Strict{secure}",
        max_age.as_secs()
    ))
    .map_err(|e| Error::internal(format!("cookie: {e}")))
}

/// The caller a token names. `active_user` requires the caller to be an
/// active user, as upstream does whenever authorization is on.
async fn authenticate(state: &AppState, token: &str, active_user: bool) -> Result<Principal> {
    let invalid = || Error::new(ErrorCode::Unauthenticated, "Invalid access token.");
    let principal = if looks_like_jwt(token) {
        state
            .security
            .as_ref()
            .ok_or_else(invalid)?
            .verify(token)?
            .sub
    } else {
        state
            .metastore
            .tokens()
            .principal_for_token(token)
            .await
            .ok()
            .flatten()
            .ok_or_else(invalid)?
    };
    if active_user {
        return Ok(Principal::authenticated(
            require_active_user(state, &principal).await?,
        ));
    }
    Ok(Principal::authenticated(principal))
}

/// Upstream: `User not allowed` unless the user exists and is enabled. The
/// caller is then named by the user's principal (its email), as grants and
/// owners are.
pub async fn require_active_user(state: &AppState, principal: &str) -> Result<String> {
    match state.metastore.scim().find_user_by_email(principal).await? {
        Some(user) if user.is_active() => Ok(user.principal().to_string()),
        _ => Err(Error::new(
            ErrorCode::PermissionDenied,
            format!("User not allowed: {principal}"),
        )),
    }
}

/// JWTs are three dot-separated segments starting with `eyJ` (the `{"` of
/// the header). A cheap shape check that keeps JWTs out of the PAT table.
fn looks_like_jwt(token: &str) -> bool {
    token.starts_with("eyJ") && token.split('.').count() == 3
}
