//! The Files API at [`uc_common::FILES_BASE`], as the Databricks Files API
//! has it: the files of volumes, and the files of tables, read-only. A path
//! is `/Volumes/<catalog>/<schema>/<volume>/...` or
//! `/Tables/<catalog>/<schema>/<table>/...`. The server reads and writes the
//! files with the credentials of the volume or table, so the caller needs
//! none. Not in upstream.
//!
//! A volume's files are read with `READ VOLUME` and written by its owner, as
//! its temporary credentials are vended. A table's files are read with
//! `SELECT`; a file written outside the table's commits would break it.
//!
//! In object storage a directory exists only through its files: creating
//! one checks the path only, and a directory without files is not found.
//! Local storage has real directories.

use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use object_store::{
    GetOptions, GetRange, ListResult, ObjectMeta, ObjectStore, PutPayload, WriteMultipart,
    list::PaginatedListOptions, local::LocalFileSystem, path::Path as ObjectPath,
};
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use uc_api::{DirectoryEntry, ListDirectoryContentsResponse};
use uc_common::{Error, NormalizedUrl, SecurableType, UriScheme, three_part_full_name};

use crate::{
    authn::principal::Principal,
    authz::{self, rules},
    http::{
        error::ApiResult,
        extract::{Json, Path, Query},
    },
    server::state::AppState,
    storage::{Opened, ServerStorage, delete_all, vendor::VendOp},
};

/// The most entries of a page, and the size of a page without
/// `page_size`, as Databricks has it.
const MAX_PAGE_SIZE: usize = 1000;

/// The size of the parts of an upload. Smaller files are written at once.
const PART_SIZE: usize = 8 * 1024 * 1024;

/// The parts of an upload that are sent at the same time.
const UPLOAD_CONCURRENCY: usize = 4;

#[derive(OpenApi)]
#[openapi(paths(
    list_directory_contents,
    get_directory_metadata,
    create_directory,
    delete_directory,
    download_file,
    get_file_metadata,
    upload_file,
    delete_file,
))]
struct FilesDoc;

/// The routes, under [`uc_common::FILES_BASE`]. A path spans several
/// segments, which the routes of the documentation cannot show: the
/// documentation names it as one.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(FilesDoc::openapi())
        .route(
            "/directories/{*directory_path}",
            get(list_directory_contents)
                .head(get_directory_metadata)
                .put(create_directory)
                .delete(delete_directory),
        )
        .route(
            "/files/{*file_path}",
            get(download_file)
                .head(get_file_metadata)
                .put(upload_file)
                .delete(delete_file),
        )
}

/// The securables that hold files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Root {
    Volumes,
    Tables,
}

impl Root {
    fn as_str(self) -> &'static str {
        match self {
            Self::Volumes => "Volumes",
            Self::Tables => "Tables",
        }
    }

    fn securable(self) -> &'static str {
        match self {
            Self::Volumes => "volume",
            Self::Tables => "table",
        }
    }
}

/// A path of the Files API: the volume or table, and the path in it.
#[derive(Debug, Clone)]
struct FilesPath {
    root: Root,
    /// Catalog, schema and name of the volume or table.
    names: [String; 3],
    /// The segments of the path in the volume or table.
    rest: Vec<String>,
}

