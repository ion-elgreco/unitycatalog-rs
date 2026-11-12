//! The server's auth chain: the `uc_auth` resolvers of the settings, the
//! server's own, and the step that names the caller a user.
//!
//! The resolvers, in order:
//!
//! 1. The auth proxy's headers (`UC_AUTH_MODE=forward`), from its
//!    addresses only ([`uc_auth::TrustedHeaders`]).
//! 2. Delta Sharing recipient tokens, on the sharing endpoints
//!    ([`SharingRecipients`]).
//! 3. The signed requests of AWS identities, for the blocks of
//!    `[[auth.issuers]]` with `kind = "aws"` ([`uc_auth::AwsResolver`]).
//! 4. The tokens of the blocks with `direct = true`
//!    ([`uc_auth::JwtResolver`]).
//! 5. The server's own tokens; any other JWT is refused ([`ServerTokens`]).
//! 6. Personal access tokens ([`PersonalAccessTokens`]).
//!
//! [`Users`] then names the caller by its user: an active user, never the
//! bootstrap admin through a proxy or a provider. The callers of AWS and
//! `direct` tokens are kept by the token, at most [`idp::ACCEPTED_TTL`].

use std::time::Duration;

use uc_auth::jwt::{decode, looks_like_jwt};
use uc_auth::{
    Admission, AuthError, Authenticator, AwsResolver, Identity, JwtKeys, JwtResolver,
    RequestContext, Resolution, Resolver, Transport, async_trait,
};
use uc_common::{API_BASE, Error, ErrorCode};

use crate::{
    authn::{
        aws, idp,
        idp::IssuerSettings,
        middleware::{
            UC_TOKEN_COOKIE, require_active_user, require_active_user_at, requires_sign_in,
        },
        security::{ADMIN, INTERNAL_ISSUER},
        sign_in::ForwardSignIn,
    },
    http::{error::mounted, outbound::SendOnIo},
    server::state::AppState,
};

/// The method of the server's own tokens.
pub const SERVER_TOKEN: &str = "server-token";
/// The method of personal access tokens.
pub const PERSONAL_ACCESS_TOKEN: &str = "personal-access-token";
/// The method of Delta Sharing recipient tokens.
pub const SHARING_RECIPIENT: &str = "sharing-recipient";
/// The claim of a server token with the id of its user
/// ([`TokenClaims::uid`](crate::authn::security::TokenClaims::uid)).
const USER_ID: &str = "uid";

/// The calls to identity providers, on the IO runtime
/// (`crate::http::outbound`).
pub struct IoTransport;

#[async_trait]
impl Transport for IoTransport {
    async fn send(&self, request: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
        request.send_on_io().await
    }
}

/// The chain of the blocks `issuers`, with the auth proxy `forward`,
/// keeping callers at most `keep`.
pub fn authenticator(
    issuers: &[IssuerSettings],
    keys: &std::sync::Arc<JwtKeys>,
    forward: Option<&ForwardSignIn>,
    keep: Duration,
) -> Authenticator<AppState> {
    let mut builder = Authenticator::builder().cookie(UC_TOKEN_COOKIE).keep(keep);
    if let Some(forward) = forward {
        builder = builder.resolver(forward.resolver().clone());
    }
    builder = builder.resolver(SharingRecipients);
    let aws: Vec<_> = (issuers.iter())
        .filter(|i| i.is_aws())
        .map(IssuerSettings::aws_provider)
        .collect();
    if !aws.is_empty() {
        builder = builder.resolver(
            AwsResolver::new(aws::PREFIX, aws::SERVER_ID_HEADER, aws).with_transport(IoTransport),
        );
    }
    let direct = (issuers.iter())
        .filter(|i| i.direct && !i.is_aws())
        .map(|i| i.jwt_provider().clone())
        .collect();
    builder
        .resolver(JwtResolver::new(direct, keys.clone()))
        .resolver(ServerTokens)
        .resolver(PersonalAccessTokens)
        .admission(Users)
        .build()
        .expect("no resource metadata URL")
}

/// Delta Sharing recipients are not users; the sharing handlers check them
/// against their recipient.
pub struct SharingRecipients;

#[async_trait]
impl Resolver<AppState> for SharingRecipients {
    async fn resolve(&self, cx: &RequestContext<'_>, state: &AppState) -> Resolution {
        let Some(token) = cx.token().filter(|t| t.starts_with("dsr_")) else {
            return Resolution::Pass;
        };
        if !mounted(cx.path(), API_BASE, "/delta-sharing") {
            return Resolution::Pass;
        }
        match state.metastore.sharing().recipient_for_token(token).await {
            Ok(Some(recipient)) => Resolution::Resolved(Identity::new(
                "sharing",
                SHARING_RECIPIENT,
                format!("recipient:{}", recipient.name),
            )),
            Ok(None) => Resolution::Refused(AuthError::invalid("no recipient has the token")),
            Err(e) => Resolution::Refused(refused(e)),
        }
    }
}

/// The server's own tokens (RS512 JWTs of the `internal` issuer, see
/// `crate::authn::security`). Any other JWT, which no `direct` block took,
/// is refused with upstream's 403: a token of another issuer must be
/// exchanged first.
pub struct ServerTokens;

