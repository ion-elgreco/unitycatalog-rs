//! The UC Delta REST API (`/delta/v1`), after upstream `DeltaApiService`.
//! Errors use the Delta body (`{"error": {"message", "type", "code"}}`),
//! see [`error_layer`].

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateStagingTable, TemporaryCredentials,
    delta_api::{
        DeltaCatalogConfig, DeltaCreateStagingTableRequest, DeltaCreateTableRequest,
        DeltaCredentialOperation, DeltaCredentialsResponse, DeltaErrorModel, DeltaErrorResponse,
        DeltaLoadTableResponse, DeltaProtocol, DeltaRenameTableRequest, DeltaReportMetricsRequest,
        DeltaStagingTableResponse, DeltaStorageCredential, DeltaStorageCredentialConfig,
        DeltaSuggestedProtocol, DeltaTableType, DeltaUpdateTableRequest,
    },
};
use uc_common::{Error, ErrorResponse, SecurableType, three_part_full_name};
use uc_delta::contract;

use crate::{
    authn::principal::Principal,
    authz::{self, rules},
    http::error::ApiResult,
    http::extract::{Json, Path, Query},
    routes::unity::temp_credentials,
    server::state::AppState,
    storage::vendor::VendOp,
};

/// `{catalog}/{schema}/{table}` path parameters.
type TablePath = Path<(String, String, String)>;

/// The endpoints `/delta/v1/config` advertises, as upstream (which also
/// lists a table listing endpoint).
const ENDPOINTS: &[&str] = &[
    "POST /v1/catalogs/{catalog}/schemas/{schema}/staging-tables",
    "POST /v1/catalogs/{catalog}/schemas/{schema}/tables",
    "GET /v1/catalogs/{catalog}/schemas/{schema}/tables",
    "GET /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    "POST /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    "DELETE /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    "HEAD /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    "POST /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/rename",
    "GET /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/credentials",
    "POST /v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/metrics",
    "GET /v1/staging-tables/{table_id}/credentials",
    "GET /v1/temporary-path-credentials",
];

/// Upstream `DeltaApiExceptionHandler`: rewrite the UC error of every
/// `/delta/v1/` response into the Delta body, with the Delta status of the
/// error code (e.g. 409 for any "already exists").
pub async fn error_layer(req: Request, next: Next) -> Response {
    let delta = req.uri().path().contains("/delta/v1/");
    let resp = next.run(req).await;
    let Some(uc) = resp.extensions().get::<ErrorResponse>().filter(|_| delta) else {
        return resp;
    };
    let code = uc.error_code;
    let status = code.delta_status();
    let body = DeltaErrorResponse {
        error: DeltaErrorModel {
            message: uc.message.clone(),
            error_type: code.delta_error_type().to_string(),
            code: status.as_u16(),
            stack: None,
        },
    };
    (status, axum::Json(body)).into_response()
}

/// The only protocol version of the Delta API that the server implements.
const PROTOCOL_VERSION: &str = "1.0";

#[derive(Debug, Deserialize)]
pub struct ConfigParams {
    pub catalog: Option<String>,
    #[serde(rename = "protocol-versions")]
    pub protocol_versions: Option<String>,
}

/// The protocol version for the client's `protocol-versions`: its highest
/// version per major version, such as `1.1,2.3`. Any 1.x covers 1.0.
fn negotiate(versions: Option<&str>) -> Result<&'static str, Error> {
    let supported = format!("Server supports protocol versions: {PROTOCOL_VERSION}.");
    let Some(versions) = versions.map(str::trim).filter(|v| !v.is_empty()) else {
        return Err(Error::invalid_argument(format!(
            "Must supply the protocol-versions parameter. {supported}"
        )));
    };
    let majors: Option<Vec<u32>> = versions
        .split(',')
        .map(|entry| {
            let (major, minor) = entry.trim().split_once('.')?;
            minor.parse::<u32>().ok()?;
            major.parse().ok()
        })
        .collect();
    match majors {
        Some(majors) if majors.contains(&1) => Ok(PROTOCOL_VERSION),
        Some(_) => Err(Error::invalid_argument(format!(
            "No mutually supported protocol version: client supports \"{versions}\". {supported}"
        ))),
        None => Err(Error::invalid_argument(format!(
            "Invalid protocol-versions parameter \"{versions}\": expected <major>.<minor> per \
             major version. {supported}"
        ))),
    }
}

