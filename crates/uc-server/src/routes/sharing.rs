//! Delta Sharing: the admin endpoints and the recipient protocol.
//!
//! - `/shares` and `/recipients` under the API ([`router`]): a UC admin
//!   manages shares and recipients and rotates recipient tokens.
//! - `/delta-sharing/shares/...` ([`protocol_router`]): a recipient signs in
//!   with its bearer token and reads its shares: their schemas and tables,
//!   and each table's version, metadata, files and changes, which the Delta
//!   kernel reads from the table's log ([`crate::sharing`]). The protocol
//!   is read-only.
//! - `/delta-sharing/files/<token>`: the files of local tables, behind URLs
//!   that the server signs, as clouds presign theirs.
//!
//! The protocol is outside the API, as the reference server's
//! `/delta-sharing`, so that recipient tokens stay apart from UC sign-in.
//! Its errors have the protocol's body ([`error_layer`]).

use std::collections::HashSet;
use std::sync::Arc;

use crate::http::extract::{Json, Path, Query};
use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use delta_kernel::{SnapshotRef, Version};
use serde::Deserialize;
use serde_json::{Value, json};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use url::Url;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use uc_api::{
    CreateRecipient, CreateShare, DataSourceFormat, ListRecipientsResponse, ListSharesResponse,
    RecipientInfo, ShareInfo, SharedDataObject, SharingGetShareResponse,
    SharingListSchemasResponse, SharingListSharesResponse, SharingListTablesResponse,
    SharingSchema, SharingShare, SharingTable, TableType, UpdateRecipient, UpdateShare,
};
use uc_common::{Error, ErrorCode, ErrorResponse, NormalizedUrl, Pagination, Result};

use crate::{
    authn::{middleware::bearer_token, principal::Principal},
    authz,
    http::error::ApiResult,
    server::state::AppState,
    sharing::{
        CAPABILITIES_HEADER, Capabilities, FILE_ID_HASH_HEADER, ResponseFormat,
        TABLE_VERSION_HEADER, file_id_hash,
        lines::{self, FileKind, Lines, SharedFile},
        parse_timestamp, predicate,
        signer::{self, FileSigner},
        table::{Action, CatalogTail, Commit, SharedTable},
    },
};

/// The prefix of the protocol, the `endpoint` of recipients' profiles.
pub const PROTOCOL_PATH: &str = "/delta-sharing";

/// The path of the files of local tables. Their URLs carry a signed token
/// instead of a bearer token.
pub const FILES_PATH: &str = "/delta-sharing/files/";

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
    Json(mut req): Json<UpdateShare>,
) -> ApiResult<Json<ShareInfo>> {
    authz::require_admin(&state, &principal.username).await?;
    let share = state.metastore.sharing().get_share(&name).await?;
    req.add_objects = shared_objects(&state, &share, req.add_objects, &req.remove_objects).await?;
    Ok(Json(
        state
            .metastore
            .sharing()
            .update_share(&name, req, principal.actor())
            .await?,
    ))
}

/// The tables to add to `share`, checked: Delta tables with a location,
/// names that the protocol allows and that no other table of the share
/// has, and an id each. Tables of the share stay as they are, unless
/// `removed`.
async fn shared_objects(
    state: &AppState,
    share: &ShareInfo,
    added: Vec<SharedDataObject>,
    removed: &[SharedDataObject],
) -> Result<Vec<SharedDataObject>> {
    let staying = share.objects.iter().filter(|o| {
        !removed
            .iter()
            .any(|r| r.source_table_full_name == o.source_table_full_name)
    });
    let shared_name = |o: &SharedDataObject| {
        format!("{}.{}", o.shared_as_schema, o.shared_as_name).to_lowercase()
    };
    let mut names: HashSet<String> = staying.clone().map(shared_name).collect();
    let mut sources: HashSet<String> = staying.map(|o| o.source_table_full_name.clone()).collect();
    let mut objects = Vec::with_capacity(added.len());
    for mut object in added {
        let full_name = object.source_table_full_name.clone();
        if !sources.insert(full_name.clone()) {
            continue;
        }
        let table = state.metastore.tables().get(&full_name).await?;
        if table.data_source_format != Some(DataSourceFormat::Delta)
            || table.storage_location.is_none()
        {
            return Err(Error::invalid_argument(format!(
                "Only Delta tables can be shared: {full_name}"
            )));
        }
        if object.shared_as_schema.is_empty() {
            object.shared_as_schema = table.schema_name.unwrap_or_default();
        }
        if object.shared_as_name.is_empty() {
            object.shared_as_name = table.name.unwrap_or_default();
        }
        validate_shared_name("shared_as_schema", &object.shared_as_schema)?;
        validate_shared_name("shared_as_name", &object.shared_as_name)?;
        if !names.insert(shared_name(&object)) {
            return Err(Error::already_exists(format!(
                "Share '{}' already has a table {}.{}",
                share.name, object.shared_as_schema, object.shared_as_name
            )));
        }
        match object.start_version {
            Some(v) if v < 0 => {
                return Err(Error::invalid_argument("start_version cannot be negative"));
            }
            Some(_) if !object.history_shared() => {
                return Err(Error::invalid_argument(
                    "start_version needs history_data_sharing_status ENABLED",
                ));
            }
            _ => {}
        }
        object.id = Some(Uuid::new_v4().to_string());
        objects.push(object);
    }
    Ok(objects)
}

