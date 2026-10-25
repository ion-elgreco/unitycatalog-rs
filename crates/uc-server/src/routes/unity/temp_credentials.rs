//! Temporary credential vending — the four `/temporary-*-credentials`
//! endpoints, after upstream's `Temporary*CredentialsService`.
//!
//! Each endpoint resolves the storage location of its target (or takes the
//! path), checks the request as upstream does, maps the operation to
//! storage privileges and calls
//! [`vend_credential`](crate::storage::vendor::vend_credential).
//! Each checks the caller's privileges as upstream's expression does.

use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    GenerateTemporaryModelVersionCredential, GenerateTemporaryPathCredential,
    GenerateTemporaryTableCredential, GenerateTemporaryVolumeCredential, ModelVersionOperation,
    ModelVersionStatus, PathOperation, TableOperation, TemporaryCredentials, VolumeOperation,
};
use uc_common::{Error, NormalizedUrl, SecurableType, three_part_full_name};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, PathOwner, rules},
    http::error::ApiResult,
    http::extract::Json,
    server::state::AppState,
    storage::vendor::{
        StoragePrivilege::{self, Select, Update},
        vend_credential,
    },
};

/// Credentials for the storage `location` of a securable; the Delta API
/// vends through this too.
pub(crate) async fn vend(
    state: &AppState,
    location: Option<String>,
    privileges: &[StoragePrivilege],
) -> ApiResult<TemporaryCredentials> {
    let location = location.ok_or_else(|| Error::invalid_argument("Storage location is null."))?;
    let path = NormalizedUrl::parse(&location)?;
    Ok(vend_credential(&state.metastore, &state.vendor, &path, privileges).await?)
}

#[utoipa::path(
    post, path = "/temporary-table-credentials",
    request_body = GenerateTemporaryTableCredential,
    responses(
        (status = 200, body = TemporaryCredentials),
        (status = 404, description = "Table or matching external_location not found"),
    ),
    tag = "TemporaryCredentials",
)]
pub async fn temp_table_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<GenerateTemporaryTableCredential>,
) -> ApiResult<Json<TemporaryCredentials>> {
    let write = req.operation != TableOperation::Read;
    authz::check_table_id(&state, &principal.username, &req.table_id, |a, l| {
        rules::vend_table(a, l, write)
    })
    .await?;
    let table_id = uc_delta::commits::parse_table_id(&req.table_id)?;
    // A table id or, while a MANAGED table is being created, a staging
    // table id (upstream `getStorageLocationForTableOrStagingTable`).
    let (location, table_type) = state
        .metastore
        .tables()
        .storage_location_by_id(&table_id)
        .await?;
    state.check_delta_api_only_for_managed(
        Some(table_type),
        "GET /delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/credentials \
         (or /delta/v1/staging-tables/{table_id}/credentials for unfinalized staging)",
    )?;
    let privileges: &[StoragePrivilege] = match req.operation {
        TableOperation::Read => &[Select],
        TableOperation::ReadWrite => &[Select, Update],
        TableOperation::UnknownTableOperation => &[],
    };
    vend(&state, location, privileges).await.map(Json)
}

#[utoipa::path(
    post, path = "/temporary-volume-credentials",
    request_body = GenerateTemporaryVolumeCredential,
    responses(
        (status = 200, body = TemporaryCredentials),
        (status = 404, description = "Volume or matching external_location not found"),
    ),
    tag = "TemporaryCredentials",
)]
pub async fn temp_volume_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<GenerateTemporaryVolumeCredential>,
) -> ApiResult<Json<TemporaryCredentials>> {
    if req.volume_id.is_empty() {
        return Err(Error::invalid_argument("Volume ID is required.").into());
    }
    let access = Access::load(&state, &principal.username).await?;
    if access.enabled() {
        let lineage = state
            .metastore
            .permissions()
            .lineage_by_id(SecurableType::Volume, &req.volume_id)
            .await?;
        let write = req.operation != VolumeOperation::ReadVolume;
        access.check(rules::vend_volume(&access, &lineage, write))?;
    }
    let volume = state.metastore.volumes().get_by_id(&req.volume_id).await?;
    let privileges: &[StoragePrivilege] = match req.operation {
        VolumeOperation::ReadVolume => &[Select],
        VolumeOperation::WriteVolume => &[Select, Update],
        VolumeOperation::UnknownVolumeOperation => &[],
    };
    vend(&state, volume.storage_location, privileges)
        .await
        .map(Json)
}

