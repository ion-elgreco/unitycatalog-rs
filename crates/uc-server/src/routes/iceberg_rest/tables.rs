//! Iceberg REST tables: native Iceberg tables and UniForm Delta tables,
//! read, written and vended for. Upstream `IcebergRestCatalogService`.

use std::collections::{BTreeMap, HashSet};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use iceberg::spec::TableMetadata;
use uc_api::{
    CreateStagingTable, CreateTable, DataSourceFormat, Privilege, TableType,
    iceberg_rest::{
        IcebergCommitTableRequest, IcebergCreateTableRequest, IcebergErrorResponse,
        IcebergListTablesResponse, IcebergLoadCredentialsResponse, IcebergLoadTableResponse,
        IcebergRenameTableRequest, IcebergStorageCredential, IcebergTableIdentifier,
    },
};
use uc_common::{
    Error, ErrorCode, MANAGED_STORAGE_PREFIX, NormalizedUrl, SecurablePropertiesMap, SecurableType,
    schema_full_name, three_part_full_name,
};
use uc_metastore::IcebergTableState;

use super::{
    IcebergError, IcebergJson, IcebergResult, PREFIX_BASE, PageParams, check_writes_enabled,
    listing,
};
use crate::{
    authn::principal::Principal,
    authz::{self, rules},
    http::extract::{Path, Query},
    http::{form_encode, header_case},
    iceberg::{
        commit::{self, CommitBody, CreateTableBody},
        config::table_config,
        location::validate_local_location,
        metadata, schema,
    },
    server::state::AppState,
    storage::ServerStorage,
    storage::vendor::VendOp,
};

/// The table, if the catalog serves it as an Iceberg table: a native
/// Iceberg table or a UniForm Delta table, both with Iceberg metadata.
async fn served_table(
    state: &AppState,
    catalog: &str,
    namespace: &str,
    table: &str,
) -> IcebergResult<IcebergTableState> {
    let state = state
        .metastore
        .iceberg_tables()
        .table_state(catalog, namespace, table)
        .await?;
    if state.metadata_location.is_none() {
        return Err(IcebergError::no_such_table(format!(
            "Table does not exist: {namespace}.{table}"
        )));
    }
    Ok(state)
}

/// Upstream `credentialsEndpoint`: the table's `/credentials` path,
/// relative to the catalog URI as Iceberg clients resolve it, so that they
/// can renew vended credentials before they expire. Names are encoded as
/// Iceberg's `RESTUtil.encodeString` does.
fn credentials_endpoint(catalog: &str, namespace: &str, table: &str) -> String {
    format!(
        "v1/{PREFIX_BASE}{catalog}/namespaces/{}/tables/{}/credentials",
        form_encode(namespace),
        form_encode(table)
    )
}

/// Upstream `getLoadCredentialPrivileges`: UniForm tables are read-only;
/// native Iceberg tables are writable for their owner or a caller with
/// `SELECT` and `MODIFY` (anyone, with authorization off).
async fn load_privileges(
    state: &AppState,
    principal: &Principal,
    served: &IcebergTableState,
    full_name: &str,
) -> IcebergResult<VendOp> {
    if served.data_source_format != Some(DataSourceFormat::Iceberg) {
        return Ok(VendOp::Read);
    }
    if !state.authorization_enabled {
        return Ok(VendOp::ReadWrite);
    }
    let access = authz::Access::load(state, principal).await?;
    let lineage = authz::lineage(state, SecurableType::Table, full_name).await?;
    let can_write = access.owner(&lineage, authz::Level::Object)
        || access.all(
            &lineage,
            authz::Level::Object,
            &[Privilege::Select, Privilege::Modify],
        );
    Ok(match can_write {
        true => VendOp::ReadWrite,
        false => VendOp::Read,
    })
}