impl FilesPath {
    /// `path` without its leading `/`, as the route takes it. Empty
    /// segments are left out, so a directory may end in `/`.
    fn parse(path: &str) -> Result<Self, Error> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let root = match segments.first() {
            Some(&"Volumes") => Root::Volumes,
            Some(&"Tables") => Root::Tables,
            _ => {
                return Err(Error::invalid_argument(format!(
                    "Invalid path /{path}: a path starts with /Volumes/<catalog>/<schema>/<volume> \
                     or /Tables/<catalog>/<schema>/<table>."
                )));
            }
        };
        let [_, catalog, schema, name, rest @ ..] = segments.as_slice() else {
            return Err(Error::invalid_argument(format!(
                "Invalid path /{path}: it must name a {}, as /{}/<catalog>/<schema>/<name>.",
                root.securable(),
                root.as_str(),
            )));
        };
        if let Some(bad) = segments
            .iter()
            .find(|s| matches!(**s, "." | "..") || s.contains(['\\', '\0']))
        {
            return Err(Error::invalid_argument(format!(
                "Invalid path /{path}: a segment may not be `{bad}`, or hold `\\` or NUL."
            )));
        }
        Ok(Self {
            root,
            names: [catalog.to_string(), schema.to_string(), name.to_string()],
            rest: rest.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn full_name(&self) -> String {
        let [catalog, schema, name] = &self.names;
        three_part_full_name(catalog, schema, name)
    }

    /// The path, which must name a file: not the volume or table itself.
    fn file(self) -> Result<Self, Error> {
        match self.rest.is_empty() {
            true => Err(Error::invalid_argument(format!(
                "Invalid path {self}: it names the {}, not a file.",
                self.root.securable()
            ))),
            false => Ok(self),
        }
    }

    /// The absolute path of the entry `parts`, relative to the volume or
    /// table.
    fn entry(&self, parts: &[String]) -> String {
        let [catalog, schema, name] = &self.names;
        let mut path = format!("/{}/{catalog}/{schema}/{name}", self.root.as_str());
        for part in parts {
            path.push('/');
            path.push_str(part);
        }
        path
    }
}

impl fmt::Display for FilesPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.entry(&self.rest))
    }
}

/// The store of the volume or table of a path, and the path's key in it.
struct Target {
    path: FilesPath,
    opened: Arc<Opened>,
    key: ObjectPath,
    /// Whether the storage is local: it has real directories.
    local: bool,
}

impl Target {
    fn store(&self) -> &Arc<dyn ObjectStore> {
        &self.opened.store
    }

    /// The error of the store for this path.
    fn error(&self, e: object_store::Error) -> Error {
        storage_error(&self.path, e)
    }

