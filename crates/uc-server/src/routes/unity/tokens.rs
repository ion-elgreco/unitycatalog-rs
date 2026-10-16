//! Personal access tokens, which belong to a signed-in user also with
//! authorization off, and the OAuth2 token endpoint.

use crate::http::extract::{Json, Path, Query};
use axum::{Form, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{CreatePat, ListPatsResponse, PatInfo};
use uc_common::{PageToken, Pagination};

use crate::{authn::principal::Principal, authz, http::error::ApiResult, server::state::AppState};

#[utoipa::path(
    post, path = "/tokens",
    request_body = CreatePat,
    responses(
        (status = 200, description = "Token created — secret only returned in this response", body = PatInfo),
    ),
    tag = "Auth",
)]
pub async fn create_pat(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreatePat>,
) -> ApiResult<Json<PatInfo>> {
    let owner = principal.signed_in()?;
    Ok(Json(state.metastore.tokens().create_pat(req, owner).await?))
}

#[utoipa::path(
    get, path = "/tokens",
    params(
        ("max_results" = Option<u32>, Query),
        ("page_token"  = Option<String>, Query),
    ),
    responses((status = 200, body = ListPatsResponse)),
    tag = "Auth",
)]
pub async fn list_pats(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListPatsResponse>> {
    let owner = principal.signed_in()?;
    let (tokens, next) = state
        .metastore
        .tokens()
        .list_pats(Some(owner), &page)
        .await?;
    Ok(Json(ListPatsResponse {
        tokens,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    delete, path = "/tokens/{id}",
    params(("id" = String, Path)),
    responses(
        (status = 200, description = "Revoked"),
        (status = 403, description = "Neither the token's owner nor an admin"),
        (status = 404, description = "Not found"),
    ),
    tag = "Auth",
)]
pub async fn revoke_pat(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let caller = principal.signed_in()?;
    let token = state.metastore.tokens().get_pat(&id).await?;
    if token.owner.as_deref() != Some(caller) {
        authz::require_signed_in_admin(&state, &principal).await?;
    }
    state.metastore.tokens().revoke_pat(&id).await?;
    Ok(StatusCode::OK)
}

// The OAuth2 token endpoint: the RFC 6749 §4.4 `client_credentials` grant
// for a personal access token. `client_id` is the PAT id and `client_secret`
// the PAT, which is also the access token.

#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenRequest {
    pub grant_type: String,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    /// Seconds until expiry: the PAT's, when `client_id` names it and it
    /// expires; else 3600.
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthError {
    pub error: &'static str,
    pub error_description: String,
}

#[utoipa::path(
    post, path = "/oauth/token",
    request_body(content = TokenRequest, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, body = TokenResponse),
        (status = 400, body = OAuthError, description = "invalid_request / unsupported_grant_type"),
        (status = 401, body = OAuthError, description = "invalid_client"),
    ),
    tag = "Auth",
)]
pub async fn oauth_token(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(mut req): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, (StatusCode, Json<OAuthError>)> {
    // RFC 6749 §2.3.1: the client may authenticate with HTTP Basic, as
    // upstream's clients do; the header wins over the body.
    if let Some((id, secret)) = basic_credentials(&headers) {
        req.client_id = Some(id);
        req.client_secret = Some(secret);
    }
    if req.grant_type != "client_credentials" {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            format!("unsupported grant_type '{}'", req.grant_type),
        ));
    }
    let Some(secret) = req.client_secret.as_deref() else {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "client_secret is required",
        ));
    };

    // No matching PAT and a database error are both 401 `invalid_client`,
    // the spec's error for any failed client authentication.
    let owner = match state.metastore.tokens().principal_for_token(secret).await {
        Ok(Some(o)) => o,
        _ => {
            return Err(oauth_error(
                StatusCode::UNAUTHORIZED,
                "invalid_client",
                "client credentials are invalid or revoked",
            ));
        }
    };

    // A `client_id` must name this secret's PAT, whose expiry then sets
    // `expires_in`.
    let mut expires_in: i64 = 3600;
    if let Some(claimed_id) = req.client_id.as_deref() {
        let prefix: String = secret.chars().take(8).collect();
        let pat = state
            .metastore
            .tokens()
            .get_pat(claimed_id)
            .await
            .ok()
            .filter(|pat| {
                pat.owner.as_deref() == Some(owner.as_str())
                    && pat.token_prefix.as_deref() == Some(prefix.as_str())
            });
        let Some(pat) = pat else {
            return Err(oauth_error(
                StatusCode::UNAUTHORIZED,
                "invalid_client",
                "client_id does not match secret",
            ));
        };
        if let Some(exp_ms) = pat.expires_at {
            let now_ms = uc_common::now_epoch_millis();
            expires_in = ((exp_ms - now_ms) / 1000).max(1);
        }
    }

    Ok(Json(TokenResponse {
        access_token: secret.to_string(),
        token_type: "Bearer",
        expires_in,
        scope: req.scope,
    }))
}

fn oauth_error(
    status: StatusCode,
    error: &'static str,
    description: impl Into<String>,
) -> (StatusCode, Json<OAuthError>) {
    (
        status,
        Json(OAuthError {
            error,
            error_description: description.into(),
        }),
    )
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_pat, list_pats))
        .routes(routes!(revoke_pat))
        .routes(routes!(oauth_token))
}

/// `Authorization: Basic base64(client_id:client_secret)`, each part
/// form-encoded (RFC 6749 §2.3.1).
fn basic_credentials(headers: &axum::http::HeaderMap) -> Option<(String, String)> {
    use base64::Engine as _;
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let (scheme, encoded) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()?;
    let text = String::from_utf8(decoded).ok()?;
    let (id, secret) = text.split_once(':')?;
    let decode = |part: &str| -> Option<String> {
        percent_encoding::percent_decode_str(&part.replace('+', " "))
            .decode_utf8()
            .ok()
            .map(|s| s.into_owned())
    };
    Some((decode(id)?, decode(secret)?))
}
