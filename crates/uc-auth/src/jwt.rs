//! The JWTs of OpenID Connect providers.
//!
//! A token's `iss` must match a [`JwtProvider`] before anything is fetched.
//! Its key comes from the issuer's discovery document and JWK set
//! ([`JwtKeys`]), which are cached per issuer and refetched when a key id is
//! unknown. An issuer is asked at most once a minute, also after a fetch
//! that failed or whose request went away, and its keys are kept while it
//! cannot be reached, or while its key set has keys but none that is
//! usable, for a day at most. A key set with no keys at all withdraws them.
//! Encryption keys are left out. An ES512 key is kept, although
//! `jsonwebtoken` does not know that `alg`. RS256/384/512 and ES256/384/512
//! are accepted, with [`DEFAULT_LEEWAY`] of clock skew.
//!
//! A pattern with `*` matches issuers that anyone can make up. The number of
//! its issuers without keys that are asked is limited for each client
//! ([`RequestContext::client`]), so one client's made-up issuers do not keep
//! the others from theirs, and for all clients together
//! ([`MAX_ASKED_ISSUERS_ALL`]), so many clients do not make a call each. An
//! issuer that does not answer is not counted as made up, so an issuer that
//! was down for a short time is asked again at once. An issuer of a pattern
//! that never had keys is forgotten [`UNKNOWN_ISSUER_TTL`] after it was last
//! asked. It is kept apart from the issuers that had keys, so made-up
//! issuers do not take their places.

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation,
    jwk::{AlgorithmParameters, EllipticCurve, Jwk, PublicKeyUse},
};
use moka::future::Cache;
use serde_json::{Map, Value};
use tokio::sync::{Mutex, OwnedMutexGuard, OwnedSemaphorePermit, RwLock, Semaphore};

use crate::http::no_answer;
use crate::window::{Client, Windows};
use crate::{
    Allowlist, AuthError, Direct, Identity, RequestContext, Resolution, Resolver, Transport,
};

/// The clock skew that the time claims of a provider's tokens may have.
pub const DEFAULT_LEEWAY: Duration = Duration::from_secs(60);
/// Keys are reused this long, and refetched for an unknown key id. An
/// issuer is asked at most [`MIN_REFRESH`] often.
const KEYS_TTL: Duration = Duration::from_secs(60 * 60);
const MIN_REFRESH: Duration = Duration::from_secs(60);
/// Keys serve this long after they were fetched while each fetch since
/// failed. After that the tokens of the issuer are refused until it answers
/// with keys, so a key that the issuer withdrew does not serve without end.
const KEPT_KEYS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// The most issuers whose keys are kept, and as many of a pattern that
/// never had keys. A pattern with `*` matches many.
const MAX_ISSUERS: u64 = 1_000;
/// How long an issuer of a pattern that never had keys is kept after it was
/// last asked: longer than [`MIN_REFRESH`], so it is not asked sooner.
const UNKNOWN_ISSUER_TTL: Duration = Duration::from_secs(2 * 60);
/// The most issuers without keys that are asked per [`MIN_REFRESH`] and
/// answer that they have none, for each client and pattern with `*`, so
/// tokens that name made-up issuers of a pattern do not make a call each.
/// An issuer that answers with keys is not counted, nor is one that does
/// not answer, nor are issuers named exactly. It is also the most of them
/// that are asked at a time: the others wait up to [`FETCH_TIMEOUT`], as
/// those asked may all have none.
const MAX_NEW_ISSUERS: u32 = 20;
/// The most issuers without keys that are asked per [`MIN_REFRESH`] for each
/// client and pattern with `*`, whatever they answer, so tokens that name
/// real issuers of a pattern do not make a call each either.
const MAX_ASKED_ISSUERS: u32 = 200;
/// The most issuers without keys that are asked per [`MIN_REFRESH`] for each
/// pattern with `*`, for all clients together, whatever they answer.
const MAX_ASKED_ISSUERS_ALL: u32 = 400;
/// How long each call to an issuer may take: its discovery document and
/// its keys fit in a request's time.
const FETCH_TIMEOUT: Duration = Duration::from_secs(4);

/// The header and claims of a token, unverified.
pub struct UnverifiedJwt {
    pub algorithm: String,
    pub key_id: Option<String>,
    pub claims: Map<String, Value>,
}

/// Split and decode a token without verifying it.
///
/// # Errors
///
/// When the token is not three segments of base64url JSON.
pub fn decode(token: &str) -> Result<UnverifiedJwt, String> {
    let mut segments = token.split('.');
    let (Some(header), Some(payload), Some(_), None) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return Err("The token was expected to have 3 parts".into());
    };
    let json = |segment: &str| -> Result<Map<String, Value>, String> {
        let bytes = URL_SAFE_NO_PAD
            .decode(segment.trim_end_matches('='))
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    };
    let header = json(header)?;
    let claims = json(payload)?;
    Ok(UnverifiedJwt {
        algorithm: header
            .get("alg")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        key_id: header
            .get("kid")
            .and_then(Value::as_str)
            .map(str::to_string),
        claims,
    })
}

/// JWTs are three dot-separated segments starting with `eyJ` (the `{"` of
/// the header): a cheap shape check that keeps other tokens away from the
/// JWT checks.
pub fn looks_like_jwt(token: &str) -> bool {
    token.starts_with("eyJ") && token.split('.').count() == 3
}

/// A JWT `aud`, which is a string or an array of strings.
pub fn audiences(claims: &Map<String, Value>) -> Vec<String> {
    match claims.get("aud") {
        Some(Value::String(aud)) => vec![aud.clone()],
        Some(Value::Array(auds)) => auds
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => vec![],
    }
}

/// A non-blank string claim.
pub fn claim<'a>(claims: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    claims
        .get(name)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
}

/// Why [`JwtKeys::verify`] refused a token. Each shows its message.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum VerifyError {
    /// The signature, issuer or times do not check out, or the issuer does
    /// not publish the key.
    #[error("{0}")]
    Invalid(String),
    /// An algorithm or key type that is not accepted.
    #[error("{0}")]
    Unsupported(String),
    /// The issuer's discovery document is not usable.
    #[error("{0}")]
    Misconfigured(String),
    /// The issuer's discovery document or keys could not be fetched.
    #[error("{0}")]
    Unavailable(String),
}

/// The signing keys of each issuer, from its discovery document.
pub struct JwtKeys {
    fetcher: Arc<Fetcher>,
    leeway: Duration,
    /// The issuers named exactly, and those of a pattern that had keys.
    issuers: Cache<String, Arc<IssuerKeys>>,
    /// The issuers of a pattern that never had keys, each for
    /// [`UNKNOWN_ISSUER_TTL`] after it was last asked.
    unknown: Cache<String, Arc<IssuerKeys>>,
    new_issuers: Arc<NewIssuers>,
}

/// The issuers without keys that were asked in the current [`MIN_REFRESH`].
struct NewIssuers {
    /// For each client and pattern with `*`.
    windows: Windows<Scope, Window>,
    /// For each pattern with `*`, of all clients.
    all: Windows<String, AllClients>,
}

/// The new issuers of a pattern, of all clients.
#[derive(Default)]
struct AllClients {
    /// The issuers asked since the start.
    asked: u32,
    /// Whether the log says that tokens of the pattern are refused.
    logged: bool,
}

/// A pattern with `*`, and the client whose tokens name its issuers.
type Scope = (String, Client);

struct Window {
    /// The issuers asked since the start.
    asked: u32,
    /// Those of them that answered, and had no keys.
    without_keys: u32,
    /// A place for each issuer that may still have no keys. An issuer holds
    /// one while it is asked, and one that answers without keys does not
    /// give it back. Closed when [`MAX_NEW_ISSUERS`] had none.
    places: Arc<Semaphore>,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            asked: 0,
            without_keys: 0,
            places: Arc::new(Semaphore::new(MAX_NEW_ISSUERS as usize)),
        }
    }
}

impl Default for NewIssuers {
    fn default() -> Self {
        Self {
            windows: Windows::new(MIN_REFRESH),
            all: Windows::new(MIN_REFRESH),
        }
    }
}