/// The protocol's rules for schema and table names: at most 255
/// characters, without spaces, `/`, `.` or control characters.
fn validate_shared_name(field: &str, name: &str) -> Result<()> {
    let bad = |c: char| c == ' ' || c == '/' || c == '.' || c.is_ascii_control();
    if name.is_empty() || name.chars().count() > 255 || name.chars().any(bad) {
        return Err(Error::invalid_argument(format!(
            "Invalid {field} '{name}': names have 1 to 255 characters, without spaces, '/', '.' \
             or control characters"
        )));
    }
    Ok(())
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
    if let Some(kind) = req.authentication_type.as_deref().filter(|k| *k != "TOKEN") {
        return Err(Error::invalid_argument(format!(
            "Unsupported authentication_type: {kind}. Recipients sign in with a bearer TOKEN"
        ))
        .into());
    }
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
    for share in &req.add_shares {
        state.metastore.sharing().get_share(share).await?;
    }
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
    let profile = serde_json::json!({
        "shareCredentialsVersion": 1,
        "endpoint": format!("{}{PROTOCOL_PATH}", server_url(&headers)),
        "bearerToken": token,
        "expirationTime": null
    });
    Ok(Json(profile))
}

/// The URL the client called the server at: `X-Forwarded-Proto` and
/// `X-Forwarded-Host` behind a proxy, else `Host`, else a placeholder.
fn server_url(headers: &HeaderMap) -> String {
    let header = |name: &str| headers.get(name).and_then(|h| h.to_str().ok());
    let host = header("x-forwarded-host")
        .or_else(|| header(header::HOST.as_str()))
        .unwrap_or("localhost:8080");
    let scheme = header("x-forwarded-proto").unwrap_or("http");
    format!("{scheme}://{host}")
}

// The protocol endpoints take no principal: the bearer token names a
// recipient, which must hold the share it reads.

async fn extract_recipient(state: &AppState, headers: &HeaderMap) -> Result<RecipientInfo> {
    let token =
        bearer_token(headers).ok_or_else(|| Error::unauthenticated("missing Bearer token"))?;
    state
        .metastore
        .sharing()
        .recipient_for_token(&token)
        .await?
        .ok_or_else(|| Error::unauthenticated("invalid or revoked recipient token"))
}

/// The share `share` (any case), for a recipient granted access to it.
async fn shared(state: &AppState, headers: &HeaderMap, share: &str) -> Result<ShareInfo> {
    let recipient = extract_recipient(state, headers).await?;
    let granted = recipient
        .shares
        .iter()
        .find(|s| s.eq_ignore_ascii_case(share))
        .ok_or_else(|| {
            Error::permission_denied(format!(
                "recipient '{}' is not granted access to share '{share}'",
                recipient.name
            ))
        })?;
    state.metastore.sharing().get_share(granted).await
}

/// The table `schema.table` (any case) of the share `share_info`.
fn shared_table<'a>(
    share_info: &'a ShareInfo,
    schema: &str,
    table: &str,
) -> Result<&'a SharedDataObject> {
    share_info
        .objects
        .iter()
        .find(|o| {
            o.shared_as_schema.eq_ignore_ascii_case(schema)
                && o.shared_as_name.eq_ignore_ascii_case(table)
        })
        .ok_or_else(|| {
            Error::not_found(format!(
                "table '{schema}.{table}' not in share '{}'",
                share_info.name
            ))
        })
}

/// `maxResults` and `pageToken` of the protocol's lists.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ProtocolPage {
    /// At most this many items (500 by default, 1000 at most).
    max_results: Option<i64>,
    /// The `nextPageToken` of the page before.
    page_token: Option<String>,
}

/// The page of `items` that `page` asks for, and the token of the next
/// page, if any. Items are in the order of `key`, which is unique.
fn page<T>(
    mut items: Vec<T>,
    key: impl Fn(&T) -> String,
    page: &ProtocolPage,
) -> Result<(Vec<T>, Option<String>)> {
    const DEFAULT: usize = 500;
    const MAX: usize = 1000;
    let max = match page.max_results {
        None => DEFAULT,
        Some(n) if n < 0 => {
            return Err(Error::invalid_argument("maxResults cannot be negative"));
        }
        Some(n) => (n as usize).min(MAX),
    };
    items.sort_by_key(&key);
    let after = match page.page_token.as_deref().filter(|t| !t.is_empty()) {
        Some(token) => Some(
            URL_SAFE_NO_PAD
                .decode(token)
                .ok()
                .and_then(|k| String::from_utf8(k).ok())
                .ok_or_else(|| Error::invalid_argument("Invalid pageToken"))?,
        ),
        None => None,
    };
    let start = after.as_deref().map_or(0, |after| {
        items.partition_point(|i| key(i).as_str() <= after)
    });
    let mut rest = items.split_off(start);
    let more = rest.len() > max;
    rest.truncate(max);
    let next = more.then(|| {
        let last = rest.last().map(&key).or(after).unwrap_or_default();
        URL_SAFE_NO_PAD.encode(last)
    });
    Ok((rest, next))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares",
    params(ProtocolPage),
    responses((status = 200, body = SharingListSharesResponse)),
    tag = "Sharing",
)]
pub async fn proto_list_shares(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ProtocolPage>,
) -> ApiResult<Json<SharingListSharesResponse>> {
    let recipient = extract_recipient(&state, &headers).await?;
    let (names, next_page_token) = page(recipient.shares, String::clone, &params)?;
    let mut items = Vec::with_capacity(names.len());
    for name in names {
        // A grant of a share that is gone lists nothing.
        let share = match state.metastore.sharing().get_share(&name).await {
            Err(e) if e.code().is_not_found() => continue,
            share => share?,
        };
        items.push(SharingShare {
            name: share.name,
            id: share.id,
            comment: share.comment,
        });
    }
    Ok(Json(SharingListSharesResponse {
        items,
        next_page_token,
    }))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}",
    params(("share" = String, Path)),
    responses((status = 200, body = SharingGetShareResponse)),
    tag = "Sharing",
)]
pub async fn proto_get_share(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(share): Path<String>,
) -> ApiResult<Json<SharingGetShareResponse>> {
    let share = shared(&state, &headers, &share).await?;
    Ok(Json(SharingGetShareResponse {
        share: SharingShare {
            name: share.name,
            id: share.id,
            comment: share.comment,
        },
    }))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas",
    params(("share" = String, Path), ProtocolPage),
    responses((status = 200, body = SharingListSchemasResponse)),
    tag = "Sharing",
)]
pub async fn proto_list_schemas(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(share): Path<String>,
    Query(params): Query<ProtocolPage>,
) -> ApiResult<Json<SharingListSchemasResponse>> {
    let share_info = shared(&state, &headers, &share).await?;
    let mut schemas: Vec<String> = share_info
        .objects
        .iter()
        .map(|o| o.shared_as_schema.clone())
        .collect();
    schemas.sort();
    schemas.dedup();
    let (schemas, next_page_token) = page(schemas, String::clone, &params)?;
    let items = schemas
        .into_iter()
        .map(|name| SharingSchema {
            name,
            share: share_info.name.clone(),
        })
        .collect();
    Ok(Json(SharingListSchemasResponse {
        items,
        next_page_token,
    }))
}

