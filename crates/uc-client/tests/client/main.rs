//! The generated client against a uc-server in this process: each route of
//! the server, with its path, query and body parameters, and its answer.
//! One module per area of the API.

mod common;

mod admin;
mod auth;
mod catalogs;
mod coverage;
mod credentials;
mod delta;
mod errors;
mod functions;
mod iceberg;
mod models;
mod permissions;
mod protection;
mod schemas;
mod scim;
mod sharing;
mod tables;
mod volumes;
