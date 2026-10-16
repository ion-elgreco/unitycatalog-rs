//! Delta Sharing: the admin endpoints and the recipient protocol.
//!
//! - `/shares` and `/recipients`: a UC admin manages shares and recipients
//!   and rotates recipient tokens.
//! - `/delta-sharing/shares/...`: a recipient signs in with its bearer token
//!   and reads the contents of its shares. The protocol is read-only.
//!
//! The protocol is under `/delta-sharing/`, so that recipient tokens stay
//! apart from UC sign-in.

use crate::http::extract::{Json, Path, Query};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::Response,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{
    CreateRecipient, CreateShare, ListRecipientsResponse, ListSharesResponse, RecipientInfo,
    ShareInfo, SharedDataObject, SharingListSchemasResponse, SharingListSharesResponse,
    SharingListTablesResponse, SharingSchema, SharingShare, SharingTable, UpdateRecipient,
    UpdateShare,
};
use uc_common::{API_BASE, Error, Pagination};

use crate::{
    authn::{middleware::bearer_token, principal::Principal},
    authz,
    http::error::ApiResult,
    server::state::AppState,
};

#[utoipa::path(
    post, path = "/shares",
    request_body = CreateShare,
    responses((status = 200, body = ShareInfo), (status = 409, description = "Already exists")),
    tag = "Sharing",
)]
pub async fn create_share(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateShare>,
) -> ApiResult<Json<ShareInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .create_share(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/shares",
    params(
        ("max_results" = Option<u32>, Query),
        ("page_token"  = Option<String>, Query),
    ),
    responses((status = 200, body = ListSharesResponse)),
    tag = "Sharing",
)]
pub async fn list_shares(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListSharesResponse>> {
    authz::require_admin(&state, &principal.username).await?;
    let (items, next_page_token) = state.metastore.sharing().list_shares(&page).await?;
    Ok(Json(ListSharesResponse {
        items,
        next_page_token,
    }))
}

#[utoipa::path(
    get, path = "/shares/{name}",
    params(("name" = String, Path)),
    responses((status = 200, body = ShareInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn get_share(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<ShareInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(state.metastore.sharing().get_share(&name).await?))
}

#[utoipa::path(
    patch, path = "/shares/{name}",
    params(("name" = String, Path)),
    request_body = UpdateShare,
    responses((status = 200, body = ShareInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn update_share(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateShare>,
) -> ApiResult<Json<ShareInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .update_share(&name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/shares/{name}",
    params(("name" = String, Path)),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn delete_share(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    state.metastore.sharing().delete_share(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/recipients",
    request_body = CreateRecipient,
    responses((status = 200, body = RecipientInfo, description = "Includes plaintext bearer_token (one-shot)")),
    tag = "Sharing",
)]
pub async fn create_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<CreateRecipient>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .create_recipient(req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/recipients",
    params(
        ("max_results" = Option<u32>, Query),
        ("page_token"  = Option<String>, Query),
    ),
    responses((status = 200, body = ListRecipientsResponse)),
    tag = "Sharing",
)]
pub async fn list_recipients(
    State(state): State<AppState>,
    principal: Principal,
    Query(page): Query<Pagination>,
) -> ApiResult<Json<ListRecipientsResponse>> {
    authz::require_admin(&state, &principal.username).await?;
    let (items, next_page_token) = state.metastore.sharing().list_recipients(&page).await?;
    Ok(Json(ListRecipientsResponse {
        items,
        next_page_token,
    }))
}

#[utoipa::path(
    get, path = "/recipients/{name}",
    params(("name" = String, Path)),
    responses((status = 200, body = RecipientInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn get_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(state.metastore.sharing().get_recipient(&name).await?))
}

#[utoipa::path(
    patch, path = "/recipients/{name}",
    params(("name" = String, Path)),
    request_body = UpdateRecipient,
    responses((status = 200, body = RecipientInfo), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn update_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
    Json(req): Json<UpdateRecipient>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .update_recipient(&name, req, principal.actor())
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/recipients/{name}",
    params(("name" = String, Path)),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    tag = "Sharing",
)]
pub async fn delete_recipient(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    state.metastore.sharing().delete_recipient(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/recipients/{name}/rotate-token",
    params(("name" = String, Path)),
    responses((status = 200, body = RecipientInfo, description = "Includes plaintext bearer_token (one-shot)")),
    tag = "Sharing",
)]
pub async fn rotate_recipient_token(
    State(state): State<AppState>,
    principal: Principal,
    Path(name): Path<String>,
) -> ApiResult<Json<RecipientInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .rotate_recipient_token(&name)
            .await?,
    ))
}