#[utoipa::path(
    post, path = "/temporary-model-version-credentials",
    request_body = GenerateTemporaryModelVersionCredential,
    responses(
        (status = 200, body = TemporaryCredentials),
        (status = 404, description = "Model version or matching external_location not found"),
    ),
    tag = "TemporaryCredentials",
)]
pub async fn temp_model_version_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<GenerateTemporaryModelVersionCredential>,
) -> ApiResult<Json<TemporaryCredentials>> {
    let full_name = three_part_full_name(&req.catalog_name, &req.schema_name, &req.model_name);
    let write = req.operation != ModelVersionOperation::ReadModelVersion;
    authz::check(
        &state,
        &principal.username,
        SecurableType::RegisteredModel,
        &full_name,
        |a, l| rules::vend_model(a, l, write),
    )
    .await?;
    let version = req.version;
    let mv = state
        .metastore
        .models()
        .get_version(&full_name, version)
        .await?;
    // Upstream `generateTemporaryModelVersionCredentials`.
    let location = mv.storage_location.clone().unwrap_or_default();
    if location.to_lowercase().starts_with("file") {
        return Err(Error::invalid_argument(format!(
            "Cannot request credentials on a model version with a file based storage location: \
             {full_name}/{version}"
        ))
        .into());
    }
    let status = mv
        .status
        .unwrap_or(ModelVersionStatus::ModelVersionStatusUnknown);
    if matches!(
        status,
        ModelVersionStatus::FailedRegistration | ModelVersionStatus::ModelVersionStatusUnknown
    ) {
        return Err(Error::invalid_argument(format!(
            "Cannot request credentials on a model version with status {}: {full_name}/{version}",
            serde_json::to_value(status)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        ))
        .into());
    }
    if status != ModelVersionStatus::PendingRegistration
        && req.operation == ModelVersionOperation::ReadWriteModelVersion
    {
        return Err(Error::invalid_argument(format!(
            "Cannot request read/write credentials on a model version that has been finalized: \
             {full_name}/{version}"
        ))
        .into());
    }
    let privileges: &[StoragePrivilege] = match req.operation {
        ModelVersionOperation::ReadModelVersion => &[Select],
        ModelVersionOperation::ReadWriteModelVersion => &[Select, Update],
        ModelVersionOperation::UnknownModelVersionOperation => {
            return Err(Error::invalid_argument(
                "Unknown operation in the request: UNKNOWN_MODEL_VERSION_OPERATION",
            )
            .into());
        }
    };
    vend(&state, mv.storage_location, privileges)
        .await
        .map(Json)
}

#[utoipa::path(
    post, path = "/temporary-path-credentials",
    request_body = GenerateTemporaryPathCredential,
    responses(
        (status = 200, body = TemporaryCredentials),
        (status = 404, description = "No matching external_location"),
    ),
    tag = "TemporaryCredentials",
)]
pub async fn temp_path_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<GenerateTemporaryPathCredential>,
) -> ApiResult<Json<TemporaryCredentials>> {
    let access = Access::load(&state, &principal.username).await?;
    if access.enabled() {
        let owner = authz::path_owner(&state, &req.url).await?;
        if owner == PathOwner::Unowned {
            return Err(authz::uncovered(&req.url).into());
        }
        access.check(path_allowed(&access, &owner, req.operation))?;
    }
    let privileges: &[StoragePrivilege] = match req.operation {
        PathOperation::PathRead => &[Select],
        PathOperation::PathReadWrite | PathOperation::PathCreateTable => &[Select, Update],
        PathOperation::UnknownPathOperation => &[],
    };
    vend(&state, Some(req.url), privileges).await.map(Json)
}

/// Upstream's path credential rule: by the operation and what owns the
/// path (a table, volume or model, else an external location).
fn path_allowed(a: &Access, owner: &PathOwner, operation: PathOperation) -> bool {
    use uc_api::Privilege::*;
    let object = authz::Level::Object;
    match (operation, owner) {
        (PathOperation::PathRead, PathOwner::Data(kind, l)) => {
            let read = match kind {
                SecurableType::Table => Select,
                SecurableType::Volume => ReadVolume,
                _ => Execute,
            };
            rules::use_parents(a, l) && a.owner_or_any(l, object, &[read])
        }
        (PathOperation::PathRead, PathOwner::ExternalLocation(l)) => {
            a.owner_or_any(l, object, &[ReadFiles])
        }
        (PathOperation::PathReadWrite, PathOwner::Data(SecurableType::Table, l)) => {
            rules::update_table(a, l)
        }
        (PathOperation::PathReadWrite, PathOwner::Data(_, l)) => {
            rules::use_parents(a, l) && a.owner(l, object)
        }
        (PathOperation::PathReadWrite, PathOwner::ExternalLocation(l)) => {
            a.owner(l, object) || a.all(l, object, &[ReadFiles, WriteFiles])
        }
        (PathOperation::PathCreateTable, PathOwner::ExternalLocation(l)) => {
            a.owner_or_any(l, object, &[CreateExternalTable])
        }
        _ => false,
    }
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(temp_table_credentials))
        .routes(routes!(temp_volume_credentials))
        .routes(routes!(temp_model_version_credentials))
        .routes(routes!(temp_path_credentials))
}