fn sharing_table(share: &ShareInfo, object: &SharedDataObject) -> SharingTable {
    SharingTable {
        name: object.shared_as_name.clone(),
        schema: object.shared_as_schema.clone(),
        share: share.name.clone(),
        share_id: share.id.clone(),
        id: object.id.clone(),
    }
}

fn table_key(table: &SharingTable) -> String {
    format!("{}.{}", table.schema, table.name)
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables",
    params(("share" = String, Path), ("schema" = String, Path), ProtocolPage),
    responses((status = 200, body = SharingListTablesResponse)),
    tag = "Sharing",
)]
pub async fn proto_list_tables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema)): Path<(String, String)>,
    Query(params): Query<ProtocolPage>,
) -> ApiResult<Json<SharingListTablesResponse>> {
    let share_info = shared(&state, &headers, &share).await?;
    let tables: Vec<SharingTable> = share_info
        .objects
        .iter()
        .filter(|o| o.shared_as_schema.eq_ignore_ascii_case(&schema))
        .map(|o| sharing_table(&share_info, o))
        .collect();
    if tables.is_empty() {
        return Err(Error::not_found(format!(
            "schema '{schema}' not in share '{}'",
            share_info.name
        ))
        .into());
    }
    let (items, next_page_token) = page(tables, table_key, &params)?;
    Ok(Json(SharingListTablesResponse {
        items,
        next_page_token,
    }))
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/all-tables",
    params(("share" = String, Path), ProtocolPage),
    responses((status = 200, body = SharingListTablesResponse)),
    tag = "Sharing",
)]
pub async fn proto_all_tables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(share): Path<String>,
    Query(params): Query<ProtocolPage>,
) -> ApiResult<Json<SharingListTablesResponse>> {
    let share_info = shared(&state, &headers, &share).await?;
    let tables = share_info
        .objects
        .iter()
        .map(|o| sharing_table(&share_info, o))
        .collect();
    let (items, next_page_token) = page(tables, table_key, &params)?;
    Ok(Json(SharingListTablesResponse {
        items,
        next_page_token,
    }))
}

/// A shared table, open for reading.
struct OpenTable {
    object: SharedDataObject,
    name: String,
    table: Arc<SharedTable>,
    signer: FileSigner,
}

impl OpenTable {
    /// Unless history is shared, only the latest version may be read.
    fn require_history(&self) -> Result<()> {
        if self.object.history_shared() {
            return Ok(());
        }
        Err(Error::invalid_argument(format!(
            "Reading table by version or timestamp is not supported because history sharing is \
             not enabled on table: {}",
            self.name
        )))
    }

    /// Versions before the share's `start_version` may not be read.
    fn require_shared(&self, version: Version) -> Result<()> {
        match self.object.start_version {
            Some(start) if (version as i64) < start => Err(Error::invalid_argument(format!(
                "You can only query table data since version {start}. The requested version is \
                 {version}."
            ))),
            _ => Ok(()),
        }
    }

    /// Run the kernel's blocking `work` on a blocking thread.
    async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(&SharedTable) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let table = self.table.clone();
        tokio::task::spawn_blocking(move || work(&table))
            .await
            .map_err(|e| Error::internal(format!("Delta kernel task: {e}")))?
    }

    /// The URL of `action`'s file, and of its deletion vector if stored in
    /// a file. Deletion vectors need the `delta` format and a client that
    /// reads them.
    async fn sign(
        &self,
        action: &Action,
        format: ResponseFormat,
        capabilities: &Capabilities,
    ) -> Result<(signer::SignedUrl, Option<signer::SignedUrl>)> {
        let path = action
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::internal("file action without a path"))?;
        let url = self.signer.sign(&self.table.file_url(path)?).await?;
        let Some(descriptor) = action.get("deletionVector") else {
            return Ok((url, None));
        };
        if format == ResponseFormat::Parquet || !capabilities.supports("deletionVectors") {
            return Err(Error::new(
                ErrorCode::Unimplemented,
                format!(
                    "Table {} has deletion vectors: read it with \
                     {CAPABILITIES_HEADER}: responseformat=delta;readerfeatures=deletionvectors",
                    self.name
                ),
            ));
        }
        let deletion_vector = match self.table.deletion_vector_url(descriptor)? {
            Some(file) => Some(self.signer.sign(&file).await?),
            None => None,
        };
        Ok((url, deletion_vector))
    }
}

