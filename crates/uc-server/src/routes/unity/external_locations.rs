use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateExternalLocation, ExternalLocationInfo, ListExternalLocationsResponse,
    UpdateExternalLocation,
};
use uc_common::{DeleteParams, PageToken, Pagination, SecurableType};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, Lineage, rules},
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
};

/// An external location as the lineage of its own checks.
fn lineage_of(info: &ExternalLocationInfo) -> Lineage {
    authz::object(
        SecurableType::ExternalLocation,
        info.id.as_deref(),
        info.owner.as_deref(),
    )
}

#[utoipa::path(
    post, path = "/external-locations",
    request_body = CreateExternalLocation,
    responses(
        (status = 200, body = ExternalLocationInfo),
        (status = 404, description = "Bound credential not found"),
        (status = 409, description = "Already exists"),
    ),
    tag = "ExternalLocations",
)]
pub async fn create_external_location(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateExternalLocation>,
) -> ApiResult<Json<ExternalLocationInfo>> {
    let access = Access::load(&state, &principal.username).await?;
    if access.enabled() {
        let credential =
            authz::lineage(&state, SecurableType::Credential, &req.credential_name).await?;
        access.check(rules::create_external_location(&access, &credential))?;
    }
    Ok(Json(
        state
            .metastore
            .external_locations()
            .create(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/external-locations",
    params(
        ("max_results" = Option<u32>, Query),
        ("page_token"  = Option<String>, Query),
    ),
    responses((status = 200, body = ListExternalLocationsResponse)),
    tag = "ExternalLocations",
)]
pub async fn list_external_locations(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListExternalLocationsResponse>> {
    let (external_locations, next) = state.metastore.external_locations().list(&page).await?;
    let access = Access::load(&state, &principal.username).await?;
    let external_locations = external_locations
        .into_iter()
        .filter(|el| rules::get_external_location(&access, &lineage_of(el)))
        .collect();
    Ok(Json(ListExternalLocationsResponse {
        external_locations,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get, path = "/external-locations/{name}",
    params(("name" = String, Path)),
    responses(
        (status = 200, body = ExternalLocationInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "ExternalLocations",
)]
pub async fn get_external_location(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<ExternalLocationInfo>> {
    let info = state.metastore.external_locations().get(&name).await?;
    let access = Access::load(&state, &principal.username).await?;
    access.check(rules::get_external_location(&access, &lineage_of(&info)))?;
    Ok(Json(info))
}

#[utoipa::path(
    patch, path = "/external-locations/{name}",
    params(("name" = String, Path)),
    request_body = UpdateExternalLocation,
    responses(
        (status = 200, body = ExternalLocationInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "ExternalLocations",
)]
pub async fn update_external_location(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateExternalLocation>,
) -> ApiResult<Json<ExternalLocationInfo>> {
    let access = Access::load(&state, &principal.username).await?;
    if access.enabled() {
        let location = authz::lineage(&state, SecurableType::ExternalLocation, &name).await?;
        let credential = match req.credential_name.as_deref() {
            Some(credential) => {
                Some(authz::lineage(&state, SecurableType::Credential, credential).await?)
            }
            None => None,
        };
        access.check(rules::update_external_location(
            &access,
            &location,
            credential.as_ref(),
        ))?;
    }
    Ok(Json(
        state
            .metastore
            .external_locations()
            .update(&name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/external-locations/{name}",
    params(
        ("name" = String, Path),
        ("force" = Option<bool>, Query),
    ),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "ExternalLocations",
)]
pub async fn delete_external_location(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Query(params): Query<DeleteParams>,
) -> ApiResult<EmptyObject> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::ExternalLocation,
        &name,
        rules::admin_or_owner,
    )
    .await?;
    state
        .metastore
        .external_locations()
        .delete(&name, params.force)
        .await?;
    Ok(EmptyObject)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_external_location, list_external_locations))
        .routes(routes!(
            get_external_location,
            update_external_location,
            delete_external_location,
        ))
}
