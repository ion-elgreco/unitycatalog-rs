//! Identity providers whose tokens the server takes, after upstream
//! `JwksOperations` and `WildcardAllowlist`.
//!
//! Each provider is an `[[auth.issuers]]` block ([`IssuerSettings`]). A
//! token's `iss` must match a block before anything is fetched. Its key
//! comes from the issuer's OIDC discovery document and JWK set, which are
//! cached per issuer and refetched when a key id is unknown (at most once a
//! minute). RS256/384/512 and ES256/384/512 are accepted, with
//! [`LEEWAY`] of clock skew.
//!
//! Every block's tokens are taken at the token exchange
//! (`crate::routes::control`). A block with `direct = true` has them taken
//! on every request too ([`accept`]): the token is checked, the caller
//! named ([`resolve`]) and its groups read, and the result is kept until
//! the token expires, at most [`ACCEPTED_TTL`].

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation,
    jwk::{AlgorithmParameters, EllipticCurve, Jwk},
};
use moka::future::Cache;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, RwLock};

use uc_api::{ScimUser, scim::ScimEmail, scim::schema_uri};
use uc_common::{Error, ErrorCode, Result, now_epoch_millis, settings};

use crate::{
    authn::security::{ADMIN, INTERNAL_ISSUER},
    http::outbound::SendOnIo,
    server::state::AppState,
};

/// The clock skew that the time claims of a provider's tokens may have.
pub const LEEWAY: Duration = Duration::from_secs(60);
/// How long an accepted token of a `direct` issuer is kept, at most: a
/// user disabled in the server keeps working this long with a token that
/// was already accepted.
pub const ACCEPTED_TTL: Duration = Duration::from_secs(5 * 60);

/// Upstream `WildcardAllowlist`: exact entries, and entries whose `*`
/// matches one DNS label (`https://*.dev.example.com`).
#[derive(Debug, Clone, Default)]
pub struct Allowlist {
    exact: HashSet<String>,
    wildcards: Vec<Vec<String>>,
}

impl Allowlist {
    pub fn new(entries: &[impl AsRef<str>]) -> Self {
        let mut list = Self::default();
        for entry in entries
            .iter()
            .map(|e| e.as_ref().trim())
            .filter(|e| !e.is_empty())
        {
            match entry.contains('*') {
                true => list
                    .wildcards
                    .push(entry.split('*').map(str::to_string).collect()),
                false => {
                    list.exact.insert(entry.to_string());
                }
            }
        }
        list
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.wildcards.is_empty()
    }

    pub fn is_allowed(&self, value: &str) -> bool {
        !value.trim().is_empty()
            && (self.exact.contains(value)
                || self.wildcards.iter().any(|parts| matches(parts, value)))
    }

    pub fn is_any_allowed(&self, values: &[String]) -> bool {
        values.iter().any(|v| self.is_allowed(v))
    }
}

/// Whether `value` is `parts` joined by runs of characters other than `.`
/// and `/` (the `[^./]+` upstream puts for each `*`).
fn matches(parts: &[String], value: &str) -> bool {
    let Some((first, rest)) = parts.split_first() else {
        return value.is_empty();
    };
    let Some(after) = value.strip_prefix(first.as_str()) else {
        return false;
    };
    if rest.is_empty() {
        return after.is_empty();
    }
    // One or more label characters, then the remaining parts.
    after
        .char_indices()
        .take_while(|(_, c)| *c != '.' && *c != '/')
        .any(|(i, c)| matches(rest, &after[i + c.len_utf8()..]))
}

/// An `[[auth.issuers]]` block: a provider, the audience of its tokens,
/// and how its tokens name the caller.
#[derive(Debug, Clone)]
pub struct IssuerSettings {
    /// The prefix of the principals it names without an email.
    pub name: String,
    /// The `iss` pattern, as written.
    pub issuer: String,
    issuers: Allowlist,
    /// The audiences, trimmed; `*` alone for any.
    pub audiences: Vec<String>,
    audience_allowlist: Allowlist,
    /// Its tokens are taken on every request, not only at the exchange.
    pub direct: bool,
    /// The claims that name the caller, the first present winning.
    pub principal_claims: Vec<String>,
    /// The claims with a machine's client id, matched to `externalId`.
    pub client_claims: Vec<String>,
    /// The claim with the names of the caller's groups.
    pub groups_claim: Option<String>,
    /// Make the user at its first request when it is missing.
    pub create_users: bool,
    /// The display name of a made user, with claims in braces.
    pub display_name: Option<String>,
}

