//! The Unity Catalog REST API (`/api/2.1/unity-catalog`, upstream
//! `all.yaml`), and the local token endpoints under the same base.

pub mod catalogs;
pub mod commits;
pub mod credentials;
pub mod external_locations;
pub mod functions;
pub mod metastore;
pub mod models;
pub mod permissions;
pub mod schemas;
pub mod search;
pub mod tables;
pub mod temp_credentials;
pub mod tokens;
pub mod volumes;
