//! Identity providers whose tokens the server takes, after upstream
//! `JwksOperations` and `WildcardAllowlist`.
//!
//! Each provider is an `[[auth.issuers]]` block ([`IssuerSettings`]). The
//! checks of their tokens are `uc_auth`'s: [`uc_auth::JwtResolver`] and
//! [`uc_auth::AwsResolver`] on requests (blocks with `direct = true`, and
//! AWS), [`uc_auth::JwtKeys`] at the token exchange
//! (`crate::routes::control`) and the web sign-in. This module names the
//! caller that a checked token stands for ([`resolve`]).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Map, Value};
use uc_auth::aws::STS_ENDPOINTS;
use uc_auth::jwt::VerifyError;
use uc_auth::{Authenticator, AwsProvider, JwtKeys, JwtProvider};

use uc_api::{ScimUser, scim::ScimEmail, scim::schema_uri};
use uc_common::{Error, ErrorCode, Result, settings};

pub(crate) use uc_auth::jwt::claim;

use crate::{
    authn::security::{ADMIN, INTERNAL_ISSUER},
    server::state::AppState,
};

/// How long the caller of a token of a `direct` issuer or an AWS identity
/// is kept, at most: a user disabled in the server keeps working this long
/// with a token that was already accepted.
pub const ACCEPTED_TTL: Duration = uc_auth::DEFAULT_KEEP;

