//! Sign-in, after upstream `AuthDecorator`.
//!
//! A request carries its token as `Authorization: Bearer <token>` or in the
//! `UC_TOKEN` cookie. The auth chain ([`crate::authn::chain`]) finds the
//! caller: from the auth proxy's headers, or from the token.
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
//! tokens. A failure of the chain is the error of [`chain::error`].

use std::time::{Duration, SystemTime};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use uc_auth::{AuthError, RequestContext, authz::AuthContext};

use uc_common::{API_BASE, CONTROL_BASE, Error, ErrorCode, FILES_BASE, Result};

use crate::{
    authn::{chain, principal::Principal},
    http::error::ApiError,
    server::state::AppState,
};

/// The cookie upstream signs in with (`ext=cookie`).
pub const UC_TOKEN_COOKIE: &str = "UC_TOKEN";

/// Marks each request with the address of its client behind the proxies of
/// `auth.trusted_proxies`. It runs before the layers that count per client:
/// the rate limit and the sign-in.
pub async fn client_layer(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let (mut parts, body) = req.into_parts();
    state.auth.proxies.mark(&mut parts);
    next.run(Request::from_parts(parts, body)).await
}

pub async fn auth_layer(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    let required = state.authorization_enabled && requires_sign_in(req.uri().path());
    let (mut parts, body) = req.into_parts();
    let caller = state.auth.authenticator.authenticate(&parts, &state).await;
    match (caller, required) {
        (Ok(identity), _) => {
            let mut context = AuthContext::for_identity(&identity).request_time(SystemTime::now());
            if let Some(peer) = RequestContext::new(&parts, None).peer() {
                context = context.source_ip(peer);
            }
            let principal = Principal::authenticated(identity.subject)
                .with_groups(identity.groups)
                .with_context(context);
            parts.extensions.insert(principal);
            next.run(Request::from_parts(parts, body)).await
        }
        (Err(e), true) => {
            if let AuthError::Unavailable(reason) = &e {
                tracing::warn!(%reason, "cannot check the caller");
            }
            ApiError(chain::error(&e)).into_response()
        }
        // Without a sign-in required, a caller that does not check out is
        // anonymous.
        (Err(_), false) => next.run(Request::from_parts(parts, body)).await,
    }
}

/// Upstream guards the API and control-plane paths, except token exchange;
/// client credentials sign in too. The Files API is guarded as the API.
pub(crate) fn requires_sign_in(path: &str) -> bool {
    let under = |base: &str| {
        path == base
            || path
                .strip_prefix(base)
                .is_some_and(|rest| rest.starts_with('/'))
    };
    let signs_in = path.strip_prefix(CONTROL_BASE) == Some("/auth/tokens")
        || path.strip_prefix(API_BASE) == Some("/oauth/token");
    (under(API_BASE) || under(CONTROL_BASE) || under(FILES_BASE)) && !signs_in
}

pub(crate) use uc_auth::{bearer_token, cookie};

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

/// Upstream: `User not allowed` unless the user that `principal` names
/// (`ScimOps::find_user_named`) exists and is enabled. The caller is then
/// named by the user's principal, as grants and owners are.
pub async fn require_active_user(state: &AppState, principal: &str) -> Result<String> {
    match state.metastore.scim().find_user_named(principal).await? {
        Some(user) if user.is_active() => Ok(user.principal().to_string()),
        _ => Err(not_allowed(principal)),
    }
}

/// As [`require_active_user`], for the user with the id `id` that a server
/// token carries, issued for `principal`: the user keeps its sign-in when
/// its email changes, and a later user with that principal does not get
/// it.
pub async fn require_active_user_at(state: &AppState, id: &str, principal: &str) -> Result<String> {
    match state.metastore.scim().get_user(id).await {
        Ok(user) if user.is_active() => Ok(user.principal().to_string()),
        Ok(_) => Err(not_allowed(principal)),
        Err(e) if e.code() == ErrorCode::NotFound => Err(not_allowed(principal)),
        Err(e) => Err(e),
    }
}

fn not_allowed(principal: &str) -> Error {
    Error::new(
        ErrorCode::PermissionDenied,
        format!("User not allowed: {principal}"),
    )
}
