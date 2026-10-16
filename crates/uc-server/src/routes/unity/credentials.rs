use crate::http::extract::{Json, Path, Query};
use axum::{extract::State, http::StatusCode};
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateCredentialRequest, CredentialInfo, CredentialPurpose, ListCredentialsResponse,
    UpdateCredentialRequest,
};
use uc_common::{DeleteParams, PageToken, Pagination, SecurableType};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, Lineage, rules},
    http::error::ApiResult,
    server::state::AppState,
};

/// A credential as the lineage of its own checks.
fn lineage_of(info: &CredentialInfo) -> Lineage {
    authz::object(
        SecurableType::Credential,
        info.id.as_deref(),
        info.owner.as_deref(),
    )
}

/// Upstream `toCredentialInfo`: AWS roles show the server's own role.
fn present(state: &AppState, mut info: CredentialInfo) -> CredentialInfo {
    if let Some(role) = info.aws_iam_role.as_mut() {
        role.unity_catalog_iam_arn = state.aws_master_role_arn.as_deref().map(str::to_string);
    }
    info
}

#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default)]
    pub purpose: Option<CredentialPurpose>,
    #[serde(flatten)]
    pub page: Pagination,
}

#[utoipa::path(
    post, path = "/credentials",
    request_body = CreateCredentialRequest,
    responses(
        (status = 200, body = CredentialInfo),
        (status = 409, description = "Already exists"),
    ),
    tag = "Credentials",
)]
pub async fn create_credential(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateCredentialRequest>,
) -> ApiResult<Json<CredentialInfo>> {
    let access = Access::load(&state, &principal.username).await?;
    access.check(rules::create_credential(&access))?;
    let info = state
        .metastore
        .credentials()
        .create(req, principal.actor())
        .await?;
    Ok(Json(present(&state, info)))
}

#[utoipa::path(
    get, path = "/credentials",
    params(
        ("max_results" = Option<u32>, Query),
        ("page_token"  = Option<String>, Query),
        ("purpose"     = Option<CredentialPurpose>, Query),
    ),
    responses((status = 200, body = ListCredentialsResponse)),
    tag = "Credentials",
)]
pub async fn list_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<ListCredentialsResponse>> {
    let (credentials, next) = state
        .metastore
        .credentials()
        .list(params.purpose, &params.page)
        .await?;
    let access = Access::load(&state, &principal.username).await?;
    Ok(Json(ListCredentialsResponse {
        credentials: credentials
            .into_iter()
            .filter(|c| rules::get_credential(&access, &lineage_of(c)))
            .map(|c| present(&state, c))
            .collect(),
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get, path = "/credentials/{name}",
    params(("name" = String, Path)),
    responses(
        (status = 200, body = CredentialInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Credentials",
)]
pub async fn get_credential(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<CredentialInfo>> {
    let info = state.metastore.credentials().get(&name).await?;
    let access = Access::load(&state, &principal.username).await?;
    access.check(rules::get_credential(&access, &lineage_of(&info)))?;
    Ok(Json(present(&state, info)))
}

#[utoipa::path(
    patch, path = "/credentials/{name}",
    params(("name" = String, Path)),
    request_body = UpdateCredentialRequest,
    responses(
        (status = 200, body = CredentialInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Credentials",
)]
pub async fn update_credential(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateCredentialRequest>,
) -> ApiResult<Json<CredentialInfo>> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Credential,
        &name,
        rules::admin_or_owner,
    )
    .await?;
    let info = state
        .metastore
        .credentials()
        .update(&name, req, principal.actor())
        .await?;
    Ok(Json(present(&state, info)))
}

#[utoipa::path(
    delete, path = "/credentials/{name}",
    params(
        ("name" = String, Path),
        ("force" = Option<bool>, Query, description = "Delete even if bound to external locations"),
    ),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 412, description = "Bound to external locations and force=false"),
    ),
    tag = "Credentials",
)]
pub async fn delete_credential(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Query(params): Query<DeleteParams>,
) -> ApiResult<StatusCode> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Credential,
        &name,
        rules::admin_or_owner,
    )
    .await?;
    state
        .metastore
        .credentials()
        .delete(&name, params.force)
        .await?;
    Ok(StatusCode::OK)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_credential, list_credentials))
        .routes(routes!(
            get_credential,
            update_credential,
            delete_credential
        ))
}
