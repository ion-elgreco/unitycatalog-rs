use crate::http::extract::{Json, Path, Query};
use axum::{extract::State, http::StatusCode};
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{CreateStagingTable, CreateTable, ListTablesResponse, StagingTableInfo, TableInfo};
use uc_common::{
    PageToken, SchemaScopedListParams as ListTablesParams, SecurableType, schema_full_name,
};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules},
    http::error::ApiResult,
    server::state::AppState,
};

#[utoipa::path(
    post,
    path = "/tables",
    request_body = CreateTable,
    responses(
        (status = 200, body = TableInfo),
        (status = 404, description = "Parent schema not found"),
        (status = 409, description = "Already exists"),
    ),
    tag = "Tables",
)]
pub async fn create_table(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateTable>,
) -> ApiResult<Json<TableInfo>> {
    let parent = schema_full_name(&req.catalog_name, &req.schema_name);
    let external = match req.table_type {
        uc_api::TableType::External => Some(req.storage_location.as_deref()),
        _ => None,
    };
    authz::check_create(
        &state,
        &principal.username,
        &parent,
        external,
        rules::create_table,
    )
    .await?;
    state.check_delta_api_only_for_managed(
        Some(req.table_type),
        "POST /delta/v1/catalogs/{catalog}/schemas/{schema}/tables",
    )?;
    let created = state
        .metastore
        .tables()
        .create(req, principal.actor())
        .await?;
    crate::observability::metrics::table_created();
    Ok(Json(created))
}

#[utoipa::path(
    get,
    path = "/tables",
    params(
        ("catalog_name" = String, Query),
        ("schema_name"  = String, Query),
        ("max_results"  = Option<u32>, Query),
        ("page_token"   = Option<String>, Query),
    ),
    responses((status = 200, body = ListTablesResponse)),
    tag = "Tables",
)]
pub async fn list_tables(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ListTablesParams>,
) -> ApiResult<Json<ListTablesResponse>> {
    let (tables, next) = state
        .metastore
        .tables()
        .list(&params.catalog_name, &params.schema_name, &params.page)
        .await?;
    let access = Access::load(&state, &principal.username).await?;
    let schema = schema_full_name(&params.catalog_name, &params.schema_name);
    let tables = authz::filter_children(
        &state,
        &access,
        SecurableType::Table,
        tables,
        |t| (schema.clone(), t.table_id.clone(), t.owner.clone()),
        rules::get_table,
    )
    .await?;
    Ok(Json(ListTablesResponse {
        tables,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get,
    path = "/tables/{full_name}",
    params(("full_name" = String, Path, description = "`<catalog>.<schema>.<table>`")),
    responses(
        (status = 200, body = TableInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Tables",
)]
pub async fn get_table(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
) -> ApiResult<Json<TableInfo>> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Table,
        &full_name,
        rules::get_table,
    )
    .await?;
    Ok(Json(state.metastore.tables().get(&full_name).await?))
}

#[utoipa::path(
    delete,
    path = "/tables/{full_name}",
    params(("full_name" = String, Path)),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "Tables",
)]
pub async fn delete_table(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
) -> ApiResult<StatusCode> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Table,
        &full_name,
        rules::own_child,
    )
    .await?;
    state.metastore.tables().delete(&full_name).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/staging-tables",
    request_body = CreateStagingTable,
    responses(
        (status = 200, body = StagingTableInfo),
        (status = 400, description = "Invalid argument"),
    ),
    tag = "Staging Tables",
)]
pub async fn create_staging_table(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateStagingTable>,
) -> ApiResult<Json<StagingTableInfo>> {
    let parent = schema_full_name(&req.catalog_name, &req.schema_name);
    authz::check(
        &state,
        &principal.username,
        SecurableType::Schema,
        &parent,
        rules::create_staging_table,
    )
    .await?;
    // Staging tables only become MANAGED Delta tables.
    state.check_delta_api_only(
        "POST /delta/v1/catalogs/{catalog}/schemas/{schema}/staging-tables",
    )?;
    Ok(Json(
        state
            .metastore
            .tables()
            .create_staging(req, principal.actor())
            .await?,
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_table, list_tables))
        .routes(routes!(get_table, delete_table))
}

pub fn staging_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(create_staging_table))
}