async fn open_table(
    state: &AppState,
    headers: &HeaderMap,
    share: &str,
    schema: &str,
    table: &str,
) -> Result<OpenTable> {
    let share_info = shared(state, headers, share).await?;
    let object = shared_table(&share_info, schema, table)?.clone();
    let name = format!(
        "{}.{}.{}",
        share_info.name, object.shared_as_schema, object.shared_as_name
    );
    let info = state
        .metastore
        .tables()
        .get(&object.source_table_full_name)
        .await
        .map_err(|e| match e.code().is_not_found() {
            true => Error::not_found(format!("The source of table {name} is gone")),
            false => e,
        })?;
    let location = info
        .storage_location
        .as_deref()
        .ok_or_else(|| Error::internal(format!("Table {name} has no location")))?;
    let location = NormalizedUrl::parse(location)?;
    let mut root = Url::parse(location.as_str())
        .map_err(|e| Error::internal(format!("location {location}: {e}")))?;
    if !root.path().ends_with('/') {
        root.set_path(&format!("{}/", root.path()));
    }
    let files_endpoint = format!("{}{FILES_PATH}", server_url(headers));
    let (store, signer) = signer::open(state, &location, &files_endpoint).await?;
    let catalog = match info.table_type == Some(TableType::Managed) {
        true => {
            let table_id = info.table_id.as_deref().unwrap_or_default();
            let (commits, latest) = state
                .metastore
                .delta_tables()
                .unbackfilled_commits(table_id)
                .await?;
            Some(CatalogTail::new(&root, &commits, latest)?)
        }
        false => None,
    };
    Ok(OpenTable {
        object,
        name,
        table: Arc::new(SharedTable::new(root, store, catalog)),
        signer,
    })
}

fn version_header(version: Version) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static(TABLE_VERSION_HEADER),
        HeaderValue::from(version),
    )
}

/// `startingTimestamp` of `version`.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct VersionParams {
    /// The first version at or after this time.
    starting_timestamp: Option<String>,
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}/version",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path), VersionParams),
    responses((status = 200, description = "No body: the version is in the header",
        headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_table_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema, table)): Path<(String, String, String)>,
    Query(params): Query<VersionParams>,
) -> ApiResult<Response> {
    let open = open_table(&state, &headers, &share, &schema, &table).await?;
    let from = match params.starting_timestamp.as_deref() {
        Some(timestamp) => {
            open.require_history()?;
            Some(parse_timestamp("startingTimestamp", timestamp)?)
        }
        None => None,
    };
    let version = open
        .blocking(move |table| {
            let latest = table.snapshot(None)?;
            match from {
                Some(timestamp) => table.version_from(&latest, timestamp),
                None => Ok(latest.version()),
            }
        })
        .await?;
    if from.is_some() {
        open.require_shared(version)?;
    }
    Ok(([version_header(version)], ()).into_response())
}