impl IssuerSettings {
    pub fn new(issuer: &settings::Issuer) -> Self {
        let names = |claims: &[String], default: &[&str]| -> Vec<String> {
            let claims: Vec<String> = claims
                .iter()
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .collect();
            match claims.is_empty() {
                true => default.iter().map(|d| d.to_string()).collect(),
                false => claims,
            }
        };
        let audiences: Vec<String> = issuer
            .audiences
            .iter()
            .map(|a| a.trim())
            .filter(|a| !a.is_empty())
            .map(str::to_string)
            .collect();
        Self {
            name: issuer.name.trim().to_string(),
            issuer: issuer.issuer.trim().to_string(),
            issuers: Allowlist::new(&[issuer.issuer.as_str()]),
            audience_allowlist: Allowlist::new(&audiences),
            audiences,
            direct: issuer.direct,
            principal_claims: names(&issuer.principal_claims, &["email", "sub"]),
            client_claims: names(&issuer.client_claims, &["azp", "client_id"]),
            groups_claim: issuer
                .groups_claim
                .as_deref()
                .map(str::trim)
                .filter(|g| !g.is_empty())
                .map(str::to_string),
            create_users: issuer.create_users,
            display_name: issuer.display_name.clone().filter(|d| !d.trim().is_empty()),
        }
    }

    /// The block of the web UI's provider when no `[[auth.issuers]]` names
    /// it: the defaults, at the sign-in only. The sign-in checks the
    /// audience itself.
    pub fn implicit(issuer: &str) -> Self {
        Self::new(&settings::Issuer {
            name: "oidc".into(),
            issuer: issuer.into(),
            audiences: vec!["*".into()],
            ..Default::default()
        })
    }

    /// Whether `iss` is this block's issuer.
    pub fn matches(&self, iss: &str) -> bool {
        self.issuers.is_allowed(iss)
    }

    fn any_audience(&self) -> bool {
        self.audiences.len() == 1 && self.audiences[0] == "*"
    }

    /// Upstream `validateAudience`: one of the token's `aud` is allowed;
    /// `*` alone allows any.
    pub fn audience_allowed(&self, claims: &Map<String, Value>) -> bool {
        self.any_audience() || self.audience_allowlist.is_any_allowed(&audiences(claims))
    }

    fn validate(
        &self,
        index: usize,
        names: &mut HashSet<String>,
    ) -> std::result::Result<(), String> {
        let invalid = |key: &str, value: &str, reason: &str| {
            format!("Invalid value for auth.issuers[{index}].{key}: '{value}'. {reason}")
        };
        let name_char =
            |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
        if self.name.is_empty() || !self.name.chars().all(name_char) {
            return Err(invalid(
                "name",
                &self.name,
                "Expected a short name of lowercase letters, digits, - and _.",
            ));
        }
        if !names.insert(self.name.clone()) {
            return Err(invalid("name", &self.name, "Another entry has it too."));
        }
        if self.issuer.is_empty() {
            return Err(invalid("issuer", "", "Expected the iss of its tokens."));
        }
        if self.issuer == "*" {
            return Err(invalid(
                "issuer",
                "*",
                "Expected an exact issuer, or `*` for one DNS label, such as \
                 https://*.dev.example.com.",
            ));
        }
        if self.issuer == INTERNAL_ISSUER {
            return Err(invalid(
                "issuer",
                INTERNAL_ISSUER,
                "This is the issuer of the server's own tokens.",
            ));
        }
        if self.audiences.is_empty() {
            return Err(invalid(
                "audiences",
                "",
                "Expected the aud of its tokens, or [\"*\"] for any.",
            ));
        }
        if self.audiences.iter().any(|a| a == "*") && self.audiences.len() > 1 {
            return Err(invalid(
                "audiences",
                &self.audiences.join(", "),
                "'*' cannot be combined with other values; use it alone to accept every audience.",
            ));
        }
        Ok(())
    }
}