#[utoipa::path(
    post, path = "/recipients/{name}/profile",
    params(("name" = String, Path)),
    responses((status = 200, description = "Delta Sharing profile.json (one-shot — contains a fresh bearer)")),
    tag = "Sharing",
)]
pub async fn recipient_profile(
    State(state): State<AppState>,
    principal: Principal,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    authz::require_admin(&state, &principal.username).await?;
    let rec = state
        .metastore
        .sharing()
        .rotate_recipient_token(&name)
        .await?;
    let token = rec.bearer_token.clone().unwrap_or_default();
    // The endpoint is the URL the client called: `X-Forwarded-Host` behind a
    // proxy, else `Host`, else a placeholder.
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(axum::http::header::HOST))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("http");
    let endpoint = format!("{scheme}://{host}{API_BASE}/delta-sharing");
    let profile = serde_json::json!({
        "shareCredentialsVersion": 1,
        "endpoint": endpoint,
        "bearerToken": token,
        "expirationTime": null
    });
    Ok(Json(profile))
}

// The protocol endpoints take no principal: the bearer token names a
// recipient, which must hold the share it reads.

async fn extract_recipient(state: &AppState, headers: &HeaderMap) -> Result<RecipientInfo, Error> {
    let token =
        bearer_token(headers).ok_or_else(|| Error::unauthenticated("missing Bearer token"))?;
    state
        .metastore
        .sharing()
        .recipient_for_token(&token)
        .await?
        .ok_or_else(|| Error::unauthenticated("invalid or revoked recipient token"))
}

/// The share `share`, for a recipient granted access to it.
async fn shared(state: &AppState, headers: &HeaderMap, share: &str) -> Result<ShareInfo, Error> {
    let recipient = extract_recipient(state, headers).await?;
    if !recipient.shares.iter().any(|s| s == share) {
        return Err(Error::permission_denied(format!(
            "recipient '{}' is not granted access to share '{share}'",
            recipient.name
        )));
    }
    state.metastore.sharing().get_share(share).await
}

/// The table `schema.table` of the share `share_info`.
fn shared_table<'a>(
    share_info: &'a ShareInfo,
    schema: &str,
    table: &str,
) -> Result<&'a SharedDataObject, Error> {
    share_info
        .objects
        .iter()
        .find(|o| o.shared_as_schema == schema && o.shared_as_name == table)
        .ok_or_else(|| {
            Error::not_found(format!(
                "table '{schema}.{table}' not in share '{}'",
                share_info.name
            ))
        })
}

#[utoipa::path(
    get, path = "/delta-sharing/shares",
    responses((status = 200, body = SharingListSharesResponse)),
    tag = "Sharing",
)]
pub async fn proto_list_shares(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<SharingListSharesResponse>> {
    let recipient = extract_recipient(&state, &headers).await?;
    let items: Vec<SharingShare> = recipient
        .shares
        .iter()
        .map(|s| SharingShare {
            name: s.clone(),
            id: None,
        })
        .collect();
    Ok(Json(SharingListSharesResponse {
        items,
        next_page_token: None,
    }))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas",
    params(("share" = String, Path)),
    responses((status = 200, body = SharingListSchemasResponse)),
    tag = "Sharing",
)]
pub async fn proto_list_schemas(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(share): Path<String>,
) -> ApiResult<Json<SharingListSchemasResponse>> {
    let share_info = shared(&state, &headers, &share).await?;
    let mut schemas: Vec<String> = share_info
        .objects
        .iter()
        .map(|o| o.shared_as_schema.clone())
        .collect();
    schemas.sort();
    schemas.dedup();
    let items: Vec<SharingSchema> = schemas
        .into_iter()
        .map(|s| SharingSchema {
            name: s,
            share: share.clone(),
        })
        .collect();
    Ok(Json(SharingListSchemasResponse {
        items,
        next_page_token: None,
    }))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables",
    params(("share" = String, Path), ("schema" = String, Path)),
    responses((status = 200, body = SharingListTablesResponse)),
    tag = "Sharing",
)]
pub async fn proto_list_tables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema)): Path<(String, String)>,
) -> ApiResult<Json<SharingListTablesResponse>> {
    let share_info = shared(&state, &headers, &share).await?;
    let items: Vec<SharingTable> = share_info
        .objects
        .into_iter()
        .filter(|o| o.shared_as_schema == schema)
        .map(|o| SharingTable {
            name: o.shared_as_name,
            schema: schema.clone(),
            share: share.clone(),
            id: None,
        })
        .collect();
    Ok(Json(SharingListTablesResponse {
        items,
        next_page_token: None,
    }))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/all-tables",
    params(("share" = String, Path)),
    responses((status = 200, body = SharingListTablesResponse)),
    tag = "Sharing",
)]
pub async fn proto_all_tables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(share): Path<String>,
) -> ApiResult<Json<SharingListTablesResponse>> {
    let share_info = shared(&state, &headers, &share).await?;
    let items: Vec<SharingTable> = share_info
        .objects
        .into_iter()
        .map(|o| SharingTable {
            name: o.shared_as_name,
            schema: o.shared_as_schema,
            share: share.clone(),
            id: None,
        })
        .collect();
    Ok(Json(SharingListTablesResponse {
        items,
        next_page_token: None,
    }))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}/version",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    responses((status = 200, description = "No body: the version is in the header",
        headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_table_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema, table)): Path<(String, String, String)>,
) -> ApiResult<Response> {
    let share_info = shared(&state, &headers, &share).await?;
    shared_table(&share_info, &schema, &table)?;
    Ok(with_table_version(Response::default()))
}