    /// The path in the volume or table of `location`, as its parts.
    fn parts_of(&self, location: &ObjectPath) -> Vec<String> {
        location
            .prefix_match(&self.opened.root)
            .map(|parts| {
                parts
                    .map(|part| {
                        percent_decode_str(part.as_ref())
                            .decode_utf8_lossy()
                            .into_owned()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The file system path of the key, for local storage.
    fn local_path(&self) -> Result<std::path::PathBuf, Error> {
        LocalFileSystem::new()
            .path_to_filesystem(&self.key)
            .map_err(|e| self.error(e))
    }

    /// The prefix of the keys in the directory of the key.
    fn prefix(&self) -> String {
        match self.key.as_ref() {
            "" => String::new(),
            key => format!("{key}/"),
        }
    }

    /// Whether the directory of the key holds any file or directory.
    async fn has_entries(&self) -> Result<bool, Error> {
        let listed = match &self.opened.pages {
            Some(pages) => {
                let options = PaginatedListOptions {
                    delimiter: Some("/".into()),
                    max_keys: Some(1),
                    ..Default::default()
                };
                let prefix = self.prefix();
                let prefix = (!prefix.is_empty()).then_some(prefix.as_str());
                pages
                    .list_paginated(prefix, options)
                    .await
                    .map_err(|e| self.error(e))?
                    .result
            }
            None => self
                .store()
                .list_with_delimiter(Some(&self.key))
                .await
                .map_err(|e| self.error(e))?,
        };
        Ok(!listed.common_prefixes.is_empty() || !listed.objects.is_empty())
    }

    /// Whether the directory of the key exists: the volume or table
    /// itself, a local directory, or a prefix of files.
    async fn directory_exists(&self) -> Result<bool, Error> {
        if self.path.rest.is_empty() {
            return Ok(true);
        }
        if self.local {
            return Ok(tokio::fs::metadata(self.local_path()?)
                .await
                .is_ok_and(|m| m.is_dir()));
        }
        self.has_entries().await
    }

    /// The metadata of the file of the key.
    async fn head(&self) -> Result<ObjectMeta, Error> {
        self.store()
            .head(&self.key)
            .await
            .map_err(|e| self.error(e))
    }
}

/// The error of the store for `path`.
fn storage_error(path: &FilesPath, e: object_store::Error) -> Error {
    match e {
        object_store::Error::NotFound { .. } => Error::not_found(format!("Not found: {path}")),
        object_store::Error::AlreadyExists { .. } => {
            Error::already_exists(format!("Already exists: {path}"))
        }
        object_store::Error::PermissionDenied { .. }
        | object_store::Error::Unauthenticated { .. } => {
            Error::permission_denied(format!("The storage refused access to {path}: {e}"))
        }
        e => Error::internal(format!("Storage error on {path}: {e}")),
    }
}

/// Check that the caller may read (`op` [`VendOp::Read`]) or write the
/// files of `path`, then open its store.
async fn open(
    state: &AppState,
    principal: &Principal,
    path: FilesPath,
    op: VendOp,
) -> ApiResult<Target> {
    let write = op == VendOp::ReadWrite;
    let full_name = path.full_name();
    let location = match path.root {
        Root::Volumes => {
            authz::check(
                state,
                &principal.username,
                SecurableType::Volume,
                &full_name,
                |a, l| rules::vend_volume(a, l, write),
            )
            .await?;
            state
                .metastore
                .volumes()
                .get(&full_name)
                .await?
                .storage_location
        }
        Root::Tables => {
            if write {
                return Err(Error::invalid_argument(format!(
                    "Invalid path {path}: the files of tables are read-only."
                ))
                .into());
            }
            authz::check(
                state,
                &principal.username,
                SecurableType::Table,
                &full_name,
                |a, l| rules::vend_table(a, l, false),
            )
            .await?;
            state
                .metastore
                .tables()
                .get(&full_name)
                .await?
                .storage_location
        }
    };
    let location = location.filter(|l| !l.is_empty()).ok_or_else(|| {
        Error::invalid_argument(format!(
            "The {} {full_name} has no storage location.",
            path.root.securable()
        ))
    })?;
    let location = NormalizedUrl::parse(&location)?;
    let storage = ServerStorage::new(state.metastore.clone(), state.vendor.clone());
    let opened = state
        .stores
        .open(&storage, &location, op)
        .await
        .map_err(|e| Error::internal(format!("Cannot open the storage of {path}: {}", e.0)))?;
    let key = path
        .rest
        .iter()
        .fold(opened.root.clone(), |key, part| key.child(part.as_str()));
    Ok(Target {
        path,
        opened,
        key,
        local: location.scheme() == UriScheme::File,
    })
}

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub page_size: Option<i64>,
    pub page_token: Option<String>,
}

#[utoipa::path(
    get, path = "/directories/{directory_path}",
    params(
        ("directory_path" = String, Path, description = "Absolute path of the directory, such as `/Volumes/<catalog>/<schema>/<volume>/<dir>`, over several segments"),
        ("page_size" = Option<i64>, Query, description = "The most entries to return; at most 1000"),
        ("page_token" = Option<String>, Query),
    ),
    responses(
        (status = 200, body = ListDirectoryContentsResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "Files",
)]
pub async fn list_directory_contents(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<ListDirectoryContentsResponse>> {
    let size = match params.page_size {
        None | Some(0) => MAX_PAGE_SIZE,
        Some(n) if n < 0 => {
            return Err(Error::invalid_argument("page_size must not be negative.").into());
        }
        Some(n) => usize::try_from(n)
            .unwrap_or(MAX_PAGE_SIZE)
            .min(MAX_PAGE_SIZE),
    };
    let token = params.page_token.filter(|t| !t.is_empty());
    let target = open(&state, &principal, FilesPath::parse(&path)?, VendOp::Read).await?;
    let (listed, next_page_token) = match &target.opened.pages {
        Some(pages) => {
            let prefix = target.prefix();
            let options = PaginatedListOptions {
                delimiter: Some("/".into()),
                max_keys: Some(size),
                page_token: token.clone(),
                ..Default::default()
            };
            let page = pages
                .list_paginated((!prefix.is_empty()).then_some(prefix.as_str()), options)
                .await
                .map_err(|e| target.error(e))?;
            (page.result, page.page_token)
        }
        None => local_page(&target, size, token.as_deref()).await?,
    };
    let empty = listed.common_prefixes.is_empty() && listed.objects.is_empty();
    if empty && token.is_none() && !target.directory_exists().await? {
        return Err(Error::not_found(format!("Not found: {}", target.path)).into());
    }
    let mut contents = Vec::new();
    for location in &listed.common_prefixes {
        let parts = target.parts_of(location);
        contents.push(DirectoryEntry {
            path: format!("{}/", target.path.entry(&parts)),
            name: parts.last().cloned().unwrap_or_default(),
            is_directory: true,
            file_size: None,
            last_modified: None,
        });
    }
    for meta in &listed.objects {
        let parts = target.parts_of(&meta.location);
        contents.push(DirectoryEntry {
            path: target.path.entry(&parts),
            name: parts.last().cloned().unwrap_or_default(),
            is_directory: false,
            file_size: i64::try_from(meta.size).ok(),
            last_modified: Some(meta.last_modified.timestamp_millis()),
        });
    }
    Ok(Json(ListDirectoryContentsResponse {
        contents,
        next_page_token,
    }))
}

/// A page of a local directory: its directories, then its files, by name;
/// the token is the number of entries before the page.
async fn local_page(
    target: &Target,
    size: usize,
    token: Option<&str>,
) -> Result<(ListResult, Option<String>), Error> {
    let start = match token {
        None => 0,
        Some(token) => token
            .parse::<usize>()
            .map_err(|_| Error::invalid_argument(format!("Invalid page token: {token}")))?,
    };
    let mut all = target
        .store()
        .list_with_delimiter(Some(&target.key))
        .await
        .map_err(|e| target.error(e))?;
    all.common_prefixes.sort();
    all.objects.sort_by(|a, b| a.location.cmp(&b.location));
    let dirs = all.common_prefixes.len();
    let total = dirs + all.objects.len();
    let end = start.saturating_add(size).min(total);
    let page = ListResult {
        common_prefixes: all
            .common_prefixes
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect(),
        objects: all
            .objects
            .into_iter()
            .skip(start.saturating_sub(dirs))
            .take(end.saturating_sub(start.max(dirs)))
            .collect(),
    };
    Ok((page, (end < total).then(|| end.to_string())))
}

#[utoipa::path(
    head, path = "/directories/{directory_path}",
    params(("directory_path" = String, Path, description = "Absolute path of the directory, over several segments")),
    responses(
        (status = 200, description = "The directory exists"),
        (status = 404, description = "Not found"),
    ),
    tag = "Files",
)]
pub async fn get_directory_metadata(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
) -> ApiResult<StatusCode> {
    let target = open(&state, &principal, FilesPath::parse(&path)?, VendOp::Read).await?;
    match target.directory_exists().await? {
        true => Ok(StatusCode::OK),
        false => Err(Error::not_found(format!("Not found: {}", target.path)).into()),
    }
}

#[utoipa::path(
    put, path = "/directories/{directory_path}",
    params(("directory_path" = String, Path, description = "Absolute path of the directory, over several segments")),
    responses(
        (status = 204, description = "Created, or there already"),
        (status = 400, description = "In a table"),
    ),
    tag = "Files",
)]
pub async fn create_directory(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
) -> ApiResult<StatusCode> {
    let target = open(
        &state,
        &principal,
        FilesPath::parse(&path)?,
        VendOp::ReadWrite,
    )
    .await?;
    if target.local {
        let dir = target.local_path()?;
        if tokio::fs::metadata(&dir).await.is_ok_and(|m| m.is_file()) {
            return Err(
                Error::already_exists(format!("A file is already there: {}", target.path)).into(),
            );
        }
        tokio::fs::create_dir_all(&dir).await.map_err(|e| {
            Error::internal(format!("Cannot create the directory {}: {e}", target.path))
        })?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct DeleteDirectoryParams {
    pub recursive: Option<bool>,
}

#[utoipa::path(
    delete, path = "/directories/{directory_path}",
    params(
        ("directory_path" = String, Path, description = "Absolute path of the directory, over several segments"),
        ("recursive" = Option<bool>, Query, description = "Also delete the files and directories in it; not in the Databricks Files API"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 400, description = "Not empty without `recursive`, or in a table"),
        (status = 404, description = "Not found"),
    ),
    tag = "Files",
)]
pub async fn delete_directory(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
    Query(params): Query<DeleteDirectoryParams>,
) -> ApiResult<StatusCode> {
    let target = open(
        &state,
        &principal,
        FilesPath::parse(&path)?,
        VendOp::ReadWrite,
    )
    .await?;
    if target.path.rest.is_empty() {
        return Err(Error::invalid_argument(format!(
            "Invalid path {}: the volume itself is not a directory to delete.",
            target.path
        ))
        .into());
    }
    let recursive = params.recursive.unwrap_or(false);
    let not_found = || Error::not_found(format!("Not found: {}", target.path));
    let not_empty = || {
        Error::failed_precondition(format!(
            "The directory is not empty: {}. Delete its files first, or set recursive.",
            target.path
        ))
    };
    if !target.local {
        // A directory without files is not there: nothing to delete.
        match (recursive, target.has_entries().await?) {
            (_, false) => {}
            (false, true) => return Err(not_empty().into()),
            (true, true) => {
                delete_all(target.store().as_ref(), &target.key)
                    .await
                    .map_err(|e| target.error(e))?;
            }
        }
        return Ok(StatusCode::NO_CONTENT);
    }
    let dir = target.local_path()?;
    let removed = match recursive {
        true => tokio::fs::remove_dir_all(&dir).await,
        false => tokio::fs::remove_dir(&dir).await,
    };
    match removed {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(match e.kind() {
            std::io::ErrorKind::NotFound => not_found(),
            std::io::ErrorKind::DirectoryNotEmpty => not_empty(),
            _ => Error::internal(format!("Cannot delete the directory {}: {e}", target.path)),
        }
        .into()),
    }
}

#[utoipa::path(
    get, path = "/files/{file_path}",
    params(("file_path" = String, Path, description = "Absolute path of the file, such as `/Volumes/<catalog>/<schema>/<volume>/<file>`, over several segments")),
    responses(
        (status = 200, description = "The bytes of the file", content_type = "application/octet-stream"),
        (status = 206, description = "The bytes of the range of the `Range` header"),
        (status = 404, description = "Not found"),
        (status = 416, description = "The range is outside the file"),
    ),
    tag = "Files",
)]
pub async fn download_file(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let target = open(
        &state,
        &principal,
        FilesPath::parse(&path)?.file()?,
        VendOp::Read,
    )
    .await?;
    let range = match headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        None => None,
        Some(value) => {
            let size = target.head().await?.size;
            match byte_range(value, size) {
                Ok(range) => range,
                Err(()) => {
                    return Ok((
                        StatusCode::RANGE_NOT_SATISFIABLE,
                        [(header::CONTENT_RANGE, format!("bytes */{size}"))],
                    )
                        .into_response());
                }
            }
        }
    };
    let options = GetOptions {
        range: range.clone().map(GetRange::Bounded),
        ..Default::default()
    };
    let result = target
        .store()
        .get_opts(&target.key, options)
        .await
        .map_err(|e| target.error(e))?;
    let meta = result.meta.clone();
    let served = result.range.clone();
    let path = target.path.clone();
    let body = result
        .into_stream()
        .map(move |chunk| chunk.map_err(|e| storage_error(&path, e)));
    let mut response = Response::new(Body::from_stream(body));
    file_headers(response.headers_mut(), &meta, served.end - served.start);
    if range.is_some() {
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        let value = format!("bytes {}-{}/{}", served.start, served.end - 1, meta.size);
        if let Ok(value) = HeaderValue::from_str(&value) {
            response.headers_mut().insert(header::CONTENT_RANGE, value);
        }
    }
    Ok(response)
}

