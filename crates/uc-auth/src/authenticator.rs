use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::header::WWW_AUTHENTICATE;
use axum::http::{HeaderMap, HeaderValue, StatusCode, request::Parts};
use axum::response::{IntoResponse, Response};
use moka::future::Cache;
use sha2::{Digest, Sha256};

use crate::context::{bearer_token, cookie};
use crate::{Admission, AuthError, Identity, RequestContext, Resolution, Resolver};

/// How long an identity is kept by its token, at most, unless set with
/// [`AuthenticatorBuilder::keep`]. A user disabled by the host keeps
/// working this long with a token that was already taken.
pub const DEFAULT_KEEP: Duration = Duration::from_secs(5 * 60);

/// Runs resolvers in order, then the host's admission, and keeps what it
/// found for the tokens of resolvers that allow it.
///
/// Built with [`Authenticator::builder`]. Cloning is cheap: clones share
/// the resolvers and what is kept.
pub struct Authenticator<S> {
    inner: Arc<Inner<S>>,
}

impl<S> Clone for Authenticator<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

struct Inner<S> {
    resolvers: Vec<Box<dyn Resolver<S>>>,
    admission: Option<Box<dyn Admission<S>>>,
    cookies: Vec<String>,
    keep: Duration,
    /// The identities found for tokens, by the hash of the token, so no
    /// token is held.
    kept: Cache<[u8; 32], Identity>,
    /// The `WWW-Authenticate` value for a request without a token.
    challenge: HeaderValue,
    /// The `resource_metadata` parameter of a challenge, if configured.
    resource_metadata: Option<String>,
}

/// A configuration [`AuthenticatorBuilder::build`] refuses.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The resource metadata URL can't be put into a `WWW-Authenticate` header.
    #[error("resource metadata URL is not a valid header parameter: {0}")]
    ResourceMetadataUrl(String),
}

/// Builds an [`Authenticator`].
pub struct AuthenticatorBuilder<S> {
    resolvers: Vec<Box<dyn Resolver<S>>>,
    admission: Option<Box<dyn Admission<S>>>,
    cookies: Vec<String>,
    keep: Duration,
    resource_metadata_url: Option<String>,
}

