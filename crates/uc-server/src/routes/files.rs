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
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use futures::StreamExt;
use object_store::{
    GetOptions, GetRange, ListResult, ObjectMeta, ObjectStore, buffered::BufWriter,
    list::PaginatedListOptions, local::LocalFileSystem, path::Path as ObjectPath,
};
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use uc_api::{DirectoryEntry, ListDirectoryContentsResponse};
use uc_common::{
    Error, NormalizedUrl, SecurableType, lenient_bool, lenient_opt_i64, three_part_full_name,
};

use crate::{
    authn::principal::Principal,
    authz::{self, rules},
    http::{
        error::ApiResult,
        extract::{Json, Path, Query},
    },
    server::state::AppState,
    storage::{Opened, delete_all, vendor::VendOp},
};

/// The most entries of a page, and the size of a page without
/// `page_size`, as Databricks has it.
const MAX_PAGE_SIZE: usize = 1000;

/// The size of the parts of an upload. Smaller files are written at once.
const PART_SIZE: usize = 8 * 1024 * 1024;

/// The routes of directories and of files, under [`uc_common::FILES_BASE`].
const DIRECTORIES: &str = "/directories";
const FILES: &str = "/files";

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
            &format!("{DIRECTORIES}/{{*directory_path}}"),
            get(list_directory_contents)
                .head(get_directory_metadata)
                .put(create_directory)
                .delete(delete_directory),
        )
        .route(
            &format!("{FILES}/{{*file_path}}"),
            get(download_file)
                .head(get_file_metadata)
                .put(upload_file)
                .delete(delete_file),
        )
}

/// Whether a request of `method` to `path`, under [`uc_common::FILES_BASE`],
/// can take longer than the request timeout: an upload takes as long as its
/// body, and the delete of a directory as long as what it holds.
pub fn takes_long(method: &Method, path: &str) -> bool {
    let under = |route: &str| {
        path.strip_prefix(route)
            .is_some_and(|rest| rest.starts_with('/'))
    };
    (method == Method::PUT && under(FILES)) || (method == Method::DELETE && under(DIRECTORIES))
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
}

impl fmt::Display for FilesPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [catalog, schema, name] = &self.names;
        write!(f, "/{}/{catalog}/{schema}/{name}", self.root.as_str())?;
        self.rest.iter().try_for_each(|part| write!(f, "/{part}"))
    }
}

/// The store of the volume or table of a path, and the path's key in it.
struct Target {
    path: FilesPath,
    opened: Arc<Opened>,
    key: ObjectPath,
}

impl Target {
    fn store(&self) -> &Arc<dyn ObjectStore> {
        &self.opened.store
    }

    /// Whether the storage is local, which has real directories: the one
    /// store without paged listing.
    fn local(&self) -> bool {
        self.opened.pages.is_none()
    }

    /// The error of the store for this path.
    fn error(&self, e: &object_store::Error) -> Error {
        storage_error(&self.path, e)
    }

    /// The file system path of the key, for local storage.
    fn local_path(&self) -> Result<std::path::PathBuf, Error> {
        LocalFileSystem::new()
            .path_to_filesystem(&self.key)
            .map_err(|e| self.error(&e))
    }

    /// Whether the key is a local directory.
    async fn local_directory(&self) -> Result<bool, Error> {
        Ok(tokio::fs::metadata(self.local_path()?)
            .await
            .is_ok_and(|m| m.is_dir()))
    }

    /// A page of the directory of the key: at most `size` directories,
    /// then files, and the token of the next page. Object storage pages
    /// at the storage. A local directory is listed whole and paged by name:
    /// the token is the number of entries before the page.
    async fn page(
        &self,
        size: usize,
        token: Option<String>,
    ) -> Result<(ListResult, Option<String>), Error> {
        let Some(pages) = &self.opened.pages else {
            return self.local_page(size, token.as_deref()).await;
        };
        let prefix = (!self.key.as_ref().is_empty()).then(|| format!("{}/", self.key));
        let options = PaginatedListOptions {
            delimiter: Some("/".into()),
            max_keys: Some(size),
            page_token: token,
            ..Default::default()
        };
        let page = pages
            .list_paginated(prefix.as_deref(), options)
            .await
            .map_err(|e| self.error(&e))?;
        Ok((page.result, page.page_token))
    }