#[utoipa::path(
    head, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    responses((status = 200, description = "The table's version, as `version`; the protocol \
        replaced this request with `version`", headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_head_table(
    state: State<AppState>,
    headers: HeaderMap,
    path: Path<(String, String, String)>,
) -> ApiResult<Response> {
    proto_table_version(state, headers, path, Query(VersionParams::default())).await
}

/// `version` and `timestamp` of `metadata`.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MetadataParams {
    /// The metadata as of this version.
    version: Option<i64>,
    /// The metadata as of this time.
    timestamp: Option<String>,
}

/// What a query reads: the latest snapshot, or that of a version or time.
#[derive(Debug, Clone, Copy)]
enum At {
    Latest,
    Version(Version),
    Timestamp(i64),
}

impl At {
    fn new(version: Option<i64>, timestamp: Option<&str>) -> Result<Self> {
        match (version, timestamp) {
            (Some(_), Some(_)) => Err(Error::invalid_argument(
                "version and timestamp cannot be used together",
            )),
            (Some(v), None) if v < 0 => Err(Error::invalid_argument("version cannot be negative")),
            (Some(v), None) => Ok(Self::Version(v as Version)),
            (None, Some(t)) => Ok(Self::Timestamp(parse_timestamp("timestamp", t)?)),
            (None, None) => Ok(Self::Latest),
        }
    }

    fn travels(self) -> bool {
        !matches!(self, Self::Latest)
    }
}

/// The snapshot `at` reads. A version must not be newer than the latest.
fn snapshot_at(table: &SharedTable, at: At) -> Result<SnapshotRef> {
    let latest = table.snapshot(None)?;
    let version = match at {
        At::Latest => return Ok(latest),
        At::Version(version) if version > latest.version() => {
            return Err(Error::invalid_argument(format!(
                "Cannot read version {version}: the latest version is {}",
                latest.version()
            )));
        }
        At::Version(version) => version,
        At::Timestamp(timestamp) => table.version_as_of(&latest, timestamp)?,
    };
    match version == latest.version() {
        true => Ok(latest),
        false => table.snapshot(Some(version)).map_err(|e| match e.code() {
            ErrorCode::Internal => e,
            _ => Error::invalid_argument(format!("Cannot read version {version}: {}", e.message())),
        }),
    }
}

/// The protocol and metadata of a snapshot, as the kernel has them, in JSON.
#[derive(Debug, Clone)]
struct TableState {
    version: Version,
    protocol: Value,
    metadata: Value,
}

impl TableState {
    fn of(snapshot: &SnapshotRef) -> Result<Self> {
        let configuration = snapshot.table_configuration();
        let json = |e: serde_json::Error| Error::internal(format!("table state: {e}"));
        Ok(Self {
            version: snapshot.version(),
            protocol: serde_json::to_value(configuration.protocol()).map_err(json)?,
            metadata: serde_json::to_value(configuration.metadata()).map_err(json)?,
        })
    }

    fn change_data_feed(&self) -> bool {
        self.metadata["configuration"]["delta.enableChangeDataFeed"].as_str() == Some("true")
    }

    /// The format of the answer: the client's choice, if the table reads
    /// in it, and the client reads the table's column mapping. A client of
    /// both formats gets `delta` for a table that may have deletion
    /// vectors.
    fn format(&self, capabilities: &Capabilities, name: &str) -> Result<ResponseFormat> {
        let parquet = lines::parquet_compatible(&self.protocol, &self.metadata);
        let format = capabilities.format(parquet && !lines::deletion_vectors(&self.protocol));
        if format == ResponseFormat::Parquet && !parquet {
            return Err(Error::new(
                ErrorCode::Unimplemented,
                format!(
                    "Table {name} has Delta features that the parquet response format cannot \
                     express: read it with {CAPABILITIES_HEADER}: responseformat=delta"
                ),
            ));
        }
        if format == ResponseFormat::Delta
            && lines::column_mapping(&self.metadata).is_some()
            && !capabilities.supports("columnMapping")
        {
            return Err(Error::new(
                ErrorCode::Unimplemented,
                format!(
                    "Table {name} has column mapping: read it with {CAPABILITIES_HEADER}: \
                     responseformat=delta;readerfeatures=columnmapping"
                ),
            ));
        }
        Ok(format)
    }
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}/metadata",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path), MetadataParams),
    responses((status = 200, description = "NDJSON: protocol + metaData lines",
        headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_table_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema, table)): Path<(String, String, String)>,
    Query(params): Query<MetadataParams>,
) -> ApiResult<Response> {
    let capabilities = Capabilities::from_headers(&headers)?;
    let file_id_hash = file_id_hash(&headers)?;
    let at = At::new(params.version, params.timestamp.as_deref())?;
    let open = open_table(&state, &headers, &share, &schema, &table).await?;
    if at.travels() {
        open.require_history()?;
    }
    let snapshot = open
        .blocking(move |table| TableState::of(&snapshot_at(table, at)?))
        .await?;
    if at.travels() {
        open.require_shared(snapshot.version)?;
    }
    let format = snapshot.format(&capabilities, &open.name)?;
    let mut lines = Lines::new(format);
    lines.protocol(&snapshot.protocol, None);
    let change_data_feed = open.object.history_shared() && snapshot.change_data_feed();
    let version = at.travels().then_some(snapshot.version);
    lines.metadata(&snapshot.metadata, version, change_data_feed);
    Ok(respond(
        lines,
        snapshot.version,
        &capabilities,
        file_id_hash,
    ))
}

/// The body of `query`, all optional. `predicateHints` (SQL), `maxFiles`,
/// `pageToken`, refresh tokens and `idempotencyKey` are read and not used:
/// the answer holds all files, at once.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    #[serde(default)]
    predicate_hints: Vec<String>,
    json_predicate_hints: Option<String>,
    limit_hint: Option<i64>,
    version: Option<i64>,
    timestamp: Option<String>,
    starting_version: Option<i64>,
    ending_version: Option<i64>,
    #[serde(default)]
    include_historical_protocol: bool,
}