/// The FileIO properties a caller gets for a served table.
async fn client_config(
    state: &AppState,
    principal: &Principal,
    served: &IcebergTableState,
    (catalog, namespace, table): (&str, &str, &str),
    table_location: &NormalizedUrl,
) -> IcebergResult<BTreeMap<String, String>> {
    let full_name = three_part_full_name(catalog, namespace, table);
    // Never a location from the metadata: that is client-written.
    let op = load_privileges(state, principal, served, &full_name).await?;
    let endpoint = credentials_endpoint(catalog, namespace, table);
    Ok(table_config(state, table_location, op, &endpoint).await?)
}

/// The persisted metadata file and table root of a served table.
fn locations(table: &IcebergTableState) -> IcebergResult<(NormalizedUrl, NormalizedUrl)> {
    let missing =
        || uc_common::Error::internal(format!("Table {} has no storage location", table.table_id));
    let metadata_location =
        NormalizedUrl::parse(table.metadata_location.as_deref().ok_or_else(missing)?)?;
    let table_location =
        NormalizedUrl::parse(table.storage_location.as_deref().ok_or_else(missing)?)?;
    Ok((metadata_location, table_location))
}

#[utoipa::path(
    get, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables",
    operation_id = "iceberg_list_tables",
    params(
        ("catalog" = String, Path),
        ("namespace" = String, Path),
        ("pageToken" = Option<String>, Query),
        ("pageSize" = Option<String>, Query),
    ),
    responses(
        (status = 200, body = IcebergListTablesResponse),
        (status = 404, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn list_tables(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace)): Path<(String, String)>,
    Query(page): Query<PageParams>,
) -> IcebergResult<Json<IcebergListTablesResponse>> {
    // Only tables with Iceberg metadata: the query picks them, so a page is
    // a page of names the client can resume after.
    let (tables, next_page_token) = listing(page.requested()?, |page| {
        let (metastore, catalog, namespace) =
            (state.metastore.clone(), catalog.clone(), namespace.clone());
        async move {
            metastore
                .iceberg_tables()
                .list_tables(&catalog, &namespace, &page)
                .await
        }
    })
    .await?;
    // Each table the caller may read (upstream `GET_TABLE` filter).
    let access = authz::Access::load(&state, &principal).await?;
    let schema = schema_full_name(&catalog, &namespace);
    let tables = authz::filter_children(
        &state,
        &access,
        SecurableType::Table,
        tables,
        |t| (schema.clone(), Some(t.table_id.clone()), t.owner.clone()),
        rules::get_table,
    )
    .await?;
    let identifiers = tables
        .into_iter()
        .map(|t| IcebergTableIdentifier {
            namespace: vec![namespace.clone()],
            name: t.name,
        })
        .collect();
    Ok(Json(IcebergListTablesResponse {
        identifiers,
        next_page_token,
    }))
}

#[utoipa::path(
    head, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
    operation_id = "iceberg_table_exists",
    params(("catalog" = String, Path), ("namespace" = String, Path), ("table" = String, Path)),
    responses(
        (status = 204, description = "The table exists"),
        (status = 404, description = "No such table"),
    ),
    tag = "Iceberg",
)]
pub async fn table_exists(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace, table)): Path<(String, String, String)>,
) -> IcebergResult<StatusCode> {
    let full_name = three_part_full_name(&catalog, &namespace, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::get_table,
    )
    .await?;
    served_table(&state, &catalog, &namespace, &table).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Default, Deserialize)]
pub struct LoadTableParams {
    pub snapshots: Option<String>,
}

/// Upstream `refsOnly`: the `snapshots` mode is `all` (the default) or
/// `refs`, the snapshots the table's refs point at.
fn refs_only(snapshots: Option<&str>) -> IcebergResult<bool> {
    match snapshots.map(str::to_ascii_lowercase).as_deref() {
        None | Some("all") => Ok(false),
        Some("refs") => Ok(true),
        Some(_) => Err(IcebergError::bad_request(format!(
            "Invalid snapshots parameter: {}. Valid values are all and refs.",
            snapshots.unwrap_or_default()
        ))),
    }
}

