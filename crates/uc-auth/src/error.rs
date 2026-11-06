/// Why a request has no identity.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum AuthError {
    /// The request carries no credential that a resolver takes.
    #[error("no credentials")]
    Missing,
    /// The credential does not check out. The reason is for the log, never
    /// for the caller: it can name issuers, accounts and keys.
    #[error("invalid credentials: {0}")]
    Invalid(String),
    /// The credential checks out, but the caller may not come in. The
    /// message is for the caller.
    #[error("{0}")]
    Forbidden(String),
    /// A service the check needs, such as the identity provider, did not
    /// answer. The message is for the caller.
    #[error("{0}")]
    Unavailable(String),
}

impl AuthError {
    /// An [`AuthError::Invalid`] with `reason` for the log.
    pub fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid(reason.into())
    }
}
