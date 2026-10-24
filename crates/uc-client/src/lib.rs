//! HTTP client for unitycatalog-rs (and any wire-compatible UC server), generated from the
//! protos in `proto/` (`just generate`).
//!
//! [`Client`] has a method for each route of the API, and a handle for each resource:
//! `client.catalog("main").get().await`. A method returns a builder: set the optional
//! arguments with its `with_*` methods, then `.await` it. A list also has `.into_stream()`,
//! which reads all pages. The request and response types are in [`models`]; the errors in
//! [`error`].

// The generated code names this crate `uc_client`, as the Python client does.
extern crate self as uc_client;

mod codegen;
mod contents;
pub mod models;

pub use codegen::*;
pub use contents::Contents;
