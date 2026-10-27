//! Request/response models for the Unity Catalog REST API.
//!
//! These types are derived from `api/all.yaml` in the upstream UC OSS repo.
//! Field names and shapes match the spec verbatim.

pub mod admin;
pub mod audit;
pub mod auth;
pub mod catalog;
pub mod credential;
pub mod delta;
pub mod delta_api;
pub mod delta_types;
pub mod external_location;
pub mod files;
pub mod function;
pub mod iceberg_rest;
pub mod metastore;
pub mod model;
pub mod permissions;
pub mod schema;
pub mod scim;
pub mod search;
pub mod sharing;
pub mod staging;
pub mod table;
pub mod volume;

pub use admin::*;
pub use audit::*;
pub use auth::*;
pub use catalog::*;
pub use credential::*;
pub use delta::*;
pub use external_location::*;
pub use files::*;
pub use function::*;
pub use metastore::*;
pub use model::*;
pub use permissions::*;
pub use schema::*;
pub use scim::*;
pub use search::*;
pub use sharing::*;
pub use staging::*;
pub use table::*;
pub use volume::*;

#[cfg(test)]
mod test_util {
    use serde::{Serialize, de::DeserializeOwned};
    use serde_json::Value;

    /// Checks that each value is sent as its name and read back from it.
    pub(crate) fn assert_wire_names<T>(cases: &[(T, &str)])
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        for (value, name) in cases {
            assert_eq!(serde_json::to_value(value).unwrap(), *name);
            assert_eq!(
                &serde_json::from_value::<T>(Value::from(*name)).unwrap(),
                value
            );
        }
    }

    /// Checks that `json` reads as a `T` and is sent back unchanged.
    pub(crate) fn assert_round_trip<T: Serialize + DeserializeOwned>(json: Value) {
        let value: T = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(value).unwrap(), json);
    }
}
