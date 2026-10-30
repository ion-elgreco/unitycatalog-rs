use crate::http::extract::{Json, Path, Query};
use axum::extract::State;
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateModelVersion, CreateRegisteredModel, FinalizeModelVersion, ListModelVersionsResponse,
    ListRegisteredModelsResponse, ModelVersionInfo, RegisteredModelInfo, UpdateModelVersion,
    UpdateRegisteredModel,
};
use uc_common::{
    DeleteParams as DeleteModelParams, Error, PageToken, Pagination, SecurableType,
    schema_full_name, three_part_full_name,
};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules},
    http::{EmptyObject, error::ApiResult},
    server::state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListModelsParams {
    pub catalog_name: Option<String>,
    pub schema_name: Option<String>,
    #[serde(flatten)]
    pub page: Pagination,
}

#[utoipa::path(
    post, path = "/models",
    request_body = CreateRegisteredModel,
    responses(
        (status = 200, body = RegisteredModelInfo),
        (status = 404, description = "Parent schema not found"),
        (status = 409, description = "Already exists"),
    ),
    tag = "RegisteredModels",
)]
pub async fn create_registered_model(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateRegisteredModel>,
) -> ApiResult<Json<RegisteredModelInfo>> {
    let parent = schema_full_name(&req.catalog_name, &req.schema_name);
    authz::check(
        &state,
        &principal,
        SecurableType::Schema,
        &parent,
        rules::create_model,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .models()
            .create_model(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/models",
    params(
        ("catalog_name" = Option<String>, Query),
        ("schema_name"  = Option<String>, Query),
        ("max_results"  = Option<u32>, Query),
        ("page_token"   = Option<String>, Query),
    ),
    responses((status = 200, body = ListRegisteredModelsResponse)),
    tag = "RegisteredModels",
)]
pub async fn list_registered_models(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ListModelsParams>,
) -> ApiResult<Json<ListRegisteredModelsResponse>> {
    let (registered_models, next) = state
        .metastore
        .models()
        .list_models(
            params.catalog_name.as_deref(),
            params.schema_name.as_deref(),
            &params.page,
        )
        .await?;
    let access = Access::load(&state, &principal).await?;
    let registered_models = authz::filter_children(
        &state,
        &access,
        SecurableType::RegisteredModel,
        registered_models,
        |m| {
            (
                schema_full_name(
                    m.catalog_name.as_deref().unwrap_or_default(),
                    m.schema_name.as_deref().unwrap_or_default(),
                ),
                m.id.clone(),
                m.owner.clone(),
            )
        },
        rules::get_model,
    )
    .await?;
    Ok(Json(ListRegisteredModelsResponse {
        registered_models,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get, path = "/models/{full_name}",
    params(("full_name" = String, Path)),
    responses(
        (status = 200, body = RegisteredModelInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "RegisteredModels",
)]
pub async fn get_registered_model(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
) -> ApiResult<Json<RegisteredModelInfo>> {
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::get_model,
    )
    .await?;
    Ok(Json(state.metastore.models().get_model(&full_name).await?))
}

#[utoipa::path(
    patch, path = "/models/{full_name}",
    params(("full_name" = String, Path)),
    request_body = UpdateRegisteredModel,
    responses(
        (status = 200, body = RegisteredModelInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "RegisteredModels",
)]
pub async fn update_registered_model(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
    Json(req): Json<UpdateRegisteredModel>,
) -> ApiResult<Json<RegisteredModelInfo>> {
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::own_child,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .models()
            .update_model(&full_name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/models/{full_name}",
    params(
        ("full_name" = String, Path),
        ("force" = Option<bool>, Query, description = "Cascade delete child versions"),
    ),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 412, description = "Has versions and force=false"),
    ),
    tag = "RegisteredModels",
)]
pub async fn delete_registered_model(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
    Query(params): Query<DeleteModelParams>,
) -> ApiResult<EmptyObject> {
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::own_child,
    )
    .await?;
    state
        .metastore
        .models()
        .delete_model(&full_name, params.force)
        .await?;
    Ok(EmptyObject)
}

#[utoipa::path(
    post, path = "/models/versions",
    request_body = CreateModelVersion,
    responses(
        (status = 200, body = ModelVersionInfo),
        (status = 404, description = "Parent registered model not found"),
    ),
    tag = "ModelVersions",
)]
pub async fn create_model_version(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateModelVersion>,
) -> ApiResult<Json<ModelVersionInfo>> {
    let parent = three_part_full_name(&req.catalog_name, &req.schema_name, &req.model_name);
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &parent,
        rules::own_child,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .models()
            .create_version(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/models/{full_name}/versions",
    params(
        ("full_name" = String, Path),
        ("max_results" = Option<u32>, Query),
        ("page_token" = Option<String>, Query),
    ),
    responses((status = 200, body = ListModelVersionsResponse)),
    tag = "ModelVersions",
)]
pub async fn list_model_versions(
    State(state): State<AppState>,
    principal: Principal,
    Path(full_name): Path<String>,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListModelVersionsResponse>> {
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::get_model,
    )
    .await?;
    let (model_versions, next) = state
        .metastore
        .models()
        .list_versions(&full_name, &page)
        .await?;
    Ok(Json(ListModelVersionsResponse {
        model_versions,
        next_page_token: next.map(PageToken::new),
    }))
}