/// The protocol sends a table's version in the `Delta-Table-Version`
/// header of `version`, `metadata` and `query`. It is always 0: the Delta
/// log is not read.
fn with_table_version(mut resp: Response) -> Response {
    resp.headers_mut().insert(
        HeaderName::from_static("delta-table-version"),
        HeaderValue::from(0),
    );
    resp
}

// `metadata` and `query` answer with NDJSON (`application/x-ndjson`): a
// `{"protocol":...}` line and a `{"metaData":...}` line. `query` has no
// `{"file":...}` lines, since the Delta log is not read.

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}/metadata",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    responses((status = 200, description = "NDJSON: protocol + metaData lines",
        headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_table_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema, table)): Path<(String, String, String)>,
) -> ApiResult<Response> {
    resolve_and_emit_metadata(&state, &headers, &share, &schema, &table).await
}

#[utoipa::path(
    post, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}/query",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    responses((status = 200, description = "NDJSON: protocol + metaData lines",
        headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_table_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema, table)): Path<(String, String, String)>,
) -> ApiResult<Response> {
    // The body's hints (`predicateHints`, `limitHint`, `version`) are not
    // read: all data is served, so the answer is that of `metadata`.
    resolve_and_emit_metadata(&state, &headers, &share, &schema, &table).await
}

async fn resolve_and_emit_metadata(
    state: &AppState,
    headers: &HeaderMap,
    share: &str,
    schema: &str,
    table: &str,
) -> ApiResult<Response> {
    let share_info = shared(state, headers, share).await?;
    let obj = shared_table(&share_info, schema, table)?;
    let info = state
        .metastore
        .tables()
        .get(&obj.source_table_full_name)
        .await?;
    Ok(ndjson_response(ndjson_metadata(&info)))
}

fn ndjson_response(body: String) -> Response {
    let mut resp = Response::new(body.into());
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    with_table_version(resp)
}

fn ndjson_metadata(info: &uc_api::TableInfo) -> String {
    let protocol = serde_json::json!({
        "protocol": { "minReaderVersion": 1 }
    });
    let metadata = serde_json::json!({
        "metaData": {
            "id": info.table_id.clone().unwrap_or_default(),
            "name": info.name.clone().unwrap_or_default(),
            "format": { "provider": "parquet" },
            "schemaString": info.columns.as_ref()
                .map(|c| serde_json::to_string(c).unwrap_or_default())
                .unwrap_or_default(),
            "partitionColumns": Vec::<String>::new(),
            "configuration": serde_json::json!({}),
        }
    });
    format!("{protocol}\n{metadata}\n")
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // Admin
        .routes(routes!(create_share, list_shares))
        .routes(routes!(get_share, update_share, delete_share))
        .routes(routes!(create_recipient, list_recipients))
        .routes(routes!(get_recipient, update_recipient, delete_recipient))
        .routes(routes!(rotate_recipient_token))
        .routes(routes!(recipient_profile))
        // Recipient protocol
        .routes(routes!(proto_list_shares))
        .routes(routes!(proto_list_schemas))
        .routes(routes!(proto_list_tables))
        .routes(routes!(proto_all_tables))
        .routes(routes!(proto_table_version))
        .routes(routes!(proto_table_metadata))
        .routes(routes!(proto_table_query))
}
