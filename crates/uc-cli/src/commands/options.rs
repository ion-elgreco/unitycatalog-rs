//! The options that many operations share: their help, value parsers and
//! paging.

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{ArgGroup, Args};
use serde::de::DeserializeOwned;
use serde_json::Value;
use uc_common::SecurablePropertiesMap;
pub use uc_common::{from_wire, wire_name};

/// The help of options that many operations share, after upstream's
/// `CliParams` descriptions.
pub mod help {
    pub const NAME: &str = "The name of the entity";
    pub const FULL_NAME: &str = "The full name: catalog.schema.name";
    pub const SCHEMA_FULL_NAME: &str = "The full name of the schema: catalog.schema";
    pub const CATALOG: &str = "The name of the catalog";
    pub const SCHEMA: &str = "The name of the schema";
    pub const NEW_NAME: &str = "The new name of the entity";
    pub const COMMENT: &str = "Comment/description of the entity";
    pub const PROPERTIES: &str =
        "The properties of the entity, as a JSON object: '{\"key1\": \"value1\"}'";
    pub const STORAGE_ROOT: &str =
        "The storage root URL for managed tables and volumes in the catalog or schema";
    pub const STORAGE_LOCATION: &str =
        "The storage location: a URL or a local path. Required for EXTERNAL";
    pub const FORCE: &str = "Delete the entity even if it is not empty";
    pub const MAX_RESULTS: &str = "The maximum number of results to return";
    pub const PAGE_TOKEN: &str = "Opaque token to retrieve the next page of results";
    pub const VERSION: &str = "The version number of the model version";
    pub const MODEL_NAME: &str = "The name of the registered model";
    pub const SECURABLE_TYPE: &str = "The type of the securable, such as catalog or table";
    pub const PRINCIPAL: &str = "The principal of the permission";
    pub const PRIVILEGE: &str = "The privilege to grant or revoke, such as \"USE CATALOG\"";
    pub const USER_ID: &str = "The unique id of the user";
    pub const EXTERNAL_ID: &str = "The identity provider's id for the user";
    pub const EMAIL: &str = "The email address of the user";
    pub const URL: &str = "The URL of the external location, such as s3://bucket/path";
    pub const CREDENTIAL_NAME: &str = "The name of the storage credential";
    pub const AWS_IAM_ROLE_ARN: &str = "The ARN of the AWS IAM role of the credential";
    pub const AZURE_DIRECTORY_ID: &str =
        "The Microsoft Entra ID tenant (directory) of the credential's Azure service principal";
    pub const AZURE_APPLICATION_ID: &str =
        "The application (client) ID of the credential's Azure service principal";
    pub const AZURE_CLIENT_SECRET: &str =
        "The client secret of the credential's Azure service principal";
    pub const AZURE_MANAGED_IDENTITY: &str = "The server's own Azure identity; with a value, \
        the resource ID of a user-assigned managed identity";
    pub const GCP_SERVICE_ACCOUNT: &str = "The server's own GCP identity; with a value, the \
        email of a service account that the server impersonates";
    pub const GCP_SERVICE_ACCOUNT_KEY: &str = "The JSON key file of a GCP service account";
}

/// `--properties`: a JSON object, whose values that are not strings become
/// their JSON text.
pub type Properties = SecurablePropertiesMap;

pub fn properties(text: &str) -> Result<Properties, String> {
    let Ok(Value::Object(map)) = serde_json::from_str(text) else {
        return Err("expected a JSON object".into());
    };
    Ok(map
        .into_iter()
        .map(|(k, v)| match v {
            Value::String(s) => (k, s),
            other => (k, other.to_string()),
        })
        .collect())
}

/// A `catalog.schema.name` full name.
#[derive(Debug, Clone)]
pub struct ThreePartName {
    pub catalog: String,
    pub schema: String,
    pub name: String,
}

pub fn three_part_name(text: &str) -> Result<ThreePartName, String> {
    match text.split('.').collect::<Vec<_>>()[..] {
        [catalog, schema, name] => Ok(ThreePartName {
            catalog: catalog.into(),
            schema: schema.into(),
            name: name.into(),
        }),
        _ => Err("expected catalog.schema.name".into()),
    }
}

/// One of `values`, in any case (with `ignore_case`), as the API value of
/// that name.
pub fn one_of_wire<T>(values: &'static [&'static str]) -> impl TypedValueParser<Value = T>
where
    T: DeserializeOwned + Clone + Send + Sync + 'static,
{
    PossibleValuesParser::new(values).try_map(move |given| {
        let name = values
            .iter()
            .find(|name| name.eq_ignore_ascii_case(&given))
            .map_or(given.as_str(), |name| name);
        from_wire(name).ok_or_else(|| format!("invalid value: {given}"))
    })
}

/// One of `values`, in any case (with `ignore_case`), as `values` writes
/// it.
pub fn one_of(values: &'static [&'static str]) -> impl TypedValueParser<Value = String> {
    PossibleValuesParser::new(values).map(move |given| {
        values
            .iter()
            .find(|name| name.eq_ignore_ascii_case(&given))
            .map_or(given, |name| name.to_string())
    })
}

pub const TABLE_TYPES: &[&str] = &["MANAGED", "EXTERNAL"];

/// Upstream's securable types, as the permissions API names them.
pub const SECURABLE_TYPES: &[&str] = &[
    "metastore",
    "catalog",
    "schema",
    "table",
    "volume",
    "function",
    "registered_model",
    "external_location",
    "credential",
];

/// Upstream's privileges.
pub const PRIVILEGES: &[&str] = &[
    "CREATE CATALOG",
    "USE CATALOG",
    "CREATE SCHEMA",
    "USE SCHEMA",
    "CREATE TABLE",
    "SELECT",
    "MODIFY",
    "CREATE FUNCTION",
    "EXECUTE",
    "CREATE VOLUME",
    "READ VOLUME",
    "CREATE MODEL",
    "CREATE EXTERNAL LOCATION",
    "READ FILES",
    "WRITE FILES",
    "CREATE EXTERNAL TABLE",
    "CREATE EXTERNAL VOLUME",
    "CREATE MANAGED STORAGE",
    "CREATE STORAGE CREDENTIAL",
];

/// The changes of an update, which needs one or more.
pub fn changes() -> ArgGroup {
    ArgGroup::new("changes").required(true).multiple(true)
}

// `--max_results` and `--page_token` of a list.
#[derive(Debug, Args)]
#[command(rename_all = "snake_case")]
pub struct Paging {
    #[arg(long, help = help::MAX_RESULTS, default_value_t = 100)]
    pub max_results: i32,
    #[arg(long, help = help::PAGE_TOKEN)]
    pub page_token: Option<String>,
}

// `--filter`, `--start_index` and `--count` of a SCIM list.
#[derive(Debug, Args)]
#[command(rename_all = "snake_case")]
pub struct ScimPaging {
    /// SCIM filter of the results, such as 'displayName eq "admins"'
    #[arg(long)]
    pub filter: Option<String>,
    /// The index (from 1) of the first result
    #[arg(long, default_value_t = 1)]
    pub start_index: i32,
    /// The number of results per page
    #[arg(long, default_value_t = 100)]
    pub count: i32,
}

// `--max_results` and `--page_token` of an Iceberg list, as its page size
// and token.
#[derive(Debug, Args)]
#[command(rename_all = "snake_case")]
pub struct IcebergPaging {
    #[arg(long, help = help::MAX_RESULTS)]
    pub max_results: Option<i32>,
    #[arg(long, help = help::PAGE_TOKEN)]
    pub page_token: Option<String>,
}