/// Token exchange and sign-in settings: `[[auth.issuers]]`,
/// `auth.access_token_timeout` and `auth.cookie_timeout`.
pub struct AuthSettings {
    pub issuers: Vec<IssuerSettings>,
    pub access_token_timeout: Duration,
    pub cookie_timeout: Duration,
    pub providers: IdentityProviders,
    /// The web UI's sign-in (`UC_AUTH_MODE`); none without it.
    pub sign_in: Option<crate::authn::sign_in::SignIn>,
    /// The tokens of `direct` issuers that [`accept`] took, by the hash of
    /// the token.
    accepted: Cache<[u8; 32], Cached>,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self::new(&settings::Auth::default())
    }
}

impl AuthSettings {
    /// The settings of `[auth]`, without the web UI's sign-in.
    pub fn new(auth: &settings::Auth) -> Self {
        Self {
            issuers: auth.issuers.iter().map(IssuerSettings::new).collect(),
            access_token_timeout: auth.access_token_timeout,
            cookie_timeout: auth.cookie_timeout,
            providers: IdentityProviders::default(),
            sign_in: None,
            accepted: accepted_cache(ACCEPTED_TTL),
        }
    }

    /// With the web UI's sign-in.
    pub fn with_sign_in(mut self, sign_in: Option<crate::authn::sign_in::SignIn>) -> Self {
        self.sign_in = sign_in;
        self
    }

    /// Keep accepted tokens at most `ttl` instead of [`ACCEPTED_TTL`]: for
    /// tests.
    pub fn with_accepted_ttl(mut self, ttl: Duration) -> Self {
        self.accepted = accepted_cache(ttl);
        self
    }

    /// The block that `iss` matches, if any.
    pub fn issuer(&self, iss: &str) -> Option<&IssuerSettings> {
        self.issuers.iter().find(|issuer| issuer.matches(iss))
    }

    /// Upstream `validateAuthAllowlistConfiguration`, for each block.
    pub fn validate(&self) -> std::result::Result<(), String> {
        let mut names = HashSet::new();
        for (index, issuer) in self.issuers.iter().enumerate() {
            issuer.validate(index, &mut names)?;
        }
        Ok(())
    }

    /// Upstream `getEffectiveCookieTimeout`: a cookie never outlives the
    /// token in it.
    pub fn effective_cookie_timeout(&self) -> Duration {
        self.cookie_timeout.min(self.access_token_timeout)
    }

    /// Whether the `UC_TOKEN` cookie is `Secure`: as upstream, unless the
    /// web sign-in's public URL is plain http
    /// ([`SignIn::secure_cookies`](crate::authn::sign_in::SignIn::secure_cookies)).
    pub fn secure_cookies(&self) -> bool {
        self.sign_in
            .as_ref()
            .is_none_or(crate::authn::sign_in::SignIn::secure_cookies)
    }
}

fn accepted_cache(ttl: Duration) -> Cache<[u8; 32], Cached> {
    Cache::builder()
        .max_capacity(10_000)
        .time_to_live(ttl.max(Duration::from_millis(1)))
        .build()
}

/// The header and claims of a token, unverified.
pub struct DecodedToken {
    pub algorithm: String,
    pub key_id: Option<String>,
    pub claims: Map<String, Value>,
}

/// Upstream `JWT.decode`: split and decode a token without verifying it.
pub fn decode(token: &str) -> std::result::Result<DecodedToken, String> {
    let mut segments = token.split('.');
    let (Some(header), Some(payload), Some(_), None) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return Err("The token was expected to have 3 parts".into());
    };
    let json = |segment: &str| -> std::result::Result<Map<String, Value>, String> {
        let bytes = URL_SAFE_NO_PAD
            .decode(segment.trim_end_matches('='))
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    };
    let header = json(header)?;
    let claims = json(payload)?;
    Ok(DecodedToken {
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

/// The signing keys of each issuer.
pub struct IdentityProviders {
    http: reqwest::Client,
    issuers: Mutex<HashMap<String, Arc<IssuerKeys>>>,
}

impl Default for IdentityProviders {
    fn default() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            http,
            issuers: Mutex::new(HashMap::new()),
        }
    }
}

struct IssuerKeys {
    keys: RwLock<Option<(Vec<Jwk>, Instant)>>,
    refresh: Mutex<()>,
}

/// Keys are reused this long, and refetched for an unknown key id at most
/// this often.
const KEYS_TTL: Duration = Duration::from_secs(60 * 60);
const MIN_REFRESH: Duration = Duration::from_secs(60);