impl NewIssuers {
    /// Read or change the current window of `scope`.
    fn window<T>(&self, scope: &Scope, f: impl FnOnce(&mut Window) -> T) -> T {
        self.windows.with(scope, |window, _| f(window))
    }

    /// Whether the log is to say now that tokens of the pattern of `scope`
    /// are refused: once per pattern and [`MIN_REFRESH`].
    fn log_refusal(&self, scope: &Scope) -> bool {
        (self.all).with(&scope.0, |all, _| !std::mem::replace(&mut all.logged, true))
    }

    /// Count an issuer of `scope` that is asked now, and give it a place
    /// to hold until it answered. It waits while [`MAX_NEW_ISSUERS`] others
    /// are asked, until `deadline` at most, and moves to the next window
    /// when that one starts. `None` when no more issuers are asked in the
    /// window, for the client or for all clients, or when no place was free
    /// by `deadline`.
    async fn reserve(
        &self,
        scope: &Scope,
        deadline: tokio::time::Instant,
    ) -> Option<OwnedSemaphorePermit> {
        loop {
            if (self.all).with(&scope.0, |all, _| all.asked >= MAX_ASKED_ISSUERS_ALL) {
                return None;
            }
            let (places, ends) = self.windows.with(scope, |window, start| {
                (window.asked < MAX_ASKED_ISSUERS)
                    .then(|| (window.places.clone(), start + MIN_REFRESH))
            })?;
            let until = deadline.min(ends.into());
            let place = tokio::time::timeout_at(until, places.clone().acquire_owned()).await;
            let Ok(place) = place else {
                if tokio::time::Instant::now() >= deadline {
                    return None;
                }
                continue;
            };
            // `None` when the window of `places` is over.
            let counted = self.window(scope, |window| {
                Arc::ptr_eq(&window.places, &places).then(|| {
                    let asked = place.is_ok() && window.asked < MAX_ASKED_ISSUERS;
                    if asked {
                        window.asked += 1;
                    }
                    asked
                })
            });
            match counted {
                Some(true) => {
                    let asked = self.all.with(&scope.0, |all, _| {
                        let asked = all.asked < MAX_ASKED_ISSUERS_ALL;
                        if asked {
                            all.asked += 1;
                        }
                        asked
                    });
                    return place.ok().filter(|_| asked);
                }
                Some(false) => return None,
                None => {}
            }
        }
    }

    /// Whether `place` is of the current window of `scope`.
    fn is_current(&self, scope: &Scope, place: &OwnedSemaphorePermit) -> bool {
        self.window(scope, |window| {
            Arc::ptr_eq(&window.places, place.semaphore())
        })
    }

    /// The issuer of `scope` that held `place` answered, and had no keys.
    fn had_no_keys(&self, scope: &Scope, place: OwnedSemaphorePermit) {
        let counted = self.window(scope, |window| {
            // A place of an older window goes back to those who wait there.
            if !Arc::ptr_eq(&window.places, place.semaphore()) {
                return false;
            }
            window.without_keys += 1;
            if window.without_keys >= MAX_NEW_ISSUERS {
                window.places.close();
            }
            true
        });
        if counted {
            place.forget();
        }
    }
}

/// The calls to issuers.
struct Fetcher {
    http: reqwest::Client,
    transport: Box<dyn Transport>,
}

#[derive(Default)]
struct IssuerKeys {
    fetched: RwLock<Fetched>,
    /// Held by the fetch that runs, if one does.
    refresh: Arc<Mutex<()>>,
    /// The place of a new issuer of a pattern, until its first fetch takes
    /// it. It is kept here, not by the request that reserved it, as another
    /// request for the issuer may start that fetch.
    place: std::sync::Mutex<Option<OwnedSemaphorePermit>>,
}

/// Keep `keys` as the entry of `issuer` in `cache`, unless a newer entry
/// took its place.
async fn keep(cache: &Cache<String, Arc<IssuerKeys>>, issuer: &str, keys: &Arc<IssuerKeys>) {
    let replaced = (cache.get(issuer).await).is_some_and(|now| !Arc::ptr_eq(&now, keys));
    if !replaced {
        cache.insert(issuer.to_string(), keys.clone()).await;
    }
}

/// What an issuer last answered.
#[derive(Default)]
struct Fetched {
    /// Its keys, and when they were fetched.
    keys: Option<(Vec<Jwk>, Instant)>,
    /// When it was last asked.
    tried: Option<Instant>,
    /// Why the last fetch failed, if it did.
    failure: Option<VerifyError>,
    /// Whether it ever answered with one key or more: it is no new issuer
    /// then, also after it withdrew its keys, and it is kept with the
    /// issuers that had keys.
    known: bool,
}

impl Fetched {
    /// The key `key_id` among keys younger than [`KEYS_TTL`].
    fn fresh(&self, key_id: Option<&str>) -> Option<Jwk> {
        self.younger(KEYS_TTL, key_id)
    }

    /// The key `key_id` among keys younger than [`KEPT_KEYS_TTL`]: those
    /// that serve while a fetch runs, and after one that failed.
    fn kept(&self, key_id: Option<&str>) -> Option<Jwk> {
        self.younger(KEPT_KEYS_TTL, key_id)
    }

    fn younger(&self, ttl: Duration, key_id: Option<&str>) -> Option<Jwk> {
        let (set, at) = self.keys.as_ref()?;
        (at.elapsed() < ttl).then(|| find(set, key_id)).flatten()
    }

    /// The key `key_id` after a fetch: from the keys just fetched, or from
    /// older ones when the fetch failed, up to [`KEPT_KEYS_TTL`] old.
    fn answer(&self, key_id: Option<&str>, issuer: &str) -> Result<Jwk, VerifyError> {
        if let Some(key) = self.kept(key_id) {
            return Ok(key);
        }
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        // A key the issuer does not publish, or no longer does: the token
        // is refused, as a tampered one is.
        let none = (self.keys.as_ref()).is_none_or(|(set, _)| set.is_empty());
        Err(VerifyError::Invalid(match key_id {
            _ if none => {
                format!("Token verification failed: issuer {issuer} has no signing keys")
            }
            Some(key_id) => {
                format!("Token verification failed: no signing key '{key_id}' at issuer {issuer}")
            }
            None => format!(
                "Token verification failed: the token names no key, and issuer {issuer} has \
                 several"
            ),
        }))
    }
}

/// The key `key_id` of `set`. A token without a key id takes the only key,
/// else the one without a key id.
fn find(set: &[Jwk], key_id: Option<&str>) -> Option<Jwk> {
    match key_id {
        Some(key_id) => set
            .iter()
            .find(|k| k.common.key_id.as_deref() == Some(key_id)),
        None if set.len() == 1 => set.first(),
        None => set.iter().find(|k| k.common.key_id.is_none()),
    }
    .cloned()
}

impl Default for JwtKeys {
    fn default() -> Self {
        Self::new(Direct)
    }
}

impl JwtKeys {
    /// Keys fetched through `transport`.
    pub fn new(transport: impl Transport) -> Self {
        let http = reqwest::Client::builder()
            .timeout(FETCH_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            fetcher: Arc::new(Fetcher {
                http,
                transport: Box::new(transport),
            }),
            leeway: DEFAULT_LEEWAY,
            issuers: Cache::builder().max_capacity(MAX_ISSUERS).build(),
            unknown: Cache::builder()
                .max_capacity(MAX_ISSUERS)
                .time_to_live(UNKNOWN_ISSUER_TTL)
                .build(),
            new_issuers: Arc::default(),
        }
    }

    /// The clock skew of the time claims ([`DEFAULT_LEEWAY`]).
    pub fn with_leeway(mut self, leeway: Duration) -> Self {
        self.leeway = leeway;
        self
    }

    /// The HTTP client of the calls to providers, for other resolvers that
    /// call them.
    pub fn http(&self) -> &reqwest::Client {
        &self.fetcher.http
    }

    /// Send `request` through the transport.
    pub async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> reqwest::Result<reqwest::Response> {
        self.fetcher.transport.send(request).await
    }

