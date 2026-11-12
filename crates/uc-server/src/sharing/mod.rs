//! The recipient side of sharing: the Delta Sharing and Open Sharing
//! protocols, served by `opensharing-engine` from the shares of the
//! metastore.
//!
//! - [`DELTA_SHARING_PREFIX`]: the Delta Sharing protocol (tables).
//! - [`OPENSHARING_PREFIX`]: the Open Sharing protocol (tables, volumes and
//!   agent skills, with temporary credentials).
//!
//! A recipient signs in with its bearer token and reads the shares it was
//! granted. The protocol is read-only. An admin manages shares and
//! recipients under the UC API ([`admin`]).

pub mod access;
pub mod admin;
mod catalog;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{Router, extract::FromRequestParts, http::request::Parts};
use opensharing_api::auth::{BearerAuthenticator, authenticated, require_bearer};
pub use opensharing_api::router::{DELTA_SHARING_PREFIX, OPENSHARING_PREFIX};
use opensharing_engine::{SharingPolicy, SharingService, policy::SharingAction};
use tracing::warn;
use uc_metastore::DynMetastore;

use crate::server::state::AppState;

/// The recipient that sent the request.
#[derive(Debug, Clone)]
pub struct Recipient {
    pub name: String,
    /// The shares the recipient was granted.
    pub shares: Vec<String>,
}

impl FromRequestParts<SharingService<Recipient>> for Recipient {
    type Rejection = opensharing_api::Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _: &SharingService<Recipient>,
    ) -> Result<Self, Self::Rejection> {
        authenticated(parts)
    }
}

/// A server error: the detail goes to the log, not to the recipient.
fn internal(error: impl std::fmt::Display) -> opensharing_api::Error {
    opensharing_api::Error::generic(error)
}

/// A metastore error as a protocol error.
fn metastore_error(error: uc_common::Error) -> opensharing_api::Error {
    if error.code().is_not_found() {
        opensharing_api::Error::NotFound
    } else {
        internal(error)
    }
}

struct RecipientTokens(DynMetastore);

#[async_trait]
impl BearerAuthenticator<Recipient> for RecipientTokens {
    async fn authenticate(&self, token: &str) -> opensharing_api::Result<Recipient> {
        match self.0.sharing().recipient_for_token(token).await {
            Ok(Some(recipient)) => Ok(Recipient {
                name: recipient.name,
                shares: recipient.shares,
            }),
            Ok(None) => Err(opensharing_api::Error::Unauthenticated),
            Err(e) => {
                warn!(error = %e, "recipient token lookup failed");
                Err(internal(e))
            }
        }
    }
}

/// A recipient reads the shares it was granted. Another share is not found,
/// so the answer does not tell whether it exists.
struct GrantedShares;

#[async_trait]
impl SharingPolicy<Recipient> for GrantedShares {
    async fn authorize(
        &self,
        action: SharingAction<'_>,
        recipient: &Recipient,
    ) -> opensharing_api::Result<()> {
        match action {
            SharingAction::ListShares => Ok(()),
            SharingAction::ReadShare { share } => {
                if recipient.shares.iter().any(|s| s == share) {
                    Ok(())
                } else {
                    Err(opensharing_api::Error::NotFound)
                }
            }
        }
    }
}

/// The protocol routes, under their own prefixes.
pub fn router(state: &AppState) -> Router<AppState> {
    let service = SharingService::new(
        Arc::new(catalog::MetastoreShares::new(state.metastore.clone())),
        Arc::new(access::ServerAccess::new(
            state.metastore.clone(),
            state.vendor.clone(),
        )),
        Arc::new(GrantedShares),
    );
    let tokens: Arc<dyn BearerAuthenticator<Recipient>> =
        Arc::new(RecipientTokens(state.metastore.clone()));
    let delta = opensharing_api::get_router::<_, Recipient>(service.clone());
    let open = opensharing_api::opensharing_router::<_, Recipient>(service);
    Router::new()
        .nest_service(DELTA_SHARING_PREFIX, require_bearer(delta, tokens.clone()))
        .nest_service(OPENSHARING_PREFIX, require_bearer(open, tokens))
}
