//! HTTP client for unitycatalog-rs (and any wire-compatible UC server).
//!
//! Covers every resource exposed by the server. Each method is a thin
//! type-safe wrapper around the upstream OpenAPI shape. [`Client::raw_json`]
//! and [`Client::post_form`] reach any other endpoint.

use std::fmt;

pub use reqwest::Method;
use reqwest::{Client as Http, RequestBuilder, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use uc_api::iceberg_rest::{
    IcebergConfigResponse, IcebergListNamespacesResponse, IcebergListTablesResponse,
    IcebergLoadTableResponse,
};
use uc_api::{
    BackupResponse, CatalogInfo, CreateCatalog, CreateCredentialRequest, CreateExternalLocation,
    CreateFunctionRequest, CreateModelVersion, CreatePat, CreateRegisteredModel, CreateSchema,
    CreateStagingTable, CreateTable, CreateVolumeRequestContent, CredentialInfo, DeltaCommit,
    DeltaGetCommits, DeltaGetCommitsResponse, ExternalLocationInfo, FinalizeModelVersion,
    FunctionInfo, GenerateTemporaryPathCredential, GenerateTemporaryTableCredential,
    GenerateTemporaryVolumeCredential, GetMetastoreSummaryResponse, ListAuditEventsResponse,
    ListCatalogsResponse, ListCredentialsResponse, ListExternalLocationsResponse,
    ListFunctionsResponse, ListModelVersionsResponse, ListPatsResponse,
    ListRegisteredModelsResponse, ListSchemasResponse, ListScimGroupsResponse,
    ListScimUsersResponse, ListTablesResponse, ListVolumesResponseContent, ModelVersionInfo,
    PatInfo, PermissionsList, RegisteredModelInfo, RestoreResponse, SchemaInfo, ScimGroup,
    ScimUser, StagingTableInfo, TableInfo, TemporaryCredentials, UpdateCatalog,
    UpdateCredentialRequest, UpdateExternalLocation, UpdateModelVersion, UpdatePermissions,
    UpdateRegisteredModel, UpdateSchema, UpdateTable, UpdateVolumeRequestContent, VolumeInfo,
};
use uc_common::{ErrorCode, ErrorResponse, SecurableType};

/// The server's paths: [`Client::new`] takes the server's origin with
/// `API_BASE`.
pub use uc_common::{API_BASE, CONTROL_BASE};

mod contents;

pub use contents::Contents;

pub type Result<T> = std::result::Result<T, Error>;

/// Why a call failed. A refused call keeps its HTTP status and body;
/// [`Error::code`] and [`Error::message`] read the server's error from
/// the body.
#[derive(Debug)]
pub enum Error {
    /// The path does not make a URL.
    Url(url::ParseError),
    /// The request did not reach the server, or the response did not come.
    Http(reqwest::Error),
    /// The server refused the call.
    Status { status: u16, body: String },
    /// The response is not the JSON the method returns.
    Json(serde_json::Error),
}

impl Error {
    /// The HTTP status of a refused call.
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Status { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The server's error code; else one for the HTTP status, or
    /// `INTERNAL` when the server was not reached.
    pub fn code(&self) -> ErrorCode {
        uc_common::Error::from(self).code()
    }

    /// The server's message; else what failed.
    pub fn message(&self) -> String {
        uc_common::Error::from(self).message().to_string()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Url(e) => Some(e),
            Self::Http(e) => Some(e),
            Self::Status { .. } => None,
            Self::Json(e) => Some(e),
        }
    }
}

impl From<&Error> for uc_common::Error {
    fn from(e: &Error) -> Self {
        match e {
            Error::Status { status, body } => status_error(*status, body),
            Error::Url(e) => Self::internal(format!("url join: {e}")),
            Error::Http(e) => Self::internal(format!("http: {}", with_cause(e))),
            Error::Json(e) => Self::internal(format!("decode: {e}")),
        }
    }
}

/// `e` and its deepest cause, which reqwest's text leaves out: "error
/// sending request for url (…): Connection refused (os error 61)".
fn with_cause(e: &reqwest::Error) -> String {
    let mut deepest = None;
    let mut next = std::error::Error::source(e);
    while let Some(cause) = next {
        deepest = Some(cause);
        next = cause.source();
    }
    match deepest {
        Some(cause) => format!("{e}: {cause}"),
        None => e.to_string(),
    }
}

impl From<Error> for uc_common::Error {
    fn from(e: Error) -> Self {
        Self::from(&e)
    }
}

/// The server's code and message from a refused call's body, else a code
/// for its status.
fn status_error(status: u16, body: &str) -> uc_common::Error {
    use uc_common::Error;
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    match serde_json::from_str::<ErrorResponse>(body) {
        Ok(body) => Error::new(body.error_code, body.message),
        Err(_) => match status {
            StatusCode::NOT_FOUND => Error::not_found(format!("http {status}")),
            StatusCode::CONFLICT => Error::already_exists(format!("http {status}")),
            StatusCode::FORBIDDEN => Error::permission_denied(format!("http {status}")),
            StatusCode::UNAUTHORIZED => Error::unauthenticated(format!("http {status}")),
            StatusCode::BAD_REQUEST => Error::invalid_argument(format!("http {status}")),
            _ => Error::internal(format!("http {status}")),
        },
    }
}

/// A page of a list: at most `max_results` items, from `page_token` (the
/// `next_page_token` of the page before).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub max_results: Option<i32>,
    pub page_token: Option<String>,
}