    /// Check the token's signature with the issuer's key, and its issuer and
    /// times. The audience is the caller's to check. The answer is the
    /// token's `exp`, in seconds since the epoch.
    ///
    /// `pattern` is the pattern with `*` that `issuer` matched, `None` for an
    /// issuer named exactly: the issuers of a pattern that have no keys yet
    /// are asked until [`MAX_NEW_ISSUERS`] of them had none in a minute, or
    /// [`MAX_ASKED_ISSUERS`] were asked, for each pattern and `client`, the
    /// address that sent the token ([`RequestContext::client`]), and until
    /// [`MAX_ASKED_ISSUERS_ALL`] were asked for all clients of the pattern.
    ///
    /// # Errors
    ///
    /// As [`VerifyError`] says.
    pub async fn verify(
        &self,
        token: &str,
        decoded: &UnverifiedJwt,
        issuer: &str,
        pattern: Option<&str>,
        client: Option<IpAddr>,
    ) -> Result<f64, VerifyError> {
        let key_id = decoded.key_id.as_deref();
        let jwk = match self.fresh(issuer, key_id).await {
            Some(jwk) => jwk,
            None => {
                let scope = pattern.map(|pattern| (pattern.to_string(), Client::of(client)));
                self.key(issuer, key_id, scope.as_ref()).await?
            }
        };
        let failed = |e: String| VerifyError::Invalid(format!("Token verification failed: {e}"));
        let unsupported =
            |kind: &str, alg: &str| VerifyError::Unsupported(format!("Unsupported {kind}: {alg}"));
        let algorithm = decoded.algorithm.as_str();
        let checked = match (&jwk.algorithm, algorithm) {
            (AlgorithmParameters::RSA(_), "RS256" | "RS384" | "RS512")
            | (AlgorithmParameters::EllipticCurve(_), "ES256" | "ES384") => {
                let algorithm: Algorithm = algorithm
                    .parse()
                    .map_err(|_| unsupported("algorithm", algorithm))?;
                let key = DecodingKey::from_jwk(&jwk).map_err(|e| failed(e.to_string()))?;
                // The signature only: `jsonwebtoken` skips an `exp` or `nbf`
                // with a fraction, so the times are checked below.
                let mut validation = Validation::new(algorithm);
                validation.set_issuer(&[issuer]);
                validation.validate_aud = false;
                validation.validate_exp = false;
                validation.required_spec_claims.clear();
                jsonwebtoken::decode::<Value>(token, &key, &validation)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            (AlgorithmParameters::EllipticCurve(params), "ES512")
                if params.curve == EllipticCurve::P521 =>
            {
                verify_es512(token, &params.x, &params.y)
            }
            (AlgorithmParameters::RSA(_), other) => {
                return Err(unsupported("RSA algorithm", other));
            }
            (AlgorithmParameters::EllipticCurve(_), other) => {
                return Err(unsupported("ECDSA algorithm", other));
            }
            (AlgorithmParameters::OctetKey(_), _) => return Err(unsupported("key type", "oct")),
            (AlgorithmParameters::OctetKeyPair(_), _) => {
                return Err(unsupported("key type", "OKP"));
            }
        };
        checked
            .and_then(|()| check_times(&decoded.claims, issuer, self.leeway))
            .map_err(failed)
    }

    /// The key `key_id` of an issuer whose keys are fresh.
    async fn fresh(&self, issuer: &str, key_id: Option<&str>) -> Option<Jwk> {
        let keys = self.issuers.get(issuer).await?;
        let fetched = keys.fetched.read().await;
        fetched.fresh(key_id)
    }

    /// The issuer's key `key_id`, fetched when unknown.
    async fn key(
        &self,
        issuer: &str,
        key_id: Option<&str>,
        scope: Option<&Scope>,
    ) -> Result<Jwk, VerifyError> {
        let too_many = || {
            if scope.is_some_and(|scope| self.new_issuers.log_refusal(scope)) {
                tracing::warn!(
                    %issuer,
                    ?scope,
                    "too many new issuers; tokens of new issuers of the pattern are refused for now"
                );
            }
            VerifyError::Unavailable(format!("Too many new issuers, try again later: {issuer}"))
        };
        // The wait for a place ends here for all the requests for one
        // issuer, also for those that wait for its fetch lock first.
        let deadline = tokio::time::Instant::now() + FETCH_TIMEOUT;
        let keys = match (self.issuers.get(issuer).await, scope) {
            (Some(keys), _) => keys,
            (None, None) => {
                (self.issuers)
                    .get_with_by_ref(issuer, async { Arc::default() })
                    .await
            }
            // A new issuer of a pattern is counted before it is kept, so
            // nothing is kept for the issuers that are not asked.
            (None, Some(scope)) => {
                let counted = async {
                    let place = (self.new_issuers.reserve(scope, deadline).await).ok_or(())?;
                    Ok::<_, ()>(Arc::new(IssuerKeys {
                        place: std::sync::Mutex::new(Some(place)),
                        ..Default::default()
                    }))
                };
                let kept = (self.unknown).try_get_with_by_ref(issuer, counted).await;
                match kept {
                    Ok(keys) => keys,
                    Err(_) => return Err(too_many()),
                }
            }
        };
        let stale = {
            let fetched = keys.fetched.read().await;
            if let Some(key) = fetched.fresh(key_id) {
                return Ok(key);
            }
            fetched.kept(key_id)
        };
        // One fetch at a time per issuer. While one runs, the keys that it
        // is to replace serve.
        let guard = match keys.refresh.clone().try_lock_owned() {
            Ok(guard) => guard,
            Err(_) => match stale {
                Some(key) => return Ok(key),
                None => keys.refresh.clone().lock_owned().await,
            },
        };
        // What a recent fetch found, or why it failed, is reused.
        let new = {
            let fetched = keys.fetched.read().await;
            if let Some(key) = fetched.fresh(key_id) {
                return Ok(key);
            }
            if fetched.tried.is_some_and(|at| at.elapsed() < MIN_REFRESH) {
                return fetched.answer(key_id, issuer);
            }
            scope.filter(|_| !fetched.known)
        };
        // An issuer that was kept before is counted again, and so is one
        // whose place is of a window that is over. The fetch lock is held
        // here, so only this request takes or reserves a place.
        let reserved = (keys.place.lock().unwrap_or_else(|e| e.into_inner())).take();
        let counted = match new {
            None => None,
            Some(scope) => {
                let place = match reserved.filter(|p| self.new_issuers.is_current(scope, p)) {
                    Some(place) => place,
                    None => {
                        (self.new_issuers.reserve(scope, deadline).await).ok_or_else(too_many)?
                    }
                };
                Some((self.new_issuers.clone(), scope.clone(), place))
            }
        };
        if counted.is_some() {
            // Kept for [`UNKNOWN_ISSUER_TTL`] from this fetch on, so the
            // issuer is not asked again sooner.
            keep(&self.unknown, issuer, &keys).await;
        }
        {
            // Asked now, also when this request goes away before the answer:
            // the fetch runs on. The fetch replaces this failure when it
            // ends, so it shows only after a fetch that did not end.
            let mut fetched = keys.fetched.write().await;
            fetched.tried = Some(Instant::now());
            if fetched.failure.is_none() {
                fetched.failure = Some(VerifyError::Unavailable(format!(
                    "The keys of issuer {issuer} are being fetched, try again later"
                )));
            }
        }
        // The fetch is a task, so a request with a kept key does not wait
        // for it.
        let task = tokio::spawn(refresh(
            self.fetcher.clone(),
            self.issuers.clone(),
            issuer.to_string(),
            keys.clone(),
            counted,
            guard,
        ));
        if let Some(key) = stale {
            return Ok(key);
        }
        if let Err(e) = task.await {
            tracing::warn!(%issuer, error = %e, "the fetch of the issuer's keys failed");
        }
        keys.fetched.read().await.answer(key_id, issuer)
    }
}

/// Fetch the keys of `issuer` and keep what it answered, then let the next
/// fetch run (`guard`). `counted` is the scope of a new issuer and the
/// place that it holds: one that answers with keys is a real one, and gives
/// its place back, and so does one that does not answer. An issuer that
/// answers with keys for the first time is kept in `issuers` from then on.
async fn refresh(
    fetcher: Arc<Fetcher>,
    issuers: Cache<String, Arc<IssuerKeys>>,
    issuer: String,
    keys: Arc<IssuerKeys>,
    counted: Option<(Arc<NewIssuers>, Scope, OwnedSemaphorePermit)>,
    guard: OwnedMutexGuard<()>,
) {
    let result = fetcher.fetch_keys(&issuer).await;
    let has_keys = matches!(&result, Ok(set) if !set.is_empty());
    let made_up = match &result {
        Ok(set) => set.is_empty(),
        Err(e) => e.no_issuer,
    };
    let mut fetched = keys.fetched.write().await;
    match result {
        Ok(set) => {
            fetched.keys = Some((set, Instant::now()));
            fetched.failure = None;
        }
        Err(FetchError { error, .. }) => {
            if fetched.keys.is_some() {
                tracing::warn!(%issuer, %error, "keeping the issuer's keys");
            }
            fetched.failure = Some(error);
        }
    }
    let first_keys = has_keys && !std::mem::replace(&mut fetched.known, true);
    drop(fetched);
    if first_keys {
        keep(&issuers, &issuer, &keys).await;
    }
    if let Some((new_issuers, scope, place)) = counted
        && made_up
    {
        new_issuers.had_no_keys(&scope, place);
    }
    drop(guard);
}

/// Why the keys of an issuer were not fetched.
struct FetchError {
    error: VerifyError,
    /// Whether the answer says that there is no such issuer. Not so when
    /// the issuer, or the way to it, is down or broken.
    no_issuer: bool,
}

impl FetchError {
    fn no_issuer(error: VerifyError) -> Self {
        Self {
            error,
            no_issuer: true,
        }
    }