impl IdentityProviders {
    /// Upstream `verifierForIssuerAndKey` and `verify`: check the token's
    /// signature with the issuer's key, and its issuer and times.
    pub async fn verify(&self, token: &str, decoded: &DecodedToken, issuer: &str) -> Result<()> {
        let key_id = decoded.key_id.clone().unwrap_or_default();
        let jwk = self.key(issuer, &key_id).await?;
        let failed = |e: String| {
            Error::new(
                ErrorCode::Unauthenticated,
                format!("Token verification failed: {e}"),
            )
        };
        let unsupported = |kind: &str, alg: &str| {
            Error::new(ErrorCode::Aborted, format!("Unsupported {kind}: {alg}"))
        };
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
                validation.leeway = LEEWAY.as_secs();
                validation.required_spec_claims.clear();
                jsonwebtoken::decode::<Value>(token, &key, &validation)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            (AlgorithmParameters::EllipticCurve(params), "ES512")
                if params.curve == EllipticCurve::P521 =>
            {
                verify_es512(token, &params.x, &params.y)
                    .and_then(|()| check_times(&decoded.claims, issuer))
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
    async fn key(&self, issuer: &str, key_id: &str) -> Result<Jwk> {
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
            Error::new(
                ErrorCode::Unauthenticated,
                format!("Token verification failed: no signing key '{key_id}' at issuer {issuer}"),
            )
        })
    }