/// Upstream `metadataETag`: a weak tag of the metadata version and the
/// snapshots mode (whose bodies differ), hashed so that it does not reveal
/// the metadata location.
fn metadata_etag(metadata_location: &str, refs_only: bool) -> String {
    let mode = if refs_only { "refs" } else { "all" };
    let digest = Sha256::digest(format!("{metadata_location}\n{mode}"));
    format!("W/\"{digest:x}\"")
}

/// Upstream `holdsETag`: `If-None-Match` lists this exact tag (`*` does not
/// count).
fn holds_etag(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|tag| tag.trim() == etag)
}

/// Upstream `suppressHistoricalSnapshots`: drop the snapshots no ref points
/// at. Version 1 metadata may have no refs; its current snapshot is `main`.
fn suppress_historical_snapshots(metadata: &mut Value) {
    let mut kept: HashSet<i64> = metadata["refs"]
        .as_object()
        .map(|refs| {
            refs.values()
                .filter_map(|r| r["snapshot-id"].as_i64())
                .collect()
        })
        .unwrap_or_default();
    kept.extend(metadata["current-snapshot-id"].as_i64());
    if let Some(snapshots) = metadata.get_mut("snapshots").and_then(Value::as_array_mut) {
        snapshots.retain(|s| {
            s["snapshot-id"]
                .as_i64()
                .is_some_and(|id| kept.contains(&id))
        });
    }
}

#[utoipa::path(
    get, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
    operation_id = "iceberg_load_table",
    params(
        ("catalog" = String, Path),
        ("namespace" = String, Path),
        ("table" = String, Path),
        ("snapshots" = Option<String>, Query, description = "`all` (default) or `refs`"),
        ("If-None-Match" = Option<String>, Header),
    ),
    responses(
        (status = 200, body = IcebergLoadTableResponse, headers(("ETag" = String))),
        (status = 304, description = "The caller holds this metadata version", headers(("ETag" = String))),
        (status = 400, body = IcebergErrorResponse),
        (status = 404, body = IcebergErrorResponse),
        (status = 500, body = IcebergErrorResponse, description = "The metadata cannot be read"),
    ),
    tag = "Iceberg",
)]
pub async fn load_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace, table)): Path<(String, String, String)>,
    Query(params): Query<LoadTableParams>,
    headers: HeaderMap,
) -> IcebergResult<Response> {
    let full_name = three_part_full_name(&catalog, &namespace, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::get_table,
    )
    .await?;
    let served = served_table(&state, &catalog, &namespace, &table).await?;
    let refs_only = refs_only(params.snapshots.as_deref())?;
    let etag = metadata_etag(
        served.metadata_location.as_deref().unwrap_or_default(),
        refs_only,
    );
    let etag_header = HeaderValue::from_str(&etag)
        .map_err(|e| uc_common::Error::internal(format!("invalid etag: {e}")))?;
    // A caller holding this tag already has the answer: the metadata file
    // is not read at all.
    let mut response = if holds_etag(&headers, &etag) {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        let (metadata_location, table_location) = locations(&served)?;
        let storage = ServerStorage::new(state.metastore.clone(), state.vendor.clone());
        let table_metadata = metadata::read(&storage, &metadata_location, &table_location).await?;
        let names = (catalog.as_str(), namespace.as_str(), table.as_str());
        let config = client_config(&state, &principal, &served, names, &table_location).await?;
        let mut resp = load_response(&table_metadata, Some(&metadata_location), config)?;
        if refs_only {
            suppress_historical_snapshots(&mut resp.0.metadata);
        }
        resp.into_response()
    };
    response.headers_mut().insert(header::ETAG, etag_header);
    header_case::spell_etag(&mut response).await;
    Ok(response)
}

