//! Sharing, the admin side: a UC admin manages shares and recipients and
//! rotates recipient tokens (`/shares`, `/recipients`).
//!
//! Recipients read their shares under `crate::sharing`.

use crate::http::extract::{Json, Path, Query};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateRecipient, CreateShare, ListRecipientsResponse, ListSharesResponse, RecipientInfo,
    ShareInfo, UpdateRecipient, UpdateShare,
};
use uc_common::Pagination;

use crate::{authn::principal::Principal, authz, http::error::ApiResult, server::state::AppState};

#[utoipa::path(
    post, path = "/shares",
    request_body = CreateShare,
    responses((status = 200, body = ShareInfo), (status = 409, description = "Already exists")),
    tag = "Sharing",
)]
pub async fn create_share(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateShare>,
) -> ApiResult<Json<ShareInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .create_share(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/shares",
    params(
        ("max_results" = Option<u32>, Query),
        ("page_token"  = Option<String>, Query),
    ),
    responses((status = 200, body = ListSharesResponse)),
    tag = "Sharing",
)]
pub async fn list_shares(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListSharesResponse>> {
    authz::require_admin(&state, &principal.username).await?;
    let (items, next_page_token) = state.metastore.sharing().list_shares(&page).await?;
    Ok(Json(ListSharesResponse {
        items,
        next_page_token,
    }))
}

#[utoipa::path(
    get, path = "/shares/{name}",
    params(("name" = String, Path)),
    responses((status = 200, body = ShareInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn get_share(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<ShareInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(state.metastore.sharing().get_share(&name).await?))
}

#[utoipa::path(
    patch, path = "/shares/{name}",
    params(("name" = String, Path)),
    request_body = UpdateShare,
    responses((status = 200, body = ShareInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn update_share(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateShare>,
) -> ApiResult<Json<ShareInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .update_share(&name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/shares/{name}",
    params(("name" = String, Path)),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn delete_share(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    state.metastore.sharing().delete_share(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/recipients",
    request_body = CreateRecipient,
    responses((status = 200, body = RecipientInfo, description = "Includes plaintext bearer_token (one-shot)")),
    tag = "Sharing",
)]
pub async fn create_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateRecipient>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .create_recipient(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/recipients",
    params(
        ("max_results" = Option<u32>, Query),
        ("page_token"  = Option<String>, Query),
    ),
    responses((status = 200, body = ListRecipientsResponse)),
    tag = "Sharing",
)]
pub async fn list_recipients(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListRecipientsResponse>> {
    authz::require_admin(&state, &principal.username).await?;
    let (items, next_page_token) = state.metastore.sharing().list_recipients(&page).await?;
    Ok(Json(ListRecipientsResponse {
        items,
        next_page_token,
    }))
}

#[utoipa::path(
    get, path = "/recipients/{name}",
    params(("name" = String, Path)),
    responses((status = 200, body = RecipientInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn get_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(state.metastore.sharing().get_recipient(&name).await?))
}

#[utoipa::path(
    patch, path = "/recipients/{name}",
    params(("name" = String, Path)),
    request_body = UpdateRecipient,
    responses((status = 200, body = RecipientInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn update_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateRecipient>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .update_recipient(&name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/recipients/{name}",
    params(("name" = String, Path)),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn delete_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    state.metastore.sharing().delete_recipient(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/recipients/{name}/rotate-token",
    params(("name" = String, Path)),
    responses((status = 200, body = RecipientInfo, description = "Includes plaintext bearer_token (one-shot)")),
    tag = "Sharing",
)]
pub async fn rotate_recipient_token(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .rotate_recipient_token(&name)
            .await?,
    ))
}

#[utoipa::path(
    post, path = "/recipients/{name}/profile",
    params(("name" = String, Path)),
    responses((status = 200, description = "Delta Sharing profile.json (one-shot — contains a fresh bearer)")),
    tag = "Sharing",
)]
pub async fn recipient_profile(
    State(state): State<AppState>,
    principal: Principal,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    authz::require_admin(&state, &principal.username).await?;
    let rec = state
        .metastore
        .sharing()
        .rotate_recipient_token(&name)
        .await?;
    let token = rec.bearer_token.clone().unwrap_or_default();
    let base = crate::http::public_url(state.auth.public_url.as_deref(), &headers);
    let endpoint = format!("{base}{}", crate::sharing::DELTA_SHARING_PREFIX);
    let profile = serde_json::json!({
        "shareCredentialsVersion": 1,
        "endpoint": endpoint,
        "bearerToken": token,
        "expirationTime": null
    });
    Ok(Json(profile))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_share, list_shares))
        .routes(routes!(get_share, update_share, delete_share))
        .routes(routes!(create_recipient, list_recipients))
        .routes(routes!(get_recipient, update_recipient, delete_recipient))
        .routes(routes!(rotate_recipient_token))
        .routes(routes!(recipient_profile))
}
