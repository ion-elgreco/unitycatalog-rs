//! The Iceberg REST catalog, after upstream `IcebergRestCatalogService`.
//! Mounted at `<API_BASE>/iceberg`; clients take the `catalogs/<catalog>`
//! prefix from `/v1/config`.
//!
//! Errors use the Iceberg body (`{"error": {"message", "type", "code"}}`)
//! with the Iceberg exception names clients read, see [`IcebergError`] and
//! [`error_layer`].

use std::collections::BTreeSet;
use std::future::Future;

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{FromRequest, Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, de::DeserializeOwned};
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateSchema,
    iceberg_rest::{
        IcebergConfigResponse, IcebergCreateNamespaceRequest, IcebergErrorModel,
        IcebergErrorResponse, IcebergListNamespacesResponse, IcebergNamespaceResponse,
        IcebergUpdateNamespacePropertiesRequest, IcebergUpdateNamespacePropertiesResponse,
    },
};
use uc_common::{
    API_BASE, DEFAULT_PAGE_SIZE, ErrorCode, ErrorResponse, Pagination, SecurableType,
    schema_full_name,
};

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules, schema_lineage},
    http::extract::{Path, Query, parse_json_body},
    server::state::AppState,
};

mod tables;

/// An Iceberg REST error: the status, the Iceberg exception name and the
/// message.
#[derive(Debug)]
pub struct IcebergError {
    status: StatusCode,
    error_type: &'static str,
    message: String,
}

impl IcebergError {
    fn new(status: StatusCode, error_type: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            error_type,
            message: message.into(),
        }
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BadRequestException", message)
    }
    pub fn no_such_table(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NoSuchTableException", message)
    }
    pub fn no_such_view(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NoSuchViewException", message)
    }
    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "AlreadyExistsException", message)
    }
    pub fn namespace_not_empty(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "NamespaceNotEmptyException", message)
    }
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "UnprocessableEntityException",
            message,
        )
    }
    pub fn commit_failed(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "CommitFailedException", message)
    }
    /// The Iceberg exception a client raises for a UC error `code`.
    fn of_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(code.iceberg_status(), code.iceberg_error_type(), message)
    }
}

impl From<uc_common::Error> for IcebergError {
    fn from(e: uc_common::Error) -> Self {
        Self::of_code(e.code(), e.message())
    }
}

impl IntoResponse for IcebergError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            tracing::error!(error_type = self.error_type, message = %self.message, "iceberg server error");
        }
        let body = IcebergErrorResponse {
            error: IcebergErrorModel {
                message: self.message,
                error_type: self.error_type.to_string(),
                code: self.status.as_u16(),
            },
        };
        (self.status, axum::Json(body)).into_response()
    }
}

pub type IcebergResult<T> = Result<T, IcebergError>;

/// A JSON request body. Unreadable bodies are a bad request whose message
/// does not quote the parser (upstream `MalformedBodyRejectingConverter`).
pub struct IcebergJson<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync> FromRequest<S> for IcebergJson<T> {
    type Rejection = IcebergError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| IcebergError::bad_request(format!("Failed to read request body: {e}")))?;
        parse_json_body(&bytes, false)
            .map(IcebergJson)
            .map_err(|e| IcebergError::bad_request(e.message()))
    }
}