/// The range of a `Range` header of the form `bytes=a-b`, `bytes=a-` or
/// `bytes=-n`, in a file of `size` bytes. A header of another form gives
/// `None`, for the whole file; a range outside the file is an error.
fn byte_range(value: &str, size: u64) -> Result<Option<Range<u64>>, ()> {
    let Some((start, end)) = value
        .trim()
        .strip_prefix("bytes=")
        .filter(|spec| !spec.contains(','))
        .and_then(|spec| spec.split_once('-'))
    else {
        return Ok(None);
    };
    let (start, end) = (start.trim(), end.trim());
    let range = match (start.parse::<u64>(), end.parse::<u64>()) {
        (Ok(start), Ok(end)) if start <= end => start..end.saturating_add(1).min(size),
        (Ok(start), Err(_)) if end.is_empty() => start..size,
        (Err(_), Ok(suffix)) if start.is_empty() && suffix > 0 => size.saturating_sub(suffix)..size,
        _ => return Ok(None),
    };
    match range.start < size {
        true => Ok(Some(range)),
        false => Err(()),
    }
}

/// The headers of a file of `meta`, whose answer has `length` bytes.
fn file_headers(headers: &mut HeaderMap, meta: &ObjectMeta, length: u64) {
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(length));
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    let modified = meta
        .last_modified
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    if let Ok(value) = HeaderValue::from_str(&modified) {
        headers.insert(header::LAST_MODIFIED, value);
    }
    if let Some(value) = meta
        .e_tag
        .as_deref()
        .and_then(|e| HeaderValue::from_str(e).ok())
    {
        headers.insert(header::ETAG, value);
    }
}