#[utoipa::path(
    get, path = "/models/{full_name}/versions/{version}",
    params(("full_name" = String, Path), ("version" = i64, Path)),
    responses(
        (status = 200, body = ModelVersionInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "ModelVersions",
)]
pub async fn get_model_version(
    State(state): State<AppState>,
    principal: Principal,
    Path((full_name, version)): Path<(String, i64)>,
) -> ApiResult<Json<ModelVersionInfo>> {
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::get_model,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .models()
            .get_version(&full_name, version)
            .await?,
    ))
}

#[utoipa::path(
    patch, path = "/models/{full_name}/versions/{version}",
    params(("full_name" = String, Path), ("version" = i64, Path)),
    request_body = UpdateModelVersion,
    responses(
        (status = 200, body = ModelVersionInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "ModelVersions",
)]
pub async fn update_model_version(
    State(state): State<AppState>,
    principal: Principal,
    Path((full_name, version)): Path<(String, i64)>,
    Json(req): Json<UpdateModelVersion>,
) -> ApiResult<Json<ModelVersionInfo>> {
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::own_child,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .models()
            .update_version(&full_name, version, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/models/{full_name}/versions/{version}",
    params(("full_name" = String, Path), ("version" = i64, Path)),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "ModelVersions",
)]
pub async fn delete_model_version(
    State(state): State<AppState>,
    principal: Principal,
    Path((full_name, version)): Path<(String, i64)>,
) -> ApiResult<EmptyObject> {
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::own_child,
    )
    .await?;
    state
        .metastore
        .models()
        .delete_version(&full_name, version)
        .await?;
    Ok(EmptyObject)
}

#[utoipa::path(
    patch, path = "/models/{full_name}/versions/{version}/finalize",
    params(("full_name" = String, Path), ("version" = i64, Path)),
    request_body = FinalizeModelVersion,
    responses(
        (status = 200, body = ModelVersionInfo),
        (status = 404, description = "Not found"),
    ),
    tag = "ModelVersions",
)]
pub async fn finalize_model_version(
    State(state): State<AppState>,
    principal: Principal,
    Path((full_name, version)): Path<(String, i64)>,
    Json(req): Json<FinalizeModelVersion>,
) -> ApiResult<Json<ModelVersionInfo>> {
    // Authorization is checked against the path, so the body must name the
    // same version.
    if req.full_name.as_deref() != Some(full_name.as_str()) || req.version != Some(version) {
        return Err(
            Error::invalid_argument("Path and request-body model version must match.").into(),
        );
    }
    authz::check(
        &state,
        &principal,
        SecurableType::RegisteredModel,
        &full_name,
        rules::own_child,
    )
    .await?;
    Ok(Json(
        state
            .metastore
            .models()
            .finalize_version(&full_name, version, principal.actor())
            .await?,
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_registered_model, list_registered_models))
        .routes(routes!(
            get_registered_model,
            update_registered_model,
            delete_registered_model,
        ))
        .routes(routes!(create_model_version))
        .routes(routes!(list_model_versions))
        .routes(routes!(
            get_model_version,
            update_model_version,
            delete_model_version,
        ))
        .routes(routes!(finalize_model_version))
}