    fn down(error: VerifyError) -> Self {
        Self {
            error,
            no_issuer: false,
        }
    }
}

impl Fetcher {
    /// The JWK set that the issuer's OIDC discovery document names.
    async fn fetch_keys(&self, issuer: &str) -> Result<Vec<Jwk>, FetchError> {
        let issuer = match issuer.starts_with("https://") || issuer.starts_with("http://") {
            true => issuer.to_string(),
            false => format!("https://{issuer}"),
        };
        let path = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        let unreachable = |e: &dyn std::fmt::Display| {
            tracing::warn!(%path, error = %e, "cannot fetch the issuer configuration");
            FetchError::down(VerifyError::Unavailable(format!(
                "Could not get issuer configuration from {path}"
            )))
        };
        let response = (self.transport.send(self.http.get(&path)))
            .await
            .map_err(|e| unreachable(&e))?;
        let status = response.status();
        if !status.is_success() {
            let error = VerifyError::Unavailable(format!(
                "Could not get issuer configuration from {path}: HTTP {}",
                status.as_u16()
            ));
            let no_issuer = status.is_client_error() && !no_answer(status);
            return Err(FetchError { error, no_issuer });
        }
        let config: Map<String, Value> = response.json().await.map_err(|_| {
            FetchError::no_issuer(VerifyError::Unavailable(format!(
                "Issuer configuration from {path} is not valid JSON"
            )))
        })?;
        if config.is_empty() {
            return Err(FetchError::no_issuer(VerifyError::Misconfigured(format!(
                "Could not get issuer configuration from {path}"
            ))));
        }
        let config_issuer = config
            .get("issuer")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if config_issuer != issuer {
            return Err(FetchError::no_issuer(VerifyError::Misconfigured(format!(
                "Issuer '{issuer}' doesn't match configuration issuer '{config_issuer}' from {path}"
            ))));
        }
        let jwks_uri = config
            .get("jwks_uri")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                FetchError::no_issuer(VerifyError::Misconfigured(
                    "JWKS configuration missing".into(),
                ))
            })?;
        let set: Value = (self.transport.send(self.http.get(jwks_uri)))
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| unreachable(&e))?
            .json()
            .await
            .map_err(|e| unreachable(&e))?;
        // An answer that is no key set is an error page or a broken issuer:
        // the keys it had are kept.
        let Some(keys) = set["keys"].as_array() else {
            tracing::warn!(%jwks_uri, "the issuer's answer is not a key set");
            return Err(FetchError::down(VerifyError::Unavailable(format!(
                "No key set from issuer {issuer}"
            ))));
        };
        // Keys of kinds that do not parse are skipped, not fatal. A key set
        // with no keys at all withdraws the keys it had. One with keys of
        // which none is usable is a broken answer: the keys it had are kept.
        let usable: Vec<Jwk> = keys.iter().filter_map(parse_jwk).collect();
        if usable.is_empty() && !keys.is_empty() {
            tracing::warn!(%jwks_uri, "the issuer's key set has no usable keys");
            return Err(FetchError::down(VerifyError::Unavailable(format!(
                "No usable keys from issuer {issuer}"
            ))));
        }
        Ok(usable)
    }
}

/// A key of a key set, `None` for a kind that is not known and for an
/// encryption key (`"use": "enc"`). The `alg` ES512, which `jsonwebtoken`
/// does not know, is left out: the algorithm is the token's, checked against
/// the kind of the key. A key with any other unknown `alg` signs no token
/// that is accepted.
fn parse_jwk(key: &Value) -> Option<Jwk> {
    let jwk: Jwk = match serde_json::from_value(key.clone()) {
        Ok(jwk) => jwk,
        Err(_) if key["alg"] == "ES512" => {
            let mut key = key.as_object()?.clone();
            key.remove("alg");
            serde_json::from_value(Value::Object(key)).ok()?
        }
        Err(_) => return None,
    };
    (jwk.common.public_key_use != Some(PublicKeyUse::Encryption)).then_some(jwk)
}

/// An ES512 (P-521) signature, which `jsonwebtoken` does not verify.
fn verify_es512(token: &str, x: &str, y: &str) -> Result<(), String> {
    use p521::ecdsa::{Signature, VerifyingKey, signature::Verifier};
    let (message, signature) = token.rsplit_once('.').ok_or("malformed token")?;
    let coordinate = |c: &str| -> Result<p521::FieldBytes, String> {
        let bytes = URL_SAFE_NO_PAD.decode(c).map_err(|e| e.to_string())?;
        (bytes.len() == 66)
            .then(|| p521::FieldBytes::clone_from_slice(&bytes))
            .ok_or("bad P-521 coordinate".into())
    };
    let point =
        p521::EncodedPoint::from_affine_coordinates(&coordinate(x)?, &coordinate(y)?, false);
    let key = VerifyingKey::from_encoded_point(&point).map_err(|e| e.to_string())?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|e| e.to_string())?;
    let signature = Signature::from_slice(&signature).map_err(|e| e.to_string())?;
    key.verify(message.as_bytes(), &signature)
        .map_err(|_| "InvalidSignature".to_string())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// The issuer and time checks of a token whose signature checks out. A
/// token must have an `exp`, which is the answer.
fn check_times(claims: &Map<String, Value>, issuer: &str, leeway: Duration) -> Result<f64, String> {
    if claims.get("iss").and_then(Value::as_str) != Some(issuer) {
        return Err("InvalidIssuer".into());
    }
    let now = now_secs() as f64;
    let leeway = leeway.as_secs_f64();
    let exp = time_claim(claims, "exp")?.ok_or("MissingRequiredClaim: exp")?;
    if exp + leeway < now {
        return Err("ExpiredSignature".into());
    }
    if time_claim(claims, "nbf")?.is_some_and(|nbf| nbf - leeway > now) {
        return Err("ImmatureSignature".into());
    }
    Ok(exp)
}

/// The time claim `name` in seconds since the epoch, which may have a
/// fraction; an error when it is there but not a number.
fn time_claim(claims: &Map<String, Value>, name: &str) -> Result<Option<f64>, String> {
    match claims.get(name) {
        None => Ok(None),
        Some(value) => (value.as_f64())
            .map(Some)
            .ok_or_else(|| format!("InvalidClaimFormat: {name}")),
    }
}

/// A provider whose tokens a [`JwtResolver`] takes: the `iss` it matches,
/// the audiences it accepts, and the claim with the caller's groups.
#[derive(Debug, Clone)]
pub struct JwtProvider {
    /// The name of the provider, the [`Identity::provider`] of its callers.
    pub name: String,
    issuers: Allowlist,
    audiences: Vec<String>,
    audience_allowlist: Allowlist,
    groups_claim: Option<String>,
}