/// Errors raised outside the handlers, under the Iceberg mount
/// (`<API_BASE>/iceberg`), get the Iceberg body: UC errors (request
/// parsing) and the bare statuses of sign-in and unrouted requests
/// (upstream `UnroutedIcebergRequestHandler`).
pub async fn error_layer(req: Request, next: Next) -> Response {
    let iceberg = req
        .uri()
        .path()
        .strip_prefix(API_BASE)
        .and_then(|rest| rest.strip_prefix("/iceberg"))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
    let resp = next.run(req).await;
    if !iceberg || !(resp.status().is_client_error() || resp.status().is_server_error()) {
        return resp;
    }
    let allow = resp.headers().get(header::ALLOW).cloned();
    let error = match resp.extensions().get::<ErrorResponse>() {
        Some(uc) => IcebergError::of_code(uc.error_code, uc.message.clone()),
        None => {
            let status = resp.status();
            let (parts, body) = resp.into_parts();
            let Ok(bytes) = axum::body::to_bytes(body, 1 << 20).await else {
                return Response::from_parts(parts, Body::empty());
            };
            // Already an Iceberg body.
            if serde_json::from_slice::<serde_json::Value>(&bytes).is_ok() {
                return Response::from_parts(parts, Body::from(bytes));
            }
            let text = String::from_utf8_lossy(&bytes).trim().to_string();
            let message = match text.is_empty() {
                true => status.canonical_reason().unwrap_or("Error").to_string(),
                false => text,
            };
            let error_type = match status {
                StatusCode::BAD_REQUEST => "BadRequestException",
                StatusCode::UNAUTHORIZED => "NotAuthorizedException",
                StatusCode::FORBIDDEN => "ForbiddenException",
                StatusCode::NOT_FOUND => "NotFoundException",
                _ => "RESTException",
            };
            IcebergError::new(status, error_type, message)
        }
    };
    let mut resp = error.into_response();
    if let Some(allow) = allow {
        resp.headers_mut().insert(header::ALLOW, allow);
    }
    resp
}

/// The `catalogs/<catalog>` prefix of every catalog route.
const PREFIX_BASE: &str = "catalogs/";

const READ_ENDPOINTS: &[&str] = &[
    "GET /v1/{prefix}/namespaces",
    "GET /v1/{prefix}/namespaces/{namespace}",
    "HEAD /v1/{prefix}/namespaces/{namespace}",
    "HEAD /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "GET /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "GET /v1/{prefix}/namespaces/{namespace}/tables/{table}/credentials",
    "GET /v1/{prefix}/namespaces/{namespace}/views/{view}",
    "POST /v1/{prefix}/namespaces/{namespace}/tables/{table}/metrics",
    "GET /v1/{prefix}/namespaces/{namespace}/tables",
];

const WRITE_ENDPOINTS: &[&str] = &[
    "POST /v1/{prefix}/namespaces",
    "DELETE /v1/{prefix}/namespaces/{namespace}",
    "POST /v1/{prefix}/namespaces/{namespace}/properties",
    "POST /v1/{prefix}/namespaces/{namespace}/tables",
    "POST /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "DELETE /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "POST /v1/{prefix}/tables/rename",
];

#[derive(Debug, Deserialize)]
pub struct ConfigParams {
    pub warehouse: Option<String>,
}