#[utoipa::path(
    get, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}/credentials",
    operation_id = "iceberg_load_credentials",
    params(("catalog" = String, Path), ("namespace" = String, Path), ("table" = String, Path)),
    responses(
        (status = 200, body = IcebergLoadCredentialsResponse),
        (status = 404, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn load_credentials(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace, table)): Path<(String, String, String)>,
) -> IcebergResult<Json<IcebergLoadCredentialsResponse>> {
    let full_name = three_part_full_name(&catalog, &namespace, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::get_table,
    )
    .await?;
    let served = served_table(&state, &catalog, &namespace, &table).await?;
    let (_, table_location) = locations(&served)?;
    // The same scope as load table, so a renewal never widens access.
    let names = (catalog.as_str(), namespace.as_str(), table.as_str());
    let config = client_config(&state, &principal, &served, names, &table_location).await?;
    // A local table vends nothing: an empty list, since Iceberg's own
    // credential type rejects an empty config.
    let storage_credentials = match config.is_empty() {
        true => vec![],
        false => vec![IcebergStorageCredential {
            prefix: table_location.to_string(),
            config,
        }],
    };
    Ok(Json(IcebergLoadCredentialsResponse {
        storage_credentials,
    }))
}

/// Upstream reads a metrics report with Iceberg's `ReportMetricsRequestParser`:
/// a scan or commit report must carry the fields that parser requires.
fn check_metrics_report(report: &serde_json::Value) -> IcebergResult<()> {
    let report = report.as_object().ok_or_else(|| {
        IcebergError::bad_request(format!(
            "Cannot parse metrics request from non-object: {report}"
        ))
    })?;
    let report_type = report
        .get("report-type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| IcebergError::bad_request("Cannot parse missing string: report-type"))?;
    let required: &[(&str, &str)] = match report_type {
        "scan-report" => &[
            ("string", "table-name"),
            ("long", "snapshot-id"),
            ("field", "filter"),
            ("int", "schema-id"),
            ("list", "projected-field-ids"),
            ("list", "projected-field-names"),
            ("field", "metrics"),
        ],
        "commit-report" => &[
            ("string", "table-name"),
            ("long", "snapshot-id"),
            ("long", "sequence-number"),
            ("string", "operation"),
            ("field", "metrics"),
        ],
        // Reports of kinds this server does not know are accepted.
        _ => &[],
    };
    match required
        .iter()
        .find(|(_, field)| !report.contains_key(*field))
    {
        Some((kind, field)) => Err(IcebergError::bad_request(format!(
            "Cannot parse missing {kind}: {field}"
        ))),
        None => Ok(()),
    }
}

#[utoipa::path(
    post, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}/metrics",
    operation_id = "iceberg_report_metrics",
    params(("catalog" = String, Path), ("namespace" = String, Path), ("table" = String, Path)),
    request_body(content = serde_json::Value, description = "An Iceberg scan or commit report"),
    responses(
        (status = 204, description = "Acknowledged and discarded"),
        (status = 400, body = IcebergErrorResponse),
        (status = 404, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn report_metrics(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace, table)): Path<(String, String, String)>,
    IcebergJson(report): IcebergJson<serde_json::Value>,
) -> IcebergResult<StatusCode> {
    check_metrics_report(&report)?;
    let full_name = three_part_full_name(&catalog, &namespace, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::get_table,
    )
    .await?;
    // Reports are not kept, but one for a table the catalog does not serve
    // is a 404.
    served_table(&state, &catalog, &namespace, &table).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/views/{view}",
    operation_id = "iceberg_load_view",
    params(("catalog" = String, Path), ("namespace" = String, Path), ("view" = String, Path)),
    responses((status = 404, body = IcebergErrorResponse, description = "Views are not served")),
    tag = "Iceberg",
)]
pub async fn load_view(
    Path((_catalog, namespace, view)): Path<(String, String, String)>,
) -> IcebergResult<StatusCode> {
    // Iceberg clients look a name up as a view once it is not found as a
    // table, so the answer is the spec's 404.
    Err(IcebergError::no_such_view(format!(
        "View does not exist: {namespace}.{view}"
    )))
}

