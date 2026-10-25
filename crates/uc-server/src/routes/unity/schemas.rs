use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{CreateSchema, ListSchemasResponse, SchemaInfo, UpdateSchema};
use uc_common::{DeleteParams, PageToken, Pagination, SecurableType};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules, schema_lineage},
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
};

/// Schema list parameters: `catalog_name` and the page, without the
/// `schema_name` of `SchemaScopedListParams`.
#[derive(Debug, Deserialize)]
pub struct ListSchemasParams {
    pub catalog_name: String,
    #[serde(flatten)]
    pub page: Pagination,
}

#[utoipa::path(
    post,
    path = "/schemas",
    request_body = CreateSchema,
    responses(
        (status = 200, description = "Created", body = SchemaInfo),
        (status = 404, description = "Parent catalog not found"),
        (status = 409, description = "Already exists"),
    ),
    tag = "Schemas",
)]
pub async fn create_schema(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateSchema>,
) -> ApiResult<Json<SchemaInfo>> {
    let catalog = authz::lineage(&state, SecurableType::Catalog, &req.catalog_name).await?;
    let access = Access::load(&state, &principal.username).await?;
    let storage_root =
        authz::storage_root_allowed(&state, &access, req.storage_root.as_deref()).await?;
    access.check(rules::create_schema(&access, &catalog) && storage_root.unwrap_or(true))?;
    Ok(Json(
        state
            .metastore
            .schemas()
            .create(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/schemas",
    params(
        ("catalog_name" = String, Query, description = "Parent catalog name"),
        ("max_results"  = Option<u32>, Query),
        ("page_token"   = Option<String>, Query),
    ),
    responses((status = 200, body = ListSchemasResponse)),
    tag = "Schemas",
)]
pub async fn list_schemas(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ListSchemasParams>,
) -> ApiResult<Json<ListSchemasResponse>> {
    let (schemas, next) = state
        .metastore
        .schemas()
        .list(&params.catalog_name, &params.page)
        .await?;
    let access = Access::load(&state, &principal.username).await?;
    let schemas = match access.enabled() {
        false => schemas,
        true => {
            let catalog =
                authz::lineage(&state, SecurableType::Catalog, &params.catalog_name).await?;
            schemas
                .into_iter()
                .filter(|s| rules::get_schema(&access, &schema_lineage(&catalog, s)))
                .collect()
        }
    };
    Ok(Json(ListSchemasResponse {
        schemas,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get,
    path = "/schemas/{full_name}",
    params(("full_name" = String, Path, description = "`<catalog>.<schema>`")),
    responses(
        (status = 200, body = SchemaInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Schemas",
)]
pub async fn get_schema(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
) -> ApiResult<Json<SchemaInfo>> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Schema,
        &full_name,
        rules::get_schema,
    )
    .await?;
    Ok(Json(state.metastore.schemas().get(&full_name).await?))
}

#[utoipa::path(
    patch,
    path = "/schemas/{full_name}",
    params(("full_name" = String, Path)),
    request_body = UpdateSchema,
    responses(
        (status = 200, body = SchemaInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Schemas",
)]
pub async fn update_schema(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
    Json(req): Json<UpdateSchema>,
) -> ApiResult<Json<SchemaInfo>> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Schema,
        &full_name,
        rules::update_schema,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .schemas()
            .update(&full_name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/schemas/{full_name}",
    params(
        ("full_name" = String, Path),
        ("force" = Option<bool>, Query, description = "Cascade delete child tables"),
    ),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 412, description = "Schema not empty and force=false"),
    ),
    tag = "Schemas",
)]
pub async fn delete_schema(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
    Query(params): Query<DeleteParams>,
) -> ApiResult<EmptyObject> {
    authz::check(
        &state,
        &principal.username,
        SecurableType::Schema,
        &full_name,
        rules::update_schema,
    )
    .await?;
    state
        .metastore
        .schemas()
        .delete(&full_name, params.force)
        .await?;
    Ok(EmptyObject)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_schema, list_schemas))
        .routes(routes!(get_schema, update_schema, delete_schema))
}