#[utoipa::path(
    get, path = "/iceberg/v1/config",
    operation_id = "iceberg_config",
    params(("warehouse" = String, Query, description = "The catalog")),
    responses(
        (status = 200, body = IcebergConfigResponse),
        (status = 400, body = IcebergErrorResponse),
        (status = 404, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn config(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ConfigParams>,
) -> IcebergResult<Json<IcebergConfigResponse>> {
    let catalog = params.warehouse.ok_or_else(|| {
        IcebergError::bad_request("Must supply a proper catalog in warehouse property.")
    })?;
    // The prefix only leads anywhere if the catalog exists.
    let lineage = authz::lineage(&state, SecurableType::Catalog, &catalog).await?;
    let access = Access::load(&state, &principal).await?;
    access.check(rules::get_catalog(&access, &lineage))?;
    let mut endpoints: Vec<String> = READ_ENDPOINTS.iter().map(|e| e.to_string()).collect();
    if state.iceberg_writes_enabled {
        endpoints.extend(WRITE_ENDPOINTS.iter().map(|e| e.to_string()));
    }
    Ok(Json(IcebergConfigResponse {
        defaults: Default::default(),
        overrides: [("prefix".to_string(), format!("{PREFIX_BASE}{catalog}"))].into(),
        endpoints: Some(endpoints),
    }))
}

/// Check `rule` for the namespace (schema) `catalog.namespace`.
async fn check_schema(
    state: &AppState,
    principal: &Principal,
    catalog: &str,
    namespace: &str,
    rule: fn(&Access, &authz::Lineage) -> bool,
) -> IcebergResult<()> {
    let lineage = authz::lineage(
        state,
        SecurableType::Schema,
        &schema_full_name(catalog, namespace),
    )
    .await?;
    let access = Access::load(state, principal).await?;
    Ok(access.check(rule(&access, &lineage))?)
}

/// Upstream `checkIcebergTableEnabled`: namespace and table writes can be
/// turned off (`tables.iceberg_writes`).
fn check_writes_enabled(state: &AppState) -> IcebergResult<()> {
    if !state.iceberg_writes_enabled {
        return Err(uc_common::Error::invalid_argument(
            "Iceberg table writes are currently disabled. To enable them, set \
             tables.iceberg_writes = true (UC_TABLES_ICEBERG_WRITES)",
        )
        .into());
    }
    Ok(())
}

/// `pageToken` and `pageSize` of a listing. The size is read as text so
/// that every value the spec does not allow is refused in the same words.
#[derive(Debug, Default, Deserialize)]
pub struct PageParams {
    #[serde(rename = "pageToken")]
    pub page_token: Option<String>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<String>,
}

impl PageParams {
    /// The page a client asked for. Without `pageToken` there is none: the
    /// whole listing is the answer. An empty token opens a paged listing.
    fn requested(&self) -> IcebergResult<Option<Pagination>> {
        let Some(token) = &self.page_token else {
            return Ok(None);
        };
        let invalid = |size: &str| {
            IcebergError::bad_request(format!(
                "Invalid pageSize: {size}. It must be a whole number of at least 1."
            ))
        };
        let size = match self.page_size.as_deref().filter(|s| !s.is_empty()) {
            None => DEFAULT_PAGE_SIZE,
            Some(s) => i64::from(s.parse::<i32>().map_err(|_| invalid(s))?),
        };
        if size < 1 {
            return Err(invalid(&size.to_string()));
        }
        let cursor = Some(token.clone()).filter(|t| !t.is_empty());
        Ok(Some(Pagination::new(
            Some(size.min(DEFAULT_PAGE_SIZE)),
            cursor,
        )))
    }
}

/// Upstream's listing loop: a requested page is answered with its token;
/// otherwise every page is read and no token is returned.
async fn listing<T, F, Fut>(
    requested: Option<Pagination>,
    mut fetch: F,
) -> IcebergResult<(Vec<T>, Option<String>)>
where
    F: FnMut(Pagination) -> Fut,
    Fut: Future<Output = uc_common::Result<(Vec<T>, Option<String>)>>,
{
    if let Some(page) = requested {
        let (items, next) = fetch(page).await?;
        return Ok((items, next.filter(|t| !t.is_empty())));
    }
    let mut all = Vec::new();
    let mut cursor = None;
    loop {
        let (items, next) = fetch(Pagination::new(None, cursor)).await?;
        all.extend(items);
        cursor = next.filter(|t| !t.is_empty());
        if cursor.is_none() {
            return Ok((all, None));
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListNamespacesParams {
    pub parent: Option<String>,
    #[serde(flatten)]
    pub page: PageParams,
}

#[utoipa::path(
    get, path = "/iceberg/v1/catalogs/{catalog}/namespaces",
    operation_id = "iceberg_list_namespaces",
    params(
        ("catalog" = String, Path),
        ("parent" = Option<String>, Query, description = "Nested namespaces are not supported: a parent has none"),
        ("pageToken" = Option<String>, Query),
        ("pageSize" = Option<String>, Query),
    ),
    responses(
        (status = 200, body = IcebergListNamespacesResponse),
        (status = 404, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn list_namespaces(
    State(state): State<AppState>,
    principal: Principal,
    Path(catalog): Path<String>,
    Query(params): Query<ListNamespacesParams>,
) -> IcebergResult<Json<IcebergListNamespacesResponse>> {
    if let Some(parent) = params.parent.as_deref().filter(|p| !p.is_empty()) {
        // A parent that exists has no children; one that does not is a 404.
        state
            .metastore
            .schemas()
            .get(&schema_full_name(&catalog, parent))
            .await?;
        return Ok(Json(IcebergListNamespacesResponse::default()));
    }
    let (schemas, next_page_token) = listing(params.page.requested()?, |page| {
        let (metastore, catalog) = (state.metastore.clone(), catalog.clone());
        async move { metastore.schemas().list(&catalog, &page).await }
    })
    .await?;
    // Each namespace the caller may read (upstream `GET_SCHEMA` filter).
    let access = Access::load(&state, &principal).await?;
    let catalog_lineage = match access.enabled() {
        true => authz::lineage(&state, SecurableType::Catalog, &catalog).await?,
        false => Default::default(),
    };
    let namespaces = schemas
        .into_iter()
        .filter(|s| rules::get_schema(&access, &schema_lineage(&catalog_lineage, s)))
        .filter_map(|s| s.name)
        .map(|name| vec![name])
        .collect();
    Ok(Json(IcebergListNamespacesResponse {
        namespaces,
        next_page_token,
    }))
}

#[utoipa::path(
    post, path = "/iceberg/v1/catalogs/{catalog}/namespaces",
    operation_id = "iceberg_create_namespace",
    params(("catalog" = String, Path)),
    request_body = IcebergCreateNamespaceRequest,
    responses(
        (status = 200, body = IcebergNamespaceResponse),
        (status = 400, body = IcebergErrorResponse),
        (status = 409, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn create_namespace(
    State(state): State<AppState>,
    principal: Principal,
    Path(catalog): Path<String>,
    IcebergJson(req): IcebergJson<IcebergCreateNamespaceRequest>,
) -> IcebergResult<Json<IcebergNamespaceResponse>> {
    let lineage = authz::lineage(&state, SecurableType::Catalog, &catalog).await?;
    let access = Access::load(&state, &principal).await?;
    access.check(rules::create_schema(&access, &lineage))?;
    check_writes_enabled(&state)?;
    let namespace = req
        .namespace
        .ok_or_else(|| IcebergError::bad_request("Invalid namespace: null"))?;
    let [name] = namespace.as_slice() else {
        return Err(IcebergError::bad_request(format!(
            "Nested namespaces are not supported: {}",
            namespace.join(".")
        )));
    };
    let create = CreateSchema {
        name: name.clone(),
        catalog_name: catalog,
        comment: None,
        properties: req.properties,
        storage_root: None,
    };
    let schema = state
        .metastore
        .schemas()
        .create(create, principal.actor())
        .await?;
    let properties = schema.properties.unwrap_or_default();
    Ok(Json(IcebergNamespaceResponse {
        namespace,
        properties,
    }))
}

#[utoipa::path(
    get, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}",
    operation_id = "iceberg_get_namespace",
    params(("catalog" = String, Path), ("namespace" = String, Path)),
    responses(
        (status = 200, body = IcebergNamespaceResponse),
        (status = 404, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn get_namespace(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace)): Path<(String, String)>,
) -> IcebergResult<Json<IcebergNamespaceResponse>> {
    check_schema(&state, &principal, &catalog, &namespace, rules::get_schema).await?;
    let schema = state
        .metastore
        .schemas()
        .get(&schema_full_name(&catalog, &namespace))
        .await?;
    let properties = schema.properties.unwrap_or_default();
    Ok(Json(IcebergNamespaceResponse {
        namespace: vec![namespace],
        properties,
    }))
}

#[utoipa::path(
    head, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}",
    operation_id = "iceberg_namespace_exists",
    params(("catalog" = String, Path), ("namespace" = String, Path)),
    responses(
        (status = 204, description = "The namespace exists"),
        (status = 404, description = "No such namespace"),
    ),
    tag = "Iceberg",
)]
pub async fn namespace_exists(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace)): Path<(String, String)>,
) -> IcebergResult<StatusCode> {
    check_schema(&state, &principal, &catalog, &namespace, rules::get_schema).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}",
    operation_id = "iceberg_drop_namespace",
    params(("catalog" = String, Path), ("namespace" = String, Path)),
    responses(
        (status = 204, description = "Dropped"),
        (status = 404, body = IcebergErrorResponse),
        (status = 409, body = IcebergErrorResponse, description = "The namespace is not empty"),
    ),
    tag = "Iceberg",
)]
pub async fn drop_namespace(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace)): Path<(String, String)>,
) -> IcebergResult<StatusCode> {
    let full_name = schema_full_name(&catalog, &namespace);
    check_schema(
        &state,
        &principal,
        &catalog,
        &namespace,
        rules::update_schema,
    )
    .await?;
    check_writes_enabled(&state)?;
    match state.metastore.schemas().delete(&full_name, false).await {
        // The metastore's failed precondition (400) is the spec's 409.
        Err(e) if e.code() == ErrorCode::FailedPrecondition => Err(
            IcebergError::namespace_not_empty(format!("Namespace is not empty: {namespace}")),
        ),
        result => {
            result?;
            Ok(StatusCode::NO_CONTENT)
        }
    }
}