impl JwtProvider {
    /// The provider `name` of the tokens whose `iss` is one of `issuers`,
    /// exact or with `*` for one DNS label. It accepts no audience until
    /// [`with_audiences`](Self::with_audiences).
    pub fn new(name: impl Into<String>, issuers: &[impl AsRef<str>]) -> Self {
        Self {
            name: name.into(),
            issuers: Allowlist::new(issuers),
            audiences: Vec::new(),
            audience_allowlist: Allowlist::default(),
            groups_claim: None,
        }
    }

    /// Accepts tokens with one of `audiences` in their `aud`, exact or with
    /// `*` for one DNS label; `*` alone accepts any.
    pub fn with_audiences(mut self, audiences: &[impl AsRef<str>]) -> Self {
        self.audiences = (audiences.iter())
            .map(|a| a.as_ref().trim())
            .filter(|a| !a.is_empty())
            .map(str::to_string)
            .collect();
        self.audience_allowlist = Allowlist::new(&self.audiences);
        self
    }

    /// The claim with the names of the caller's groups: an array of strings
    /// or one string. See [`groups`](Self::groups).
    pub fn with_groups_claim(mut self, claim: Option<impl Into<String>>) -> Self {
        self.groups_claim = claim
            .map(Into::into)
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        self
    }

    /// Whether `iss` is this provider's.
    pub fn matches(&self, iss: &str) -> bool {
        self.issuers.is_allowed(iss)
    }

    /// The pattern with `*` that `iss` matches; `None` when the provider
    /// names it exactly, or does not have it. See [`issuer_pattern`].
    pub fn pattern(&self, iss: &str) -> Option<&str> {
        self.issuers.matching(iss).flatten()
    }

    /// Whether one of the token's `aud` is accepted.
    pub fn audience_allowed(&self, claims: &Map<String, Value>) -> bool {
        let any = self.audiences.len() == 1 && self.audiences[0] == "*";
        any || self.audience_allowlist.is_any_allowed(&audiences(claims))
    }

    /// The caller's groups, each once, as `<provider>:<group>`: a provider
    /// can name its own groups only, not those of another provider. Every
    /// issuer that a pattern with `*` matches is the same provider, and
    /// names the same groups.
    pub fn groups(&self, claims: &Map<String, Value>) -> Vec<String> {
        let Some(name) = &self.groups_claim else {
            return Vec::new();
        };
        let names: Vec<&str> = match claims.get(name) {
            Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
            Some(Value::String(one)) => vec![one.as_str()],
            _ => Vec::new(),
        };
        let mut seen = HashSet::new();
        names
            .into_iter()
            .map(str::trim)
            .filter(|g| !g.is_empty() && seen.insert(*g))
            .map(|g| format!("{}:{g}", self.name))
            .collect()
    }
}

/// The pattern with `*` that `iss` matches among `providers`, for
/// [`JwtKeys::verify`]: the first one's that has `iss`. `None` when one of
/// them names `iss` exactly, as it is no pattern's issuer then, or when none
/// has it.
pub fn issuer_pattern<'a>(
    providers: impl IntoIterator<Item = &'a JwtProvider>,
    iss: &str,
) -> Option<&'a str> {
    let mut patterns = (providers.into_iter())
        .filter(|provider| provider.matches(iss))
        .map(|provider| provider.pattern(iss));
    let first = patterns.next().flatten()?;
    patterns.all(|pattern| pattern.is_some()).then_some(first)
}

/// Takes the JWTs of its providers, checked with [`JwtKeys`].
///
/// A token that is not a JWT, or whose `iss` is no provider's, passes. A
/// token is the first provider's that has its `iss` and takes its `aud`;
/// one that does not check out is refused. The caller is the token's `sub`,
/// with its claims; the identity is kept until the token's `exp`. A token
/// without `sub` has an empty subject: a host that takes such tokens names
/// the caller from the claims in its admission.
pub struct JwtResolver {
    providers: Vec<JwtProvider>,
    keys: Arc<JwtKeys>,
}

impl JwtResolver {
    pub fn new(providers: Vec<JwtProvider>, keys: Arc<JwtKeys>) -> Self {
        Self { providers, keys }
    }
}

#[async_trait]
impl<S: Send + Sync> Resolver<S> for JwtResolver {
    async fn resolve(&self, cx: &RequestContext<'_>, _state: &S) -> Resolution {
        let Some(token) = cx.token().filter(|t| looks_like_jwt(t)) else {
            return Resolution::Pass;
        };
        let Ok(decoded) = decode(token) else {
            return Resolution::Pass;
        };
        let iss = claim(&decoded.claims, "iss")
            .unwrap_or_default()
            .to_string();
        let iss = iss.as_str();
        let providers: Vec<&JwtProvider> =
            self.providers.iter().filter(|p| p.matches(iss)).collect();
        if providers.is_empty() {
            return Resolution::Pass;
        }
        let pattern = issuer_pattern(providers.iter().copied(), iss);
        let exp = match (self.keys.verify(token, &decoded, iss, pattern, cx.client())).await {
            Ok(exp) => exp,
            // The caller may well be signed in. The log has the cause from
            // where it arose.
            Err(VerifyError::Unavailable(reason)) => {
                tracing::debug!(%iss, %reason, "token not checked");
                return Resolution::Refused(AuthError::Unavailable(reason));
            }
            Err(e) => {
                tracing::warn!(%iss, reason = %e, "token refused");
                return Resolution::Refused(AuthError::invalid(e.to_string()));
            }
        };
        let claims = decoded.claims;
        let Some(provider) = (providers.into_iter()).find(|p| p.audience_allowed(&claims)) else {
            tracing::warn!(
                %iss,
                audience = ?audiences(&claims),
                "token refused: audience not allowed"
            );
            return Resolution::Refused(AuthError::invalid("audience not allowed"));
        };
        let left = Duration::from_secs_f64((exp - now_secs() as f64).clamp(0.0, 1e9));
        let mut identity = Identity::new(
            &provider.name,
            Identity::JWT,
            claim(&claims, "sub").unwrap_or_default(),
        );
        identity.groups = provider.groups(&claims);
        identity.claims = claims;
        identity.keep_until = Some(Instant::now() + left);
        Resolution::Resolved(identity)
    }

