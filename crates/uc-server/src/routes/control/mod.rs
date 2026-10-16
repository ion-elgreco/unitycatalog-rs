//! The control plane (`/api/1.0/unity-control`, upstream `control.yaml`):
//! token exchange and sign-out, after upstream `AuthService` and
//! `TokenExchangeSubjectTokenHandler`.

pub mod scim;
pub mod scim_filter;

use std::time::Duration;

use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_common::{Error, Result};

use crate::{
    authn::idp::{self, ACCESS_TOKEN, resolve_principal},
    authn::middleware::{UC_TOKEN_COOKIE, cookie, token_cookie},
    http::error::ApiResult,
    http::extract::Query,
    server::state::AppState,
};

const TOKEN_EXCHANGE: &str = "urn:ietf:params:oauth:grant-type:token-exchange";

/// RFC 8693 token exchange request (`OAuthTokenExchangeForm`).
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct OAuthTokenExchangeForm {
    pub grant_type: Option<String>,
    pub scope: Option<String>,
    pub requested_token_type: Option<String>,
    pub subject_token: Option<String>,
    pub subject_token_type: Option<String>,
    pub actor_token: Option<String>,
    pub actor_token_type: Option<String>,
}

/// RFC 8693 token exchange response (`OAuthTokenExchangeInfo`).
#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthTokenExchangeInfo {
    pub access_token: String,
    pub issued_token_type: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct TokenParams {
    /// `cookie`: also set the token as the `UC_TOKEN` cookie.
    pub ext: Option<String>,
}

/// The form, sent as `application/x-www-form-urlencoded` (or as JSON).
fn parse_form(headers: &HeaderMap, body: &[u8]) -> Result<OAuthTokenExchangeForm> {
    let form = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/x-www-form-urlencoded"));
    match form {
        true => serde_urlencoded::from_bytes(body)
            .map_err(|e| Error::invalid_argument(format!("Malformed form: {e}"))),
        false => crate::http::extract::parse_json_body(body, true),
    }
}

#[utoipa::path(
    post, path = "/auth/tokens",
    params(("ext" = Option<String>, Query, description = "`cookie` to also set the `UC_TOKEN` cookie")),
    request_body(content = OAuthTokenExchangeForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, body = OAuthTokenExchangeInfo),
        (status = 400, description = "Unsupported request, or authorization is off"),
        (status = 401, description = "The subject token is not accepted"),
    ),
    tag = "OAuth2",
)]
pub async fn grant_token(
    State(state): State<AppState>,
    Query(params): Query<TokenParams>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Response> {
    let cookie_requested = match params.ext.as_deref() {
        None => false,
        Some("cookie") => true,
        Some(other) => {
            return Err(Error::invalid_argument(format!("Unsupported ext: {other}")).into());
        }
    };
    let form = parse_form(&headers, &body)?;
    let shown = |v: &Option<String>| v.clone().unwrap_or_else(|| "null".into());
    if form.grant_type.as_deref() != Some(TOKEN_EXCHANGE) {
        return Err(Error::invalid_argument(format!(
            "Unsupported grant type: {}",
            shown(&form.grant_type)
        ))
        .into());
    }
    if form.requested_token_type.as_deref() != Some(ACCESS_TOKEN) {
        return Err(Error::invalid_argument(format!(
            "Unsupported requested token type: {}",
            shown(&form.requested_token_type)
        ))
        .into());
    }
    let Some(subject_token_type) = form.subject_token_type.clone() else {
        return Err(Error::invalid_argument(
            "Subject token type is required but was not specified",
        )
        .into());
    };
    if form.actor_token_type.is_some() {
        return Err(Error::invalid_argument("Actor tokens not currently supported").into());
    }
    if !state.authorization_enabled {
        return Err(Error::invalid_argument("Authorization is disabled").into());
    }
    let subject_token = form.subject_token.clone().unwrap_or_default();
    let decoded = idp::decode(&subject_token)
        .map_err(|e| Error::unauthenticated(format!("Invalid token: {e}")))?;

    // The issuer is checked before anything is fetched from it.
    let settings = &state.auth;
    if settings.allowed_issuers.is_empty() {
        return Err(Error::invalid_argument(
            "No allowed issuers configured. Set auth.allowed_issuers (UC_AUTH_ALLOWED_ISSUERS)",
        )
        .into());
    }
    let issuer = decoded
        .claims
        .get("iss")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if !settings.allowed_issuers.is_allowed(&issuer) {
        tracing::warn!(%issuer, "token rejected: issuer not allowed");
        return Err(Error::unauthenticated("Invalid issuer").into());
    }
    settings
        .providers
        .verify(&subject_token, &decoded, &issuer)
        .await?;
    check_audience(settings, &decoded.claims)?;

    let principal = resolve_principal(&state, &subject_token_type, &decoded.claims).await?;
    let security = state
        .security
        .as_ref()
        .ok_or_else(|| Error::internal("The server has no signing key"))?;
    let ttl = settings.access_token_timeout;
    let access_token = security.create_access_token(&principal, ttl)?;
    let mut response = Json(OAuthTokenExchangeInfo {
        access_token: access_token.clone(),
        issued_token_type: ACCESS_TOKEN.into(),
        token_type: "Bearer".into(),
        expires_in: ttl.as_secs(),
    })
    .into_response();
    if cookie_requested {
        let cookie = token_cookie(
            &access_token,
            settings.effective_cookie_timeout(),
            settings.secure_cookies(),
        )?;
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    Ok(response)
}

/// Upstream `validateAudience`: `auth.audiences` must be set; `*` alone
/// disables the check; otherwise one `aud` must match.
fn check_audience(settings: &idp::AuthSettings, claims: &Map<String, Value>) -> Result<()> {
    if settings.audiences.is_empty() {
        return Err(Error::invalid_argument(
            "No audiences configured. Set auth.audiences (UC_AUTH_AUDIENCES)",
        ));
    }
    if settings.audiences.len() == 1 && settings.audiences[0] == "*" {
        return Ok(());
    }
    if settings
        .audience_allowlist
        .is_any_allowed(&idp::audiences(claims))
    {
        return Ok(());
    }
    tracing::warn!(audience = ?idp::audiences(claims), "token rejected: audience not allowed");
    Err(Error::unauthenticated("Invalid audience"))
}

#[utoipa::path(
    post, path = "/auth/logout",
    responses((status = 200, description = "The `UC_TOKEN` cookie, if any, is expired")),
    tag = "OAuth2",
)]
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let mut response = (StatusCode::OK, Json(serde_json::json!({}))).into_response();
    if cookie(&headers, UC_TOKEN_COOKIE).is_some() {
        let cleared = token_cookie("", Duration::ZERO, state.auth.secure_cookies())?;
        response.headers_mut().insert(header::SET_COOKIE, cleared);
    }
    Ok(response)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(grant_token))
        .routes(routes!(logout))
}
