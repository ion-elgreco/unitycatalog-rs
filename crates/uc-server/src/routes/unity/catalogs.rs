use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{CatalogInfo, CreateCatalog, ListCatalogsResponse, UpdateCatalog};
use uc_common::{BrowseParams, DeleteParams, PageToken, Pagination, SecurableType};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, Lineage, Read, rules},
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
};

/// A catalog as the lineage of its own checks.
fn lineage_of(info: &CatalogInfo) -> Lineage {
    let catalog = authz::node(
        SecurableType::Catalog,
        info.id.as_deref(),
        info.name.as_deref().unwrap_or_default(),
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
    let access = Access::load(&state, &principal).await?;
    let storage_root =
        authz::storage_root_allowed(&state, &access, req.storage_root.as_deref()).await?;
    access.check(rules::create_catalog(&access, storage_root).await)?;
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
    params(
        Pagination,
        ("include_browse" = Option<bool>, Query, description = "Also the catalogs seen through BROWSE only"),
    ),
    responses((status = 200, description = "OK", body = ListCatalogsResponse)),
    tag = "Catalogs",
)]
pub async fn list_catalogs(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
    Query(browse): Query<BrowseParams>,
) -> ApiResult<Json<ListCatalogsResponse>> {
    let access = Access::load(&state, &principal).await?;
    let (catalogs, next) = state.metastore.catalogs().list(&page).await?;
    // Upstream filters each page, which may then hold fewer items.
    let mut kept = Vec::with_capacity(catalogs.len());
    for catalog in catalogs {
        let lineage = lineage_of(&catalog);
        if rules::get_catalog(&access, &lineage).await {
            kept.push(catalog);
        } else if browse.include_browse && rules::browse_catalog(&access, &lineage).await {
            kept.push(catalog.into_browse_only());
        }
    }
    access.failed()?;
    let catalogs = kept;
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
    Query(browse): Query<BrowseParams>,
) -> ApiResult<Json<CatalogInfo>> {
    let info = state.metastore.catalogs().get(&name).await?;
    let access = Access::load(&state, &principal).await?;
    let read = access
        .read(
            &lineage_of(&info),
            browse.include_browse,
            rules::get_catalog,
            rules::browse_catalog,
        )
        .await?;
    Ok(Json(match read {
        Read::Full => info,
        Read::BrowseOnly => info.into_browse_only(),
    }))
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
    let rename = req.new_name.is_some();
    authz::check(
        &state,
        &principal,
        SecurableType::Catalog,
        &name,
        async |a, l| rules::update_catalog(a, l, rename).await,
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
) -> ApiResult<EmptyObject> {
    authz::check(
        &state,
        &principal,
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
    Ok(EmptyObject)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_catalog, list_catalogs))
        .routes(routes!(get_catalog, update_catalog, delete_catalog))
}