impl Page {
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(max) = self.max_results {
            query.push(("max_results", max.to_string()));
        }
        if let Some(token) = &self.page_token {
            query.push(("page_token", token.clone()));
        }
        query
    }

    /// The Iceberg REST names: `pageSize` and `pageToken`.
    fn iceberg_query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(token) = &self.page_token {
            query.push(("pageToken", token.clone()));
        }
        if let Some(size) = self.max_results {
            query.push(("pageSize", size.to_string()));
        }
        query
    }
}

/// A page of a SCIM list: `filter` (such as `displayName eq "admins"`),
/// from `start_index` (1 is the first), `count` per page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScimPage {
    pub filter: Option<String>,
    pub start_index: Option<i32>,
    pub count: Option<i32>,
}

impl ScimPage {
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(filter) = &self.filter {
            query.push(("filter", filter.clone()));
        }
        if let Some(start) = self.start_index {
            query.push(("startIndex", start.to_string()));
        }
        if let Some(count) = self.count {
            query.push(("count", count.to_string()));
        }
        query
    }
}

/// Which audit events to list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub principal: Option<String>,
    /// An HTTP method, such as `DELETE`.
    pub method: Option<String>,
    pub path_prefix: Option<String>,
    /// The first time, in epoch milliseconds.
    pub since: Option<i64>,
    /// The last time, in epoch milliseconds.
    pub until: Option<i64>,
    pub max_results: Option<i32>,
    /// The token of the page after the last one.
    pub page_token: Option<String>,
}

impl AuditFilter {
    fn query(&self) -> Vec<(&'static str, String)> {
        [
            ("principal", self.principal.clone()),
            ("method", self.method.clone()),
            ("path_prefix", self.path_prefix.clone()),
            ("since", self.since.map(|t| t.to_string())),
            ("until", self.until.map(|t| t.to_string())),
            ("max_results", self.max_results.map(|m| m.to_string())),
            ("page_token", self.page_token.clone()),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect()
    }
}

/// No body: the type of `None` for [`Client::call`].
const NO_BODY: Option<&()> = None;

#[derive(Clone)]
pub struct Client {
    http: Http,
    base: Url,
    bearer: Option<String>,
}

impl Client {
    pub fn new(base: impl AsRef<str>) -> uc_common::Result<Self> {
        let mut base: Url = base
            .as_ref()
            .parse()
            .map_err(|e| uc_common::Error::invalid_argument(format!("invalid base URL: {e}")))?;
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        Ok(Self {
            http: Http::new(),
            base,
            bearer: None,
        })
    }

