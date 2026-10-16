//! Prometheus metrics, after upstream's `/metrics` (Micrometer), on the
//! observability port:
//!
//! - `http_server_requests_total{hostname_pattern, service, method,
//!   http_status}`: completed API requests, labelled with upstream's
//!   service class and handler (`io.unitycatalog.server.service.CatalogService`,
//!   `createCatalog`). Unrouted requests are upstream's `FallbackService`
//!   with the HTTP verb; routes upstream lacks are `unitycatalog-rs`.
//! - `uc_securable_table_creations_total`: tables created through UC, Delta
//!   or Iceberg REST.
//! - `process_cpu_usage`, `system_cpu_usage`, `system_cpu_count`,
//!   `system_load_average_1m`, `process_start_time_seconds`,
//!   `process_uptime_seconds`, `process_resident_memory_bytes`.
//!
//! The recorder is installed when the app is built, so no sample is lost
//! before the first scrape.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use axum::{
    Router,
    body::Body,
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use uc_common::{API_BASE, CONTROL_BASE};

use crate::server::state::AppState;

static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the global Prometheus recorder, once.
pub fn init_metrics() -> &'static PrometheusHandle {
    HANDLE.get_or_init(|| {
        PrometheusBuilder::new()
            .install_recorder()
            .expect("failed to install Prometheus recorder")
    })
}

/// `/metrics`.
pub fn metrics_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route(
        "/metrics",
        get(|| async {
            let handle = init_metrics();
            record_process();
            (StatusCode::OK, handle.render())
        }),
    )
}

/// One more table securable created.
pub fn table_created() {
    metrics::counter!("uc_securable_table_creations_total").increment(1);
}

/// Count each API request as upstream does.
pub async fn metrics_layer(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let verb = req.method().as_str().to_string();
    let matched = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string());
    let response = next.run(req).await;
    let (service, method) = labels(&verb, matched.as_deref());
    metrics::counter!(
        "http_server_requests_total",
        "hostname_pattern" => format!("*:{}", state.api_port),
        "service" => service,
        "method" => method,
        "http_status" => response.status().as_u16().to_string(),
    )
    .increment(1);
    response
}

/// Upstream's service and handler for a request.
fn labels(verb: &str, matched: Option<&str>) -> (String, String) {
    let Some(path) = matched else {
        return (
            "com.linecorp.armeria.server.FallbackService".to_string(),
            verb.to_string(),
        );
    };
    let relative = |base: &str| {
        path.strip_prefix(base)
            .filter(|rest| rest.is_empty() || rest.starts_with('/'))
    };
    let key = match (relative(API_BASE), relative(CONTROL_BASE)) {
        (Some(rest), _) => Some(("api", rest)),
        (None, Some(rest)) => Some(("control", rest)),
        _ => None,
    };
    let found =
        key.and_then(|(base, rest)| routes().get(&(base, verb.to_string(), template(rest))));
    match found {
        Some((service, method)) => (service.to_string(), method.to_string()),
        None => ("unitycatalog-rs".to_string(), verb.to_string()),
    }
}

/// A path with each parameter as `{}`: `/catalogs/{}`.
fn template(path: &str) -> String {
    let segments: Vec<&str> = path
        .split('/')
        .map(
            |s| match s.starts_with('{') || s.starts_with(':') || s.starts_with('*') {
                true => "{}",
                false => s,
            },
        )
        .collect();
    match segments.join("/") {
        p if p.is_empty() => "/".to_string(),
        p => p,
    }
}

type RouteKey = (&'static str, String, String);

fn routes() -> &'static HashMap<RouteKey, (&'static str, &'static str)> {
    static TABLE: OnceLock<HashMap<RouteKey, (&'static str, &'static str)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ROUTES
            .iter()
            .map(|(base, verb, path, service, method)| {
                (
                    (*base, verb.to_string(), template(path)),
                    (*service, *method),
                )
            })
            .collect()
    })
}

/// Process and system gauges, refreshed at each scrape; CPU usage is over
/// the time since the last one.
fn record_process() {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    static SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();
    let Ok(mut system) = SYSTEM.get_or_init(|| Mutex::new(System::new())).lock() else {
        return;
    };
    let Ok(pid) = sysinfo::get_current_pid() else {
        return;
    };
    system.refresh_cpu_usage();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );
    let cpus = system.cpus().len().max(1) as f64;
    metrics::gauge!("system_cpu_count").set(cpus);
    metrics::gauge!("system_cpu_usage").set(f64::from(system.global_cpu_usage()) / 100.0);
    metrics::gauge!("system_load_average_1m").set(System::load_average().one);
    if let Some(process) = system.process(pid) {
        metrics::gauge!("process_cpu_usage").set(f64::from(process.cpu_usage()) / 100.0 / cpus);
        metrics::gauge!("process_resident_memory_bytes").set(process.memory() as f64);
        metrics::gauge!("process_start_time_seconds").set(process.start_time() as f64);
        metrics::gauge!("process_uptime_seconds").set(process.run_time() as f64);
    }
}

