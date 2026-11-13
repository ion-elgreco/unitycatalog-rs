use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateExternalLocation, ExternalLocationInfo, ListExternalLocationsResponse,
    UpdateExternalLocation,
};
use uc_common::{BrowseParams, DeleteParams, PageToken, Pagination, SecurableType};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, Lineage, Read, rules},
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
    let access = Access::load(&state, &principal).await?;
    if access.enabled() {
        let credential =
            authz::lineage(&state, SecurableType::Credential, &req.credential_name).await?;
        access.check(rules::create_external_location(&access, &credential).await)?;
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
    Query(browse): Query<BrowseParams>,
) -> ApiResult<Json<ListExternalLocationsResponse>> {
    let (external_locations, next) = state.metastore.external_locations().list(&page).await?;
    let access = Access::load(&state, &principal).await?;
    let mut kept = Vec::with_capacity(external_locations.len());
    for location in external_locations {
        let lineage = lineage_of(&location);
        if rules::get_external_location(&access, &lineage).await {
            kept.push(location);
        } else if browse.include_browse && rules::browse_external_location(&access, &lineage).await
        {
            kept.push(location.into_browse_only());
        }
    }
    let external_locations = kept;
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
    Query(browse): Query<BrowseParams>,
) -> ApiResult<Json<ExternalLocationInfo>> {
    let info = state.metastore.external_locations().get(&name).await?;
    let access = Access::load(&state, &principal).await?;
    let read = access
        .read(
            &lineage_of(&info),
            browse.include_browse,
            rules::get_external_location,
            rules::browse_external_location,
        )
        .await?;
    Ok(Json(match read {
        Read::Full => info,
        Read::BrowseOnly => info.into_browse_only(),
    }))
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
    let access = Access::load(&state, &principal).await?;
    if access.enabled() {
        let location = authz::lineage(&state, SecurableType::ExternalLocation, &name).await?;
        let credential = match req.credential_name.as_deref() {
            Some(credential) => {
                Some(authz::lineage(&state, SecurableType::Credential, credential).await?)
            }
            None => None,
        };
        access.check(
            rules::update_external_location(&access, &location, credential.as_ref()).await,
        )?;
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
        &principal,
        SecurableType::ExternalLocation,
        &name,
        rules::admin_or_manage,
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