    async fn local_page(
        &self,
        size: usize,
        token: Option<&str>,
    ) -> Result<(ListResult, Option<String>), Error> {
        let start = match token {
            None => 0,
            Some(token) => token
                .parse::<usize>()
                .map_err(|_| Error::invalid_argument(format!("Invalid page token: {token}")))?,
        };
        let mut all = self
            .store()
            .list_with_delimiter(Some(&self.key))
            .await
            .map_err(|e| self.error(&e))?;
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

    /// Whether the directory of the key holds any file or directory.
    async fn has_entries(&self) -> Result<bool, Error> {
        let (first, _) = self.page(1, None).await?;
        Ok(!is_empty(&first))
    }

    /// Whether the directory of the key exists: the volume or table
    /// itself, a local directory, or a prefix of files.
    async fn directory_exists(&self) -> Result<bool, Error> {
        match (self.path.rest.is_empty(), self.local()) {
            (true, _) => Ok(true),
            (false, true) => self.local_directory().await,
            (false, false) => self.has_entries().await,
        }
    }

    /// The metadata of the file of the key.
    async fn head(&self) -> Result<ObjectMeta, Error> {
        self.store()
            .head(&self.key)
            .await
            .map_err(|e| self.error(&e))
    }
}

fn is_empty(listed: &ListResult) -> bool {
    listed.common_prefixes.is_empty() && listed.objects.is_empty()
}

/// The last part of `location`, as its name.
fn name_of(location: &ObjectPath) -> String {
    let part = location.filename().unwrap_or_default();
    percent_decode_str(part).decode_utf8_lossy().into_owned()
}

/// The error of the store for `path`.
fn storage_error(path: &FilesPath, e: &object_store::Error) -> Error {
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
    let user = &principal.username;
    // The check's error comes first: it is a 404 for a missing securable.
    let location = match path.root {
        Root::Volumes => {
            let (allowed, volume) = tokio::join!(
                authz::check(state, user, SecurableType::Volume, &full_name, |a, l| {
                    rules::vend_volume(a, l, write)
                }),
                state.metastore.volumes().get(&full_name),
            );
            allowed?;
            volume?.storage_location
        }
        Root::Tables => {
            if write {
                return Err(Error::invalid_argument(format!(
                    "Invalid path {path}: the files of tables are read-only."
                ))
                .into());
            }
            let (allowed, table) = tokio::join!(
                authz::check(state, user, SecurableType::Table, &full_name, |a, l| {
                    rules::vend_table(a, l, false)
                }),
                state.metastore.tables().get(&full_name),
            );
            allowed?;
            table?.storage_location
        }
    };
    let location = location.filter(|l| !l.is_empty()).ok_or_else(|| {
        Error::invalid_argument(format!(
            "The {} {full_name} has no storage location.",
            path.root.securable()
        ))
    })?;
    let location = NormalizedUrl::parse(&location)?;
    let opened = state
        .stores
        .open(&state.metastore, &state.vendor, &location, op)
        .await
        .map_err(|e| Error::internal(format!("Cannot open the storage of {path}: {}", e.0)))?;
    let key = path
        .rest
        .iter()
        .fold(opened.root.clone(), |key, part| key.child(part.as_str()));
    Ok(Target { path, opened, key })
}

#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default, deserialize_with = "lenient_opt_i64")]
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
    let first = token.is_none();
    let target = open(&state, &principal, FilesPath::parse(&path)?, VendOp::Read).await?;
    let (listed, next_page_token) = target.page(size, token).await?;
    // An empty first page below the root: in object storage the directory
    // is not there; a local one can be empty.
    if is_empty(&listed)
        && first
        && !target.path.rest.is_empty()
        && !(target.local() && target.local_directory().await?)
    {
        return Err(Error::not_found(format!("Not found: {}", target.path)).into());
    }
    let base = target.path.to_string();
    let directories = listed.common_prefixes.iter().map(|location| {
        let name = name_of(location);
        DirectoryEntry {
            path: format!("{base}/{name}/"),
            name,
            is_directory: true,
            file_size: None,
            last_modified: None,
        }
    });
    let files = listed.objects.iter().map(|meta| {
        let name = name_of(&meta.location);
        DirectoryEntry {
            path: format!("{base}/{name}"),
            name,
            is_directory: false,
            file_size: i64::try_from(meta.size).ok(),
            last_modified: Some(meta.last_modified.timestamp_millis()),
        }
    });
    Ok(Json(ListDirectoryContentsResponse {
        contents: directories.chain(files).collect(),
        next_page_token,
    }))
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
    if target.local() {
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
    #[serde(default, deserialize_with = "lenient_bool")]
    pub recursive: bool,
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
    let not_empty = || {
        Error::failed_precondition(format!(
            "The directory is not empty: {}. Delete its files first, or set recursive.",
            target.path
        ))
    };
    // In object storage a directory without files is not there: nothing to
    // delete.
    if !target.local() {
        if params.recursive {
            delete_all(target.store().as_ref(), &target.key)
                .await
                .map_err(|e| target.error(&e))?;
        } else if target.has_entries().await? {
            return Err(not_empty().into());
        }
        return Ok(StatusCode::NO_CONTENT);
    }
    let dir = target.local_path()?;
    let removed = match params.recursive {
        true => tokio::fs::remove_dir_all(&dir).await,
        false => tokio::fs::remove_dir(&dir).await,
    };
    match removed {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(match e.kind() {
            std::io::ErrorKind::NotFound => Error::not_found(format!("Not found: {}", target.path)),
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
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(byte_range);
    let options = GetOptions {
        range: range.clone(),
        ..Default::default()
    };
    let not_satisfiable = |size: u64| {
        Ok((
            StatusCode::RANGE_NOT_SATISFIABLE,
            [(header::CONTENT_RANGE, format!("bytes */{size}"))],
        )
            .into_response())
    };
    let result = match target.store().get_opts(&target.key, options).await {
        Ok(result) => result,
        // A range outside the file fails: read the size for the answer.
        Err(e) => {
            let unsatisfiable = match (&range, &e) {
                (_, object_store::Error::NotFound { .. }) | (None, _) => None,
                (Some(range), _) => {
                    let size = target.head().await?.size;
                    range.as_range(size).is_err().then_some(size)
                }
            };
            return match unsatisfiable {
                Some(size) => not_satisfiable(size),
                None => Err(target.error(&e).into()),
            };
        }
    };
    let meta = result.meta.clone();
    let served = result.range.clone();
    if range.is_some() && served.is_empty() {
        return not_satisfiable(meta.size);
    }
    let path = target.path.clone();
    let body = result
        .into_stream()
        .map(move |chunk| chunk.map_err(|e| storage_error(&path, &e)));
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
/// `bytes=-n`. A header of another form gives `None`: the whole file.
fn byte_range(value: &str) -> Option<GetRange> {
    let (start, end) = value
        .trim()
        .strip_prefix("bytes=")
        .filter(|spec| !spec.contains(','))?
        .split_once('-')?;
    let (start, end) = (start.trim(), end.trim());
    match (start.parse::<u64>(), end.parse::<u64>()) {
        (Ok(start), Ok(end)) if start <= end => {
            Some(GetRange::Bounded(start..end.saturating_add(1)))
        }
        (Ok(start), Err(_)) if end.is_empty() => Some(GetRange::Offset(start)),
        (Err(_), Ok(suffix)) if start.is_empty() && suffix > 0 => Some(GetRange::Suffix(suffix)),
        _ => None,
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
    #[serde(default, deserialize_with = "lenient_bool")]
    pub overwrite: bool,
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
    if !params.overwrite {
        match target.store().head(&target.key).await {
            Ok(_) => {
                return Err(Error::already_exists(format!(
                    "The file already exists: {}. Set overwrite to replace it.",
                    target.path
                ))
                .into());
            }
            Err(object_store::Error::NotFound { .. }) => {}
            Err(e) => return Err(target.error(&e).into()),
        }
    }
    write_body(&target, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Write `body` to the key: at once when it is smaller than a part, else
/// part by part.
async fn write_body(target: &Target, body: Body) -> Result<(), Error> {
    let mut writer =
        BufWriter::with_capacity(target.store().clone(), target.key.clone(), PART_SIZE)
            .with_max_concurrency(UPLOAD_CONCURRENCY);
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let written = match chunk {
            Ok(chunk) => writer.put(chunk).await.map_err(|e| target.error(&e)),
            Err(e) => Err(Error::invalid_argument(format!(
                "The upload of {} broke off: {e}",
                target.path
            ))),
        };
        if let Err(e) = written {
            let _ = writer.abort().await;
            return Err(e);
        }
    }
    // The writer gives the store's errors back as I/O errors.
    writer.shutdown().await.map_err(|e| {
        match e
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<object_store::Error>())
        {
            Some(inner) => target.error(inner),
            None => Error::internal(format!("Storage error on {}: {e}", target.path)),
        }
    })
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
        .map_err(|e| target.error(&e))?;
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
        // In a file of 100 bytes; `Some(None)` is outside it.
        let range = |value| byte_range(value).map(|r| r.as_range(100).ok());
        assert_eq!(range("bytes=0-9"), Some(Some(0..10)));
        assert_eq!(range("bytes=90-200"), Some(Some(90..100)));
        assert_eq!(range("bytes=95-"), Some(Some(95..100)));
        assert_eq!(range("bytes=-10"), Some(Some(90..100)));
        assert_eq!(range("bytes=-500"), Some(Some(0..100)));
        assert_eq!(range("bytes=100-"), Some(None));
        for whole in ["bytes=0-1,5-6", "items=0-1", "bytes=5-1", "bytes=-0"] {
            assert!(byte_range(whole).is_none(), "{whole}");
        }
    }

    #[test]
    fn uploads_and_directory_deletes_take_long() {
        assert!(takes_long(&Method::PUT, "/files/Volumes/c/s/v/a"));
        assert!(takes_long(&Method::DELETE, "/directories/Volumes/c/s/v/a"));
        assert!(!takes_long(&Method::GET, "/files/Volumes/c/s/v/a"));
        assert!(!takes_long(&Method::DELETE, "/files/Volumes/c/s/v/a"));
        assert!(!takes_long(&Method::PUT, "/filesystem/x"));
    }
}
