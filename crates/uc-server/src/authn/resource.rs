//! `GET /.well-known/oauth-protected-resource`: what the server accepts
//! as a bearer token, for clients that find their identity in their
//! environment (RFC 9728, OAuth 2.0 Protected Resource Metadata).
//!
//! The standard members: `resource`, the server's `auth.public_url`;
//! `authorization_servers`, the exact issuers of `[[auth.issuers]]` with
//! `direct = true`; and `bearer_methods_supported`. `issuers` adds, for
//! every block, its `name`, `kind` (`oidc` or `aws`), `issuer` pattern (for
//! AWS, the STS endpoint, empty for AWS's own), `audiences` and whether it
//! is `direct`: a client picks the block of its
//! provider and asks the provider for a token with that audience; an AWS
//! client signs a request for STS with that server id. The endpoint needs no sign-in and says
//! nothing secret: the same facts are in each token the server takes.

use axum::{Json, Router, extract::State, http::header, response::IntoResponse, routing::get};
use serde_json::json;

use crate::server::state::AppState;

pub const PATH: &str = "/.well-known/oauth-protected-resource";

async fn metadata(State(state): State<AppState>) -> impl IntoResponse {
    let auth = &state.auth;
    let issuers: Vec<_> = auth
        .issuers
        .iter()
        .map(|issuer| {
            json!({
                "name": issuer.name,
                "kind": if issuer.is_aws() { "aws" } else { "oidc" },
                "issuer": issuer.issuer,
                "audiences": issuer.audiences,
                "direct": issuer.direct,
            })
        })
        .collect();
    let authorization_servers: Vec<&str> = auth
        .issuers
        .iter()
        .filter(|issuer| issuer.direct && !issuer.is_aws() && !issuer.issuer.contains('*'))
        .map(|issuer| issuer.issuer.as_str())
        .collect();
    let mut body = json!({
        "authorization_servers": authorization_servers,
        "bearer_methods_supported": ["header"],
        "resource_name": "Unity Catalog",
        "issuers": issuers,
    });
    if let Some(public_url) = &auth.public_url {
        body["resource"] = json!(public_url);
    }
    ([(header::CACHE_CONTROL, "max-age=300")], Json(body))
}

pub fn router() -> Router<AppState> {
    Router::new().route(PATH, get(metadata))
}
