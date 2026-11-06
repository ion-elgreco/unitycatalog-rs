//! The JWTs of OpenID Connect providers.
//!
//! A token's `iss` must match a [`JwtProvider`] before anything is fetched.
//! Its key comes from the issuer's discovery document and JWK set
//! ([`JwtKeys`]), which are cached per issuer and refetched when a key id is
//! unknown, at most once a minute. RS256/384/512 and ES256/384/512 are
//! accepted, with [`DEFAULT_LEEWAY`] of clock skew.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation,
    jwk::{AlgorithmParameters, EllipticCurve, Jwk},
};
use serde_json::{Map, Value};
use tokio::sync::{Mutex, RwLock};

use crate::{
    Allowlist, AuthError, Direct, Identity, RequestContext, Resolution, Resolver, Transport,
};

/// The clock skew that the time claims of a provider's tokens may have.
pub const DEFAULT_LEEWAY: Duration = Duration::from_secs(60);
/// Keys are reused this long, and refetched for an unknown key id at most
/// [`MIN_REFRESH`] often.
const KEYS_TTL: Duration = Duration::from_secs(60 * 60);
const MIN_REFRESH: Duration = Duration::from_secs(60);
/// How long the identity of a token without `exp` may be kept: as long as
/// the [`Authenticator`](crate::Authenticator) keeps any.
const NO_EXPIRY: Duration = Duration::from_secs(24 * 60 * 60);

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
    http: reqwest::Client,
    transport: Box<dyn Transport>,
    leeway: Duration,
    issuers: Mutex<HashMap<String, Arc<IssuerKeys>>>,
}

struct IssuerKeys {
    keys: RwLock<Option<(Vec<Jwk>, Instant)>>,
    refresh: Mutex<()>,
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
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            http,
            transport: Box::new(transport),
            leeway: DEFAULT_LEEWAY,
            issuers: Mutex::new(HashMap::new()),
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
        &self.http
    }

    /// Send `request` through the transport.
    pub async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> reqwest::Result<reqwest::Response> {
        self.transport.send(request).await
    }

    /// Check the token's signature with the issuer's key, and its issuer and
    /// times. The audience is the caller's to check.
    ///
    /// # Errors
    ///
    /// As [`VerifyError`] says.
    pub async fn verify(
        &self,
        token: &str,
        decoded: &UnverifiedJwt,
        issuer: &str,
    ) -> Result<(), VerifyError> {
        let key_id = decoded.key_id.clone().unwrap_or_default();
        let jwk = self.key(issuer, &key_id).await?;
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
                let mut validation = Validation::new(algorithm);
                validation.set_issuer(&[issuer]);
                validation.validate_aud = false;
                validation.validate_nbf = true;
                validation.leeway = self.leeway.as_secs();
                validation.required_spec_claims.clear();
                jsonwebtoken::decode::<Value>(token, &key, &validation)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            (AlgorithmParameters::EllipticCurve(params), "ES512")
                if params.curve == EllipticCurve::P521 =>
            {
                verify_es512(token, &params.x, &params.y)
                    .and_then(|()| check_times(&decoded.claims, issuer, self.leeway))
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
        checked.map_err(failed)
    }

    /// The issuer's key `key_id`, fetched when unknown.
    async fn key(&self, issuer: &str, key_id: &str) -> Result<Jwk, VerifyError> {
        let keys = self
            .issuers
            .lock()
            .await
            .entry(issuer.to_string())
            .or_insert_with(|| {
                Arc::new(IssuerKeys {
                    keys: RwLock::new(None),
                    refresh: Mutex::new(()),
                })
            })
            .clone();
        let find = |keys: &Option<(Vec<Jwk>, Instant)>| {
            keys.as_ref()
                .filter(|(_, at)| at.elapsed() < KEYS_TTL)
                .and_then(|(set, _)| {
                    set.iter()
                        .find(|k| k.common.key_id.as_deref() == Some(key_id))
                        .cloned()
                })
        };
        if let Some(key) = find(&*keys.keys.read().await) {
            return Ok(key);
        }
        // One fetch at a time per issuer; recent fetches are reused.
        let _guard = keys.refresh.lock().await;
        let recent = keys
            .keys
            .read()
            .await
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() < MIN_REFRESH);
        if !recent {
            let set = self.fetch_keys(issuer).await?;
            *keys.keys.write().await = Some((set, Instant::now()));
        }
        let found = find(&*keys.keys.read().await);
        // A key the issuer does not publish, or no longer does: the token
        // is refused, as a tampered one is.
        found.ok_or_else(|| {
            VerifyError::Invalid(format!(
                "Token verification failed: no signing key '{key_id}' at issuer {issuer}"
            ))
        })
    }

    /// The JWK set that the issuer's OIDC discovery document names.
    async fn fetch_keys(&self, issuer: &str) -> Result<Vec<Jwk>, VerifyError> {
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
            VerifyError::Unavailable(format!("Could not get issuer configuration from {path}"))
        };
        let response = self
            .send(self.http.get(&path))
            .await
            .map_err(|e| unreachable(&e))?;
        let status = response.status();
        if !status.is_success() {
            return Err(VerifyError::Unavailable(format!(
                "Could not get issuer configuration from {path}: HTTP {}",
                status.as_u16()
            )));
        }
        let config: Map<String, Value> = response.json().await.map_err(|_| {
            VerifyError::Unavailable(format!(
                "Issuer configuration from {path} is not valid JSON"
            ))
        })?;
        if config.is_empty() {
            return Err(VerifyError::Misconfigured(format!(
                "Could not get issuer configuration from {path}"
            )));
        }
        let config_issuer = config
            .get("issuer")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if config_issuer != issuer {
            return Err(VerifyError::Misconfigured(format!(
                "Issuer '{issuer}' doesn't match configuration issuer '{config_issuer}' from {path}"
            )));
        }
        let jwks_uri = config
            .get("jwks_uri")
            .and_then(Value::as_str)
            .ok_or_else(|| VerifyError::Misconfigured("JWKS configuration missing".into()))?;
        let set: Value = self
            .send(self.http.get(jwks_uri))
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| unreachable(&e))?
            .json()
            .await
            .map_err(|e| unreachable(&e))?;
        // Keys of kinds that do not parse are skipped, not fatal.
        Ok(set["keys"]
            .as_array()
            .map(|keys| {
                keys.iter()
                    .filter_map(|k| serde_json::from_value(k.clone()).ok())
                    .collect()
            })
            .unwrap_or_default())
    }
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

