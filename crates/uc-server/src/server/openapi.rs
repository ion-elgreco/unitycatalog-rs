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
        (name = "Sharing",              description = "Delta Sharing — shares, recipients, recipient-protocol reads"),
        (name = "Files",                description = "Files of volumes and tables, as the Databricks Files API"),
    ),
)]
pub struct ApiDoc;
