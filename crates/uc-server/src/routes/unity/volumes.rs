use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateVolumeRequestContent, ListVolumesResponseContent, UpdateVolumeRequestContent, VolumeInfo,
};
use uc_common::{PageToken, SchemaScopedListParams as ListParams, SecurableType, schema_full_name};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules},
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
};

#[utoipa::path(
    post, path = "/volumes",
    request_body = CreateVolumeRequestContent,
    responses(
        (status = 200, body = VolumeInfo),
        (status = 404, description = "Parent schema not found"),
        (status = 409, description = "Already exists"),
    ),
    tag = "Volumes",
)]
pub async fn create_volume(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateVolumeRequestContent>,
) -> ApiResult<Json<VolumeInfo>> {
    let parent = schema_full_name(&req.catalog_name, &req.schema_name);
    let external = match req.volume_type {
        uc_api::VolumeType::External => Some(req.storage_location.as_deref()),
        _ => None,
    };
    authz::check_create(
        &state,
        &principal.username,
        &parent,
        external,
        rules::create_volume,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .volumes()
            .create(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/volumes",
    params(
        ("catalog_name" = String, Query),
        ("schema_name"  = String, Query),
        ("max_results"  = Option<u32>, Query),
        ("page_token"   = Option<String>, Query),
    ),
    responses((status = 200, body = ListVolumesResponseContent)),
    tag = "Volumes",
)]
pub async fn list_volumes(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<ListVolumesResponseContent>> {
    let (volumes, next) = state
        .metastore
        .volumes()
        .list(&params.catalog_name, &params.schema_name, &params.page)
        .await?;
    let access = Access::load(&state, &principal.username).await?;
    let schema = schema_full_name(&params.catalog_name, &params.schema_name);
    let volumes = authz::filter_children(
        &state,
        &access,
        SecurableType::Volume,
        volumes,
        |v| (schema.clone(), v.volume_id.clone(), v.owner.clone()),
        rules::get_volume,
    )
    .await?;
    Ok(Json(ListVolumesResponseContent {
        volumes,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get, path = "/volumes/{name}",
    params(("name" = String, Path, description = "Three-level full name")),
    responses(
        (status = 200, body = VolumeInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Volumes",
)]
pub async fn get_volume(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<VolumeInfo>> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Volume,
        &name,
        rules::get_volume,
    )
    .await?;
    Ok(Json(state.metastore.volumes().get(&name).await?))
}

#[utoipa::path(
    patch, path = "/volumes/{name}",
    params(("name" = String, Path)),
    request_body = UpdateVolumeRequestContent,
    responses(
        (status = 200, body = VolumeInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Volumes",
)]
pub async fn update_volume(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateVolumeRequestContent>,
) -> ApiResult<Json<VolumeInfo>> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Volume,
        &name,
        rules::own_child,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .volumes()
            .update(&name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/volumes/{name}",
    params(("name" = String, Path)),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "Volumes",
)]
pub async fn delete_volume(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<EmptyObject> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Volume,
        &name,
        rules::own_child,
    )
    .await?;
    state.metastore.volumes().delete(&name).await?;
    Ok(EmptyObject)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_volume, list_volumes))
        .routes(routes!(get_volume, update_volume, delete_volume))
}
