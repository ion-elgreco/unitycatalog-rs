use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{PermissionsList, UpdatePermissions};
use uc_common::SecurableType;

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules},
    http::error::ApiResult,
    server::state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct PrincipalFilter {
    #[serde(default)]
    pub principal: Option<String>,
}

#[utoipa::path(
    get, path = "/permissions/{securable_type}/{full_name}",
    params(
        ("securable_type" = SecurableType, Path),
        ("full_name"      = String,         Path),
        ("principal"      = Option<String>, Query),
    ),
    responses((status = 200, body = PermissionsList)),
    tag = "Permissions",
)]
pub async fn get_permissions(
    State(state): State<AppState>,
    principal: Principal,
    Path((securable_type, full_name)): Path<(SecurableType, String)>,
    Query(filter): Query<PrincipalFilter>,
) -> ApiResult<Json<PermissionsList>> {
    let access = Access::load(&state, &principal.username).await?;
    let mut only = filter.principal;
    if access.enabled() {
        let lineage = authz::lineage(&state, securable_type, &full_name).await?;
        // Upstream `getAuthorization`: others see their own grants only.
        if !rules::see_permissions(&access, &lineage) {
            if only.as_deref().is_some_and(|p| p != access.principal()) {
                return Ok(Json(PermissionsList {
                    privilege_assignments: vec![],
                }));
            }
            only = Some(access.principal().to_string());
        }
    }
    Ok(Json(
        state
            .metastore
            .permissions()
            .get(securable_type, &full_name, only.as_deref())
            .await?,
    ))
}

#[utoipa::path(
    patch, path = "/permissions/{securable_type}/{full_name}",
    params(
        ("securable_type" = SecurableType, Path),
        ("full_name"      = String,         Path),
    ),
    request_body = UpdatePermissions,
    responses((status = 200, body = PermissionsList)),
    tag = "Permissions",
)]
pub async fn update_permissions(
    State(state): State<AppState>,
    principal: Principal,
    Path((securable_type, full_name)): Path<(SecurableType, String)>,
    Json(req): Json<UpdatePermissions>,
) -> ApiResult<Json<PermissionsList>> {
    authz::check(
        &state,
        &principal.username,
        securable_type,
        &full_name,
        |a, l| rules::update_permissions(a, securable_type, l),
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .permissions()
            .update(securable_type, &full_name, req)
            .await?,
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_permissions, update_permissions))
}