/// An `[[auth.issuers]]` block: a provider, the audience of its tokens,
/// and how its tokens name the caller.
#[derive(Debug, Clone)]
pub struct IssuerSettings {
    /// The prefix of the principals it names without an email.
    pub name: String,
    /// What its tokens are.
    pub kind: settings::IssuerKind,
    /// The `iss` pattern, as written; for AWS, the STS endpoint, empty for
    /// AWS's own.
    pub issuer: String,
    /// For AWS: the STS endpoints.
    endpoints: Vec<String>,
    /// The checks of its tokens, on requests and at the exchange.
    jwt: JwtProvider,
    /// The audiences, trimmed; `*` alone for any.
    pub audiences: Vec<String>,
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
    /// For AWS: the accounts whose identities it takes.
    pub accounts: Vec<String>,
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
        let trimmed = |values: &[String]| -> Vec<String> {
            values
                .iter()
                .map(|a| a.trim())
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect()
        };
        let name = issuer.name.trim().to_string();
        let audiences = trimmed(&issuer.audiences);
        let groups_claim = (issuer.groups_claim.as_deref())
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .map(str::to_string);
        let aws = issuer.kind == settings::IssuerKind::Aws;
        // An `iss` is matched exactly, with its trailing `/` if it has one;
        // an STS endpoint is an origin.
        let pattern = match aws {
            true => issuer.issuer.trim().trim_end_matches('/'),
            false => issuer.issuer.trim(),
        };
        let endpoints: Vec<String> = match (aws, pattern.is_empty()) {
            (true, true) => STS_ENDPOINTS.iter().map(|e| e.to_string()).collect(),
            (true, false) => vec![pattern.to_string()],
            (false, _) => Vec::new(),
        };
        let jwt_issuers: &[&str] = match aws {
            true => &[],
            false => &[pattern],
        };
        // An AWS identity is named by its ARN: the role of an assumed role.
        let (principal_claims, client_claims): (&[&str], &[&str]) = match aws {
            true => (&["arn"], &["arn"]),
            false => (&["email", "sub"], &["azp", "client_id"]),
        };
        Self {
            jwt: JwtProvider::new(&name, jwt_issuers)
                .with_audiences(&audiences)
                .with_groups_claim(groups_claim.as_deref()),
            name,
            kind: issuer.kind,
            issuer: pattern.to_string(),
            endpoints,
            audiences,
            // AWS identities are taken on every request; there is no
            // exchange for them.
            direct: issuer.direct || aws,
            principal_claims: names(&issuer.principal_claims, principal_claims),
            client_claims: names(&issuer.client_claims, client_claims),
            groups_claim,
            create_users: issuer.create_users,
            display_name: issuer.display_name.clone().filter(|d| !d.trim().is_empty()),
            accounts: trimmed(&issuer.accounts),
        }
    }

    /// Whether this is a block of `kind = "aws"`.
    pub fn is_aws(&self) -> bool {
        self.kind == settings::IssuerKind::Aws
    }

    /// The provider of `uc_auth`'s JWT checks.
    pub fn jwt_provider(&self) -> &JwtProvider {
        &self.jwt
    }

    /// For AWS: the provider of `uc_auth`'s checks of signed requests, whose
    /// server ids are the block's `audiences`.
    pub fn aws_provider(&self) -> AwsProvider {
        AwsProvider::new(&self.name, self.audiences.iter(), self.accounts.iter())
            .with_endpoints(&self.endpoints)
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

    /// Whether `iss` is this block's issuer: never for AWS, whose tokens
    /// are not JWTs.
    pub fn matches(&self, iss: &str) -> bool {
        self.jwt.matches(iss)
    }

    /// Upstream `validateAudience`: one of the token's `aud` is allowed;
    /// `*` alone allows any.
    pub fn audience_allowed(&self, claims: &Map<String, Value>) -> bool {
        self.jwt.audience_allowed(claims)
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
        if self.is_aws() {
            return self.validate_aws(index);
        }
        if self.issuer.is_empty() {
            return Err(invalid("issuer", "", "Expected the iss of its tokens."));
        }
        if self.issuer.contains('*')
            && let Some(reason) = uc_auth::url_pattern_error(&self.issuer)
        {
            return Err(invalid(
                "issuer",
                &self.issuer,
                &format!(
                    "{reason} Expected an exact issuer, or `*` for one DNS label, such as \
                     https://*.dev.example.com."
                ),
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
        if self.direct && self.audiences.iter().any(|a| a.contains('*')) {
            return Err(invalid(
                "audiences",
                &self.audiences.join(", "),
                "With direct = true, expected the aud of tokens for this server, without '*': \
                 with it, a token that the provider made for another application would sign \
                 in here.",
            ));
        }
        if self.issuer.contains('*')
            && let Some(groups_claim) = &self.groups_claim
        {
            return Err(invalid(
                "groups_claim",
                groups_claim,
                "Not with an issuer with '*': every issuer that it matches, such as each tenant, \
                 would name the groups of the block.",
            ));
        }
        Ok(())
    }

    /// The checks of a block of `kind = "aws"`.
    fn validate_aws(&self, index: usize) -> std::result::Result<(), String> {
        let invalid = |key: &str, value: &str, reason: &str| {
            format!("Invalid value for auth.issuers[{index}].{key}: '{value}'. {reason}")
        };
        if self.audiences.is_empty() || self.audiences.iter().any(|a| a.contains('*')) {
            return Err(invalid(
                "audiences",
                &self.audiences.join(", "),
                "Expected the server id that the signed requests carry, such as the server's \
                 URL; no '*', or a request signed for another server would pass.",
            ));
        }
        if !self.issuer.is_empty() {
            let path = (self.issuer.split_once("://")).is_some_and(|(_, rest)| rest.contains('/'));
            if let Some(reason) = uc_auth::url_pattern_error(&self.issuer)
                .or_else(|| path.then(|| "Expected no path.".to_string()))
            {
                return Err(invalid(
                    "issuer",
                    &self.issuer,
                    &format!(
                        "{reason} Expected the URL of an STS endpoint, such as a VPC endpoint."
                    ),
                ));
            }
        }
        if self.accounts.is_empty() {
            return Err(invalid(
                "accounts",
                "",
                "Expected the AWS accounts whose identities it takes.",
            ));
        }
        if let Some(account) =
            (self.accounts.iter()).find(|a| a.len() != 12 || !a.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(invalid(
                "accounts",
                account,
                "Expected a 12-digit AWS account id.",
            ));
        }
        if self.groups_claim.is_some() {
            return Err(invalid(
                "groups_claim",
                self.groups_claim.as_deref().unwrap_or_default(),
                "AWS identities carry no groups.",
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
    /// The signing keys of the issuers, shared by the requests, the token
    /// exchange and the web sign-in.
    pub keys: Arc<JwtKeys>,
    /// The web UI's sign-in (`UC_AUTH_MODE`); none without it.
    pub sign_in: Option<crate::authn::sign_in::SignIn>,
    /// Who sent a request (`crate::authn::chain`).
    pub authenticator: Authenticator<AppState>,
    /// How long the authenticator keeps a caller by its token.
    keep: Duration,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self::new(&settings::Auth::default())
    }
}

impl AuthSettings {
    /// The settings of `[auth]`, without the web UI's sign-in.
    pub fn new(auth: &settings::Auth) -> Self {
        let issuers: Vec<IssuerSettings> = auth.issuers.iter().map(IssuerSettings::new).collect();
        let keys = Arc::new(JwtKeys::new(crate::authn::chain::IoTransport));
        Self {
            authenticator: crate::authn::chain::authenticator(&issuers, &keys, None, ACCEPTED_TTL),
            issuers,
            access_token_timeout: auth.access_token_timeout,
            cookie_timeout: auth.cookie_timeout,
            keys,
            sign_in: None,
            keep: ACCEPTED_TTL,
        }
    }

    /// With the web UI's sign-in.
    pub fn with_sign_in(mut self, sign_in: Option<crate::authn::sign_in::SignIn>) -> Self {
        self.sign_in = sign_in;
        self.rebuild()
    }

    /// Keep accepted tokens at most `ttl` instead of [`ACCEPTED_TTL`]: for
    /// tests.
    pub fn with_accepted_ttl(mut self, ttl: Duration) -> Self {
        self.keep = ttl;
        self.rebuild()
    }

    fn rebuild(mut self) -> Self {
        let forward = self.sign_in.as_ref().and_then(|s| s.forward());
        self.authenticator =
            crate::authn::chain::authenticator(&self.issuers, &self.keys, forward, self.keep);
        self
    }

    /// The first block that `iss` matches, if any.
    pub fn issuer(&self, iss: &str) -> Option<&IssuerSettings> {
        self.issuers.iter().find(|issuer| issuer.matches(iss))
    }

    /// The pattern with `*` that `iss` matches, for [`JwtKeys::verify`]:
    /// `None` when a block names it exactly.
    pub fn pattern(&self, iss: &str) -> Option<&str> {
        let mut patterns = (self.issuers.iter())
            .filter(|issuer| issuer.matches(iss))
            .map(|issuer| issuer.jwt.pattern(iss));
        let first = patterns.next().flatten()?;
        patterns.all(|p| p.is_some()).then_some(first)
    }

    /// The first block that `iss` matches and that takes the audience of
    /// `claims`, if any: blocks of one issuer can differ by audience.
    pub fn issuer_taking(&self, iss: &str, claims: &Map<String, Value>) -> Option<&IssuerSettings> {
        (self.issuers.iter()).find(|issuer| issuer.matches(iss) && issuer.audience_allowed(claims))
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

/// The error of a provider's token that [`JwtKeys::verify`] refused, as
/// upstream answers it.
pub(crate) fn verify_error(e: VerifyError) -> Error {
    match e {
        VerifyError::Invalid(message) => Error::new(ErrorCode::Unauthenticated, message),
        VerifyError::Unavailable(message) => Error::internal(message),
        other => Error::new(ErrorCode::Aborted, other.to_string()),
    }
}

/// The token type of the server's access tokens, and of OAuth access
/// tokens as subject tokens; any other subject token type is an ID token.
pub(crate) const ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";

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
/// 1. The first of `principal_claims` present. A value that is an email
///    names the user with that email, unless the provider says
///    `email_verified: false`; any other value names the user
///    `<issuer name>:<value>`.
/// 2. Nobody found, and the token is an application's own
///    ([`application`]): the user whose `externalId` is
///    `<issuer name>:<client id>`.
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
        // An AWS identity is named by its ARN, which may hold an `@`.
        let is_email = !issuer.is_aws() && is_email(value);
        if is_email && email_unverified(claims) {
            return Err(not_allowed());
        }
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
    let external_id = application(issuer, claims, kind);
    if let Some(external_id) = &external_id
        && let Some(user) = scim.find_user_by_external_id(external_id).await?
    {
        return assumable(user).ok_or_else(not_allowed);
    }
    match candidate {
        Some((principal, is_email)) if issuer.create_users && principal != ADMIN => {
            create(state, issuer, claims, principal, is_email, external_id).await
        }
        _ => Err(not_allowed()),
    }
}

/// Whether a claim's value is an email: one `@` between a local part and a
/// domain with a dot, and no space, `:` or `/`, which subjects and ARNs
/// hold.
fn is_email(value: &str) -> bool {
    match value.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !domain.contains('@')
                && !value.contains([' ', ':', '/'])
        }
        None => false,
    }
}

/// The `externalId` of an application's user, `<issuer name>:<client id>`
/// with the first of `client_claims` present, when the token is that
/// application's own: an access token with `idtyp = app` (Entra), or whose
/// `sub` is its client id (RFC 9068), or an AWS identity. A person's token
/// carries the client id of the tool or the page that the person signed in
/// with, which many people share, so it is no application's. The name of
/// the issuer keeps the client ids of providers apart.
fn application(
    issuer: &IssuerSettings,
    claims: &Map<String, Value>,
    kind: TokenKind,
) -> Option<String> {
    let client = (issuer.client_claims.iter()).find_map(|name| claim(claims, name))?;
    let own = issuer.is_aws()
        || (kind == TokenKind::AccessToken
            && (claim(claims, "idtyp").is_some_and(|t| t.eq_ignore_ascii_case("app"))
                || claim(claims, "sub") == Some(client)));
    own.then(|| format!("{}:{client}", issuer.name))
}

/// Make the user `principal` at its first request: its email when the
/// principal is one, `external_id` for an application ([`application`]),
/// and `displayName` from the issuer's template.
async fn create(
    state: &AppState,
    issuer: &IssuerSettings,
    claims: &Map<String, Value>,
    principal: String,
    is_email: bool,
    external_id: Option<String>,
) -> Result<String> {
    let scim = state.metastore.scim();
    let display_name = issuer
        .display_name
        .as_deref()
        .map(|text| template(text, claims))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| principal.clone());
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

/// The active user that a request's token names ([`resolve`]): "User not
/// allowed" is 403 on a request, as the caller did sign in.
pub(crate) async fn name_caller(
    state: &AppState,
    issuer: &IssuerSettings,
    claims: &Map<String, Value>,
) -> Result<String> {
    resolve(state, issuer, claims, TokenKind::AccessToken)
        .await
        .map_err(|e| match e.code() {
            ErrorCode::InvalidArgument => {
                Error::new(ErrorCode::PermissionDenied, e.message().to_string())
            }
            _ => e,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            error(&["name = \"a\"\nissuer = \"https://a\"\naudiences = [\"*\"]\ndirect = true"])
                .contains("[0].audiences: '*'. With direct = true")
        );
        assert!(
            error(&[
                "name = \"a\"\nissuer = \"https://a\"\naudiences = [\"api://*\"]\ndirect = true"
            ])
            .contains("[0].audiences: 'api://*'. With direct = true")
        );
        // A `*` stays under one domain.
        for pattern in ["https://*", "http://*", "https://*.com", "accounts.*.com"] {
            let block = format!("name = \"a\"\nissuer = \"{pattern}\"\naudiences = [\"a\"]");
            assert!(error(&[&block]).contains("[0].issuer"), "{pattern}");
        }
        // A pattern's issuers do not name groups.
        assert!(
            error(&[
                "name = \"a\"\nissuer = \"https://login.example.com/*/v2.0\"\n\
                     audiences = [\"a\"]\ngroups_claim = \"roles\""
            ])
            .contains("[0].groups_claim: 'roles'")
        );
        // An STS endpoint is a URL without a path.
        for endpoint in [
            "sts.amazonaws.com",
            "https://sts.amazonaws.com/x",
            "https://*",
        ] {
            let block = format!(
                "name = \"a\"\nkind = \"aws\"\nissuer = \"{endpoint}\"\naudiences = [\"https://uc\"]\n\
                 accounts = [\"123456789012\"]"
            );
            assert!(error(&[&block]).contains("[0].issuer"), "{endpoint}");
        }
    }

    #[test]
    fn emails_and_patterns() {
        for email in ["alice@example.com", "Alice.Smith@contoso.onmicrosoft.com"] {
            assert!(is_email(email), "{email}");
        }
        for other in [
            "alice",
            "abc@clients",
            "arn:aws:iam::1:user/bob@corp.com",
            "a@b@c.com",
            "@example.com",
            "a b@example.com",
            "a@.com",
        ] {
            assert!(!is_email(other), "{other}");
        }
        let settings = AuthSettings::new(&settings::Auth {
            issuers: vec![
                issuer("name = \"a\"\nissuer = \"https://*.a.example\"\naudiences = [\"a\"]"),
                issuer("name = \"b\"\nissuer = \"https://x.a.example\"\naudiences = [\"b\"]"),
            ],
            ..Default::default()
        });
        assert_eq!(
            settings.pattern("https://y.a.example"),
            Some("https://*.a.example")
        );
        // Named exactly by one block: no pattern's.
        assert_eq!(settings.pattern("https://x.a.example"), None);
        assert_eq!(settings.pattern("https://other"), None);
    }

    #[test]
    fn the_block_of_an_issuer() {
        let settings = AuthSettings::new(&settings::Auth {
            issuers: vec![
                issuer("name = \"a\"\nissuer = \"https://a\"\naudiences = [\"a\"]"),
                issuer("name = \"b\"\nissuer = \"https://*.b\"\naudiences = [\"b\"]"),
                issuer("name = \"a2\"\nissuer = \"https://a\"\naudiences = [\"a2\"]"),
                issuer(
                    "name = \"v1\"\nissuer = \"https://sts.windows.net/t/\"\naudiences = [\"a\"]",
                ),
            ],
            ..Default::default()
        });
        assert_eq!(settings.issuer("https://a").unwrap().name, "a");
        assert_eq!(settings.issuer("https://x.b").unwrap().name, "b");
        assert!(settings.issuer("https://b").is_none());
        assert!(settings.issuer("").is_none());
        // An issuer with a trailing `/` is matched as it is.
        assert_eq!(
            settings.issuer("https://sts.windows.net/t/").unwrap().name,
            "v1"
        );
        assert!(settings.issuer("https://sts.windows.net/t").is_none());
        // Blocks of one issuer differ by audience.
        let aud = |aud: &str| -> Map<String, Value> {
            serde_json::from_value(serde_json::json!({ "aud": aud })).unwrap()
        };
        let taking = |aud_: &str| settings.issuer_taking("https://a", &aud(aud_));
        assert_eq!(taking("a").unwrap().name, "a");
        assert_eq!(taking("a2").unwrap().name, "a2");
        assert!(taking("x").is_none());
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
        assert_eq!(
            with("roles").jwt_provider().groups(&claims),
            ["a:Writers", "a:Readers"]
        );
        assert_eq!(with("one").jwt_provider().groups(&claims), ["a:Admins"]);
        assert!(with("none").jwt_provider().groups(&claims).is_empty());
        assert!(with(" ").jwt_provider().groups(&claims).is_empty());
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