#[utoipa::path(
    post, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}/query",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path)),
    request_body(content = Object, description = "predicateHints, jsonPredicateHints, \
        limitHint, version, timestamp, startingVersion, endingVersion, \
        includeHistoricalProtocol; all optional"),
    responses((status = 200, description = "NDJSON: protocol, metaData and file lines",
        headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_table_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema, table)): Path<(String, String, String)>,
    body: Bytes,
) -> ApiResult<Response> {
    let request: QueryRequest = match body.iter().all(u8::is_ascii_whitespace) {
        true => QueryRequest::default(),
        false => crate::http::extract::parse_json_body(&body, true)?,
    };
    let capabilities = Capabilities::from_headers(&headers)?;
    let file_id_hash = file_id_hash(&headers)?;
    let picked = [
        request.version.is_some(),
        request.timestamp.is_some(),
        request.starting_version.is_some(),
    ];
    if picked.iter().filter(|p| **p).count() > 1 {
        return Err(Error::invalid_argument(
            "Only one of version, timestamp and startingVersion can be set",
        )
        .into());
    }
    let open = open_table(&state, &headers, &share, &schema, &table).await?;
    if let Some(start) = request.starting_version {
        if start < 0 {
            return Err(Error::invalid_argument("startingVersion cannot be negative").into());
        }
        if let Some(end) = request.ending_version.filter(|end| *end < start) {
            return Err(Error::invalid_argument(format!(
                "startingVersion({start}) must be smaller than or equal to endingVersion({end})"
            ))
            .into());
        }
        open.require_history()?;
        open.require_shared(start as Version)?;
        return data_changes(open, request, capabilities, file_id_hash).await;
    }
    let at = At::new(request.version, request.timestamp.as_deref())?;
    if at.travels() {
        open.require_history()?;
    }
    let hint = request.json_predicate_hints.clone();
    // A limit counts the rows of all files, so it holds without filters
    // only.
    let limit = match (&hint, request.predicate_hints.is_empty()) {
        (None, true) => request.limit_hint.map(|l| l.max(0) as u64),
        _ => None,
    };
    let (snapshot, timestamp, files) = open
        .blocking(move |table| {
            let snapshot = snapshot_at(table, at)?;
            let predicate = hint.and_then(|hint| predicate::from_json(&hint, &snapshot.schema()));
            let timestamp = match at.travels() {
                true => Some(table.timestamp(&snapshot)?),
                false => None,
            };
            let files = table.files(&snapshot, predicate, limit)?;
            Ok((TableState::of(&snapshot)?, timestamp, files))
        })
        .await?;
    if at.travels() {
        open.require_shared(snapshot.version)?;
    }
    let format = snapshot.format(&capabilities, &open.name)?;
    let mut lines = Lines::new(format);
    lines.protocol(&snapshot.protocol, None);
    let change_data_feed = open.object.history_shared() && snapshot.change_data_feed();
    let version = at.travels().then_some(snapshot.version);
    lines.metadata(&snapshot.metadata, version, change_data_feed);
    let commit = timestamp.map(|t| (snapshot.version, t));
    for action in &files {
        let (url, deletion_vector) = open.sign(action, format, &capabilities).await?;
        lines.file(SharedFile {
            kind: FileKind::Snapshot,
            action,
            url,
            deletion_vector,
            commit,
        });
    }
    if capabilities.end_stream_action {
        lines.end_stream();
    }
    Ok(respond(
        lines,
        snapshot.version,
        &capabilities,
        file_id_hash,
    ))
}

/// `query` with `startingVersion`: the files each commit added and
/// removed since then, and the table's later metadata, for streaming.
async fn data_changes(
    open: OpenTable,
    request: QueryRequest,
    capabilities: Capabilities,
    file_id_hash: Option<String>,
) -> ApiResult<Response> {
    let start = request.starting_version.unwrap_or_default() as Version;
    let end = request.ending_version.map(|end| end as Version);
    let (head, commits) = open
        .blocking(move |table| {
            let latest = table.snapshot(None)?;
            if start > latest.version() {
                return Err(Error::invalid_argument(format!(
                    "startingVersion {start} is after the latest version {}",
                    latest.version()
                )));
            }
            let end = end.map_or(latest.version(), |end| end.min(latest.version()));
            let head = match start == latest.version() {
                true => latest,
                false => table.snapshot(Some(start))?,
            };
            Ok((TableState::of(&head)?, table.commits(start, end)?))
        })
        .await?;
    let format = head.format(&capabilities, &open.name)?;
    let protocols = request.include_historical_protocol && format == ResponseFormat::Delta;
    let mut lines = Lines::new(format);
    lines.protocol(&head.protocol, protocols.then_some(start));
    let change_data_feed = head.change_data_feed();
    lines.metadata(&head.metadata, Some(start), change_data_feed);
    for commit in &commits {
        let at = Some((commit.version, commit.timestamp));
        for action in &commit.actions {
            let (kind, action) = match action.iter().next() {
                Some((kind, Value::Object(action))) => (kind.as_str(), action),
                _ => continue,
            };
            let file_kind = match kind {
                "add" if data_change(action) => FileKind::Add,
                "remove" if data_change(action) => FileKind::Remove,
                "metaData" if commit.version > start => {
                    let metadata = Value::Object(action.clone());
                    lines.metadata(&metadata, Some(commit.version), change_data_feed);
                    continue;
                }
                "protocol" if protocols && commit.version > start => {
                    lines.protocol(&Value::Object(action.clone()), Some(commit.version));
                    continue;
                }
                _ => continue,
            };
            let (url, deletion_vector) = open.sign(action, format, &capabilities).await?;
            lines.file(SharedFile {
                kind: file_kind,
                action,
                url,
                deletion_vector,
                commit: at,
            });
        }
    }
    if capabilities.end_stream_action {
        lines.end_stream();
    }
    Ok(respond(lines, start, &capabilities, file_id_hash))
}

/// Whether a file action changes the table's data; compactions do not.
fn data_change(action: &Action) -> bool {
    action.get("dataChange").and_then(Value::as_bool) != Some(false)
}

/// The query of `changes`.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ChangesParams {
    /// The first version, included.
    starting_version: Option<i64>,
    /// The first version at or after this time.
    starting_timestamp: Option<String>,
    /// The last version, included; the latest by default.
    ending_version: Option<i64>,
    /// The last version at or before this time.
    ending_timestamp: Option<String>,
    /// Send the metadata of later versions too.
    #[serde(default)]
    include_historical_metadata: bool,
    /// Send the protocols of later versions too (`delta` format only).
    #[serde(default)]
    include_historical_protocol: bool,
}

