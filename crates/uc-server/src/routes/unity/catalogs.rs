use crate::http::extract::{Json, Path, Query};
use axum::{extract::State, http::StatusCode};
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{CatalogInfo, CreateCatalog, ListCatalogsResponse, UpdateCatalog};
use uc_common::{DeleteParams, PageToken, Pagination, SecurableType};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, Lineage, rules},
    http::error::ApiResult,
    server::state::AppState,
};

/// A catalog as the lineage of its own checks.
fn lineage_of(info: &CatalogInfo) -> Lineage {
    let catalog = authz::node(
        SecurableType::Catalog,
        info.id.as_deref(),
        info.owner.as_deref(),
    );
    Lineage {
        catalog: catalog.clone(),
        schema: None,
        object: catalog,
    }
}

#[utoipa::path(
    post,
    path = "/catalogs",
    request_body = CreateCatalog,
    responses(
        (status = 200, description = "Created", body = CatalogInfo),
        (status = 400, description = "Invalid argument"),
        (status = 409, description = "Already exists"),
    ),
    tag = "Catalogs",
)]
pub async fn create_catalog(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateCatalog>,
) -> ApiResult<Json<CatalogInfo>> {
    let access = Access::load(&state, &principal.username).await?;
    let storage_root =
        authz::storage_root_allowed(&state, &access, req.storage_root.as_deref()).await?;
    access.check(rules::create_catalog(&access, storage_root))?;
    let info = state
        .metastore
        .catalogs()
        .create(req, principal.actor())
        .await?;
    Ok(Json(info))
}

#[utoipa::path(
    get,
    path = "/catalogs",
    params(Pagination),
    responses((status = 200, description = "OK", body = ListCatalogsResponse)),
    tag = "Catalogs",
)]
pub async fn list_catalogs(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListCatalogsResponse>> {
    let access = Access::load(&state, &principal.username).await?;
    let (catalogs, next) = state.metastore.catalogs().list(&page).await?;
    // Upstream filters each page, which may then hold fewer items.
    let catalogs = catalogs
        .into_iter()
        .filter(|c| rules::get_catalog(&access, &lineage_of(c)))
        .collect();
    Ok(Json(ListCatalogsResponse {
        catalogs,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get,
    path = "/catalogs/{name}",
    params(("name" = String, Path, description = "Catalog name")),
    responses(
        (status = 200, description = "OK", body = CatalogInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Catalogs",
)]
pub async fn get_catalog(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<CatalogInfo>> {
    let info = state.metastore.catalogs().get(&name).await?;
    let access = Access::load(&state, &principal.username).await?;
    access.check(rules::get_catalog(&access, &lineage_of(&info)))?;
    Ok(Json(info))
}

#[utoipa::path(
    patch,
    path = "/catalogs/{name}",
    params(("name" = String, Path, description = "Catalog name")),
    request_body = UpdateCatalog,
    responses(
        (status = 200, description = "OK", body = CatalogInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Catalogs",
)]
pub async fn update_catalog(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateCatalog>,
) -> ApiResult<Json<CatalogInfo>> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Catalog,
        &name,
        rules::update_catalog,
    )
    .await?;
    let info = state
        .metastore
        .catalogs()
        .update(&name, req, principal.actor())
        .await?;
    Ok(Json(info))
}

#[utoipa::path(
    delete,
    path = "/catalogs/{name}",
    params(
        ("name" = String, Path, description = "Catalog name"),
        ("force" = Option<bool>, Query, description = "Cascade delete child schemas/tables"),
    ),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "Catalogs",
)]
pub async fn delete_catalog(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Query(params): Query<DeleteParams>,
) -> ApiResult<StatusCode> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Catalog,
        &name,
        rules::delete_catalog,
    )
    .await?;
    state
        .metastore
        .catalogs()
        .delete(&name, params.force)
        .await?;
    Ok(StatusCode::OK)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_catalog, list_catalogs))
        .routes(routes!(get_catalog, update_catalog, delete_catalog))
}
