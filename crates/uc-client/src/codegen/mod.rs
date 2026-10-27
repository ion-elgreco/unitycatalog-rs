// @generated — do not edit by hand.
pub mod admin;
pub mod audit;
pub mod auth;
pub mod catalogs;
pub mod credentials;
pub mod delta_api;
pub mod delta_commits;
pub mod external_locations;
pub mod files;
pub mod functions;
pub mod health;
pub mod iceberg;
pub mod metastore;
pub mod model_versions;
pub mod permissions;
pub mod recipients;
pub mod registered_models;
pub mod schemas;
pub mod scim;
pub mod search;
pub mod shares;
pub mod sharing;
pub mod staging_tables;
pub mod tables;
pub mod temporary_credentials;
pub mod tokens;
pub mod volumes;
pub use catalogs::CatalogClient;
pub use credentials::CredentialClient;
pub use external_locations::ExternalLocationClient;
pub use functions::FunctionClient;
pub use recipients::RecipientClient;
pub use registered_models::RegisteredModelClient;
pub use schemas::SchemaClient;
pub use shares::ShareClient;
pub use tables::TableClient;
pub use tokens::TokenClient;
pub use volumes::VolumeClient;

#[allow(clippy::too_many_arguments, clippy::doc_lazy_continuation)]
pub mod client;
#[allow(unused_imports)]
pub use client::*;

pub mod error;
pub mod transport;
use futures::Future;

pub(super) fn stream_paginated<F, Fut, S, T>(
    state: S,
    op: F,
) -> impl futures::Stream<Item = crate::codegen::error::Result<T>>
where
    F: Fn(S, Option<String>) -> Fut + Copy,
    Fut: Future<Output = crate::codegen::error::Result<(T, S, Option<String>)>>,
{
    enum PaginationState<T> {
        Start(T),
        HasMore(T, String),
        Done,
    }
    futures::stream::unfold(PaginationState::Start(state), move |state| async move {
        let (s, page_token) = match state {
            PaginationState::Start(s) => (s, None),
            PaginationState::HasMore(s, page_token) if !page_token.is_empty() => {
                (s, Some(page_token))
            }
            _ => {
                return None;
            }
        };
        let (resp, s, continuation) = match op(s, page_token).await {
            Ok(resp) => resp,
            Err(e) => return Some((Err(e), PaginationState::Done)),
        };
        let next_state = match continuation {
            Some(token) => PaginationState::HasMore(s, token),
            None => PaginationState::Done,
        };
        Some((Ok(resp), next_state))
    })
}

/// Percent-encode a value for a URL path. `keep_slash` keeps `/`, for a value that
/// spans several path segments; such a value follows a `/` of the template, so its own
/// leading `/` is dropped.
fn encode_path_value(value: &impl ::std::fmt::Display, keep_slash: bool) -> String {
    use ::std::fmt::Write as _;
    let value = value.to_string();
    let value = match keep_slash {
        true => value.trim_start_matches('/'),
        false => value.as_str(),
    };
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            b'/' if keep_slash => out.push('/'),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}
