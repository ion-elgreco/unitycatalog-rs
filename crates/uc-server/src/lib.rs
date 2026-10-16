//! The HTTP server as a library, so that integration tests and embedding
//! callers can build the app without `main`.

pub mod authn;
pub mod authz;
pub mod http;
pub mod iceberg;
pub mod observability;
pub mod routes;
pub mod server;
pub mod storage;

pub use server::app::{build_app, build_observability_app};
pub use server::config::ServerConfig;
pub use server::state::AppState;
