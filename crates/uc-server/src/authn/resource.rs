//! `GET /.well-known/oauth-protected-resource`: what the server accepts
//! as a bearer token, for clients that find their identity in their
//! environment (RFC 9728, OAuth 2.0 Protected Resource Metadata).
//!
//! The standard members: `resource`, the server's `auth.public_url`;
//! `authorization_servers`, the exact issuers of `[[auth.issuers]]` with
//! `direct = true`; and `bearer_methods_supported`. `issuers` adds, for
//! every block, its `name`, `issuer` pattern, `audiences` and whether it is
//! `direct`: a client picks the block of its provider and asks the provider
//! for a token with that audience. The endpoint needs no sign-in and says
//! nothing secret: the same facts are in each token the server takes.

use axum::{Json, Router, extract::State, http::header, response::IntoResponse, routing::get};
use serde_json::json;

use crate::{authn::sign_in::SignIn, server::state::AppState};

pub const PATH: &str = "/.well-known/oauth-protected-resource";

async fn metadata(State(state): State<AppState>) -> impl IntoResponse {
    let auth = &state.auth;
    let issuers: Vec<_> = auth
        .issuers
        .iter()
        .map(|issuer| {
            json!({
                "name": issuer.name,
                "issuer": issuer.issuer,
                "audiences": issuer.audiences,
                "direct": issuer.direct,
            })
        })
        .collect();
    let authorization_servers: Vec<&str> = auth
        .issuers
        .iter()
        .filter(|issuer| issuer.direct && !issuer.issuer.contains('*'))
        .map(|issuer| issuer.issuer.as_str())
        .collect();
    let mut body = json!({
        "authorization_servers": authorization_servers,
        "bearer_methods_supported": ["header"],
        "resource_name": "Unity Catalog",
        "issuers": issuers,
    });
    if let Some(SignIn::Oidc(oidc)) = auth.sign_in.as_ref() {
        body["resource"] = json!(oidc.public_url);
    }
    ([(header::CACHE_CONTROL, "max-age=300")], Json(body))
}

pub fn router() -> Router<AppState> {
    Router::new().route(PATH, get(metadata))
}
