//! HTTP client for unitycatalog-rs (and any wire-compatible UC server), generated from the
//! protos in `proto/` (`just generate`).
//!
//! [`Client`] has a method for each route of the API, and a handle for each resource:
//! `client.catalog("main").get().await`. A method returns a builder: set the optional
//! arguments with its `with_*` methods, then `.await` it. A list also has `.into_stream()`,
//! which reads all pages. The request and response types are in [`models`]; the errors in
//! [`error`].
//!
//! The transport is olai-http's `CloudClient`, and `WasmClient` in the browser. A client signs
//! in with a fixed token ([`Client::new_with_token`]), with a bearer provider of olai-http
//! ([`Client::new_with_bearer`]), which keeps and refreshes its tokens, or with any olai-http
//! signer ([`Client::new_with_signer`]). The `identity` feature has the providers of cloud
//! identities, and [`identity::detect`], which finds one in the environment.

// The generated code names this crate `uc_client`, as the Python client does.
extern crate self as uc_client;

mod codegen;
mod contents;
#[cfg(feature = "identity")]
pub mod identity;
pub mod models;

pub use codegen::*;
pub use contents::Contents;
