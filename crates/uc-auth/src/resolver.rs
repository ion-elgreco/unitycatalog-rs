use async_trait::async_trait;

use crate::{AuthError, Identity, RequestContext};

/// What a [`Resolver`] says of a request.
pub enum Resolution {
    /// Not for this resolver, such as a token of another kind: the next
    /// resolver looks.
    Pass,
    /// The caller.
    Resolved(Identity),
    /// The request is for this resolver, and it does not check out. No
    /// other resolver looks.
    Refused(AuthError),
}

/// One way to find who sent a request.
///
/// `S` is the host's state, as axum handlers get it; resolvers that need
/// none implement `Resolver<S>` for every `S`.
#[async_trait]
pub trait Resolver<S>: Send + Sync + 'static {
    async fn resolve(&self, cx: &RequestContext<'_>, state: &S) -> Resolution;

    /// Whether the identities it finds may be kept by the request's token
    /// until their [`keep_until`](Identity::keep_until). The
    /// [`Authenticator`](crate::Authenticator) looks in what it keeps just
    /// before the first resolver that says so, so resolvers before it,
    /// such as [`TrustedHeaders`](crate::TrustedHeaders), still come first.
    fn keeps(&self) -> bool {
        false
    }
}

/// The host's step after a resolver found the caller: map the identity to
/// one of the host's users, or refuse it.
///
/// For example: look the subject up, make the user at its first request,
/// or refuse a user that is disabled. What the host keeps of an identity
/// ([`Resolver::keeps`]) is what this step returned.
#[async_trait]
pub trait Admission<S>: Send + Sync + 'static {
    async fn admit(
        &self,
        identity: Identity,
        cx: &RequestContext<'_>,
        state: &S,
    ) -> Result<Identity, AuthError>;
}