    /// Upstream `loadJwkProvider`: the JWK set the issuer's OIDC discovery
    /// document names.
    async fn fetch_keys(&self, issuer: &str) -> Result<Vec<Jwk>> {
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
            Error::internal(format!("Could not get issuer configuration from {path}"))
        };
        let response = self
            .http
            .get(&path)
            .send_on_io()
            .await
            .map_err(|e| unreachable(&e))?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::internal(format!(
                "Could not get issuer configuration from {path}: HTTP {}",
                status.as_u16()
            )));
        }
        let config: Map<String, Value> = response.json().await.map_err(|_| {
            Error::internal(format!(
                "Issuer configuration from {path} is not valid JSON"
            ))
        })?;
        if config.is_empty() {
            return Err(Error::new(
                ErrorCode::Aborted,
                format!("Could not get issuer configuration from {path}"),
            ));
        }
        let config_issuer = config
            .get("issuer")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if config_issuer != issuer {
            return Err(Error::new(
                ErrorCode::Aborted,
                format!(
                    "Issuer '{issuer}' doesn't match configuration issuer '{config_issuer}' from {path}"
                ),
            ));
        }
        let jwks_uri = config
            .get("jwks_uri")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::new(ErrorCode::Aborted, "JWKS configuration missing"))?;
        let set: Value = self
            .http
            .get(jwks_uri)
            .send_on_io()
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
fn verify_es512(token: &str, x: &str, y: &str) -> std::result::Result<(), String> {
    use p521::ecdsa::{Signature, VerifyingKey, signature::Verifier};
    let (message, signature) = token.rsplit_once('.').ok_or("malformed token")?;
    let coordinate = |c: &str| -> std::result::Result<p521::FieldBytes, String> {
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

/// The issuer and time checks for tokens verified outside `jsonwebtoken`,
/// with [`LEEWAY`].
fn check_times(claims: &Map<String, Value>, issuer: &str) -> std::result::Result<(), String> {
    if claims.get("iss").and_then(Value::as_str) != Some(issuer) {
        return Err("InvalidIssuer".into());
    }
    let now = now_epoch_millis() / 1000;
    let leeway = LEEWAY.as_secs() as i64;
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

/// The token type of the server's access tokens, and of OAuth access
/// tokens as subject tokens; any other subject token type is an ID token.
pub(crate) const ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";

/// A non-blank string claim.
pub(crate) fn claim<'a>(claims: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    claims
        .get(name)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
}

/// Whether the provider marks the token's email as not verified
/// (`email_verified: false`). A provider that lets its users type any
/// address does so until they prove it.
pub(crate) fn email_unverified(claims: &Map<String, Value>) -> bool {
    match claims.get("email_verified") {
        Some(Value::Bool(verified)) => !verified,
        Some(Value::String(text)) => text.trim().eq_ignore_ascii_case("false"),
        _ => false,
    }
}

/// What a provider's token is: an OpenID Connect ID token, which names a
/// person, or an OAuth access token, which may be an application's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    IdToken,
    AccessToken,
}

/// Upstream `TokenExchangeSubjectTokenHandler`, as a walk over the
/// issuer's claims: the active user the token names.
///
/// 1. The first of `principal_claims` present. An email names the user
///    with that email, unless the provider says `email_verified: false`;
///    any other value names the user `<issuer name>:<value>`.
/// 2. Nobody found, and the token is an application's own (an access
///    token with `idtyp = app`, or without an email-like claim): the first
///    of `client_claims` present that is a user's `externalId`. A person's
///    token from `az` or `gcloud` carries that tool's client id, so a
///    person is never matched by it.
/// 3. Nobody found: with `create_users`, the user is made; else
///    "User not allowed".
///
/// A user that exists but is disabled names nobody, and the bootstrap
/// `admin` can never be assumed.
pub(crate) async fn resolve(
    state: &AppState,
    issuer: &IssuerSettings,
    claims: &Map<String, Value>,
    kind: TokenKind,
) -> Result<String> {
    let scim = state.metastore.scim();
    let not_allowed = || Error::invalid_argument("User not allowed");
    // The principal of a user that is missing: (name, whether an email).
    let mut candidate: Option<(String, bool)> = None;
    for name in &issuer.principal_claims {
        let Some(value) = claim(claims, name) else {
            continue;
        };
        if name == "email" && email_unverified(claims) {
            return Err(not_allowed());
        }
        let is_email = value.contains('@');
        let principal = match is_email {
            true => value.to_string(),
            false => format!("{}:{value}", issuer.name),
        };
        match scim.find_user_by_email(&principal).await? {
            Some(user) => return assumable(user).ok_or_else(not_allowed),
            None => {
                candidate = Some((principal, is_email));
                break;
            }
        }
    }
    let app_token = kind == TokenKind::AccessToken
        && (claim(claims, "idtyp").is_some_and(|t| t.eq_ignore_ascii_case("app"))
            || !candidate.as_ref().is_some_and(|(_, is_email)| *is_email));
    if app_token {
        for name in &issuer.client_claims {
            let Some(client) = claim(claims, name) else {
                continue;
            };
            if let Some(user) = scim.find_user_by_external_id(client).await? {
                return assumable(user).ok_or_else(not_allowed);
            }
        }
    }
    match candidate {
        Some((principal, is_email)) if issuer.create_users && principal != ADMIN => {
            create(state, issuer, claims, principal, is_email, app_token).await
        }
        _ => Err(not_allowed()),
    }
}

/// Make the user `principal` at its first request: its email when the
/// principal is one, `externalId` from the first client claim of an
/// application's token, and `displayName` from the issuer's template.
async fn create(
    state: &AppState,
    issuer: &IssuerSettings,
    claims: &Map<String, Value>,
    principal: String,
    is_email: bool,
    app_token: bool,
) -> Result<String> {
    let scim = state.metastore.scim();
    let display_name = issuer
        .display_name
        .as_deref()
        .map(|text| template(text, claims))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| principal.clone());
    let mut external_id = None;
    if app_token
        && let Some(client) = issuer
            .client_claims
            .iter()
            .find_map(|name| claim(claims, name))
        && scim.find_user_by_external_id(client).await?.is_none()
    {
        external_id = Some(client.to_string());
    }
    let user = ScimUser {
        schemas: vec![schema_uri::USER.to_string()],
        user_name: principal.clone(),
        display_name: Some(display_name),
        external_id,
        emails: is_email.then(|| {
            vec![ScimEmail {
                value: principal.clone(),
                primary: Some(true),
                r#type: None,
            }]
        }),
        active: Some(true),
        ..Default::default()
    };
    match scim.create_user(user).await {
        Ok(user) => {
            tracing::info!(%principal, issuer = %issuer.name, "made the user at its first request");
            Ok(user.principal().to_string())
        }
        // Another request, or another replica, made it first.
        Err(e) if e.code().is_already_exists() => scim
            .find_user_by_email(&principal)
            .await?
            .and_then(assumable)
            .ok_or_else(|| Error::invalid_argument("User not allowed")),
        Err(e) => Err(e),
    }
}