#[utoipa::path(
    head, path = "/files/{file_path}",
    params(("file_path" = String, Path, description = "Absolute path of the file, over several segments")),
    responses(
        (status = 200, description = "The headers of the file: `Content-Length`, `Content-Type` and `Last-Modified`"),
        (status = 404, description = "Not found"),
    ),
    tag = "Files",
)]
pub async fn get_file_metadata(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
) -> ApiResult<Response> {
    let target = open(
        &state,
        &principal,
        FilesPath::parse(&path)?.file()?,
        VendOp::Read,
    )
    .await?;
    let meta = target.head().await?;
    let mut response = Response::new(Body::empty());
    file_headers(response.headers_mut(), &meta, meta.size);
    Ok(response)
}

#[derive(Debug, Deserialize)]
pub struct UploadParams {
    pub overwrite: Option<bool>,
}

#[utoipa::path(
    put, path = "/files/{file_path}",
    params(
        ("file_path" = String, Path, description = "Absolute path of the file, over several segments"),
        ("overwrite" = Option<bool>, Query, description = "Replace a file that is already there"),
    ),
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    responses(
        (status = 204, description = "Written"),
        (status = 400, description = "In a table"),
        (status = 409, description = "The file is there, and `overwrite` is not set"),
    ),
    tag = "Files",
)]
pub async fn upload_file(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
    Query(params): Query<UploadParams>,
    body: Body,
) -> ApiResult<StatusCode> {
    let target = open(
        &state,
        &principal,
        FilesPath::parse(&path)?.file()?,
        VendOp::ReadWrite,
    )
    .await?;
    if !params.overwrite.unwrap_or(false) {
        match target.store().head(&target.key).await {
            Ok(_) => {
                return Err(Error::already_exists(format!(
                    "The file already exists: {}. Set overwrite to replace it.",
                    target.path
                ))
                .into());
            }
            Err(object_store::Error::NotFound { .. }) => {}
            Err(e) => return Err(target.error(e).into()),
        }
    }
    write_body(&target, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Write `body` to the key: at once when it is smaller than a part, else
/// part by part.
async fn write_body(target: &Target, body: Body) -> Result<(), Error> {
    let cut_off = |e: axum::Error| {
        Error::invalid_argument(format!("The upload of {} broke off: {e}", target.path))
    };
    let mut stream = body.into_data_stream();
    let mut first = BytesMut::new();
    while first.len() < PART_SIZE {
        match stream.next().await {
            Some(chunk) => first.extend_from_slice(&chunk.map_err(cut_off)?),
            None => {
                target
                    .store()
                    .put(&target.key, PutPayload::from(first.freeze()))
                    .await
                    .map_err(|e| target.error(e))?;
                return Ok(());
            }
        }
    }
    let upload = target
        .store()
        .put_multipart(&target.key)
        .await
        .map_err(|e| target.error(e))?;
    let mut writer = WriteMultipart::new_with_chunk_size(upload, PART_SIZE);
    writer.put(first.freeze());
    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                let _ = writer.abort().await;
                return Err(cut_off(e));
            }
        };
        if let Err(e) = writer.wait_for_capacity(UPLOAD_CONCURRENCY).await {
            let _ = writer.abort().await;
            return Err(target.error(e));
        }
        writer.put(chunk);
    }
    writer.finish().await.map_err(|e| target.error(e))?;
    Ok(())
}