/// The issuer and time checks for tokens verified outside `jsonwebtoken`.
fn check_times(claims: &Map<String, Value>, issuer: &str, leeway: Duration) -> Result<(), String> {
    if claims.get("iss").and_then(Value::as_str) != Some(issuer) {
        return Err("InvalidIssuer".into());
    }
    let now = now_secs();
    let leeway = leeway.as_secs() as i64;
    if claims
        .get("exp")
        .and_then(Value::as_i64)
        .is_some_and(|exp| exp + leeway < now)
    {
        return Err("ExpiredSignature".into());
    }
    if claims
        .get("nbf")
        .and_then(Value::as_i64)
        .is_some_and(|nbf| nbf - leeway > now)
    {
        return Err("ImmatureSignature".into());
    }
    Ok(())
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
    /// or one string.
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

    /// Whether one of the token's `aud` is accepted.
    pub fn audience_allowed(&self, claims: &Map<String, Value>) -> bool {
        let any = self.audiences.len() == 1 && self.audiences[0] == "*";
        any || self.audience_allowlist.is_any_allowed(&audiences(claims))
    }

    /// The names of the caller's groups, each once.
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
            .map(str::to_string)
            .collect()
    }
}

/// Takes the JWTs of its providers, checked with [`JwtKeys`].
///
/// A token that is not a JWT, or whose `iss` is no provider's, passes. A
/// token of a provider that does not check out is refused. The caller is
/// the token's `sub`, with its claims; the identity is kept until the
/// token's `exp`.
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
        let iss = claim(&decoded.claims, "iss").unwrap_or_default();
        let Some(provider) = self.providers.iter().find(|p| p.matches(iss)) else {
            return Resolution::Pass;
        };
        if let Err(e) = self.keys.verify(token, &decoded, iss).await {
            tracing::warn!(provider = %provider.name, reason = %e, "token refused");
            return Resolution::Refused(AuthError::invalid(e.to_string()));
        }
        let claims = decoded.claims;
        if !provider.audience_allowed(&claims) {
            tracing::warn!(
                provider = %provider.name,
                audience = ?audiences(&claims),
                "token refused: audience not allowed"
            );
            return Resolution::Refused(AuthError::invalid("audience not allowed"));
        }
        let left = match claims.get("exp").and_then(Value::as_i64) {
            Some(exp) => Duration::from_secs((exp - now_secs()).max(0) as u64),
            None => NO_EXPIRY,
        };
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
        assert_eq!(provider.groups(&claims), ["Writers", "Readers"]);
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
    }
}