#[async_trait]
impl Resolver<AppState> for ServerTokens {
    async fn resolve(&self, cx: &RequestContext<'_>, state: &AppState) -> Resolution {
        let Some(token) = cx.token().filter(|t| looks_like_jwt(t)) else {
            return Resolution::Pass;
        };
        let Ok(decoded) = decode(token) else {
            return Resolution::Refused(AuthError::invalid("not a JWT"));
        };
        match idp::claim(&decoded.claims, "iss") {
            Some(INTERNAL_ISSUER) => {
                let claims = (state.security.as_ref())
                    .ok_or("the server has no signing key".to_string())
                    .and_then(|security| (security.verify(token)).map_err(|e| e.to_string()));
                match claims {
                    Ok(claims) => {
                        let mut identity = Identity::new(INTERNAL_ISSUER, SERVER_TOKEN, claims.sub);
                        if let Some(uid) = claims.uid {
                            identity.claims.insert(USER_ID.into(), uid.into());
                        }
                        Resolution::Resolved(identity)
                    }
                    Err(reason) => Resolution::Refused(AuthError::Invalid(reason)),
                }
            }
            Some(iss) => {
                match state.auth.issuer(iss) {
                    Some(issuer) => tracing::debug!(
                        issuer = %issuer.name,
                        "token refused: the issuer's tokens are taken at the exchange only"
                    ),
                    // The `iss` is the token's own text: the log has it
                    // with control characters escaped.
                    None => {
                        tracing::debug!(?iss, "token refused: no auth.issuers block has the issuer")
                    }
                }
                Resolution::Refused(AuthError::Forbidden("Invalid access token.".into()))
            }
            None => Resolution::Refused(AuthError::invalid("no iss")),
        }
    }
}

/// Personal access tokens: any token that no other resolver took.
pub struct PersonalAccessTokens;

#[async_trait]
impl Resolver<AppState> for PersonalAccessTokens {
    async fn resolve(&self, cx: &RequestContext<'_>, state: &AppState) -> Resolution {
        let Some(token) = cx.token() else {
            return Resolution::Pass;
        };
        match state.metastore.tokens().principal_for_token(token).await {
            Ok(Some(principal)) => {
                Resolution::Resolved(Identity::new("pat", PERSONAL_ACCESS_TOKEN, principal))
            }
            Ok(None) => Resolution::Refused(AuthError::invalid("no personal access token")),
            Err(e) => Resolution::Refused(refused(e)),
        }
    }
}

/// Names the caller by its user, as upstream: "User not allowed" unless the
/// user exists and is active.
///
/// - Recipients are not users.
/// - The auth proxy's user must be an active user, and never the bootstrap
///   admin, whom only the server's own tokens name.
/// - The server's tokens and personal access tokens name an active user
///   where a sign-in is required: a server token the user whose id it
///   carries, by its principal of that moment. With authorization off,
///   they name the caller as they are.
/// - A provider's caller is named as its block says
///   ([`idp::resolve`]), made at its first request with `create_users`.
pub struct Users;

#[async_trait]
impl Admission<AppState> for Users {
    async fn admit(
        &self,
        mut identity: Identity,
        cx: &RequestContext<'_>,
        state: &AppState,
    ) -> Result<Identity, AuthError> {
        match identity.method {
            SHARING_RECIPIENT => {}
            Identity::TRUSTED_HEADERS => {
                // The user is checked, not the header: a name finds its
                // user in any case.
                let named = require_active_user(state, &identity.subject)
                    .await
                    .map_err(refused)?;
                if named == ADMIN {
                    return Err(AuthError::Forbidden(format!("User not allowed: {ADMIN}")));
                }
                identity.subject = named;
            }
            SERVER_TOKEN | PERSONAL_ACCESS_TOKEN => {
                if state.authorization_enabled && requires_sign_in(cx.path()) {
                    let user_id = identity.claims.get(USER_ID).and_then(|id| id.as_str());
                    let named = match user_id {
                        Some(id) => require_active_user_at(state, id, &identity.subject).await,
                        None => require_active_user(state, &identity.subject).await,
                    };
                    identity.subject = named.map_err(refused)?;
                }
            }
            _ => {
                let issuer = (state.auth.issuers.iter())
                    .find(|issuer| issuer.name == identity.provider)
                    .ok_or_else(|| AuthError::invalid("no auth.issuers block of the provider"))?;
                identity.subject = idp::name_caller(state, issuer, &identity.claims)
                    .await
                    .map_err(refused)?;
            }
        }
        Ok(identity)
    }
}

/// The server's error as the chain's.
fn refused(e: Error) -> AuthError {
    match e.code() {
        ErrorCode::PermissionDenied => AuthError::Forbidden(e.message().to_string()),
        ErrorCode::Unauthenticated => AuthError::Invalid(e.message().to_string()),
        _ => AuthError::Unavailable(e.message().to_string()),
    }
}

/// The chain's error as the server's: the reason of an invalid token is for
/// the log only.
pub fn error(e: &AuthError) -> Error {
    match e {
        AuthError::Missing => Error::new(ErrorCode::Unauthenticated, "No authorization found."),
        AuthError::Forbidden(message) => Error::new(ErrorCode::PermissionDenied, message.clone()),
        AuthError::Unavailable(message) => Error::internal(message.clone()),
        _ => Error::new(ErrorCode::Unauthenticated, "Invalid access token."),
    }
}
