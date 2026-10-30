use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{CreateFunctionRequest, FunctionInfo, ListFunctionsResponse};
use uc_common::{PageToken, SchemaScopedListParams as ListParams, SecurableType, schema_full_name};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules},
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
};

#[utoipa::path(
    post, path = "/functions",
    request_body = CreateFunctionRequest,
    responses(
        (status = 200, body = FunctionInfo),
        (status = 404, description = "Parent schema not found"),
        (status = 409, description = "Already exists"),
    ),
    tag = "Functions",
)]
pub async fn create_function(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateFunctionRequest>,
) -> ApiResult<Json<FunctionInfo>> {
    let info = &req.function_info;
    let parent = schema_full_name(
        info.catalog_name.as_deref().unwrap_or_default(),
        info.schema_name.as_deref().unwrap_or_default(),
    );
    authz::check(
        &state,
        &principal,
        SecurableType::Schema,
        &parent,
        rules::create_function,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .functions()
            .create(req.function_info, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/functions",
    params(
        ("catalog_name" = String, Query),
        ("schema_name"  = String, Query),
        ("max_results"  = Option<u32>, Query),
        ("page_token"   = Option<String>, Query),
    ),
    responses((status = 200, body = ListFunctionsResponse)),
    tag = "Functions",
)]
pub async fn list_functions(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<ListFunctionsResponse>> {
    let (functions, next) = state
        .metastore
        .functions()
        .list(&params.catalog_name, &params.schema_name, &params.page)
        .await?;
    let access = Access::load(&state, &principal).await?;
    let schema = schema_full_name(&params.catalog_name, &params.schema_name);
    let functions = authz::filter_children(
        &state,
        &access,
        SecurableType::Function,
        functions,
        |f| (schema.clone(), f.function_id.clone(), f.owner.clone()),
        rules::get_function,
    )
    .await?;
    Ok(Json(ListFunctionsResponse {
        functions,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get, path = "/functions/{name}",
    params(("name" = String, Path)),
    responses(
        (status = 200, body = FunctionInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "Functions",
)]
pub async fn get_function(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<FunctionInfo>> {
    authz::check(
        &state,
        &principal,
        SecurableType::Function,
        &name,
        rules::get_function,
    )
    .await?;
    Ok(Json(state.metastore.functions().get(&name).await?))
}

#[utoipa::path(
    delete, path = "/functions/{name}",
    params(("name" = String, Path)),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "Functions",
)]
pub async fn delete_function(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<EmptyObject> {
    authz::check(
        &state,
        &principal,
        SecurableType::Function,
        &name,
        rules::own_child,
    )
    .await?;
    state.metastore.functions().delete(&name).await?;
    Ok(EmptyObject)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_function, list_functions))
        .routes(routes!(get_function, delete_function))
}