#[utoipa::path(
    post, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/properties",
    operation_id = "iceberg_update_namespace_properties",
    params(("catalog" = String, Path), ("namespace" = String, Path)),
    request_body = IcebergUpdateNamespacePropertiesRequest,
    responses(
        (status = 200, body = IcebergUpdateNamespacePropertiesResponse),
        (status = 404, body = IcebergErrorResponse),
        (status = 422, body = IcebergErrorResponse, description = "A key is both set and removed"),
    ),
    tag = "Iceberg",
)]
pub async fn update_namespace_properties(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace)): Path<(String, String)>,
    IcebergJson(req): IcebergJson<IcebergUpdateNamespacePropertiesRequest>,
) -> IcebergResult<Json<IcebergUpdateNamespacePropertiesResponse>> {
    let full_name = schema_full_name(&catalog, &namespace);
    check_schema(
        &state,
        &principal,
        &catalog,
        &namespace,
        rules::update_schema,
    )
    .await?;
    check_writes_enabled(&state)?;
    let updates = req.updates.unwrap_or_default();
    // Each key once, in the order asked.
    let mut seen = BTreeSet::new();
    let removals: Vec<String> = req
        .removals
        .unwrap_or_default()
        .into_iter()
        .filter(|k| seen.insert(k.clone()))
        .collect();
    let both: Vec<&str> = removals
        .iter()
        .filter(|k| updates.contains_key(*k))
        .map(String::as_str)
        .collect();
    if !both.is_empty() {
        return Err(IcebergError::unprocessable(format!(
            "Invalid namespace properties update: cannot simultaneously set and remove keys: [{}]",
            both.join(", ")
        )));
    }
    let changes = state
        .metastore
        .schemas()
        .apply_property_changes(&full_name, updates, removals, principal.actor())
        .await?;
    Ok(Json(IcebergUpdateNamespacePropertiesResponse {
        updated: changes.updated,
        removed: changes.removed,
        missing: Some(changes.missing),
    }))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(config))
        .routes(routes!(list_namespaces, create_namespace))
        .routes(routes!(get_namespace, namespace_exists, drop_namespace))
        .routes(routes!(update_namespace_properties))
        .routes(routes!(tables::list_tables, tables::create_table))
        .routes(routes!(
            tables::load_table,
            tables::table_exists,
            tables::commit_table,
            tables::drop_table
        ))
        .routes(routes!(tables::rename_table))
        .routes(routes!(tables::load_credentials))
        .routes(routes!(tables::report_metrics))
        .routes(routes!(tables::load_view))
}