    pub fn with_bearer(mut self, token: impl Into<String>) -> Self {
        self.bearer = Some(token.into());
        self
    }

    pub async fn create_catalog(&self, req: &CreateCatalog) -> Result<CatalogInfo> {
        self.call(Method::POST, "catalogs", &[], Some(req)).await
    }
    pub async fn get_catalog(&self, name: &str) -> Result<CatalogInfo> {
        self.get(&path_segments(["catalogs", name]), &[]).await
    }
    pub async fn list_catalogs(&self, page: &Page) -> Result<ListCatalogsResponse> {
        self.get("catalogs", &page.query()).await
    }
    pub async fn update_catalog(&self, name: &str, req: &UpdateCatalog) -> Result<CatalogInfo> {
        let path = path_segments(["catalogs", name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_catalog(&self, name: &str, force: bool) -> Result<()> {
        self.delete(&path_segments(["catalogs", name]), &force_query(force))
            .await
    }

    pub async fn create_schema(&self, req: &CreateSchema) -> Result<SchemaInfo> {
        self.call(Method::POST, "schemas", &[], Some(req)).await
    }
    pub async fn get_schema(&self, full_name: &str) -> Result<SchemaInfo> {
        self.get(&path_segments(["schemas", full_name]), &[]).await
    }
    pub async fn list_schemas(
        &self,
        catalog_name: &str,
        page: &Page,
    ) -> Result<ListSchemasResponse> {
        let mut query = vec![("catalog_name", catalog_name.to_string())];
        query.extend(page.query());
        self.get("schemas", &query).await
    }
    pub async fn update_schema(&self, full_name: &str, req: &UpdateSchema) -> Result<SchemaInfo> {
        let path = path_segments(["schemas", full_name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_schema(&self, full_name: &str, force: bool) -> Result<()> {
        self.delete(&path_segments(["schemas", full_name]), &force_query(force))
            .await
    }

    pub async fn create_table(&self, req: &CreateTable) -> Result<TableInfo> {
        self.call(Method::POST, "tables", &[], Some(req)).await
    }
    /// A table. Upstream's server can show streaming tables and
    /// materialized views as MANAGED tables, as upstream's CLI asks.
    pub async fn get_table(
        &self,
        full_name: &str,
        read_streaming_table_as_managed: bool,
        read_materialized_view_as_managed: bool,
    ) -> Result<TableInfo> {
        let query = [
            (
                "read_streaming_table_as_managed",
                read_streaming_table_as_managed.to_string(),
            ),
            (
                "read_materialized_view_as_managed",
                read_materialized_view_as_managed.to_string(),
            ),
        ];
        self.get(&path_segments(["tables", full_name]), &query)
            .await
    }
    pub async fn list_tables(
        &self,
        catalog: &str,
        schema: &str,
        page: &Page,
    ) -> Result<ListTablesResponse> {
        self.get("tables", &in_schema(catalog, schema, page)).await
    }
    /// New comments for a table and its columns. This server's addition:
    /// upstream's API has no table update.
    pub async fn update_table(&self, full_name: &str, req: &UpdateTable) -> Result<TableInfo> {
        let path = path_segments(["tables", full_name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_table(&self, full_name: &str) -> Result<()> {
        self.delete(&path_segments(["tables", full_name]), &[])
            .await
    }
    pub async fn create_staging_table(&self, req: &CreateStagingTable) -> Result<StagingTableInfo> {
        self.call(Method::POST, "staging-tables", &[], Some(req))
            .await
    }

    pub async fn create_volume(&self, req: &CreateVolumeRequestContent) -> Result<VolumeInfo> {
        self.call(Method::POST, "volumes", &[], Some(req)).await
    }
    pub async fn get_volume(&self, full_name: &str) -> Result<VolumeInfo> {
        self.get(&path_segments(["volumes", full_name]), &[]).await
    }
    pub async fn list_volumes(
        &self,
        catalog: &str,
        schema: &str,
        page: &Page,
    ) -> Result<ListVolumesResponseContent> {
        self.get("volumes", &in_schema(catalog, schema, page)).await
    }
    pub async fn update_volume(
        &self,
        full_name: &str,
        req: &UpdateVolumeRequestContent,
    ) -> Result<VolumeInfo> {
        let path = path_segments(["volumes", full_name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_volume(&self, full_name: &str) -> Result<()> {
        self.delete(&path_segments(["volumes", full_name]), &[])
            .await
    }

    pub async fn create_function(&self, req: &CreateFunctionRequest) -> Result<FunctionInfo> {
        self.call(Method::POST, "functions", &[], Some(req)).await
    }
    pub async fn get_function(&self, full_name: &str) -> Result<FunctionInfo> {
        self.get(&path_segments(["functions", full_name]), &[])
            .await
    }
    pub async fn list_functions(
        &self,
        catalog: &str,
        schema: &str,
        page: &Page,
    ) -> Result<ListFunctionsResponse> {
        self.get("functions", &in_schema(catalog, schema, page))
            .await
    }
    pub async fn delete_function(&self, full_name: &str) -> Result<()> {
        self.delete(&path_segments(["functions", full_name]), &[])
            .await
    }

    pub async fn create_registered_model(
        &self,
        req: &CreateRegisteredModel,
    ) -> Result<RegisteredModelInfo> {
        self.call(Method::POST, "models", &[], Some(req)).await
    }
    pub async fn get_registered_model(&self, full_name: &str) -> Result<RegisteredModelInfo> {
        self.get(&path_segments(["models", full_name]), &[]).await
    }
    /// The registered models, of a catalog or schema when given.
    pub async fn list_registered_models(
        &self,
        catalog: Option<&str>,
        schema: Option<&str>,
        page: &Page,
    ) -> Result<ListRegisteredModelsResponse> {
        let mut query = Vec::new();
        if let Some(catalog) = catalog {
            query.push(("catalog_name", catalog.to_string()));
        }
        if let Some(schema) = schema {
            query.push(("schema_name", schema.to_string()));
        }
        query.extend(page.query());
        self.get("models", &query).await
    }
    pub async fn update_registered_model(
        &self,
        full_name: &str,
        req: &UpdateRegisteredModel,
    ) -> Result<RegisteredModelInfo> {
        let path = path_segments(["models", full_name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_registered_model(&self, full_name: &str, force: bool) -> Result<()> {
        self.delete(&path_segments(["models", full_name]), &force_query(force))
            .await
    }
    pub async fn create_model_version(&self, req: &CreateModelVersion) -> Result<ModelVersionInfo> {
        self.call(Method::POST, "models/versions", &[], Some(req))
            .await
    }
    pub async fn get_model_version(
        &self,
        full_name: &str,
        version: i64,
    ) -> Result<ModelVersionInfo> {
        self.get(&version_path(full_name, version), &[]).await
    }
    pub async fn list_model_versions(
        &self,
        full_name: &str,
        page: &Page,
    ) -> Result<ListModelVersionsResponse> {
        let path = path_segments(["models", full_name, "versions"]);
        self.get(&path, &page.query()).await
    }
    pub async fn update_model_version(
        &self,
        full_name: &str,
        version: i64,
        req: &UpdateModelVersion,
    ) -> Result<ModelVersionInfo> {
        let path = version_path(full_name, version);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_model_version(&self, full_name: &str, version: i64) -> Result<()> {
        self.delete(&version_path(full_name, version), &[]).await
    }
    /// Mark a model version READY.
    pub async fn finalize_model_version(
        &self,
        full_name: &str,
        version: i64,
    ) -> Result<ModelVersionInfo> {
        let path = format!("{}/finalize", version_path(full_name, version));
        let req = FinalizeModelVersion {
            full_name: Some(full_name.to_string()),
            version: Some(version),
        };
        self.call(Method::PATCH, &path, &[], Some(&req)).await
    }

    pub async fn create_credential(&self, req: &CreateCredentialRequest) -> Result<CredentialInfo> {
        self.call(Method::POST, "credentials", &[], Some(req)).await
    }
    pub async fn get_credential(&self, name: &str) -> Result<CredentialInfo> {
        self.get(&path_segments(["credentials", name]), &[]).await
    }
    pub async fn list_credentials(&self, page: &Page) -> Result<ListCredentialsResponse> {
        self.get("credentials", &page.query()).await
    }
    pub async fn update_credential(
        &self,
        name: &str,
        req: &UpdateCredentialRequest,
    ) -> Result<CredentialInfo> {
        let path = path_segments(["credentials", name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_credential(&self, name: &str, force: bool) -> Result<()> {
        self.delete(&path_segments(["credentials", name]), &force_query(force))
            .await
    }

    pub async fn generate_temporary_table_credentials(
        &self,
        req: &GenerateTemporaryTableCredential,
    ) -> Result<TemporaryCredentials> {
        self.call(Method::POST, "temporary-table-credentials", &[], Some(req))
            .await
    }
    pub async fn generate_temporary_volume_credentials(
        &self,
        req: &GenerateTemporaryVolumeCredential,
    ) -> Result<TemporaryCredentials> {
        self.call(Method::POST, "temporary-volume-credentials", &[], Some(req))
            .await
    }
    pub async fn generate_temporary_path_credentials(
        &self,
        req: &GenerateTemporaryPathCredential,
    ) -> Result<TemporaryCredentials> {
        self.call(Method::POST, "temporary-path-credentials", &[], Some(req))
            .await
    }

    pub async fn create_external_location(
        &self,
        req: &CreateExternalLocation,
    ) -> Result<ExternalLocationInfo> {
        self.call(Method::POST, "external-locations", &[], Some(req))
            .await
    }
    pub async fn get_external_location(&self, name: &str) -> Result<ExternalLocationInfo> {
        self.get(&path_segments(["external-locations", name]), &[])
            .await
    }
    pub async fn list_external_locations(
        &self,
        page: &Page,
    ) -> Result<ListExternalLocationsResponse> {
        self.get("external-locations", &page.query()).await
    }
    pub async fn update_external_location(
        &self,
        name: &str,
        req: &UpdateExternalLocation,
    ) -> Result<ExternalLocationInfo> {
        let path = path_segments(["external-locations", name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn delete_external_location(&self, name: &str, force: bool) -> Result<()> {
        let path = path_segments(["external-locations", name]);
        self.delete(&path, &force_query(force)).await
    }

    /// The privileges on a securable; only `principal`'s when given.
    pub async fn get_permissions(
        &self,
        securable_type: SecurableType,
        full_name: &str,
        principal: Option<&str>,
    ) -> Result<PermissionsList> {
        let path = path_segments(["permissions", securable_type.as_str(), full_name]);
        let query: Vec<_> = principal
            .map(|p| ("principal", p.to_string()))
            .into_iter()
            .collect();
        self.get(&path, &query).await
    }
    pub async fn update_permissions(
        &self,
        securable_type: SecurableType,
        full_name: &str,
        req: &UpdatePermissions,
    ) -> Result<PermissionsList> {
        let path = path_segments(["permissions", securable_type.as_str(), full_name]);
        self.call(Method::PATCH, &path, &[], Some(req)).await
    }
    pub async fn metastore_summary(&self) -> Result<GetMetastoreSummaryResponse> {
        self.get("metastore_summary", &[]).await
    }

    pub async fn create_pat(&self, req: &CreatePat) -> Result<PatInfo> {
        self.call(Method::POST, "tokens", &[], Some(req)).await
    }
    /// The caller's own tokens, one page.
    pub async fn list_pats(&self, page: &Page) -> Result<ListPatsResponse> {
        self.get("tokens", &page.query()).await
    }
    pub async fn revoke_pat(&self, id: &str) -> Result<()> {
        self.delete(&path_segments(["tokens", id]), &[]).await
    }

    pub async fn create_user(&self, user: &ScimUser) -> Result<ScimUser> {
        self.call(Method::POST, &scim(["Users"]), &[], Some(user))
            .await
    }
    pub async fn get_user(&self, id: &str) -> Result<ScimUser> {
        self.get(&scim(["Users", id]), &[]).await
    }
    /// The signed-in user (`/scim2/Me`).
    pub async fn me(&self) -> Result<ScimUser> {
        self.get(&scim(["Me"]), &[]).await
    }
    pub async fn list_users(&self, page: &ScimPage) -> Result<ListScimUsersResponse> {
        self.get(&scim(["Users"]), &page.query()).await
    }
    /// Replace the user `id`, as SCIM's PUT.
    pub async fn update_user(&self, id: &str, user: &ScimUser) -> Result<ScimUser> {
        self.call(Method::PUT, &scim(["Users", id]), &[], Some(user))
            .await
    }
    pub async fn delete_user(&self, id: &str) -> Result<()> {
        self.delete(&scim(["Users", id]), &[]).await
    }
    pub async fn create_group(&self, group: &ScimGroup) -> Result<ScimGroup> {
        self.call(Method::POST, &scim(["Groups"]), &[], Some(group))
            .await
    }
    pub async fn get_group(&self, id: &str) -> Result<ScimGroup> {
        self.get(&scim(["Groups", id]), &[]).await
    }
    pub async fn list_groups(&self, page: &ScimPage) -> Result<ListScimGroupsResponse> {
        self.get(&scim(["Groups"]), &page.query()).await
    }
    /// Replace the group `id`, as SCIM's PUT: its name and its members.
    pub async fn update_group(&self, id: &str, group: &ScimGroup) -> Result<ScimGroup> {
        self.call(Method::PUT, &scim(["Groups", id]), &[], Some(group))
            .await
    }
    pub async fn delete_group(&self, id: &str) -> Result<()> {
        self.delete(&scim(["Groups", id]), &[]).await
    }

    /// A snapshot of the whole catalog, for [`Client::restore`].
    pub async fn backup(&self) -> Result<BackupResponse> {
        self.get("admin/backup", &[]).await
    }
    /// Replace the catalog with a snapshot.
    pub async fn restore(&self, snapshot: &BackupResponse) -> Result<RestoreResponse> {
        self.call(Method::POST, "admin/restore", &[], Some(snapshot))
            .await
    }
    pub async fn list_audit_events(&self, filter: &AuditFilter) -> Result<ListAuditEventsResponse> {
        self.get("admin/audit", &filter.query()).await
    }

    /// The Iceberg client config of a catalog (the warehouse).
    pub async fn iceberg_config(&self, catalog: &str) -> Result<IcebergConfigResponse> {
        let query = [("warehouse", catalog.to_string())];
        self.get("iceberg/v1/config", &query).await
    }
    pub async fn list_iceberg_namespaces(
        &self,
        catalog: &str,
        page: &Page,
    ) -> Result<IcebergListNamespacesResponse> {
        let path = path_segments(["iceberg", "v1", "catalogs", catalog, "namespaces"]);
        self.get(&path, &page.iceberg_query()).await
    }
    pub async fn list_iceberg_tables(
        &self,
        catalog: &str,
        namespace: &str,
        page: &Page,
    ) -> Result<IcebergListTablesResponse> {
        let path = path_segments([
            "iceberg",
            "v1",
            "catalogs",
            catalog,
            "namespaces",
            namespace,
            "tables",
        ]);
        self.get(&path, &page.iceberg_query()).await
    }
    pub async fn load_iceberg_table(
        &self,
        catalog: &str,
        namespace: &str,
        table: &str,
    ) -> Result<IcebergLoadTableResponse> {
        let path = path_segments([
            "iceberg",
            "v1",
            "catalogs",
            catalog,
            "namespaces",
            namespace,
            "tables",
            table,
        ]);
        self.get(&path, &[]).await
    }

    /// The commits the server holds for a table, and its latest version.
    pub async fn get_delta_commits(
        &self,
        req: &DeltaGetCommits,
    ) -> Result<DeltaGetCommitsResponse> {
        self.call(Method::GET, "delta/preview/commits", &[], Some(req))
            .await
    }
    /// Ratify a staged commit; a conflict is status 409.
    pub async fn commit_delta(&self, req: &DeltaCommit) -> Result<()> {
        let request = self.request(Method::POST, "delta/preview/commits", &[], Some(req))?;
        send(request).await.map(|_| ())
    }

    /// Call any endpoint. `path` is below the base URL (`"catalogs"`), or
    /// from the server's root (`"/api/1.0/unity-control/scim2/Users"`).
    /// The JSON response; `null` when it has none.
    pub async fn raw_json(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value> {
        self.call(method, path, query, body).await
    }

    /// `POST` a form to `path` (as [`Client::raw_json`] takes it), as the
    /// OAuth token exchange takes it; the JSON response.
    pub async fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<Value> {
        let url = self.base.join(path).map_err(Error::Url)?;
        parse(send(self.http.post(url).form(form)).await?)
    }

    fn request<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<RequestBuilder> {
        let url = self.base.join(path).map_err(Error::Url)?;
        let mut request = self.http.request(method, url).query(query);
        if let Some(token) = &self.bearer {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        Ok(request)
    }

    /// Call `path`; the response as `T`.
    async fn call<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<T> {
        parse(send(self.request(method, path, query, body)?).await?)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        self.call(Method::GET, path, query, NO_BODY).await
    }

    /// `DELETE`, whatever the response's body.
    async fn delete(&self, path: &str, query: &[(&str, String)]) -> Result<()> {
        send(self.request(Method::DELETE, path, query, NO_BODY)?)
            .await
            .map(|_| ())
    }
}

/// The body of a 2xx response; else the status and body.
async fn send(request: RequestBuilder) -> Result<String> {
    let response = request.send().await.map_err(Error::Http)?;
    let status = response.status();
    let body = response.text().await.map_err(Error::Http)?;
    if status.is_success() {
        Ok(body)
    } else {
        Err(Error::Status {
            status: status.as_u16(),
            body,
        })
    }
}

/// A response body as `T`; an empty one is `null`.
fn parse<T: DeserializeOwned>(body: String) -> Result<T> {
    if body.trim().is_empty() {
        serde_json::from_value(Value::Null)
    } else {
        serde_json::from_str(&body)
    }
    .map_err(Error::Json)
}

fn force_query(force: bool) -> [(&'static str, String); 1] {
    [("force", force.to_string())]
}

/// `catalog_name`, `schema_name` and the page.
fn in_schema(catalog: &str, schema: &str, page: &Page) -> Vec<(&'static str, String)> {
    let mut query = vec![
        ("catalog_name", catalog.to_string()),
        ("schema_name", schema.to_string()),
    ];
    query.extend(page.query());
    query
}

fn version_path(full_name: &str, version: i64) -> String {
    path_segments(["models", full_name, "versions", &version.to_string()])
}

/// A SCIM path, from the server's root.
fn scim<'a>(segments: impl IntoIterator<Item = &'a str>) -> String {
    format!("{CONTROL_BASE}/scim2/{}", path_segments(segments))
}

/// Percent-encode each segment with the path-safe set and join with `/`.
/// Use it for every segment that carries user data, such as a name or an
/// id, because these can hold `?`, `#`, `/` or `%`.
pub fn path_segments<'a>(segments: impl IntoIterator<Item = &'a str>) -> String {
    use percent_encoding::utf8_percent_encode;
    let mut out = String::new();
    for (i, s) in segments.into_iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        out.push_str(&utf8_percent_encode(s, PATH).to_string());
    }
    out
}

// Path-safe set per RFC 3986 ("pchar" minus the reserved characters that
// delimit segments).
const PATH: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'?')
    .add(b'#')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b'%');