/// The load-table answer for `table_metadata`, written at
/// `metadata_location` (none for a staged create).
fn load_response(
    table_metadata: &TableMetadata,
    metadata_location: Option<&NormalizedUrl>,
    config: BTreeMap<String, String>,
) -> IcebergResult<Json<IcebergLoadTableResponse>> {
    let metadata = serde_json::to_value(table_metadata)
        .map_err(|e| Error::internal(format!("cannot serialize table metadata: {e}")))?;
    Ok(Json(IcebergLoadTableResponse {
        metadata_location: metadata_location.map(metadata::client_location),
        metadata,
        config,
    }))
}

/// Upstream `finalizeIcebergTableCreation`: write the first metadata file,
/// then register the table (its columns, properties and metadata pointer)
/// in one step, removing the file if that fails. A commit that loses to an
/// existing table is a 409 commit failure.
async fn finalize_creation(
    state: &AppState,
    principal: &Principal,
    (catalog, namespace, name): (&str, &str, &str),
    table_type: TableType,
    table_metadata: TableMetadata,
    from_commit: bool,
) -> IcebergResult<Json<IcebergLoadTableResponse>> {
    let table_location = NormalizedUrl::parse(table_metadata.location())?;
    let metadata_location = metadata::new_metadata_location(&table_metadata, 0, &table_location)?;
    let properties: SecurablePropertiesMap =
        table_metadata.properties().clone().into_iter().collect();
    let create = CreateTable {
        name: name.to_string(),
        catalog_name: catalog.to_string(),
        schema_name: namespace.to_string(),
        table_type,
        data_source_format: Some(DataSourceFormat::Iceberg),
        columns: schema::to_column_infos(table_metadata.current_schema())?,
        storage_location: Some(table_metadata.location().to_string()),
        comment: None,
        properties: Some(properties),
        view_definition: None,
        view_dependencies: None,
    };
    let storage = ServerStorage::new(state.metastore.clone(), state.vendor.clone());
    metadata::write(
        &storage,
        &table_metadata,
        &metadata_location,
        &table_location,
    )
    .await?;
    let info = match state
        .metastore
        .iceberg_tables()
        .create_table(create, &metadata_location, principal.actor())
        .await
    {
        Ok(info) => info,
        Err(e) => {
            metadata::delete(&storage, &metadata_location, &table_location).await;
            if from_commit && e.code() == ErrorCode::TableAlreadyExists {
                return Err(IcebergError::commit_failed(format!(
                    "Requirement failed: table already exists: {namespace}.{name}"
                )));
            }
            return Err(e.into());
        }
    };
    crate::observability::metrics::table_created();
    // Credentials for the persisted location, not the client's metadata.
    let persisted = NormalizedUrl::parse(info.storage_location.as_deref().unwrap_or_default())?;
    let endpoint = credentials_endpoint(catalog, namespace, name);
    let config = table_config(state, &persisted, VendOp::ReadWrite, &endpoint).await?;
    load_response(&table_metadata, Some(&metadata_location), config)
}

