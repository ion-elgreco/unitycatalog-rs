//! Identity providers whose tokens the token exchange accepts, after
//! upstream `JwksOperations` and `WildcardAllowlist`.
//!
//! A subject token's issuer must be in `auth.allowed_issuers`
//! (`UC_AUTH_ALLOWED_ISSUERS`) before anything is fetched. Its key comes from
//! the issuer's OIDC discovery document and JWK set, which are cached per
//! issuer and refetched when a key id is unknown (at most once a minute).
//! RS256/384/512 and ES256/384/512 are accepted.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation,
    jwk::{AlgorithmParameters, EllipticCurve, Jwk},
};
use serde_json::{Map, Value};
use tokio::sync::{Mutex, RwLock};

use uc_api::ScimUser;
use uc_common::{Error, ErrorCode, Result, now_epoch_millis, settings};

use crate::{authn::security::ADMIN, http::outbound::SendOnIo, server::state::AppState};

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

/// Token exchange and sign-in settings: `auth.allowed_issuers`,
/// `auth.audiences`, `auth.access_token_timeout` and
/// `auth.cookie_timeout`.
pub struct AuthSettings {
    pub allowed_issuers: Allowlist,
    /// The configured audiences; `*` alone disables the check.
    pub audiences: Vec<String>,
    pub audience_allowlist: Allowlist,
    pub access_token_timeout: Duration,
    pub cookie_timeout: Duration,
    pub providers: IdentityProviders,
    /// The web UI's sign-in (`UC_AUTH_MODE`); none without it.
    pub sign_in: Option<crate::authn::sign_in::SignIn>,
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
            allowed_issuers: Allowlist::new(&auth.allowed_issuers),
            audiences: auth
                .audiences
                .iter()
                .map(|a| a.trim())
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect(),
            audience_allowlist: Allowlist::new(&auth.audiences),
            access_token_timeout: auth.access_token_timeout,
            cookie_timeout: auth.cookie_timeout,
            providers: IdentityProviders::default(),
            sign_in: None,
        }
    }

    /// With the web UI's sign-in.
    pub fn with_sign_in(mut self, sign_in: Option<crate::authn::sign_in::SignIn>) -> Self {
        self.sign_in = sign_in;
        self
    }

    /// Upstream `validateAuthAllowlistConfiguration`.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.audiences.iter().any(|a| a == "*") && self.audiences.len() > 1 {
            return Err(
                "auth.audiences cannot combine '*' with other values; use '*' alone to \
                        disable audience validation"
                    .into(),
            );
        }
        if self.allowed_issuers.exact.contains("*") {
            return Err(
                "auth.allowed_issuers cannot be '*'; use explicit issuers or wildcard \
                        patterns such as https://*.dev.example.com"
                    .into(),
            );
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
                validation.leeway = 0;
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
        found.ok_or_else(|| {
            Error::internal(format!(
                "Could not get signing key '{key_id}' for issuer {issuer}"
            ))
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

/// The issuer and time checks for tokens verified outside `jsonwebtoken`.
fn check_times(claims: &Map<String, Value>, issuer: &str) -> std::result::Result<(), String> {
    if claims.get("iss").and_then(Value::as_str) != Some(issuer) {
        return Err("InvalidIssuer".into());
    }
    let now = now_epoch_millis() / 1000;
    if claims
        .get("exp")
        .and_then(Value::as_i64)
        .is_some_and(|exp| exp < now)
    {
        return Err("ExpiredSignature".into());
    }
    if claims
        .get("nbf")
        .and_then(Value::as_i64)
        .is_some_and(|nbf| nbf > now)
    {
        return Err("ImmatureSignature".into());
    }
    Ok(())
}

/// The token type of the server's access tokens, and of OAuth access
/// tokens as subject tokens.
pub(crate) const ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";
/// The subject token type of an OpenID Connect ID token.
pub(crate) const ID_TOKEN: &str = "urn:ietf:params:oauth:token-type:id_token";

/// A non-blank string claim.
fn claim<'a>(claims: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
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

/// Upstream `TokenExchangeSubjectTokenHandler`: the active user the token
/// names. `id_token`s name the user by `email` (or `sub`); an
/// `access_token` without `email` names the OAuth client first (`azp`, then
/// `client_id`, matched to a user's `externalId`). An email that the
/// provider has not verified names no user, or anyone could take a
/// colleague's address. The bootstrap `admin` can never be assumed.
pub(crate) async fn resolve_principal(
    state: &AppState,
    subject_token_type: &str,
    claims: &Map<String, Value>,
) -> Result<String> {
    let by_claims = || async {
        // Upstream reads `email` when present, else `sub`.
        let subject = match claims.contains_key("email") {
            true if email_unverified(claims) => None,
            true => claim(claims, "email"),
            false => claim(claims, "sub"),
        };
        match subject {
            Some(subject) => assumable(state.metastore.scim().find_user_by_email(subject).await?),
            None => Ok(None),
        }
    };
    let by_client = || async {
        match claim(claims, "azp").or_else(|| claim(claims, "client_id")) {
            Some(client) => assumable(
                state
                    .metastore
                    .scim()
                    .find_user_by_external_id(client)
                    .await?,
            ),
            None => Ok(None),
        }
    };
    let found = match subject_token_type == ACCESS_TOKEN {
        true if claim(claims, "email").is_some() => match by_claims().await? {
            Some(user) => Some(user),
            None => by_client().await?,
        },
        true => match by_client().await? {
            Some(user) => Some(user),
            None => by_claims().await?,
        },
        false => by_claims().await?,
    };
    found.ok_or_else(|| Error::invalid_argument("User not allowed"))
}

/// The user's principal, if the user is active and not the bootstrap admin.
fn assumable(user: Option<ScimUser>) -> Result<Option<String>> {
    Ok(user
        .filter(|u| u.is_active() && u.principal() != ADMIN && u.user_name != ADMIN)
        .map(|u| u.principal().to_string()))
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

    #[test]
    fn a_cookie_never_outlives_its_token() {
        let settings = AuthSettings::new(&settings::Auth {
            audiences: vec!["*".into(), "uc".into()],
            access_token_timeout: Duration::from_secs(10),
            cookie_timeout: Duration::from_secs(20),
            ..Default::default()
        });
        assert!(settings.validate().is_err());
        assert_eq!(settings.effective_cookie_timeout(), Duration::from_secs(10));
    }
}