    fn keeps(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_match_issuers_audiences_and_groups() {
        let provider = JwtProvider::new("entra", &["https://login.example.com/*/v2.0"])
            .with_audiences(&[" api://app ", ""])
            .with_groups_claim(Some("roles"));
        assert!(provider.matches("https://login.example.com/t1/v2.0"));
        assert!(!provider.matches("https://login.example.com/t1/x/v2.0"));
        let mut claims: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "aud": ["x", "api://app"], "roles": ["Writers", " Readers ", "", "Writers"],
        }))
        .unwrap();
        assert!(provider.audience_allowed(&claims));
        assert_eq!(provider.groups(&claims), ["entra:Writers", "entra:Readers"]);
        claims.insert("aud".into(), Value::from("x"));
        assert!(!provider.audience_allowed(&claims));
        let any = JwtProvider::new("any", &["https://x"]).with_audiences(&["*"]);
        assert!(any.audience_allowed(&claims));
        assert!(any.groups(&claims).is_empty());
        let none = JwtProvider::new("none", &["https://x"]);
        assert!(!none.audience_allowed(&claims));
    }

    #[test]
    fn tokens_are_decoded_without_checks() {
        let segment = |json: &str| URL_SAFE_NO_PAD.encode(json);
        let token = format!(
            "{}.{}.sig",
            segment(r#"{"alg":"RS256","kid":"k1"}"#),
            segment(r#"{"iss":"https://x","aud":"a"}"#)
        );
        assert!(looks_like_jwt(&token));
        let decoded = decode(&token).unwrap();
        assert_eq!(
            (decoded.algorithm.as_str(), decoded.key_id.as_deref()),
            ("RS256", Some("k1"))
        );
        assert_eq!(claim(&decoded.claims, "iss"), Some("https://x"));
        assert_eq!(audiences(&decoded.claims), ["a"]);
        assert!(decode("a.b").is_err());
        assert!(!looks_like_jwt("dapi123"));
    }

    fn jwk(key_id: Option<&str>) -> Jwk {
        let mut value = serde_json::json!({ "kty": "RSA", "n": "AQAB", "e": "AQAB" });
        if let Some(key_id) = key_id {
            value["kid"] = key_id.into();
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn keys_are_found_by_their_id() {
        let one = [jwk(Some("k1"))];
        assert!(find(&one, Some("k1")).is_some());
        assert!(find(&one, Some("k2")).is_none());
        // A token without a key id takes the only key, else the one
        // without a key id.
        assert!(find(&one, None).is_some());
        let two = [jwk(Some("k1")), jwk(None)];
        assert_eq!(find(&two, None).unwrap().common.key_id, None);
        assert!(find(&[jwk(Some("k1")), jwk(Some("k2"))], None).is_none());
    }

    #[test]
    fn keys_are_kept_while_the_issuer_cannot_be_reached() {
        let fetched = Fetched {
            keys: Some((vec![jwk(Some("k1"))], Instant::now())),
            tried: Some(Instant::now()),
            failure: Some(VerifyError::Unavailable("down".into())),
            known: true,
        };
        assert!(fetched.answer(Some("k1"), "https://x").is_ok());
        assert!(matches!(
            fetched.answer(Some("k2"), "https://x"),
            Err(VerifyError::Unavailable(_))
        ));
        assert!(matches!(
            Fetched::default().answer(Some("k1"), "https://x"),
            Err(VerifyError::Invalid(_))
        ));
        // Not without end: the issuer may have withdrawn them.
        let Some(old) = Instant::now().checked_sub(KEPT_KEYS_TTL + Duration::from_secs(1)) else {
            return;
        };
        let fetched = Fetched {
            keys: Some((vec![jwk(Some("k1"))], old)),
            ..fetched
        };
        assert!(fetched.kept(Some("k1")).is_none());
        assert!(matches!(
            fetched.answer(Some("k1"), "https://x"),
            Err(VerifyError::Unavailable(_))
        ));
    }

    /// Answers every request after `delay`, and counts them: with HTTP 500,
    /// as an issuer that is down, or with HTTP 404, as a host that is no
    /// issuer.
    #[derive(Clone, Default)]
    struct Down {
        calls: Arc<std::sync::atomic::AtomicUsize>,
        delay: Duration,
        no_issuer: bool,
    }

    impl Down {
        fn after(delay: Duration) -> Self {
            Self {
                delay,
                ..Default::default()
            }
        }

        fn no_issuer(delay: Duration) -> Self {
            Self {
                delay,
                no_issuer: true,
                ..Default::default()
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Transport for Down {
        async fn send(&self, _: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
            (self.calls).fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            let status = if self.no_issuer { 404 } else { 500 };
            let response = axum::http::Response::builder().status(status).body("");
            Ok(response.unwrap().into())
        }
    }

    /// The scope of `pattern` and a client without an address.
    fn scope(pattern: &str) -> Scope {
        (pattern.to_string(), Client::of(None))
    }

    #[tokio::test]
    async fn issuers_are_asked_at_most_once_a_minute() {
        let down = Down::no_issuer(Duration::ZERO);
        let keys = JwtKeys::new(down.clone());
        for _ in 0..3 {
            let key = keys.key("https://a", Some("k1"), None).await;
            assert!(matches!(key, Err(VerifyError::Unavailable(_))));
        }
        assert_eq!(down.calls(), 1);
        // Made-up issuers of a pattern with `*`.
        for n in 0..2 * MAX_NEW_ISSUERS {
            let key = (keys.key(
                &format!("https://t{n}.a"),
                Some("k1"),
                Some(&scope("https://*.a")),
            ))
            .await;
            assert!(matches!(key, Err(VerifyError::Unavailable(_))));
        }
        assert_eq!(down.calls(), 1 + MAX_NEW_ISSUERS as usize);
        // Nothing is kept for those that were not asked, and those that
        // were are kept apart from the issuers named exactly.
        keys.issuers.run_pending_tasks().await;
        keys.unknown.run_pending_tasks().await;
        assert_eq!(keys.issuers.entry_count(), 1);
        assert_eq!(keys.unknown.entry_count(), u64::from(MAX_NEW_ISSUERS));
        // They take nothing from issuers named exactly, nor from another
        // pattern.
        assert!(keys.key("https://b", Some("k1"), None).await.is_err());
        let other = scope("https://*.b");
        let other = keys.key("https://t.b", Some("k1"), Some(&other)).await;
        assert!(other.is_err());
        assert_eq!(down.calls(), 3 + MAX_NEW_ISSUERS as usize);
    }

    /// Answers as every issuer, with the key set that it holds.
    #[derive(Clone)]
    struct Serves(Arc<std::sync::Mutex<String>>);

    impl Serves {
        /// Answer with `key_set` from now on.
        fn set(&self, key_set: &str) {
            *self.0.lock().unwrap() = key_set.to_string();
        }

        fn new(key_set: &str) -> Self {
            Self(Arc::new(std::sync::Mutex::new(key_set.to_string())))
        }
    }

    #[async_trait]
    impl Transport for Serves {
        async fn send(
            &self,
            request: reqwest::RequestBuilder,
        ) -> reqwest::Result<reqwest::Response> {
            let url = request.build()?.url().to_string();
            let body = match url.strip_suffix("/.well-known/openid-configuration") {
                Some(issuer) => {
                    serde_json::json!({ "issuer": issuer, "jwks_uri": format!("{issuer}/keys") })
                        .to_string()
                }
                None => self.0.lock().unwrap().clone(),
            };
            Ok(axum::http::Response::new(body).into())
        }
    }

    const KEY_SET: &str = r#"{"keys": [{"kty": "RSA", "kid": "k1", "n": "AQAB", "e": "AQAB"}]}"#;

    /// Wait until the fetch of the keys of `issuer` ended.
    async fn fetch_ended(keys: &JwtKeys, issuer: &str) {
        let entry = keys.issuers.get(issuer).await.unwrap();
        drop(entry.refresh.lock().await);
    }

    /// Make the next request for `issuer` fetch its keys again.
    async fn expire(keys: &JwtKeys, issuer: &str) {
        let Some(old) = Instant::now().checked_sub(KEYS_TTL + Duration::from_secs(1)) else {
            panic!("the clock is too young for this test");
        };
        let entry = keys.issuers.get(issuer).await.unwrap();
        let mut fetched = entry.fetched.write().await;
        fetched.tried = None;
        if let Some((_, at)) = &mut fetched.keys {
            *at = old;
        }
    }

    /// The key `k1` of `issuer` once its keys were fetched again: the
    /// request that starts the fetch has the kept key, and does not wait.
    async fn refetched(
        keys: &JwtKeys,
        issuer: &str,
        scope: Option<&Scope>,
    ) -> Result<Jwk, VerifyError> {
        expire(keys, issuer).await;
        assert!(keys.key(issuer, Some("k1"), scope).await.is_ok());
        fetch_ended(keys, issuer).await;
        keys.key(issuer, Some("k1"), scope).await
    }

    #[tokio::test]
    async fn an_empty_key_set_withdraws_the_keys() {
        let serves = Serves::new(KEY_SET);
        let keys = JwtKeys::new(serves.clone());
        assert!(keys.key("https://a", Some("k1"), None).await.is_ok());
        // An error page is no key set: the keys are kept.
        serves.set(r#"{"error": "down"}"#);
        assert!(refetched(&keys, "https://a", None).await.is_ok());
        // So is a key set with keys of which none is usable.
        serves.set(r#"{"keys": [{"kty": "EC", "crv": "secp256k1"}]}"#);
        assert!(refetched(&keys, "https://a", None).await.is_ok());
        serves.set(r#"{"keys": []}"#);
        let key = refetched(&keys, "https://a", None).await;
        assert!(matches!(key, Err(VerifyError::Invalid(_))), "{key:?}");
    }

    #[tokio::test]
    async fn an_issuer_that_withdrew_its_keys_is_no_new_issuer() {
        let serves = Serves::new(KEY_SET);
        let keys = JwtKeys::new(serves.clone());
        let scope = scope("https://*.a");
        let pattern = Some(&scope);
        assert!(keys.key("https://t.a", Some("k1"), pattern).await.is_ok());
        serves.set(r#"{"keys": []}"#);
        let key = refetched(&keys, "https://t.a", pattern).await;
        assert!(matches!(key, Err(VerifyError::Invalid(_))), "{key:?}");
        // No kept key now: the request waits for the fetch.
        expire(&keys, "https://t.a").await;
        let key = keys.key("https://t.a", Some("k1"), pattern).await;
        assert!(matches!(key, Err(VerifyError::Invalid(_))), "{key:?}");
        let counts = (keys.new_issuers).window(&scope, |w| (w.asked, w.without_keys));
        assert_eq!(counts, (1, 0));
        // It is kept with the issuers that had keys.
        assert!(keys.issuers.get("https://t.a").await.is_some());
    }

    #[tokio::test]
    async fn issuers_that_do_not_answer_are_not_counted_as_made_up() {
        let down = Down::default();
        let keys = JwtKeys::new(down.clone());
        let scope = scope("https://*.a");
        for n in 0..2 * MAX_NEW_ISSUERS {
            let issuer = format!("https://t{n}.a");
            let key = keys.key(&issuer, Some("k1"), Some(&scope)).await;
            assert!(matches!(key, Err(VerifyError::Unavailable(_))));
        }
        assert_eq!(down.calls(), 2 * MAX_NEW_ISSUERS as usize);
        let counts = (keys.new_issuers).window(&scope, |w| (w.asked, w.without_keys));
        assert_eq!(counts, (2 * MAX_NEW_ISSUERS, 0));
        // They are not asked again within the minute.
        assert!(
            keys.key("https://t0.a", Some("k1"), Some(&scope))
                .await
                .is_err()
        );
        assert_eq!(down.calls(), 2 * MAX_NEW_ISSUERS as usize);
    }

    #[test]
    fn an_issuer_without_keys_is_not_said_to_have_several() {
        let withdrawn = Fetched {
            keys: Some((Vec::new(), Instant::now())),
            ..Default::default()
        };
        for key_id in [None, Some("k1")] {
            let Err(VerifyError::Invalid(reason)) = withdrawn.answer(key_id, "https://x") else {
                panic!("not refused");
            };
            assert!(
                reason.ends_with("issuer https://x has no signing keys"),
                "{reason}"
            );
        }
        let two = Fetched {
            keys: Some((vec![jwk(Some("k1")), jwk(Some("k2"))], Instant::now())),
            ..Default::default()
        };
        let Err(VerifyError::Invalid(reason)) = two.answer(None, "https://x") else {
            panic!("not refused");
        };
        assert!(reason.ends_with("has several"), "{reason}");
    }

    #[tokio::test(start_paused = true)]
    async fn a_request_with_a_kept_key_does_not_wait_for_the_fetch() {
        let serves = Serves::new(KEY_SET);
        let keys = JwtKeys::new(serves);
        assert!(keys.key("https://a", Some("k1"), None).await.is_ok());
        let slow = Down::after(Duration::from_secs(3));
        let keys = JwtKeys {
            fetcher: Arc::new(Fetcher {
                http: reqwest::Client::new(),
                transport: Box::new(slow.clone()),
            }),
            ..keys
        };
        expire(&keys, "https://a").await;
        let asked = tokio::time::Instant::now();
        assert!(keys.key("https://a", Some("k1"), None).await.is_ok());
        assert_eq!(asked.elapsed(), Duration::ZERO);
        fetch_ended(&keys, "https://a").await;
        assert_eq!(slow.calls(), 1);
    }

    #[tokio::test]
    async fn new_issuers_of_all_clients_are_asked_up_to_a_limit() {
        let new_issuers = NewIssuers::default();
        let deadline = tokio::time::Instant::now() + FETCH_TIMEOUT;
        let client = |n: u32| -> Scope {
            let address = std::net::Ipv4Addr::from(n).into();
            ("https://*.a".to_string(), Client::of(Some(address)))
        };
        for n in 0..MAX_ASKED_ISSUERS_ALL {
            assert!(new_issuers.reserve(&client(n), deadline).await.is_some());
        }
        let next = client(MAX_ASKED_ISSUERS_ALL);
        assert!(new_issuers.reserve(&next, deadline).await.is_none());
        // The log says so once.
        assert!(new_issuers.log_refusal(&next));
        assert!(!new_issuers.log_refusal(&client(0)));
        // Another pattern has its own.
        let other = scope("https://*.b");
        assert!(new_issuers.reserve(&other, deadline).await.is_some());
    }

    #[test]
    fn an_issuer_that_a_provider_names_is_no_pattern_s() {
        let pattern = JwtProvider::new("tenants", &["https://*.a"]);
        let exact = JwtProvider::new("one", &["https://t.a"]);
        let other = JwtProvider::new("other", &["https://b"]);
        let iss = "https://t.a";
        assert_eq!(issuer_pattern([&pattern, &other], iss), Some("https://*.a"));
        assert_eq!(issuer_pattern([&pattern, &exact], iss), None);
        assert_eq!(issuer_pattern([&exact, &pattern], iss), None);
        assert_eq!(issuer_pattern([&other], iss), None);
    }

    /// Requests for one new issuer that arrive together when one place is
    /// left: one of them reserves it, and any of them may start the fetch.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn one_new_issuer_takes_one_place() {
        for _ in 0..50 {
            let keys = Arc::new(JwtKeys::new(Serves::new(KEY_SET)));
            let places = keys
                .new_issuers
                .window(&scope("https://*.a"), |w| w.places.clone());
            let _taken = (places.clone())
                .acquire_many_owned(MAX_NEW_ISSUERS - 1)
                .await
                .unwrap();
            let mut all = tokio::task::JoinSet::new();
            for _ in 0..8 {
                let keys = keys.clone();
                all.spawn(async move {
                    let scope = scope("https://*.a");
                    let key = keys.key("https://t.a", Some("k1"), Some(&scope));
                    tokio::time::timeout(Duration::from_secs(2), key).await
                });
            }
            for key in all.join_all().await {
                assert!(matches!(key, Ok(Ok(_))), "{key:?}");
            }
            assert_eq!(
                keys.new_issuers.window(&scope("https://*.a"), |w| w.asked),
                1
            );
            assert_eq!(places.available_permits(), 1);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn new_issuers_wait_for_a_place_for_a_limited_time() {
        let new_issuers = NewIssuers::default();
        let places = new_issuers.window(&scope("https://*.a"), |w| w.places.clone());
        let taken = places.acquire_many_owned(MAX_NEW_ISSUERS).await.unwrap();
        let waited = tokio::time::Instant::now();
        let deadline = waited + FETCH_TIMEOUT;
        assert!(
            new_issuers
                .reserve(&scope("https://*.a"), deadline)
                .await
                .is_none()
        );
        assert_eq!(waited.elapsed(), FETCH_TIMEOUT);
        assert_eq!(new_issuers.window(&scope("https://*.a"), |w| w.asked), 0);
        drop(taken);
        // A free place is taken also after the deadline.
        assert!(
            new_issuers
                .reserve(&scope("https://*.a"), deadline)
                .await
                .is_some()
        );
    }

    /// Requests for one issuer that is kept, when no place is free: they
    /// wait together, not one after the other.
    #[tokio::test(start_paused = true)]
    async fn requests_for_one_issuer_wait_for_a_place_together() {
        let keys = Arc::new(JwtKeys::new(Down::default()));
        keys.issuers
            .insert("https://t.a".into(), Arc::default())
            .await;
        let places = keys
            .new_issuers
            .window(&scope("https://*.a"), |w| w.places.clone());
        let _taken = places.acquire_many_owned(MAX_NEW_ISSUERS).await.unwrap();
        let waited = tokio::time::Instant::now();
        let mut all = tokio::task::JoinSet::new();
        for _ in 0..10 {
            let keys = keys.clone();
            all.spawn(async move {
                keys.key("https://t.a", Some("k1"), Some(&scope("https://*.a")))
                    .await
            });
        }
        for key in all.join_all().await {
            assert!(matches!(key, Err(VerifyError::Unavailable(_))), "{key:?}");
        }
        assert_eq!(waited.elapsed(), FETCH_TIMEOUT);
    }

    /// The place of an issuer whose first fetch did not start in the window
    /// of the place: the fetch is counted in the window that it runs in.
    #[tokio::test]
    async fn a_place_of_an_old_window_is_not_used() {
        let keys = JwtKeys::new(Serves::new(KEY_SET));
        let pattern = &scope("https://*.a");
        let deadline = tokio::time::Instant::now() + FETCH_TIMEOUT;
        let place = keys.new_issuers.reserve(pattern, deadline).await.unwrap();
        let entry = Arc::new(IssuerKeys {
            place: std::sync::Mutex::new(Some(place)),
            ..Default::default()
        });
        keys.issuers.insert("https://t.a".into(), entry).await;
        keys.new_issuers.window(pattern, |w| *w = Window::default());
        assert!(
            keys.key("https://t.a", Some("k1"), Some(pattern))
                .await
                .is_ok()
        );
        assert_eq!(keys.new_issuers.window(pattern, |w| w.asked), 1);
    }

    #[tokio::test]
    async fn made_up_issuers_of_one_client_leave_the_others_theirs() {
        let down = Down::no_issuer(Duration::ZERO);
        let keys = JwtKeys::new(down.clone());
        let scope = |peer: &str| ("https://*.a".to_string(), Client::of(peer.parse().ok()));
        let (one, other) = (scope("192.0.2.1"), scope("192.0.2.2"));
        for n in 0..2 * MAX_NEW_ISSUERS {
            let key = (keys.key(&format!("https://t{n}.a"), Some("k1"), Some(&one))).await;
            assert!(key.is_err());
        }
        assert_eq!(down.calls(), MAX_NEW_ISSUERS as usize);
        assert!(
            keys.key("https://real.a", Some("k1"), Some(&other))
                .await
                .is_err()
        );
        assert_eq!(down.calls(), 1 + MAX_NEW_ISSUERS as usize);
    }

    #[tokio::test]
    async fn real_issuers_of_a_pattern_are_asked_up_to_a_limit() {
        let keys = JwtKeys::new(Serves::new(KEY_SET));
        let key = |n: u32| {
            let issuer = format!("https://t{n}.a");
            let keys = &keys;
            async move {
                keys.key(&issuer, Some("k1"), Some(&scope("https://*.a")))
                    .await
            }
        };
        for n in 0..MAX_ASKED_ISSUERS {
            assert!(key(n).await.is_ok(), "issuer {n}");
        }
        let refused = key(MAX_ASKED_ISSUERS).await;
        assert!(matches!(refused, Err(VerifyError::Unavailable(_))));
        // Those that have keys still serve.
        assert!(key(0).await.is_ok());
    }

    #[test]
    fn keys_with_an_unknown_alg_are_kept() {
        let key = |alg: &str| {
            parse_jwk(&serde_json::json!({
                "kty": "EC", "crv": "P-521", "kid": "k1", "alg": alg, "x": "AQ", "y": "AQ",
            }))
        };
        assert_eq!(key("ES512").unwrap().common.key_id.as_deref(), Some("k1"));
        assert!(key("ES384").unwrap().common.key_algorithm.is_some());
        // A key that signs no token that is accepted, and an encryption key.
        assert!(key("ECDH-ES").is_none());
        assert!(
            parse_jwk(&serde_json::json!({
                "kty": "RSA", "use": "enc", "alg": "RSA-OAEP", "n": "AQAB", "e": "AQAB",
            }))
            .is_none()
        );
        assert!(parse_jwk(&serde_json::json!({ "kty": "EC", "crv": "secp256k1" })).is_none());
        assert!(parse_jwk(&serde_json::json!("k1")).is_none());
    }

    /// Requests that arrive together: the issuers over the limit wait for
    /// those asked, and are refused when these had no keys.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn issuers_that_are_not_asked_leave_nothing() {
        let down = Down::no_issuer(Duration::from_millis(20));
        let keys = Arc::new(JwtKeys::new(down.clone()));
        let mut all = tokio::task::JoinSet::new();
        for n in 0..3 * MAX_NEW_ISSUERS {
            let keys = keys.clone();
            all.spawn(async move {
                let issuer = format!("https://t{n}.a");
                keys.key(&issuer, Some("k1"), Some(&scope("https://*.a")))
                    .await
            });
        }
        for key in all.join_all().await {
            assert!(matches!(key, Err(VerifyError::Unavailable(_))));
        }
        assert_eq!(down.calls(), MAX_NEW_ISSUERS as usize);
        keys.unknown.run_pending_tasks().await;
        assert_eq!(keys.unknown.entry_count(), u64::from(MAX_NEW_ISSUERS));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn real_issuers_that_arrive_together_are_all_asked() {
        let keys = Arc::new(JwtKeys::new(Serves::new(KEY_SET)));
        let mut all = tokio::task::JoinSet::new();
        for n in 0..3 * MAX_NEW_ISSUERS {
            let keys = keys.clone();
            all.spawn(async move {
                let issuer = format!("https://t{}.a", n / 2);
                keys.key(&issuer, Some("k1"), Some(&scope("https://*.a")))
                    .await
            });
        }
        for key in all.join_all().await {
            assert!(key.is_ok(), "{:?}", key.err());
        }
        keys.issuers.run_pending_tasks().await;
        assert_eq!(
            keys.issuers.entry_count(),
            u64::from(3 * MAX_NEW_ISSUERS / 2)
        );
    }

    #[tokio::test]
    async fn a_fetch_outlives_the_request_that_started_it() {
        let down = Down::after(Duration::from_millis(200));
        let keys = JwtKeys::new(down.clone());
        let cancelled = tokio::time::timeout(
            Duration::from_millis(20),
            keys.key("https://a", Some("k1"), None),
        );
        assert!(cancelled.await.is_err());
        // The next request waits for that fetch, and takes what it found.
        for _ in 0..2 {
            let key = keys.key("https://a", Some("k1"), None).await;
            assert!(
                matches!(&key, Err(VerifyError::Unavailable(m)) if m.contains("HTTP 500")),
                "{:?}",
                key.err()
            );
        }
        assert_eq!(down.calls(), 1);
    }

    #[tokio::test]
    async fn old_keys_serve_while_a_fetch_runs() {
        let Some(old) = Instant::now().checked_sub(KEYS_TTL + Duration::from_secs(1)) else {
            return;
        };
        let keys = JwtKeys::new(Down::default());
        let issuer = Arc::new(IssuerKeys {
            fetched: RwLock::new(Fetched {
                keys: Some((vec![jwk(Some("k1"))], old)),
                ..Default::default()
            }),
            ..Default::default()
        });
        keys.issuers
            .insert("https://a".into(), issuer.clone())
            .await;
        let _running = issuer.refresh.clone().lock_owned().await;
        assert!(keys.key("https://a", Some("k1"), None).await.is_ok());
    }

    #[test]
    fn times_have_a_leeway() {
        let claims = |exp: i64| -> Map<String, Value> {
            serde_json::from_value(serde_json::json!({ "iss": "i", "exp": now_secs() + exp }))
                .unwrap()
        };
        let leeway = Duration::from_secs(60);
        assert!(check_times(&claims(-30), "i", leeway).is_ok());
        assert!(check_times(&claims(-90), "i", leeway).is_err());
        assert!(check_times(&claims(10), "other", leeway).is_err());
        // A time with a fraction is a time; one that is not a number fails.
        let claims = |exp: Value| -> Map<String, Value> {
            serde_json::from_value(serde_json::json!({ "iss": "i", "exp": exp })).unwrap()
        };
        let past = now_secs() as f64 - 120.5;
        assert!(check_times(&claims(past.into()), "i", leeway).is_err());
        assert!(check_times(&claims((past + 240.0).into()), "i", leeway).is_ok());
        assert!(check_times(&claims("soon".into()), "i", leeway).is_err());
        // A token without `exp` would never expire.
        let no_exp: Map<String, Value> =
            serde_json::from_value(serde_json::json!({ "iss": "i" })).unwrap();
        assert_eq!(
            check_times(&no_exp, "i", leeway).unwrap_err(),
            "MissingRequiredClaim: exp"
        );
    }
}