#[utoipa::path(
    get, path = "/delta/v1/config",
    operation_id = "delta_get_config",
    params(
        ("catalog" = String, Query, description = "Catalog name"),
        ("protocol-versions" = String, Query, description = "Highest supported version per major version"),
    ),
    responses(
        (status = 200, body = DeltaCatalogConfig),
        (status = 400, body = DeltaErrorResponse),
        (status = 404, body = DeltaErrorResponse),
    ),
    tag = "DeltaConfiguration",
)]
pub async fn get_config(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ConfigParams>,
) -> ApiResult<Json<DeltaCatalogConfig>> {
    let catalog = params.catalog.filter(|c| !c.is_empty()).ok_or_else(|| {
        Error::invalid_argument("Must supply a proper catalog in catalog parameter.")
    })?;
    let protocol_version = negotiate(params.protocol_versions.as_deref())?;
    let lineage = authz::lineage(&state, SecurableType::Catalog, &catalog).await?;
    let access = authz::Access::load(&state, &principal).await?;
    access.check(authz::rules::get_catalog(&access, &lineage))?;
    Ok(Json(DeltaCatalogConfig {
        endpoints: ENDPOINTS
            .iter()
            .map(|e| e.replace("{catalog}", &catalog))
            .collect(),
        protocol_version: protocol_version.to_string(),
    }))
}

/// Upstream `loadTableForDelta`.
async fn load(
    state: &AppState,
    catalog: &str,
    schema: &str,
    table: &str,
) -> ApiResult<DeltaLoadTableResponse> {
    let full_name = three_part_full_name(catalog, schema, table);
    Ok(state
        .metastore
        .delta_tables()
        .load_table(&full_name)
        .await?)
}

#[utoipa::path(
    get, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    operation_id = "delta_load_table",
    params(("catalog" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    responses(
        (status = 200, body = DeltaLoadTableResponse),
        (status = 400, body = DeltaErrorResponse),
        (status = 404, body = DeltaErrorResponse),
    ),
    tag = "DeltaTables",
)]
pub async fn load_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema, table)): TablePath,
) -> ApiResult<Json<DeltaLoadTableResponse>> {
    let full_name = three_part_full_name(&catalog, &schema, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::get_table,
    )
    .await?;
    Ok(Json(load(&state, &catalog, &schema, &table).await?))
}

#[utoipa::path(
    post, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    operation_id = "delta_update_table",
    params(("catalog" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    request_body = DeltaUpdateTableRequest,
    responses(
        (status = 200, body = DeltaLoadTableResponse),
        (status = 400, body = DeltaErrorResponse),
        (status = 404, body = DeltaErrorResponse),
        (status = 409, body = DeltaErrorResponse, description = "Commit version or requirement conflict"),
        (status = 429, body = DeltaErrorResponse, description = "Too many unbackfilled commits"),
    ),
    tag = "DeltaTables",
)]
pub async fn update_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema, table)): TablePath,
    Json(req): Json<DeltaUpdateTableRequest>,
) -> ApiResult<Json<DeltaLoadTableResponse>> {
    let full_name = three_part_full_name(&catalog, &schema, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::update_table,
    )
    .await?;
    let outcome = state
        .metastore
        .delta_tables()
        .update_table(&full_name, req, principal.actor())
        .await?;
    // A replay of a purged version is settled by the file contents.
    super::unity::commits::verify_replay(&state, outcome).await?;
    Ok(Json(load(&state, &catalog, &schema, &table).await?))
}

