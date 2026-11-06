//! Who sent a request: a chain of resolvers behind an axum layer.
//!
//! The crate turns a request into an [`Identity`] and does nothing else.
//! Minting tokens, sign-in pages and the users of the host are not here.
//!
//! - A [`Resolver`] looks at the whole request ([`RequestContext`]): its
//!   headers, the bearer token, the address of the connection. It says
//!   "not mine" ([`Resolution::Pass`]), names the caller
//!   ([`Resolution::Resolved`]) or refuses the request
//!   ([`Resolution::Refused`]).
//! - An [`Authenticator`] runs its resolvers in order. The first that does
//!   not pass decides. The host's [`Admission`] then maps the identity to
//!   one of its users, or refuses it.
//! - [`auth_layer`] runs an authenticator in front of a router and puts the
//!   identity into the request; handlers take it as an extractor.
//!
//! The resolvers of the crate:
//!
//! - [`TrustedHeaders`]: an auth proxy names the user in its headers. Only
//!   connections from the proxy's addresses count.
//! - [`JwtResolver`]: the JWTs of OpenID Connect providers, with keys from
//!   each issuer's discovery document ([`JwtKeys`]).
//! - [`AwsResolver`]: a signed `sts:GetCallerIdentity` request as the bearer
//!   value, which the resolver sends to STS.
//!
//! A host adds its own, such as for its own tokens or API keys. Each
//! resolver gets the host's state `S`, as axum handlers do.
//!
//! # Example
//!
//! ```no_run
//! use axum::{Router, routing::get};
//! use uc_auth::{Authenticator, Identity, TrustedHeaders, auth_layer};
//!
//! async fn whoami(identity: Identity) -> String {
//!     identity.subject
//! }
//!
//! # fn build() -> Result<Router, Box<dyn std::error::Error>> {
//! let proxy = TrustedHeaders::new("proxy", ["10.0.0.0/8".parse()?], ["Remote-User"]);
//! let authenticator = Authenticator::<()>::builder().resolver(proxy).build()?;
//! let app = auth_layer(Router::new().route("/whoami", get(whoami)), authenticator, ());
//! # Ok(app)
//! # }
//! ```

mod allowlist;
mod authenticator;
pub mod aws;
mod context;
mod error;
mod headers;
mod http;
mod identity;
pub mod jwt;
mod layer;
mod resolver;

pub use allowlist::Allowlist;
/// Re-exported for implementing [`Resolver`], [`Admission`] and [`Transport`].
pub use async_trait::async_trait;
pub use authenticator::{Authenticator, AuthenticatorBuilder, ConfigError, DEFAULT_KEEP};
pub use aws::{AwsProvider, AwsResolver};
pub use context::{RequestContext, bearer_token, cookie};
pub use error::AuthError;
pub use headers::TrustedHeaders;
pub use http::{Direct, Transport};
pub use identity::{Identity, MissingIdentity};
pub use ipnet::IpNet;
pub use jwt::{JwtKeys, JwtProvider, JwtResolver};
pub use layer::auth_layer;
pub use resolver::{Admission, Resolution, Resolver};