#[utoipa::path(
    get, path = "/delta-sharing/shares/{share}/schemas/{schema}/tables/{table}/changes",
    params(("share" = String, Path), ("schema" = String, Path), ("table" = String, Path), ChangesParams),
    responses((status = 200, description = "NDJSON: protocol, metaData and change lines",
        headers(("Delta-Table-Version" = i64)))),
    tag = "Sharing",
)]
pub async fn proto_table_changes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((share, schema, table)): Path<(String, String, String)>,
    Query(params): Query<ChangesParams>,
) -> ApiResult<Response> {
    let capabilities = Capabilities::from_headers(&headers)?;
    let file_id_hash = file_id_hash(&headers)?;
    let open = open_table(&state, &headers, &share, &schema, &table).await?;
    if !open.object.history_shared() {
        return Err(Error::invalid_argument(format!(
            "cdf is not enabled on table {}: its history is not shared",
            open.name
        ))
        .into());
    }
    let timestamp = |name: &str, value: &Option<String>| {
        value
            .as_deref()
            .map(|t| parse_timestamp(name, t))
            .transpose()
    };
    let starting_timestamp = timestamp("startingTimestamp", &params.starting_timestamp)?;
    let ending_timestamp = timestamp("endingTimestamp", &params.ending_timestamp)?;
    let negative = [params.starting_version, params.ending_version]
        .into_iter()
        .flatten()
        .any(|v| v < 0);
    if negative {
        return Err(Error::invalid_argument("Versions cannot be negative").into());
    }
    let (starting_version, ending_version) = (params.starting_version, params.ending_version);
    if starting_version.is_none() && starting_timestamp.is_none() {
        return Err(
            Error::invalid_argument("startingVersion or startingTimestamp is required").into(),
        );
    }
    let historical_metadata = params.include_historical_metadata;
    let (start, head, commits) = open
        .blocking(move |table| {
            let latest = table.snapshot(None)?;
            let start = match (starting_version, starting_timestamp) {
                (Some(version), _) => version as Version,
                (None, Some(timestamp)) => table.version_from(&latest, timestamp)?,
                (None, None) => unreachable!("checked above"),
            };
            let end = match (ending_version, ending_timestamp) {
                (Some(version), _) => version as Version,
                (None, Some(timestamp)) => table.version_until(&latest, timestamp)?,
                (None, None) => latest.version(),
            };
            if start > end || end > latest.version() {
                return Err(Error::invalid_argument(format!(
                    "Invalid version range {start} to {end}: the versions must be in order, up \
                     to the latest version {}",
                    latest.version()
                )));
            }
            let first = match start == latest.version() {
                true => latest.clone(),
                false => table.snapshot(Some(start))?,
            };
            let first = TableState::of(&first)?;
            let head = match historical_metadata {
                true => first.clone(),
                false => TableState::of(&latest)?,
            };
            let commits = table.commits(start, end)?;
            require_change_data_feed(&first, &commits)?;
            Ok((start, head, commits))
        })
        .await?;
    open.require_shared(start)?;
    let format = head.format(&capabilities, &open.name)?;
    let protocols = params.include_historical_protocol && format == ResponseFormat::Delta;
    let mut lines = Lines::new(format);
    lines.protocol(&head.protocol, protocols.then_some(head.version));
    lines.metadata(&head.metadata, Some(head.version), true);
    for commit in &commits {
        let at = Some((commit.version, commit.timestamp));
        let has_cdc = commit.actions.iter().any(|a| a.contains_key("cdc"));
        for action in &commit.actions {
            let (kind, action) = match action.iter().next() {
                Some((kind, Value::Object(action))) => (kind.as_str(), action),
                _ => continue,
            };
            // A commit's change data files hold all its changes; without
            // them, the files it added and removed are its changes.
            let file_kind = match kind {
                "cdc" => FileKind::Cdc,
                "add" if !has_cdc && data_change(action) => FileKind::Add,
                "remove" if !has_cdc && data_change(action) => FileKind::Remove,
                "metaData" if historical_metadata && commit.version > start => {
                    let metadata = Value::Object(action.clone());
                    lines.metadata(&metadata, Some(commit.version), true);
                    continue;
                }
                "protocol" if protocols && commit.version > start => {
                    lines.protocol(&Value::Object(action.clone()), Some(commit.version));
                    continue;
                }
                _ => continue,
            };
            let (url, deletion_vector) = open.sign(action, format, &capabilities).await?;
            lines.file(SharedFile {
                kind: file_kind,
                action,
                url,
                deletion_vector,
                commit: at,
            });
        }
    }
    if capabilities.end_stream_action {
        lines.end_stream();
    }
    Ok(respond(lines, start, &capabilities, file_id_hash))
}

/// The change data feed must be on from the first version of a range to
/// its last.
fn require_change_data_feed(first: &TableState, commits: &[Commit]) -> Result<()> {
    let off = |version: Version| {
        Err(Error::invalid_argument(format!(
            "Change data feed is not enabled on the table at version {version}: set \
             delta.enableChangeDataFeed = true"
        )))
    };
    if !first.change_data_feed() {
        return off(first.version);
    }
    for commit in commits {
        for action in &commit.actions {
            if let Some(metadata) = action.get("metaData")
                && metadata["configuration"]["delta.enableChangeDataFeed"].as_str() != Some("true")
            {
                return off(commit.version);
            }
        }
    }
    Ok(())
}

/// The NDJSON response of `lines`, for the table version `version`.
fn respond(
    lines: Lines,
    version: Version,
    capabilities: &Capabilities,
    file_id_hash: Option<String>,
) -> Response {
    let format = lines.format();
    let mut response = Response::new(Body::from(lines.into_body()));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson; charset=utf-8"),
    );
    let (name, value) = version_header(version);
    headers.insert(name, value);
    if let Ok(value) = HeaderValue::from_str(&capabilities.response_header(format)) {
        headers.insert(HeaderName::from_static(CAPABILITIES_HEADER), value);
    }
    if let Some(Ok(value)) = file_id_hash.map(|h| HeaderValue::from_str(&h)) {
        headers.insert(HeaderName::from_static(FILE_ID_HASH_HEADER), value);
    }
    response
}