/// Upstream's routes: base (`api` or `control`), verb, path below the base,
/// service class and handler, from its annotated services.
const ROUTES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "control",
        "POST",
        "/auth/tokens",
        "io.unitycatalog.server.service.AuthService",
        "grantToken",
    ),
    (
        "control",
        "POST",
        "/auth/logout",
        "io.unitycatalog.server.service.AuthService",
        "logout",
    ),
    (
        "control",
        "GET",
        "/scim2/Users",
        "io.unitycatalog.server.service.Scim2UserService",
        "getScimUsers",
    ),
    (
        "control",
        "POST",
        "/scim2/Users",
        "io.unitycatalog.server.service.Scim2UserService",
        "createScimUser",
    ),
    (
        "control",
        "GET",
        "/scim2/Users/{id}",
        "io.unitycatalog.server.service.Scim2UserService",
        "getUser",
    ),
    (
        "control",
        "PUT",
        "/scim2/Users/{id}",
        "io.unitycatalog.server.service.Scim2UserService",
        "updateUser",
    ),
    (
        "control",
        "DELETE",
        "/scim2/Users/{id}",
        "io.unitycatalog.server.service.Scim2UserService",
        "deleteUser",
    ),
    (
        "control",
        "PATCH",
        "/scim2/Users/{id}",
        "io.unitycatalog.server.service.Scim2UserService",
        "patchUser",
    ),
    (
        "control",
        "GET",
        "/scim2/Me",
        "io.unitycatalog.server.service.Scim2SelfService",
        "getCurrentUser",
    ),
    (
        "api",
        "GET",
        "/permissions/metastore/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getMetastoreAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/catalog/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getCatalogAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/schema/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getSchemaAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/table/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getTableAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/function/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getFunctionAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/volume/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getVolumeAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/registered_model/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getRegisteredModelAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/external_location/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getExternalLocationAuthorization",
    ),
    (
        "api",
        "GET",
        "/permissions/credential/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "getCredentialAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/metastore/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateMetastoreAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/catalog/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateCatalogAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/schema/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateSchemaAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/table/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateTableAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/function/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateFunctionAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/volume/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateVolumeAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/registered_model/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateRegisteredModelAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/external_location/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateExternalLocationAuthorization",
    ),
    (
        "api",
        "PATCH",
        "/permissions/credential/{name}",
        "io.unitycatalog.server.service.PermissionService",
        "updateCredentialAuthorization",
    ),
    (
        "api",
        "POST",
        "/catalogs",
        "io.unitycatalog.server.service.CatalogService",
        "createCatalog",
    ),
    (
        "api",
        "GET",
        "/catalogs",
        "io.unitycatalog.server.service.CatalogService",
        "listCatalogs",
    ),
    (
        "api",
        "GET",
        "/catalogs/{name}",
        "io.unitycatalog.server.service.CatalogService",
        "getCatalog",
    ),
    (
        "api",
        "PATCH",
        "/catalogs/{name}",
        "io.unitycatalog.server.service.CatalogService",
        "updateCatalog",
    ),
    (
        "api",
        "DELETE",
        "/catalogs/{name}",
        "io.unitycatalog.server.service.CatalogService",
        "deleteCatalog",
    ),
    (
        "api",
        "POST",
        "/schemas",
        "io.unitycatalog.server.service.SchemaService",
        "createSchema",
    ),
    (
        "api",
        "GET",
        "/schemas",
        "io.unitycatalog.server.service.SchemaService",
        "listSchemas",
    ),
    (
        "api",
        "GET",
        "/schemas/{full_name}",
        "io.unitycatalog.server.service.SchemaService",
        "getSchema",
    ),
    (
        "api",
        "PATCH",
        "/schemas/{full_name}",
        "io.unitycatalog.server.service.SchemaService",
        "updateSchema",
    ),
    (
        "api",
        "DELETE",
        "/schemas/{full_name}",
        "io.unitycatalog.server.service.SchemaService",
        "deleteSchema",
    ),
    (
        "api",
        "POST",
        "/volumes",
        "io.unitycatalog.server.service.VolumeService",
        "createVolume",
    ),
    (
        "api",
        "GET",
        "/volumes",
        "io.unitycatalog.server.service.VolumeService",
        "listVolumes",
    ),
    (
        "api",
        "GET",
        "/volumes/{full_name}",
        "io.unitycatalog.server.service.VolumeService",
        "getVolume",
    ),
    (
        "api",
        "PATCH",
        "/volumes/{full_name}",
        "io.unitycatalog.server.service.VolumeService",
        "updateVolume",
    ),
    (
        "api",
        "DELETE",
        "/volumes/{full_name}",
        "io.unitycatalog.server.service.VolumeService",
        "deleteVolume",
    ),
    (
        "api",
        "POST",
        "/tables",
        "io.unitycatalog.server.service.TableService",
        "createTable",
    ),
    (
        "api",
        "GET",
        "/tables/{full_name}",
        "io.unitycatalog.server.service.TableService",
        "getTable",
    ),
    (
        "api",
        "GET",
        "/tables",
        "io.unitycatalog.server.service.TableService",
        "listTables",
    ),
    (
        "api",
        "DELETE",
        "/tables/{full_name}",
        "io.unitycatalog.server.service.TableService",
        "deleteTable",
    ),
    (
        "api",
        "POST",
        "/staging-tables",
        "io.unitycatalog.server.service.StagingTableService",
        "createStagingTable",
    ),
    (
        "api",
        "POST",
        "/functions",
        "io.unitycatalog.server.service.FunctionService",
        "createFunction",
    ),
    (
        "api",
        "GET",
        "/functions",
        "io.unitycatalog.server.service.FunctionService",
        "listFunctions",
    ),
    (
        "api",
        "GET",
        "/functions/{name}",
        "io.unitycatalog.server.service.FunctionService",
        "getFunction",
    ),
    (
        "api",
        "DELETE",
        "/functions/{name}",
        "io.unitycatalog.server.service.FunctionService",
        "deleteFunction",
    ),
    (
        "api",
        "POST",
        "/models",
        "io.unitycatalog.server.service.ModelService",
        "createRegisteredModel",
    ),
    (
        "api",
        "GET",
        "/models",
        "io.unitycatalog.server.service.ModelService",
        "listRegisteredModels",
    ),
    (
        "api",
        "GET",
        "/models/{full_name}",
        "io.unitycatalog.server.service.ModelService",
        "getRegisteredModel",
    ),
    (
        "api",
        "PATCH",
        "/models/{full_name}",
        "io.unitycatalog.server.service.ModelService",
        "updateRegisteredModel",
    ),
    (
        "api",
        "DELETE",
        "/models/{full_name}",
        "io.unitycatalog.server.service.ModelService",
        "deleteRegisteredModel",
    ),
    (
        "api",
        "POST",
        "/models/versions",
        "io.unitycatalog.server.service.ModelService",
        "createModelVersion",
    ),
    (
        "api",
        "GET",
        "/models/{full_name}/versions",
        "io.unitycatalog.server.service.ModelService",
        "listModelVersions",
    ),
    (
        "api",
        "GET",
        "/models/{full_name}/versions/{version}",
        "io.unitycatalog.server.service.ModelService",
        "getModelVersion",
    ),
    (
        "api",
        "PATCH",
        "/models/{full_name}/versions/{version}",
        "io.unitycatalog.server.service.ModelService",
        "updateModelVersion",
    ),
    (
        "api",
        "DELETE",
        "/models/{full_name}/versions/{version}",
        "io.unitycatalog.server.service.ModelService",
        "deleteModelVersion",
    ),
    (
        "api",
        "PATCH",
        "/models/{full_name}/versions/{version}/finalize",
        "io.unitycatalog.server.service.ModelService",
        "finalizeModelVersion",
    ),
    (
        "api",
        "GET",
        "/metastore_summary",
        "io.unitycatalog.server.service.MetastoreService",
        "getMetastoreSummary",
    ),
    (
        "api",
        "POST",
        "/temporary-table-credentials",
        "io.unitycatalog.server.service.TemporaryTableCredentialsService",
        "generateTemporaryTableCredential",
    ),
    (
        "api",
        "POST",
        "/temporary-volume-credentials",
        "io.unitycatalog.server.service.TemporaryVolumeCredentialsService",
        "generateTemporaryVolumeCredential",
    ),
    (
        "api",
        "POST",
        "/temporary-model-version-credentials",
        "io.unitycatalog.server.service.TemporaryModelVersionCredentialsService",
        "generateTemporaryModelVersionCredentials",
    ),
    (
        "api",
        "POST",
        "/temporary-path-credentials",
        "io.unitycatalog.server.service.TemporaryPathCredentialsService",
        "generateTemporaryPathCredential",
    ),
    (
        "api",
        "POST",
        "/credentials",
        "io.unitycatalog.server.service.CredentialService",
        "createCredential",
    ),
    (
        "api",
        "GET",
        "/credentials",
        "io.unitycatalog.server.service.CredentialService",
        "listCredentials",
    ),
    (
        "api",
        "GET",
        "/credentials/{name}",
        "io.unitycatalog.server.service.CredentialService",
        "getCredential",
    ),
    (
        "api",
        "PATCH",
        "/credentials/{name}",
        "io.unitycatalog.server.service.CredentialService",
        "updateCredential",
    ),
    (
        "api",
        "DELETE",
        "/credentials/{name}",
        "io.unitycatalog.server.service.CredentialService",
        "deleteCredential",
    ),
    (
        "api",
        "POST",
        "/delta/preview/commits",
        "io.unitycatalog.server.service.DeltaCommitsService",
        "postCommit",
    ),
    (
        "api",
        "GET",
        "/delta/preview/commits",
        "io.unitycatalog.server.service.DeltaCommitsService",
        "getCommits",
    ),
    (
        "api",
        "POST",
        "/external-locations",
        "io.unitycatalog.server.service.ExternalLocationService",
        "createExternalLocation",
    ),
    (
        "api",
        "GET",
        "/external-locations",
        "io.unitycatalog.server.service.ExternalLocationService",
        "listExternalLocations",
    ),
    (
        "api",
        "GET",
        "/external-locations/{name}",
        "io.unitycatalog.server.service.ExternalLocationService",
        "getExternalLocation",
    ),
    (
        "api",
        "PATCH",
        "/external-locations/{name}",
        "io.unitycatalog.server.service.ExternalLocationService",
        "updateExternalLocation",
    ),
    (
        "api",
        "DELETE",
        "/external-locations/{name}",
        "io.unitycatalog.server.service.ExternalLocationService",
        "deleteExternalLocation",
    ),
    (
        "api",
        "GET",
        "/iceberg/v1/config",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "config",
    ),
    (
        "api",
        "GET",
        "/iceberg/v1/catalogs/{catalog}/namespaces",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "listNamespaces",
    ),
    (
        "api",
        "HEAD",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "namespaceExists",
    ),
    (
        "api",
        "GET",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "getNamespace",
    ),
    (
        "api",
        "POST",
        "/iceberg/v1/catalogs/{catalog}/namespaces",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "createNamespace",
    ),
    (
        "api",
        "DELETE",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "dropNamespace",
    ),
    (
        "api",
        "POST",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/properties",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "updateNamespaceProperties",
    ),
    (
        "api",
        "HEAD",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "tableExists",
    ),
    (
        "api",
        "GET",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "loadTable",
    ),
    (
        "api",
        "GET",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}/credentials",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "loadCredentials",
    ),
    (
        "api",
        "POST",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "createTable",
    ),
    (
        "api",
        "POST",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "updateTable",
    ),
    (
        "api",
        "DELETE",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "dropTable",
    ),
    (
        "api",
        "POST",
        "/iceberg/v1/catalogs/{catalog}/tables/rename",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "renameTable",
    ),
    (
        "api",
        "GET",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/views/{view}",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "loadView",
    ),
    (
        "api",
        "POST",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables/{table}/metrics",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "reportMetrics",
    ),
    (
        "api",
        "GET",
        "/iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables",
        "io.unitycatalog.server.service.IcebergRestCatalogService",
        "listTables",
    ),
    (
        "api",
        "GET",
        "/delta/v1/config",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "getConfig",
    ),
    (
        "api",
        "GET",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "loadTable",
    ),
    (
        "api",
        "HEAD",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "tableExists",
    ),
    (
        "api",
        "DELETE",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "deleteTable",
    ),
    (
        "api",
        "POST",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/staging-tables",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "createStagingTable",
    ),
    (
        "api",
        "POST",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "createTable",
    ),
    (
        "api",
        "POST",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "updateTable",
    ),
    (
        "api",
        "POST",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/rename",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "renameTable",
    ),
    (
        "api",
        "POST",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/metrics",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "reportMetrics",
    ),
    (
        "api",
        "GET",
        "/delta/v1/catalogs/{catalog}/schemas/{schema}/tables/{table}/credentials",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "getTableCredentials",
    ),
    (
        "api",
        "GET",
        "/delta/v1/staging-tables/{table_id}/credentials",
        "io.unitycatalog.server.service.delta.DeltaApiService",
        "getStagingTableCredentials",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_ignore_parameter_names() {
        assert_eq!(template("/catalogs/{name}"), "/catalogs/{}");
        assert_eq!(
            template("/models/:full_name/versions/:version"),
            "/models/{}/versions/{}"
        );
        assert_eq!(template(""), "/");
    }
}