#[utoipa::path(
    head, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    operation_id = "delta_table_exists",
    params(("catalog" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    responses((status = 204, description = "The table exists"), (status = 404, body = DeltaErrorResponse)),
    tag = "DeltaTables",
)]
pub async fn table_exists(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema, table)): TablePath,
) -> ApiResult<StatusCode> {
    let full_name = three_part_full_name(&catalog, &schema, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::get_table,
    )
    .await?;
    state.metastore.tables().get(&full_name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
    operation_id = "delta_delete_table",
    params(("catalog" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    responses((status = 204, description = "Deleted"), (status = 404, body = DeltaErrorResponse)),
    tag = "DeltaTables",
)]
pub async fn delete_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema, table)): TablePath,
) -> ApiResult<StatusCode> {
    let full_name = three_part_full_name(&catalog, &schema, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::own_child,
    )
    .await?;
    state.metastore.tables().delete(&full_name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/rename",
    operation_id = "delta_rename_table",
    params(("catalog" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    request_body = DeltaRenameTableRequest,
    responses(
        (status = 204, description = "Renamed"),
        (status = 404, body = DeltaErrorResponse),
        (status = 409, body = DeltaErrorResponse),
    ),
    tag = "DeltaTables",
)]
pub async fn rename_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema, table)): TablePath,
    Json(req): Json<DeltaRenameTableRequest>,
) -> ApiResult<StatusCode> {
    let full_name = three_part_full_name(&catalog, &schema, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::rename_table,
    )
    .await?;
    let new_name = req
        .new_name
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| Error::invalid_argument("New table name is required."))?;
    state
        .metastore
        .tables()
        .rename(&full_name, &new_name, principal.actor())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/metrics",
    operation_id = "delta_report_metrics",
    params(("catalog" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    request_body = DeltaReportMetricsRequest,
    responses((status = 204, description = "Received"), (status = 400, body = DeltaErrorResponse)),
    tag = "DeltaTables",
)]
pub async fn report_metrics(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema, table)): TablePath,
    Json(req): Json<DeltaReportMetricsRequest>,
) -> ApiResult<StatusCode> {
    let full_name = three_part_full_name(&catalog, &schema, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::update_table,
    )
    .await?;
    // Upstream acknowledges the metrics and does not store them.
    let table_id = req
        .table_id
        .ok_or_else(|| Error::invalid_argument("table-id is required."))?;
    let table_id = uc_delta::commits::parse_table_id(&table_id)?;
    let info = state.metastore.tables().get(&full_name).await?;
    if info.table_id.as_deref() != Some(table_id.as_str()) {
        return Err(Error::invalid_argument(format!(
            "table-id {table_id} does not match table {catalog}.{schema}.{table}."
        ))
        .into());
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Upstream `DeltaCredentialsMapper`: one credential for the vended path.
fn credentials_response(
    credentials: TemporaryCredentials,
    operation: DeltaCredentialOperation,
) -> DeltaCredentialsResponse {
    let mut config = DeltaStorageCredentialConfig::default();
    if let Some(aws) = credentials.aws_temp_credentials {
        config.s3_access_key_id = aws.access_key_id;
        config.s3_secret_access_key = aws.secret_access_key;
        config.s3_session_token = aws.session_token;
    }
    if let Some(azure) = credentials.azure_user_delegation_sas {
        config.azure_sas_token = azure.sas_token;
    }
    if let Some(gcp) = credentials.gcp_oauth_token {
        config.gcs_oauth_token = gcp.oauth_token;
    }
    DeltaCredentialsResponse {
        storage_credentials: vec![DeltaStorageCredential {
            prefix: credentials.url.unwrap_or_default(),
            operation,
            config,
            expiration_time_ms: credentials.expiration_time,
        }],
    }
}

async fn vend_for(
    state: &AppState,
    location: Option<String>,
    operation: DeltaCredentialOperation,
) -> ApiResult<DeltaCredentialsResponse> {
    let op = match operation {
        DeltaCredentialOperation::Read => VendOp::Read,
        DeltaCredentialOperation::ReadWrite => VendOp::ReadWrite,
    };
    let credentials = temp_credentials::vend(state, location, op.privileges()).await?;
    Ok(credentials_response(credentials, operation))
}

#[utoipa::path(
    post, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/staging-tables",
    operation_id = "delta_create_staging_table",
    params(("catalog" = String, Path), ("schema" = String, Path)),
    request_body = DeltaCreateStagingTableRequest,
    responses(
        (status = 200, body = DeltaStagingTableResponse),
        (status = 404, body = DeltaErrorResponse),
        (status = 409, body = DeltaErrorResponse),
    ),
    tag = "DeltaTables",
)]
pub async fn create_staging_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema)): Path<(String, String)>,
    Json(req): Json<DeltaCreateStagingTableRequest>,
) -> ApiResult<Json<DeltaStagingTableResponse>> {
    let parent = uc_common::schema_full_name(&catalog, &schema);
    authz::check(
        &state,
        &principal,
        SecurableType::Schema,
        &parent,
        rules::create_staging_table,
    )
    .await?;
    let name = req
        .name
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| Error::invalid_argument("Staging table name is required."))?;
    let staging = state
        .metastore
        .tables()
        .create_staging(
            CreateStagingTable {
                name,
                catalog_name: catalog,
                schema_name: schema,
            },
            principal.actor(),
        )
        .await?;
    let table_id = staging.id.unwrap_or_default();
    let credentials = vend_for(
        &state,
        staging.staging_location.clone(),
        DeltaCredentialOperation::ReadWrite,
    )
    .await?;
    Ok(Json(DeltaStagingTableResponse {
        table_id: table_id.clone(),
        table_type: DeltaTableType::Managed,
        location: staging.staging_location.unwrap_or_default(),
        storage_credentials: credentials.storage_credentials,
        required_protocol: DeltaProtocol {
            min_reader_version: Some(contract::REQUIRED_MIN_READER_VERSION),
            min_writer_version: Some(contract::REQUIRED_MIN_WRITER_VERSION),
            reader_features: Some(contract::required_reader_features()),
            writer_features: Some(contract::required_writer_features()),
        },
        suggested_protocol: DeltaSuggestedProtocol {
            reader_features: contract::suggested_reader_features(),
            writer_features: contract::suggested_writer_features(),
        },
        required_properties: contract::required_properties(&table_id),
        suggested_properties: contract::suggested_properties(),
    }))
}