#[utoipa::path(
    post, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables",
    operation_id = "iceberg_create_table",
    params(("catalog" = String, Path), ("namespace" = String, Path)),
    request_body = IcebergCreateTableRequest,
    responses(
        (status = 200, body = IcebergLoadTableResponse),
        (status = 400, body = IcebergErrorResponse),
        (status = 403, body = IcebergErrorResponse, description = "A local location outside the allowed roots"),
        (status = 409, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn create_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace)): Path<(String, String)>,
    IcebergJson(req): IcebergJson<CreateTableBody>,
) -> IcebergResult<Json<IcebergLoadTableResponse>> {
    // Upstream `CREATE_TABLE`: a table with a location is EXTERNAL.
    let location = req.location.as_deref().filter(|l| !l.is_empty());
    authz::check_create(
        &state,
        &principal,
        &schema_full_name(&catalog, &namespace),
        location.map(Some),
        rules::create_table,
    )
    .await?;
    check_writes_enabled(&state)?;
    let name = req
        .name
        .ok_or_else(|| IcebergError::bad_request("Invalid table name: null"))?;
    let table_schema = req
        .schema
        .ok_or_else(|| IcebergError::bad_request("Invalid schema: null"))?;
    // Without a location the table is MANAGED, at a staging location.
    let (table_type, location) = match location {
        None => {
            let staging = CreateStagingTable {
                name: name.clone(),
                catalog_name: catalog.clone(),
                schema_name: namespace.clone(),
            };
            let staging = state
                .metastore
                .tables()
                .create_staging(staging, principal.actor())
                .await?;
            let location = staging
                .staging_location
                .ok_or_else(|| Error::internal("staging table without a location"))?;
            (TableType::Managed, NormalizedUrl::parse(&location)?)
        }
        Some(raw) => {
            let location = NormalizedUrl::parse(raw)?;
            // Before any directory or file is made there.
            validate_local_location(&state, &location).await?;
            (TableType::External, location)
        }
    };
    let properties = req.properties.unwrap_or_default();
    let table_metadata = commit::new_table_metadata(
        table_schema,
        req.partition_spec,
        req.write_order,
        &location,
        properties,
    )?;
    let names = (catalog.as_str(), namespace.as_str(), name.as_str());
    if req.stage_create == Some(true) {
        // Nothing is registered and no metadata is written; a MANAGED create
        // keeps its staging table.
        if table_type == TableType::External {
            match state
                .metastore
                .tables()
                .get(&three_part_full_name(&catalog, &namespace, &name))
                .await
            {
                Ok(_) => {
                    return Err(IcebergError::already_exists(format!(
                        "Table already exists: {namespace}.{name}"
                    )));
                }
                Err(e) if e.code() == ErrorCode::TableNotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        metadata::prepare_table_location(&table_metadata, &location)?;
        let config = table_config(
            &state,
            &location,
            VendOp::ReadWrite,
            &credentials_endpoint(&catalog, &namespace, &name),
        )
        .await?;
        return load_response(&table_metadata, None, config);
    }
    finalize_creation(&state, &principal, names, table_type, table_metadata, false).await
}

/// Upstream `commitStagedCreate`: materialize a staged create. The table is
/// MANAGED at a location with UC's managed marker (which must be the
/// caller's staging table), EXTERNAL elsewhere.
async fn commit_staged_create(
    state: &AppState,
    principal: &Principal,
    names: (&str, &str, &str),
    body: CommitBody,
) -> IcebergResult<Json<IcebergLoadTableResponse>> {
    let full_name = three_part_full_name(names.0, names.1, names.2);
    let table_metadata =
        commit::staged_create_metadata(&full_name, &body.requirements, body.updates)?;
    let location = NormalizedUrl::parse(table_metadata.location())?;
    let table_type = match table_metadata.location().contains(MANAGED_STORAGE_PREFIX) {
        true => {
            // Before any metadata is written; the create checks it again.
            state
                .metastore
                .tables()
                .require_owned_staging(&location, principal.actor())
                .await?;
            TableType::Managed
        }
        false => {
            validate_local_location(state, &location).await?;
            TableType::External
        }
    };
    finalize_creation(state, principal, names, table_type, table_metadata, true).await
}

#[utoipa::path(
    post, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
    operation_id = "iceberg_commit_table",
    params(("catalog" = String, Path), ("namespace" = String, Path), ("table" = String, Path)),
    request_body = IcebergCommitTableRequest,
    responses(
        (status = 200, body = IcebergLoadTableResponse),
        (status = 400, body = IcebergErrorResponse),
        (status = 404, body = IcebergErrorResponse),
        (status = 409, body = IcebergErrorResponse, description = "A requirement no longer holds"),
    ),
    tag = "Iceberg",
)]
pub async fn commit_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace, table)): Path<(String, String, String)>,
    IcebergJson(body): IcebergJson<CommitBody>,
) -> IcebergResult<Json<IcebergLoadTableResponse>> {
    let full_name = three_part_full_name(&catalog, &namespace, &table);
    let staged_create = commit::is_staged_create(&body.requirements);
    // Upstream `UPDATE_ICEBERG_TABLE`: a staged create is a table creation,
    // MANAGED at a managed location; anything else changes the table.
    if staged_create {
        let location = body.updates.iter().find_map(|u| match u {
            iceberg::TableUpdate::SetLocation { location } => Some(location.as_str()),
            _ => None,
        });
        let external = location.filter(|l| !l.contains(MANAGED_STORAGE_PREFIX));
        authz::check_create(
            &state,
            &principal,
            &schema_full_name(&catalog, &namespace),
            external.map(Some),
            rules::create_table,
        )
        .await?;
    } else {
        authz::check(
            &state,
            &principal,
            SecurableType::Table,
            &full_name,
            rules::update_table,
        )
        .await?;
    }
    check_writes_enabled(&state)?;
    commit::require_at_most_one_set_location(&body.updates)?;
    let names = (catalog.as_str(), namespace.as_str(), table.as_str());
    if staged_create {
        return commit_staged_create(&state, &principal, names, body).await;
    }
    // UniForm metadata is derived from the Delta log: read-only here.
    let served = state
        .metastore
        .iceberg_tables()
        .table_state(&catalog, &namespace, &table)
        .await?;
    if served.data_source_format != Some(DataSourceFormat::Iceberg) {
        return Err(IcebergError::bad_request(format!(
            "Table is not a native Iceberg table and cannot be committed to via the Iceberg REST \
             catalog: {full_name}"
        )));
    }
    let Some(expected) = served.metadata_location.clone() else {
        return Err(IcebergError::no_such_table(format!(
            "Table does not exist: {namespace}.{table}"
        )));
    };
    let (metadata_location, table_location) = locations(&served)?;
    let storage = ServerStorage::new(state.metastore.clone(), state.vendor.clone());
    let base = metadata::read(&storage, &metadata_location, &table_location).await?;
    commit::check_requirements(&body.requirements, &base)?;
    let endpoint = credentials_endpoint(&catalog, &namespace, &table);
    let Some(updated) = commit::apply_updates(base.clone(), &metadata_location, body.updates)?
    else {
        // Nothing changes: no new file, no pointer move.
        let config = table_config(&state, &table_location, VendOp::ReadWrite, &endpoint).await?;
        return load_response(&base, Some(&metadata_location), config);
    };
    let version = metadata::parse_metadata_version(&metadata_location) + 1;
    let new_location = metadata::new_metadata_location(&updated, version, &table_location)?;
    let columns = schema::to_column_infos(updated.current_schema())?;
    let properties: SecurablePropertiesMap = updated.properties().clone().into_iter().collect();
    metadata::write(&storage, &updated, &new_location, &table_location).await?;
    // The pointer swap decides the race; the loser's file is removed.
    let swapped = state
        .metastore
        .iceberg_tables()
        .commit_table(
            &catalog,
            &namespace,
            &table,
            &expected,
            &new_location,
            columns,
            properties,
            principal.actor(),
        )
        .await;
    if let Err(e) = swapped {
        metadata::delete(&storage, &new_location, &table_location).await;
        return Err(e.into());
    }
    let config = table_config(&state, &table_location, VendOp::ReadWrite, &endpoint).await?;
    load_response(&updated, Some(&new_location), config)
}