impl<S: Send + Sync + 'static> Authenticator<S> {
    pub fn builder() -> AuthenticatorBuilder<S> {
        AuthenticatorBuilder {
            resolvers: Vec::new(),
            admission: None,
            cookies: Vec::new(),
            keep: DEFAULT_KEEP,
            resource_metadata_url: None,
        }
    }

    /// The token of a request: its `Authorization: Bearer` header, else the
    /// first of the configured cookies it has.
    pub fn token(&self, headers: &HeaderMap) -> Option<String> {
        bearer_token(headers)
            .or_else(|| (self.inner.cookies.iter()).find_map(|name| cookie(headers, name)))
    }

    /// Who sent the request of `parts`.
    ///
    /// # Errors
    ///
    /// [`AuthError::Missing`] when no resolver takes the request and it has
    /// no token; [`AuthError::Invalid`] when it has a token that no resolver
    /// takes; otherwise what the deciding resolver or the admission says.
    pub async fn authenticate(&self, parts: &Parts, state: &S) -> Result<Identity, AuthError> {
        let cx = RequestContext::new(parts, self.token(&parts.headers));
        self.resolve(&cx, state).await
    }

    /// As [`authenticate`](Self::authenticate), for a context the host made.
    pub async fn resolve(&self, cx: &RequestContext<'_>, state: &S) -> Result<Identity, AuthError> {
        let mut looked = false;
        for resolver in &self.inner.resolvers {
            if resolver.keeps() && !looked {
                looked = true;
                if let Some(identity) = self.kept(cx).await {
                    return Ok(identity);
                }
            }
            let identity = match resolver.resolve(cx, state).await {
                Resolution::Pass => continue,
                Resolution::Refused(error) => return Err(error),
                Resolution::Resolved(identity) => identity,
            };
            let identity = match &self.inner.admission {
                Some(admission) => admission.admit(identity, cx, state).await?,
                None => identity,
            };
            if resolver.keeps() {
                self.keep(cx, &identity).await;
            }
            return Ok(identity);
        }
        match cx.token() {
            Some(_) => Err(AuthError::invalid("no resolver takes the token")),
            None => Err(AuthError::Missing),
        }
    }

    async fn kept(&self, cx: &RequestContext<'_>) -> Option<Identity> {
        let identity = self.inner.kept.get(&token_key(cx.token()?)).await?;
        identity
            .keep_until
            .is_some_and(|until| Instant::now() < until)
            .then_some(identity)
    }

    async fn keep(&self, cx: &RequestContext<'_>, identity: &Identity) {
        let (Some(token), Some(until)) = (cx.token(), identity.keep_until) else {
            return;
        };
        let until = until.min(Instant::now() + self.inner.keep);
        if until <= Instant::now() {
            return;
        }
        let mut kept = identity.clone();
        kept.keep_until = Some(until);
        self.inner.kept.insert(token_key(token), kept).await;
    }

    /// The response for a request that failed with `error`.
    ///
    /// A 401 carries an RFC 6750 `WWW-Authenticate: Bearer` challenge, with
    /// `error="invalid_token"` when a token was presented, and the
    /// configured `resource_metadata` URL (RFC 9728). [`AuthError::Forbidden`]
    /// is 403 and [`AuthError::Unavailable`] 503. Neither the token nor the
    /// reason is included.
    pub fn reject(&self, error: &AuthError) -> Response {
        match error {
            AuthError::Missing => (
                StatusCode::UNAUTHORIZED,
                [(WWW_AUTHENTICATE, self.inner.challenge.clone())],
            )
                .into_response(),
            AuthError::Invalid(_) => {
                let mut challenge = r#"Bearer error="invalid_token""#.to_string();
                if let Some(metadata) = &self.inner.resource_metadata {
                    challenge.push_str(", ");
                    challenge.push_str(metadata);
                }
                let challenge =
                    HeaderValue::try_from(challenge).expect("built from validated parts");
                (StatusCode::UNAUTHORIZED, [(WWW_AUTHENTICATE, challenge)]).into_response()
            }
            AuthError::Forbidden(_) => StatusCode::FORBIDDEN.into_response(),
            AuthError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}

impl<S: Send + Sync + 'static> AuthenticatorBuilder<S> {
    /// Adds a resolver after those added before.
    pub fn resolver(mut self, resolver: impl Resolver<S>) -> Self {
        self.resolvers.push(Box::new(resolver));
        self
    }

    /// The host's step after a resolver found the caller.
    pub fn admission(mut self, admission: impl Admission<S>) -> Self {
        self.admission = Some(Box::new(admission));
        self
    }

    /// Also takes the token from cookie `name`, when the request has no
    /// `Authorization: Bearer` header.
    pub fn cookie(mut self, name: impl Into<String>) -> Self {
        self.cookies.push(name.into());
        self
    }

    /// How long an identity is kept by its token, at most ([`DEFAULT_KEEP`]).
    /// [`Duration::ZERO`] keeps none.
    pub fn keep(mut self, keep: Duration) -> Self {
        self.keep = keep;
        self
    }

    /// The URL of the resource's OAuth protected resource metadata (RFC
    /// 9728), sent in the `resource_metadata` parameter of every challenge.
    pub fn resource_metadata_url(mut self, url: impl Into<String>) -> Self {
        self.resource_metadata_url = Some(url.into());
        self
    }

    /// # Errors
    ///
    /// When the resource metadata URL contains characters a quoted header
    /// parameter can't hold.
    pub fn build(self) -> Result<Authenticator<S>, ConfigError> {
        let resource_metadata = match self.resource_metadata_url {
            Some(url) if url.contains(['"', '\\']) || HeaderValue::try_from(&url).is_err() => {
                return Err(ConfigError::ResourceMetadataUrl(url));
            }
            Some(url) => Some(format!(r#"resource_metadata="{url}""#)),
            None => None,
        };
        let challenge = match &resource_metadata {
            Some(metadata) => format!("Bearer {metadata}"),
            None => "Bearer".to_owned(),
        };
        Ok(Authenticator {
            inner: Arc::new(Inner {
                resolvers: self.resolvers,
                admission: self.admission,
                cookies: self.cookies,
                keep: self.keep,
                kept: Cache::builder()
                    .max_capacity(10_000)
                    .time_to_live(self.keep.max(Duration::from_millis(1)))
                    .build(),
                challenge: HeaderValue::try_from(challenge).expect("validated above"),
                resource_metadata,
            }),
        })
    }
}

/// The key of a token among the kept identities: its hash.
fn token_key(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use axum::http::{Request, header};

    use super::*;

    /// Takes tokens that start with its prefix, keeps them if `keeps`.
    struct Prefix {
        prefix: &'static str,
        keeps: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Resolver<()> for Prefix {
        async fn resolve(&self, cx: &RequestContext<'_>, _: &()) -> Resolution {
            let Some(rest) = cx.token().and_then(|t| t.strip_prefix(self.prefix)) else {
                return Resolution::Pass;
            };
            self.calls.fetch_add(1, Ordering::SeqCst);
            match rest {
                "bad" => Resolution::Refused(AuthError::invalid("bad")),
                _ => {
                    let mut identity = Identity::new(self.prefix, "test", rest);
                    identity.keep_until = Some(Instant::now() + Duration::from_secs(60));
                    Resolution::Resolved(identity)
                }
            }
        }

        fn keeps(&self) -> bool {
            self.keeps
        }
    }

    /// Refuses `banned`, names everyone else `user:<subject>`.
    struct Users;

    #[async_trait]
    impl Admission<()> for Users {
        async fn admit(
            &self,
            mut identity: Identity,
            _: &RequestContext<'_>,
            _: &(),
        ) -> Result<Identity, AuthError> {
            if identity.subject == "banned" {
                return Err(AuthError::Forbidden("User not allowed".into()));
            }
            identity.subject = format!("user:{}", identity.subject);
            Ok(identity)
        }
    }

    fn parts(token: Option<&str>, cookie: Option<&str>) -> Parts {
        let mut request = Request::builder();
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, format!("session={cookie}"));
        }
        request.body(()).unwrap().into_parts().0
    }

    #[tokio::test]
    async fn the_first_resolver_that_does_not_pass_decides() {
        let calls = Arc::new(AtomicUsize::new(0));
        let auth = Authenticator::builder()
            .resolver(Prefix {
                prefix: "a.",
                keeps: false,
                calls: calls.clone(),
            })
            .resolver(Prefix {
                prefix: "b.",
                keeps: true,
                calls: calls.clone(),
            })
            .admission(Users)
            .cookie("session")
            .build()
            .unwrap();
        let subject = |result: Result<Identity, AuthError>| result.map(|i| i.subject);

        assert_eq!(
            subject(auth.authenticate(&parts(Some("a.x"), None), &()).await).unwrap(),
            "user:x"
        );
        assert_eq!(
            subject(auth.authenticate(&parts(None, Some("b.y")), &()).await).unwrap(),
            "user:y"
        );
        assert!(matches!(
            auth.authenticate(&parts(Some("a.bad"), None), &()).await,
            Err(AuthError::Invalid(_))
        ));
        assert!(matches!(
            auth.authenticate(&parts(Some("b.banned"), None), &()).await,
            Err(AuthError::Forbidden(_))
        ));
        assert!(matches!(
            auth.authenticate(&parts(Some("c.z"), None), &()).await,
            Err(AuthError::Invalid(_))
        ));
        assert!(matches!(
            auth.authenticate(&parts(None, None), &()).await,
            Err(AuthError::Missing)
        ));

        // What `b.` found is kept by the token, after the admission; what
        // `a.` found is not.
        calls.store(0, Ordering::SeqCst);
        for _ in 0..3 {
            auth.authenticate(&parts(Some("a.x"), None), &())
                .await
                .unwrap();
            let kept = auth.authenticate(&parts(Some("b.y"), None), &()).await;
            assert_eq!(subject(kept).unwrap(), "user:y");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn rejections_carry_a_challenge() {
        let auth = Authenticator::<()>::builder()
            .resource_metadata_url("https://api.example.com/.well-known/oauth-protected-resource")
            .build()
            .unwrap();
        let missing = auth.reject(&AuthError::Missing);
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            missing.headers()[WWW_AUTHENTICATE],
            r#"Bearer resource_metadata="https://api.example.com/.well-known/oauth-protected-resource""#
        );
        let invalid = auth.reject(&AuthError::invalid("issuer x"));
        let challenge = invalid.headers()[WWW_AUTHENTICATE].to_str().unwrap();
        assert!(challenge.starts_with(r#"Bearer error="invalid_token", resource_metadata="#));
        assert!(!challenge.contains("issuer x"));
        assert_eq!(
            auth.reject(&AuthError::Forbidden("no".into())).status(),
            StatusCode::FORBIDDEN
        );
        assert!(
            Authenticator::<()>::builder()
                .resource_metadata_url("https://x/\"")
                .build()
                .is_err()
        );
    }
}