#[utoipa::path(
    post, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables",
    operation_id = "delta_create_table",
    params(("catalog" = String, Path), ("schema" = String, Path)),
    request_body = DeltaCreateTableRequest,
    responses(
        (status = 200, body = DeltaLoadTableResponse),
        (status = 400, body = DeltaErrorResponse),
        (status = 404, body = DeltaErrorResponse),
        (status = 409, body = DeltaErrorResponse),
    ),
    tag = "DeltaTables",
)]
pub async fn create_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema)): Path<(String, String)>,
    Json(req): Json<DeltaCreateTableRequest>,
) -> ApiResult<Json<DeltaLoadTableResponse>> {
    let external = match req.table_type {
        Some(DeltaTableType::External) => Some(req.location.as_deref()),
        _ => None,
    };
    authz::check_create(
        &state,
        &principal,
        &uc_common::schema_full_name(&catalog, &schema),
        external,
        rules::create_table,
    )
    .await?;
    // Upstream `DeltaCreateTableMapper` then the shared UC create: a MANAGED
    // table takes over the staging table at its location.
    let created = state
        .metastore
        .delta_tables()
        .create_table(&catalog, &schema, &req, principal.actor())
        .await?;
    crate::observability::metrics::table_created();
    Ok(Json(created))
}

#[derive(Debug, Deserialize)]
pub struct OperationParams {
    pub operation: DeltaCredentialOperation,
}

#[utoipa::path(
    get, path = "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/credentials",
    operation_id = "delta_table_credentials",
    params(
        ("catalog" = String, Path), ("schema" = String, Path), ("table" = String, Path),
        ("operation" = DeltaCredentialOperation, Query),
    ),
    responses((status = 200, body = DeltaCredentialsResponse), (status = 404, body = DeltaErrorResponse)),
    tag = "DeltaTemporaryCredentials",
)]
pub async fn table_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, schema, table)): TablePath,
    Query(params): Query<OperationParams>,
) -> ApiResult<Json<DeltaCredentialsResponse>> {
    let write = params.operation == DeltaCredentialOperation::ReadWrite;
    let full_name = three_part_full_name(&catalog, &schema, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        |a, l| rules::vend_table(a, l, write),
    )
    .await?;
    let info = state.metastore.tables().get(&full_name).await?;
    Ok(Json(
        vend_for(&state, info.storage_location, params.operation).await?,
    ))
}

#[utoipa::path(
    get, path = "/delta/v1/staging-tables/{table_id}/credentials",
    operation_id = "delta_staging_table_credentials",
    params(("table_id" = String, Path, description = "Staging table UUID")),
    responses((status = 200, body = DeltaCredentialsResponse), (status = 404, body = DeltaErrorResponse)),
    tag = "DeltaTemporaryCredentials",
)]
pub async fn staging_table_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Path(table_id): Path<String>,
) -> ApiResult<Json<DeltaCredentialsResponse>> {
    // Upstream: the staging table's OWNER, its creator.
    authz::check_table_id(&state, &principal, &table_id, |a, l| {
        a.owner(l, authz::Level::Object)
    })
    .await?;
    // Always read-write: the client writes the first commit.
    let table_id = uc_delta::commits::parse_table_id(&table_id)?;
    let location = state.metastore.tables().staging_location(&table_id).await?;
    Ok(Json(
        vend_for(&state, Some(location), DeltaCredentialOperation::ReadWrite).await?,
    ))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_config))
        .routes(routes!(create_staging_table))
        .routes(routes!(create_table))
        .routes(routes!(
            load_table,
            update_table,
            table_exists,
            delete_table
        ))
        .routes(routes!(rename_table))
        .routes(routes!(report_metrics))
        .routes(routes!(table_credentials))
        .routes(routes!(staging_table_credentials))
}