/// The signature of a file URL, as `signer::FileSigner::Local` writes it.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FileParams {
    /// `r`: the URL reads.
    sp: Option<String>,
    /// When the URL expires, in epoch seconds.
    se: Option<i64>,
    /// The server's signature.
    sig: Option<String>,
}

#[utoipa::path(
    get, path = "/delta-sharing/files/{file}",
    params(
        ("file" = String, Path, description = "From a file URL of `query` or `changes`"),
        FileParams,
    ),
    responses(
        (status = 200, description = "The file; ranges are served too"),
        (status = 403, description = "The URL is invalid or has expired"),
    ),
    tag = "Sharing",
)]
pub async fn proto_file(
    State(state): State<AppState>,
    Path(file): Path<String>,
    Query(params): Query<FileParams>,
    request: Request,
) -> ApiResult<Response> {
    let denied = || Error::permission_denied("The file URL is invalid or has expired");
    let security = state.security.as_ref().ok_or_else(denied)?;
    let file = URL_SAFE_NO_PAD
        .decode(&file)
        .ok()
        .and_then(|f| String::from_utf8(f).ok())
        .ok_or_else(denied)?;
    let (Some("r"), Some(expires_at), Some(signature)) =
        (params.sp.as_deref(), params.se, params.sig.as_deref())
    else {
        return Err(denied().into());
    };
    if !security.verify_file(&file, expires_at, signature) {
        return Err(denied().into());
    }
    let path = Url::parse(&file)
        .ok()
        .filter(|url| url.scheme() == "file")
        .and_then(|url| url.to_file_path().ok())
        .ok_or_else(denied)?;
    let Ok(response) = ServeFile::new(path).oneshot(request).await;
    Ok(response.map(Body::new))
}

/// Protocol errors have the protocol's body: `{"errorCode", "message"}`,
/// with the reference server's codes. Server errors keep their details in
/// the log.
pub async fn error_layer(req: Request, next: Next) -> Response {
    let sharing = req.uri().path().contains("/delta-sharing/");
    let resp = next.run(req).await;
    let Some(uc) = resp.extensions().get::<ErrorResponse>().filter(|_| sharing) else {
        return resp;
    };
    let (status, code) = match uc.error_code {
        ErrorCode::InvalidArgument | ErrorCode::OutOfRange => {
            (StatusCode::BAD_REQUEST, "INVALID_PARAMETER_VALUE")
        }
        ErrorCode::Unimplemented
        | ErrorCode::FailedPrecondition
        | ErrorCode::UnsupportedTableFormat => (StatusCode::BAD_REQUEST, "UNSUPPORTED_OPERATION"),
        code if code.is_not_found() => (StatusCode::NOT_FOUND, "RESOURCE_DOES_NOT_EXIST"),
        ErrorCode::PermissionDenied => (StatusCode::FORBIDDEN, "PERMISSION_DENIED"),
        ErrorCode::Unauthenticated => (StatusCode::UNAUTHORIZED, "UNAUTHENTICATED"),
        _ => (resp.status(), "INTERNAL_ERROR"),
    };
    let message = match status.is_server_error() {
        true => "",
        false => uc.message.as_str(),
    };
    (
        status,
        Json(json!({ "errorCode": code, "message": message })),
    )
        .into_response()
}

/// The admin endpoints, under the API.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_share, list_shares))
        .routes(routes!(get_share, update_share, delete_share))
        .routes(routes!(create_recipient, list_recipients))
        .routes(routes!(get_recipient, update_recipient, delete_recipient))
        .routes(routes!(rotate_recipient_token))
        .routes(routes!(recipient_profile))
}

/// The recipient protocol, at the server's root.
pub fn protocol_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(proto_list_shares))
        .routes(routes!(proto_get_share))
        .routes(routes!(proto_list_schemas))
        .routes(routes!(proto_list_tables))
        .routes(routes!(proto_all_tables))
        .routes(routes!(proto_head_table))
        .routes(routes!(proto_table_version))
        .routes(routes!(proto_table_metadata))
        .routes(routes!(proto_table_query))
        .routes(routes!(proto_table_changes))
        .routes(routes!(proto_file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_follow_their_tokens() {
        let names = || vec!["c".to_string(), "a".into(), "b".into()];
        let params = |max: i64, token: Option<String>| ProtocolPage {
            max_results: Some(max),
            page_token: token,
        };
        let (first, next) = page(names(), String::clone, &params(2, None)).unwrap();
        assert_eq!(first, ["a", "b"]);
        let (second, last) = page(names(), String::clone, &params(2, next)).unwrap();
        assert_eq!((second, last), (vec!["c".to_string()], None));
        // No items, but where to go on from.
        let (none, next) = page(names(), String::clone, &params(0, None)).unwrap();
        assert!(none.is_empty());
        let (all, _) = page(names(), String::clone, &params(5, next)).unwrap();
        assert_eq!(all, ["a", "b", "c"]);
        assert!(page(names(), String::clone, &params(-1, None)).is_err());
        assert!(page(names(), String::clone, &params(1, Some("%".into()))).is_err());
    }

    #[test]
    fn shared_names_follow_the_protocol() {
        assert!(validate_shared_name("n", "events_2024").is_ok());
        for bad in ["", "a.b", "a b", "a/b", "a\u{7}"] {
            assert!(validate_shared_name("n", bad).is_err(), "{bad:?}");
        }
        assert!(validate_shared_name("n", &"x".repeat(256)).is_err());
    }
}
