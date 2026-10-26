//! Shared types, error model, and IDs used across the unitycatalog-rs workspace.

pub mod error;
pub mod install;
pub mod location;
pub mod logo;
pub mod name;
pub mod page;
pub mod pem;
pub mod securable;
pub mod settings;
pub mod time;
#[cfg(feature = "ui")]
pub mod ui;
pub mod validation;
pub mod wire;

pub use error::{ERROR_INFO_TYPE, Error, ErrorCode, ErrorDetail, ErrorResponse, Result};
pub use location::{MANAGED_STORAGE_PREFIX, ManagedResource, NormalizedUrl, UriScheme};
pub use name::{schema_full_name, three_part_full_name};
pub use page::{
    DEFAULT_PAGE_SIZE, DeleteParams, PageToken, Pagination, SchemaScopedListParams, lenient_bool,
    lenient_opt_i64,
};
pub use securable::{SecurablePropertiesMap, SecurableType, serialize_properties};
pub use time::{EpochMillis, now_epoch_millis};
pub use validation::{MAX_NAME_LENGTH, require_non_empty, validate_sql_object_name};
pub use wire::{from_wire, wire_name};

/// The path of the API, as upstream's server has it: the Unity Catalog,
/// Delta, Iceberg REST and Delta Sharing APIs are below it.
pub const API_BASE: &str = "/api/2.1/unity-catalog";

/// The path of the control plane, as upstream's server has it: the token
/// exchange and SCIM are below it.
pub const CONTROL_BASE: &str = "/api/1.0/unity-control";
