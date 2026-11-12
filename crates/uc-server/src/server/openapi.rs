use utoipa::OpenApi;

use uc_api::DeltaCommitResponse;
use uc_common::{ErrorCode, ErrorResponse, SecurableType};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Unity Catalog",
        version = "0.1.0",
        description = "Rust reimplementation of the Unity Catalog OSS REST API.",
    ),
    // The routes add the schemas of their bodies and responses; these are
    // the ones no route names.
    components(schemas(ErrorResponse, ErrorCode, SecurableType, DeltaCommitResponse)),
    tags(
        (name = "Catalogs",            description = "Catalog CRUD"),
        (name = "Schemas",             description = "Schema CRUD (per catalog)"),
        (name = "Tables",              description = "Table CRUD (per schema)"),
        (name = "Staging Tables",      description = "Server-allocated staging URIs"),
        (name = "Delta Commits",       description = "Delta-log commit coordination"),
        (name = "Volumes",             description = "Managed and external volumes"),
        (name = "Functions",           description = "SQL / external functions (UDFs)"),
        (name = "RegisteredModels",    description = "ML registered models"),
        (name = "ModelVersions",       description = "Versions of registered models"),
        (name = "Credentials",         description = "Storage credentials"),
        (name = "ExternalLocations",   description = "Paths backed by credentials"),
        (name = "TemporaryCredentials", description = "Short-lived credential vending"),
        (name = "Permissions",          description = "Grant / revoke privileges on securables"),
        (name = "Metastore",            description = "Metastore-level information"),
        (name = "Auth",                 description = "Personal Access Tokens"),
        (name = "Iceberg",              description = "Apache Iceberg REST Catalog endpoints"),
        (name = "SCIM",                 description = "SCIM 2.0 user provisioning"),
        (name = "Health",               description = "Liveness, readiness, and generic health probes"),
        (name = "Admin",                description = "Operator endpoints — backup, restore"),
        (name = "Sharing",              description = "Shares, their permissions, and recipients with their activation links"),
        (name = "Files",                description = "Files of volumes and tables, as the Databricks Files API"),
    ),
)]
pub struct ApiDoc;

/// The operations of sharing: shares and recipients, whose routes are of
/// `opensharing-admin-api`, and the Delta Sharing and Open Sharing
/// protocols, of `opensharing-api`. Those routes have no utoipa paths, so
/// their document comes from the protos and from the OpenAPI document of
/// the protocols: `just docs-reference` writes it.
pub fn sharing() -> serde_json::Value {
    serde_json::from_str(include_str!("sharing_openapi.json"))
        .expect("the OpenAPI document of sharing")
}

/// `api` as JSON, with the operations, components and tags of
/// [`sharing`]. They are added as JSON: the document of sharing has parts
/// that the types of utoipa do not read. No component of it has the name
/// of a component of `api`.
pub fn with_sharing(api: &utoipa::openapi::OpenApi) -> serde_json::Value {
    let mut document = serde_json::to_value(api).expect("the OpenAPI document is JSON");
    let sharing = sharing();
    let object = |value: &serde_json::Value| value.as_object().cloned().unwrap_or_default();
    let paths = document["paths"].as_object_mut().expect("paths");
    paths.extend(object(&sharing["paths"]));
    for (kind, items) in object(&sharing["components"]) {
        let components = &mut document["components"][kind.as_str()];
        if components.is_null() {
            *components = serde_json::Value::Object(Default::default());
        }
        let components = components.as_object_mut().expect("components");
        for (name, item) in object(&items) {
            let taken = components.insert(name.clone(), item);
            debug_assert!(taken.is_none(), "two {kind} have the name {name}");
        }
    }
    let tags = document["tags"].as_array_mut().expect("tags");
    tags.extend(sharing["tags"].as_array().cloned().unwrap_or_default());
    document
}
