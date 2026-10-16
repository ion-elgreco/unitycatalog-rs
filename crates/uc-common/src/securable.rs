use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};
use utoipa::ToSchema;

use crate::Error;

/// The properties of a securable: upstream `SecurablePropertiesMap`, a
/// string-to-string map. It is a `BTreeMap`, so JSON serialization is
/// deterministic.
pub type SecurablePropertiesMap = BTreeMap<String, String>;

/// Serializes properties that are not set as `{}`, as upstream does for
/// catalogs, schemas and tables. deltalake's Unity Catalog client needs the
/// field of a table.
pub fn serialize_properties<S: Serializer>(
    properties: &Option<SecurablePropertiesMap>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match properties {
        Some(properties) => properties.serialize(serializer),
        None => SecurablePropertiesMap::new().serialize(serializer),
    }
}

/// The kinds of objects that can be granted permissions in Unity Catalog.
/// Matches the path parameter of `/permissions/{securable_type}/{full_name}`.
/// Variants are spec-aligned (lowercase snake_case strings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecurableType {
    Metastore,
    Catalog,
    Schema,
    Table,
    Volume,
    Function,
    RegisteredModel,
    ExternalLocation,
    /// Spec value is `credential` (not `storage_credential`).
    Credential,
}

impl SecurableType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Metastore => "metastore",
            Self::Catalog => "catalog",
            Self::Schema => "schema",
            Self::Table => "table",
            Self::Volume => "volume",
            Self::Function => "function",
            Self::RegisteredModel => "registered_model",
            Self::ExternalLocation => "external_location",
            Self::Credential => "credential",
        }
    }
}

impl std::fmt::Display for SecurableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SecurableType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "metastore" => Self::Metastore,
            "catalog" => Self::Catalog,
            "schema" => Self::Schema,
            "table" => Self::Table,
            "volume" => Self::Volume,
            "function" => Self::Function,
            "registered_model" => Self::RegisteredModel,
            "external_location" => Self::ExternalLocation,
            "credential" => Self::Credential,
            _ => {
                return Err(Error::invalid_argument(format!(
                    "unknown securable_type '{s}'"
                )));
            }
        })
    }
}