#[utoipa::path(
    delete, path = "/files/{file_path}",
    params(("file_path" = String, Path, description = "Absolute path of the file, over several segments")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 400, description = "In a table"),
        (status = 404, description = "Not found"),
    ),
    tag = "Files",
)]
pub async fn delete_file(
    State(state): State<AppState>,
    principal: Principal,
    Path(path): Path<String>,
) -> ApiResult<StatusCode> {
    let target = open(
        &state,
        &principal,
        FilesPath::parse(&path)?.file()?,
        VendOp::ReadWrite,
    )
    .await?;
    target.head().await?;
    target
        .store()
        .delete(&target.key)
        .await
        .map_err(|e| target.error(e))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paths() {
        let path = FilesPath::parse("Volumes/c/s/v/a/b.txt").unwrap();
        assert_eq!(path.root, Root::Volumes);
        assert_eq!(path.full_name(), "c.s.v");
        assert_eq!(path.rest, ["a", "b.txt"]);
        assert_eq!(path.to_string(), "/Volumes/c/s/v/a/b.txt");
        let path = FilesPath::parse("Tables/c/s/t/").unwrap();
        assert_eq!((path.root, path.rest.len()), (Root::Tables, 0));
        assert!(path.file().is_err());
        for bad in [
            "Volumes/c/s",
            "Other/c/s/v",
            "Volumes/c/s/v/../w",
            "Volumes/c/s/v/./a",
            "Volumes/c/s/v/a\\b",
        ] {
            assert!(FilesPath::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn reads_ranges() {
        assert_eq!(byte_range("bytes=0-9", 100), Ok(Some(0..10)));
        assert_eq!(byte_range("bytes=90-200", 100), Ok(Some(90..100)));
        assert_eq!(byte_range("bytes=95-", 100), Ok(Some(95..100)));
        assert_eq!(byte_range("bytes=-10", 100), Ok(Some(90..100)));
        assert_eq!(byte_range("bytes=-500", 100), Ok(Some(0..100)));
        assert_eq!(byte_range("bytes=100-", 100), Err(()));
        assert_eq!(byte_range("bytes=0-1,5-6", 100), Ok(None));
        assert_eq!(byte_range("items=0-1", 100), Ok(None));
        assert_eq!(byte_range("bytes=5-1", 100), Ok(None));
    }
}