#[derive(Debug, Default, Deserialize)]
pub struct DropTableParams {
    #[serde(rename = "purgeRequested")]
    pub purge_requested: Option<String>,
}

/// Upstream `validatePurgeRequested`: `true` or `false` in any case, or
/// `1` or `0` (pyiceberg sends Python's `True`).
fn validate_purge_requested(value: Option<&str>) -> IcebergResult<()> {
    let value = value.unwrap_or_default();
    let boolean = ["true", "false", "1", "0"].contains(&value.to_ascii_lowercase().as_str());
    if !value.is_empty() && !boolean {
        return Err(IcebergError::bad_request(format!(
            "Invalid purgeRequested: {value}. It must be true or false."
        )));
    }
    Ok(())
}

#[utoipa::path(
    delete, path = "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
    operation_id = "iceberg_drop_table",
    params(
        ("catalog" = String, Path),
        ("namespace" = String, Path),
        ("table" = String, Path),
        ("purgeRequested" = Option<String>, Query, description = "Accepted; MANAGED tables are always cleaned up"),
    ),
    responses(
        (status = 204, description = "Dropped"),
        (status = 400, body = IcebergErrorResponse),
        (status = 404, body = IcebergErrorResponse),
    ),
    tag = "Iceberg",
)]
pub async fn drop_table(
    State(state): State<AppState>,
    principal: Principal,
    Path((catalog, namespace, table)): Path<(String, String, String)>,
    Query(params): Query<DropTableParams>,
) -> IcebergResult<StatusCode> {
    let full_name = three_part_full_name(&catalog, &namespace, &table);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::own_child,
    )
    .await?;
    check_writes_enabled(&state)?;
    validate_purge_requested(params.purge_requested.as_deref())?;
    let served = state
        .metastore
        .iceberg_tables()
        .table_state(&catalog, &namespace, &table)
        .await?;
    if served.data_source_format != Some(DataSourceFormat::Iceberg) {
        return Err(IcebergError::bad_request(format!(
            "Table {full_name} was not created through the Iceberg REST catalog; drop it through \
             the Unity Catalog API instead."
        )));
    }
    // EXTERNAL files stay; a MANAGED table's directory is cleaned up.
    state.metastore.tables().delete(&full_name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/iceberg/v1/catalogs/{catalog}/tables/rename",
    operation_id = "iceberg_rename_table",
    params(("catalog" = String, Path)),
    request_body = IcebergRenameTableRequest,
    responses(
        (status = 204, description = "Renamed"),
        (status = 400, body = IcebergErrorResponse),
        (status = 404, body = IcebergErrorResponse),
        (status = 409, body = IcebergErrorResponse, description = "The destination exists"),
        (status = 501, body = IcebergErrorResponse, description = "A move to another namespace"),
    ),
    tag = "Iceberg",
)]
pub async fn rename_table(
    State(state): State<AppState>,
    principal: Principal,
    Path(catalog): Path<String>,
    IcebergJson(req): IcebergJson<IcebergRenameTableRequest>,
) -> IcebergResult<StatusCode> {
    check_writes_enabled(&state)?;
    let source = req
        .source
        .ok_or_else(|| IcebergError::bad_request("Invalid source table: null"))?;
    let destination = req
        .destination
        .ok_or_else(|| IcebergError::bad_request("Invalid destination table: null"))?;
    let (from, to) = (source.namespace.join("."), destination.namespace.join("."));
    if from != to {
        // UC cannot move a table between schemas.
        return Err(Error::new(
            ErrorCode::Unimplemented,
            format!("Renaming a table into another namespace is not supported: {from} to {to}"),
        )
        .into());
    }
    let full_name = three_part_full_name(&catalog, &from, &source.name);
    authz::check(
        &state,
        &principal,
        SecurableType::Table,
        &full_name,
        rules::rename_table,
    )
    .await?;
    let served = state
        .metastore
        .iceberg_tables()
        .table_state(&catalog, &from, &source.name)
        .await?;
    if served.data_source_format != Some(DataSourceFormat::Iceberg) {
        return Err(IcebergError::bad_request(format!(
            "Table {full_name} was not created through the Iceberg REST catalog; rename it \
             through the Unity Catalog API instead."
        )));
    }
    state
        .metastore
        .tables()
        .rename(&full_name, &destination.name, principal.actor())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