/// `text` with each `{claim}` replaced by the claim's text, or by nothing.
fn template(text: &str, claims: &Map<String, Value>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        match rest[start + 1..].find('}') {
            Some(end) => {
                let name = &rest[start + 1..start + 1 + end];
                match claims.get(name) {
                    Some(Value::String(value)) => out.push_str(value),
                    Some(Value::Number(value)) => out.push_str(&value.to_string()),
                    _ => {}
                }
                rest = &rest[start + 1 + end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// The user's principal, if the user is active and not the bootstrap admin.
fn assumable(user: ScimUser) -> Option<String> {
    Some(user)
        .filter(|u| u.is_active() && u.principal() != ADMIN && u.user_name != ADMIN)
        .map(|u| u.principal().to_string())
}

/// The names of the caller's groups: the issuer's `groups_claim`, an
/// array of strings or one string.
pub(crate) fn groups(claims: &Map<String, Value>, issuer: &IssuerSettings) -> Vec<String> {
    let Some(name) = &issuer.groups_claim else {
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

/// The caller that a token of a `direct` issuer names, and its groups.
#[derive(Debug, Clone)]
pub struct Accepted {
    pub principal: String,
    pub groups: Vec<String>,
}

#[derive(Clone)]
struct Cached {
    accepted: Arc<Accepted>,
    expires: Instant,
}

/// Take `token` of the `direct` issuer `issuer` on a request: check it with
/// the issuer's keys and audience, name the caller ([`resolve`]) and read
/// its groups. The result is kept until the token expires, at most
/// [`ACCEPTED_TTL`], so a busy client costs one check and two lookups per
/// token, not per request. A token that does not check out is "Invalid
/// access token." (401), with the reason in the log; a caller that names
/// no active user is "User not allowed" (403).
pub(crate) async fn accept(
    state: &AppState,
    issuer: &IssuerSettings,
    token: &str,
    decoded: &DecodedToken,
) -> Result<Accepted> {
    let invalid = || Error::new(ErrorCode::Unauthenticated, "Invalid access token.");
    let key: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    let cache = &state.auth.accepted;
    if let Some(cached) = cache.get(&key).await
        && Instant::now() < cached.expires
    {
        return Ok((*cached.accepted).clone());
    }
    let claims = &decoded.claims;
    let iss = claim(claims, "iss").unwrap_or_default();
    if let Err(e) = state.auth.providers.verify(token, decoded, iss).await {
        tracing::warn!(issuer = %issuer.name, reason = %e.message(), "token refused");
        return Err(invalid());
    }
    if !issuer.audience_allowed(claims) {
        tracing::warn!(
            issuer = %issuer.name,
            audience = ?audiences(claims),
            "token refused: audience not allowed"
        );
        return Err(invalid());
    }
    let principal = resolve(state, issuer, claims, TokenKind::AccessToken)
        .await
        .map_err(|e| match e.code() {
            ErrorCode::InvalidArgument => {
                Error::new(ErrorCode::PermissionDenied, e.message().to_string())
            }
            _ => e,
        })?;
    let accepted = Accepted {
        principal,
        groups: groups(claims, issuer),
    };
    let now = now_epoch_millis() / 1000;
    let left = claims
        .get("exp")
        .and_then(Value::as_i64)
        .map(|exp| Duration::from_secs((exp - now).max(0) as u64))
        .unwrap_or(ACCEPTED_TTL)
        .min(ACCEPTED_TTL);
    if !left.is_zero() {
        cache
            .insert(
                key,
                Cached {
                    accepted: Arc::new(accepted.clone()),
                    expires: Instant::now() + left,
                },
            )
            .await;
    }
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlists() {
        let list = Allowlist::new(&["https://accounts.example.com", " https://*.dev.example.com"]);
        assert!(list.is_allowed("https://accounts.example.com"));
        assert!(list.is_allowed("https://acme.dev.example.com"));
        assert!(!list.is_allowed("https://a.b.dev.example.com"));
        assert!(!list.is_allowed("https://.dev.example.com"));
        assert!(!list.is_allowed("https://evil.com/x.dev.example.com"));
        assert!(!list.is_allowed(""));
        assert!(Allowlist::new(&[" ", ""]).is_empty());
        assert!(Allowlist::new(&["api://*"]).is_allowed("api://uc"));
    }

    fn issuer(text: &str) -> settings::Issuer {
        settings::Settings::from_toml(&format!("[[auth.issuers]]\n{text}"))
            .unwrap()
            .auth
            .issuers
            .remove(0)
    }

    #[test]
    fn blocks_have_defaults_and_are_checked() {
        let entra = IssuerSettings::new(&issuer(
            r#"
            name = "entra"
            issuer = "https://login.microsoftonline.com/*/v2.0"
            audiences = [" api://uc ", ""]
            "#,
        ));
        assert!(entra.matches("https://login.microsoftonline.com/t1/v2.0"));
        assert!(!entra.matches("https://login.microsoftonline.com/t1/x/v2.0"));
        assert_eq!(entra.audiences, ["api://uc"]);
        assert_eq!(entra.principal_claims, ["email", "sub"]);
        assert_eq!(entra.client_claims, ["azp", "client_id"]);
        assert!(!entra.direct && !entra.create_users);
        let mut claims = Map::new();
        claims.insert("aud".into(), Value::from(vec!["x", "api://uc"]));
        assert!(entra.audience_allowed(&claims));
        claims.insert("aud".into(), Value::from("x"));
        assert!(!entra.audience_allowed(&claims));
        let any = IssuerSettings::new(&issuer(
            "name = \"any\"\nissuer = \"https://x\"\naudiences = [\"*\"]",
        ));
        assert!(any.audience_allowed(&claims));

        let error = |blocks: &[&str]| {
            AuthSettings::new(&settings::Auth {
                issuers: blocks.iter().map(|b| issuer(b)).collect(),
                ..Default::default()
            })
            .validate()
            .unwrap_err()
        };
        let ok = "name = \"a\"\nissuer = \"https://a\"\naudiences = [\"a\"]";
        assert!(
            AuthSettings::new(&settings::Auth {
                issuers: vec![issuer(ok)],
                ..Default::default()
            })
            .validate()
            .is_ok()
        );
        assert!(error(&[ok, ok]).contains("auth.issuers[1].name: 'a'. Another entry"));
        assert!(
            error(&["name = \"A b\"\nissuer = \"https://a\"\naudiences = [\"a\"]"])
                .contains("[0].name")
        );
        assert!(
            error(&["name = \"a\"\nissuer = \"*\"\naudiences = [\"a\"]"])
                .contains("[0].issuer: '*'")
        );
        assert!(
            error(&["name = \"a\"\nissuer = \"internal\"\naudiences = [\"a\"]"])
                .contains("own tokens")
        );
        assert!(
            error(&["name = \"a\"\nissuer = \"https://a\"\naudiences = [\" \"]"])
                .contains("[0].audiences: ''")
        );
        assert!(
            error(&["name = \"a\"\nissuer = \"https://a\"\naudiences = [\"*\", \"a\"]"])
                .contains("cannot be combined")
        );
    }

    #[test]
    fn the_block_of_an_issuer() {
        let settings = AuthSettings::new(&settings::Auth {
            issuers: vec![
                issuer("name = \"a\"\nissuer = \"https://a\"\naudiences = [\"a\"]"),
                issuer("name = \"b\"\nissuer = \"https://*.b\"\naudiences = [\"b\"]"),
            ],
            ..Default::default()
        });
        assert_eq!(settings.issuer("https://a").unwrap().name, "a");
        assert_eq!(settings.issuer("https://x.b").unwrap().name, "b");
        assert!(settings.issuer("https://b").is_none());
        assert!(settings.issuer("").is_none());
    }

    #[test]
    fn templates_and_groups() {
        let claims: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "app_displayname": "ETL", "n": 7, "roles": ["Writers", " Readers ", "", "Writers"],
            "one": "Admins",
        }))
        .unwrap();
        assert_eq!(template("{app_displayname} ({n}){x}", &claims), "ETL (7)");
        assert_eq!(template(" {missing} ", &claims), "");
        assert_eq!(template("{open", &claims), "{open");
        let with = |groups_claim: &str| {
            IssuerSettings::new(&issuer(&format!(
                "name = \"a\"\nissuer = \"https://a\"\naudiences = [\"a\"]\ngroups_claim = \"{groups_claim}\""
            )))
        };
        assert_eq!(groups(&claims, &with("roles")), ["Writers", "Readers"]);
        assert_eq!(groups(&claims, &with("one")), ["Admins"]);
        assert!(groups(&claims, &with("none")).is_empty());
        assert!(groups(&claims, &with(" ")).is_empty());
    }

    #[test]
    fn a_cookie_never_outlives_its_token() {
        let settings = AuthSettings::new(&settings::Auth {
            access_token_timeout: Duration::from_secs(10),
            cookie_timeout: Duration::from_secs(20),
            ..Default::default()
        });
        assert!(settings.validate().is_ok());
        assert_eq!(settings.effective_cookie_timeout(), Duration::from_secs(10));
    }
}
