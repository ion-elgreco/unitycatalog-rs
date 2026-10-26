use crate::http::extract::{Json, Path};
use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{Protection, UpdateProtection};
use uc_common::SecurableType;
use uc_metastore::protection::protectable;

use crate::{
    authn::principal::Principal,
    authz::{self, rules},
    http::error::ApiResult,
    server::state::AppState,
};

#[utoipa::path(
    get, path = "/protection/{securable_type}/{full_name}",
    params(
        ("securable_type" = SecurableType, Path),
        ("full_name"      = String,         Path),
    ),
    responses((status = 200, body = Protection)),
    tag = "Protection",
)]
pub async fn get_protection(
    State(state): State<AppState>,
    principal: Principal,
    Path((securable_type, full_name)): Path<(SecurableType, String)>,
) -> ApiResult<Json<Protection>> {
    protectable(securable_type)?;
    authz::check(
        &state,
        &principal.username,
        securable_type,
        &full_name,
        |a, l| rules::read_securable(a, securable_type, l),
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .protection()
            .get(securable_type, &full_name)
            .await?,
    ))
}

#[utoipa::path(
    put, path = "/protection/{securable_type}/{full_name}",
    params(
        ("securable_type" = SecurableType, Path),
        ("full_name"      = String,         Path),
    ),
    request_body = UpdateProtection,
    responses((status = 200, body = Protection)),
    tag = "Protection",
)]
pub async fn update_protection(
    State(state): State<AppState>,
    principal: Principal,
    Path((securable_type, full_name)): Path<(SecurableType, String)>,
    Json(req): Json<UpdateProtection>,
) -> ApiResult<Json<Protection>> {
    protectable(securable_type)?;
    authz::check(
        &state,
        &principal.username,
        securable_type,
        &full_name,
        |a, l| rules::protect(a, securable_type, l),
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .protection()
            .set(securable_type, &full_name, req.protected, principal.actor())
            .await?,
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_protection, update_protection))
}
