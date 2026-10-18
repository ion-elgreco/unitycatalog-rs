//! Sign-in to the web UI, in one of two modes; the browser never holds
//! provider settings.
//!
//! `auth.mode = "oidc"`: the server signs users in with an OpenID Connect
//! provider (authorization code flow with PKCE).
//!
//! - `GET /auth/login?rd=<path>` sends the browser to the provider.
//! - `GET /auth/callback` takes the code, checks the ID token, and sets the
//!   `UC_TOKEN` cookie of the token exchange for the user its email names;
//!   then it returns the browser to `rd`.
//! - A sign-in that fails returns the browser to the web UI's login page,
//!   `/login?error=<code>`, which tells the user what failed ([`Failure`]).
//!   The log has the reason.
//!
//! `auth.mode = "forward"`: an auth proxy in front of the server (such as
//! oauth2-proxy or Authelia) signs users in and names the user in its
//! headers on every request. The server takes them from the proxy's
//! addresses only ([`ForwardSignIn`], `crate::authn::middleware`); requests from
//! elsewhere, such as API clients, sign in with their tokens.
//!
//! In both, `GET /auth/logout` expires the `UC_TOKEN` cookie. With `oidc`
//! and `oidc_end_session`, it then ends the provider's session too, at the
//! end-session endpoint of its discovery document (OpenID Connect
//! RP-Initiated Logout), which returns the browser to `<public_url>/`;
//! behind a proxy it goes to the proxy's sign-out.
//!
//! `GET /auth/mode` tells the web UI how it signs in:
//! `{"mode": "oidc", "logout_url": "/auth/logout"}`. `mode` is
//! `auth.mode`, or `off` with authorization off and no sign-in: no one
//! signs in, so the UI asks no one who is signed in. `logout_url` is where
//! "Log out" goes; it is null behind a proxy without `forward_logout_url`,
//! as the proxy would sign the user in again at once, and with `off`; the
//! UI then hides "Log out".
//!
//! The settings are in `[auth]` (`uc_common::settings`), or in the
//! `UC_AUTH_*` variables: `mode` (`none`, `oidc` or `forward`); for `oidc`,
//! `public_url`, `oidc_issuer`, `oidc_client_id`, `oidc_client_secret`,
//! `oidc_scopes` and `oidc_end_session`; for `forward`,
//! `forward_trusted_proxies`, `forward_email_header`, `forward_user_header`
//! and `forward_logout_url`.

use std::net::IpAddr;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uc_common::ErrorCode;
use uc_common::settings::{self, SignInMode};

use crate::{
    authn::idp,
    authn::middleware::{UC_TOKEN_COOKIE, cookie, token_cookie},
    authn::security::SecurityContext,
    http::form_encode,
    http::outbound::SendOnIo,
    server::state::AppState,
};

/// The cookie that carries the signed sign-in state from `/auth/login` to
/// `/auth/callback`.
const LOGIN_COOKIE: &str = "UC_LOGIN";
const CALLBACK: &str = "/auth/callback";
const LOGOUT: &str = "/auth/logout";
/// The web UI's login page, where a failed sign-in returns.
const LOGIN_PAGE: &str = "/login";
/// How long a sign-in at the provider may take.
const LOGIN_TTL: Duration = Duration::from_secs(10 * 60);
const DEFAULT_SCOPES: &str = "openid email profile";

/// The sign-in of `auth.mode`; `none` is no sign-in.
pub enum SignIn {
    Oidc(OidcSignIn),
    Forward(ForwardSignIn),
}

impl SignIn {
    /// The sign-in of `[auth]`; `None` for `mode = "none"`.
    pub fn new(auth: &settings::Auth) -> Result<Option<Self>, String> {
        match auth.mode {
            SignInMode::None => Ok(None),
            SignInMode::Oidc => OidcSignIn::new(auth).map(|o| Some(Self::Oidc(o))),
            SignInMode::Forward => ForwardSignIn::new(auth).map(|f| Some(Self::Forward(f))),
        }
    }

    pub fn forward(&self) -> Option<&ForwardSignIn> {
        match self {
            Self::Forward(forward) => Some(forward),
            Self::Oidc(_) => None,
        }
    }

    /// Whether the `UC_TOKEN` cookie is `Secure`: as upstream, unless the
    /// public URL is plain http, whose pages browsers give no `Secure`
    /// cookie to.
    pub fn secure_cookies(&self) -> bool {
        match self {
            Self::Oidc(oidc) => oidc.secure(),
            Self::Forward(_) => true,
        }
    }
}

/// The OpenID Connect provider of `auth.mode = "oidc"`.
pub struct OidcSignIn {
    /// The issuer, as the ID tokens name it; its discovery document has the
    /// endpoints.
    pub issuer: String,
    pub client_id: String,
    /// Without one, the server signs in as a public client (PKCE only).
    pub client_secret: Option<String>,
    /// The server's external URL, without a trailing `/`: the provider
    /// returns the browser to `<public_url>/auth/callback`. Never taken from
    /// the request's `Host`.
    pub public_url: String,
    pub scopes: String,
    /// Whether "Log out" ends the provider's session too.
    pub end_session: bool,
    endpoints: tokio::sync::OnceCell<Endpoints>,
    http: reqwest::Client,
}

/// The endpoints of the provider's discovery document.
struct Endpoints {
    authorization: String,
    token: String,
    /// Where "Log out" ends the provider's session; not every provider has
    /// one.
    end_session: Option<String>,
}

impl OidcSignIn {
    fn new(auth: &settings::Auth) -> Result<Self, String> {
        let required = |value: &Option<String>, key: &str| {
            set(value).ok_or_else(|| {
                format!(
                    "{} is required with auth.mode = \"oidc\"",
                    settings::name(key)
                )
            })
        };
        let public_url = required(&auth.public_url, "auth.public_url")?
            .trim_end_matches('/')
            .to_string();
        if !(public_url.starts_with("https://") || public_url.starts_with("http://")) {
            return Err(settings::invalid(
                "auth.public_url",
                &public_url,
                "Expected an http or https URL.",
            ));
        }
        Ok(Self {
            issuer: required(&auth.oidc_issuer, "auth.oidc_issuer")?,
            client_id: required(&auth.oidc_client_id, "auth.oidc_client_id")?,
            client_secret: set(&auth.oidc_client_secret),
            public_url,
            scopes: set(&auth.oidc_scopes).unwrap_or_else(|| DEFAULT_SCOPES.into()),
            end_session: auth.oidc_end_session,
            endpoints: tokio::sync::OnceCell::new(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                // The token endpoint answers; it does not redirect.
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default(),
        })
    }

    fn redirect_uri(&self) -> String {
        format!("{}{CALLBACK}", self.public_url)
    }

    /// Cookies are `Secure` when the public URL is https.
    fn secure(&self) -> bool {
        self.public_url.starts_with("https://")
    }

    /// The provider's endpoints, from its discovery document: fetched at
    /// the first sign-in (so that a provider that is down does not stop the
    /// server), then kept; a failure is tried again at the next sign-in.
    async fn endpoints(&self) -> Result<&Endpoints, String> {
        self.endpoints
            .get_or_try_init(|| async {
                let url = format!(
                    "{}/.well-known/openid-configuration",
                    self.issuer.trim_end_matches('/')
                );
                let config: Value = self
                    .http
                    .get(&url)
                    .send_on_io()
                    .await
                    .and_then(|r| r.error_for_status())
                    .map_err(|e| format!("{url}: {e}"))?
                    .json()
                    .await
                    .map_err(|e| format!("{url}: {e}"))?;
                if config["issuer"].as_str() != Some(self.issuer.as_str()) {
                    return Err(format!(
                        "{url} names the issuer {}, not {}",
                        config["issuer"], self.issuer
                    ));
                }
                let endpoint = |name: &str| {
                    config[name]
                        .as_str()
                        .map(str::to_string)
                        .ok_or_else(|| format!("{url} has no {name}"))
                };
                Ok(Endpoints {
                    authorization: endpoint("authorization_endpoint")?,
                    token: endpoint("token_endpoint")?,
                    end_session: endpoint("end_session_endpoint").ok(),
                })
            })
            .await
    }

    /// Where `/auth/logout` sends the browser to end the provider's session:
    /// its end-session endpoint, which returns the browser to
    /// `<public_url>/`. The request names the client, not the session: the
    /// server keeps no ID token for `id_token_hint`, and some providers
    /// refuse an expired one. None without the endpoint, or when the
    /// discovery document cannot be read.
    async fn end_session_url(&self) -> Option<String> {
        let endpoints = match self.endpoints().await {
            Ok(endpoints) => endpoints,
            Err(e) => {
                tracing::warn!(reason = %e, "cannot end the provider's session at logout");
                return None;
            }
        };
        let end_session = endpoints.end_session.as_deref()?;
        let query = serde_urlencoded::to_string([
            ("client_id", self.client_id.as_str()),
            ("post_logout_redirect_uri", &format!("{}/", self.public_url)),
        ])
        .unwrap_or_default();
        Some(with_query(end_session, &query))
    }

    /// The ID token for `code`, from the token endpoint.
    async fn exchange(&self, code: &str, verifier: &str) -> Result<String, String> {
        let endpoints = self.endpoints().await?;
        let redirect_uri = self.redirect_uri();
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &self.client_id),
            ("code_verifier", verifier),
        ];
        let mut request = self.http.post(&endpoints.token).form(&form);
        // `client_secret_basic`, the default method; RFC 6749 §2.3.1 encodes
        // both parts first.
        if let Some(secret) = &self.client_secret {
            request = request.basic_auth(form_encode(&self.client_id), Some(form_encode(secret)));
        }
        let response = request.send_on_io().await.map_err(|e| e.to_string())?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "HTTP {}: {} {}",
                status.as_u16(),
                body["error"].as_str().unwrap_or_default(),
                body["error_description"].as_str().unwrap_or_default()
            ));
        }
        body["id_token"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "the answer has no id_token".into())
    }
}

/// The auth proxy of `auth.mode = "forward"`: it signs users in, and names
/// the user in its headers on every request.
pub struct ForwardSignIn {
    /// The proxies whose headers count, by the address of the connection,
    /// never by `X-Forwarded-For`.
    pub trusted_proxies: Vec<ipnet::IpNet>,
    /// The header with the user's email (default `Remote-Email`).
    pub email_header: String,
    /// The header with the user, without the email header (default
    /// `Remote-User`).
    pub user_header: String,
    /// The proxy's sign-out, where `/auth/logout` sends the browser.
    pub logout_url: Option<String>,
}

impl ForwardSignIn {
    fn new(auth: &settings::Auth) -> Result<Self, String> {
        let trusted_proxies = auth
            .forward_trusted_proxies
            .iter()
            .map(|proxy| parse_cidr(proxy.trim()))
            .collect::<Result<Vec<_>, _>>()?;
        if trusted_proxies.is_empty() {
            return Err(format!(
                "{} is required with auth.mode = \"forward\": the proxies' CIDRs or \
                 addresses (0.0.0.0/0 only on purpose)",
                settings::name("auth.forward_trusted_proxies")
            ));
        }
        Ok(Self {
            trusted_proxies,
            email_header: set(&auth.forward_email_header).unwrap_or_else(|| "Remote-Email".into()),
            user_header: set(&auth.forward_user_header).unwrap_or_else(|| "Remote-User".into()),
            logout_url: set(&auth.forward_logout_url),
        })
    }

    /// Whether `peer`, the address of the connection, is a trusted proxy. An
    /// IPv4 peer of a dual-stack listener shows as `::ffff:a.b.c.d`.
    pub fn trusts(&self, peer: IpAddr) -> bool {
        let v4 = match peer {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4),
            IpAddr::V4(_) => None,
        };
        self.trusted_proxies
            .iter()
            .any(|net| net.contains(&peer) || v4.is_some_and(|v4| net.contains(&v4)))
    }

    /// The user the proxy names: its email header, else its user header.
    pub fn user(&self, headers: &HeaderMap) -> Option<String> {
        let get = |name: &str| {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        };
        get(&self.email_header).or_else(|| get(&self.user_header))
    }
}

/// A CIDR, or an address as a network of one.
fn parse_cidr(text: &str) -> Result<ipnet::IpNet, String> {
    text.parse::<ipnet::IpNet>()
        .or_else(|_| text.parse::<IpAddr>().map(ipnet::IpNet::from))
        .map_err(|_| {
            settings::invalid(
                "auth.forward_trusted_proxies",
                text,
                "Expected a CIDR or an address.",
            )
        })
}

/// The text of a setting, trimmed; none when empty.
fn set(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// 32 random bytes, base64url: a state, a nonce or a PKCE verifier.
fn random() -> String {
    URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

/// RFC 7636 `S256`: the challenge of a PKCE verifier.
fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// `url` with `query` added to its query, if any.
fn with_query(url: &str, query: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{query}")
}

/// A path of this server to return to: `/…`, never another host (`//…`,
/// `/\…`) or control characters; else `/`.
fn local_path(rd: Option<&str>) -> String {
    match rd {
        Some(path)
            if path.starts_with('/')
                && !path.starts_with("//")
                && !path.contains('\\')
                && !path.chars().any(char::is_control) =>
        {
            path.to_string()
        }
        _ => "/".into(),
    }
}

/// What `/auth/login` hands to `/auth/callback`, signed by the server.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LoginState {
    pub state: String,
    pub nonce: String,
    pub verifier: String,
    /// The path to return to.
    pub rd: String,
}

fn login_cookie(oidc: &OidcSignIn, value: &str, max_age: Duration) -> Option<HeaderValue> {
    // `Lax`: the provider's redirect to the callback is a cross-site
    // navigation, which does not carry `Strict` cookies.
    let secure = if oidc.secure() { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{LOGIN_COOKIE}={value}; Max-Age={}; Path={CALLBACK}; HttpOnly; SameSite=Lax{secure}",
        max_age.as_secs()
    ))
    .ok()
}

/// Why a sign-in failed: the `error` of `/login?error=<code>`, which the
/// web UI shows. The log has the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// `provider`: the provider answered with an error or without a code,
    /// or its discovery document cannot be read.
    Provider,
    /// `expired`: no valid `UC_LOGIN` cookie (the sign-in took too long, or
    /// it did not start here), or the state does not match.
    Expired,
    /// `token`: the provider did not take the code, or the ID token is
    /// invalid, is for another client, or has another nonce.
    Token,
    /// `user`: no active user has the ID token's email.
    User,
    /// `server`: the server failed.
    Server,
}

impl Failure {
    /// The `<code>` of `/login?error=<code>`.
    pub fn code(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Expired => "expired",
            Self::Token => "token",
            Self::User => "user",
            Self::Server => "server",
        }
    }
}

/// Log why the sign-in failed, and send the browser back to the web UI's
/// login page, which tells the user.
fn back_to_login(failure: Failure, reason: &str) -> Response {
    tracing::warn!(error = failure.code(), %reason, "sign-in failed");
    Redirect::to(&format!("{LOGIN_PAGE}?error={}", failure.code())).into_response()
}

/// A plain-text page for a sign-in that cannot go on.
fn page(status: StatusCode, text: impl Into<String>) -> Response {
    (status, text.into()).into_response()
}

fn not_configured() -> Response {
    page(
        StatusCode::NOT_FOUND,
        "Sign-in to the web UI is not configured: set auth.mode = \"oidc\" \
         (UC_AUTH_MODE) and the oidc_* settings of [auth] on the server.",
    )
}

/// Behind an auth proxy, the proxy signs users in, not this server.
fn through_the_proxy(forward: &ForwardSignIn) -> Response {
    page(
        StatusCode::NOT_FOUND,
        format!(
            "This server signs users in through the auth proxy in front of it \
             (auth.mode = \"forward\"), which names the user in {} or {}. This request came \
             without them: open the UI through the proxy.",
            forward.email_header, forward.user_header
        ),
    )
}

/// The OpenID Connect sign-in and the signing keys.
fn oidc_sign_in(state: &AppState) -> Option<(&OidcSignIn, &SecurityContext)> {
    match (state.auth.sign_in.as_ref(), state.security.as_deref()) {
        (Some(SignIn::Oidc(oidc)), Some(security)) => Some((oidc, security)),
        _ => None,
    }
}

/// The page that says why there is no OpenID Connect sign-in.
fn no_oidc_sign_in(state: &AppState) -> Response {
    match state.auth.sign_in.as_ref().and_then(SignIn::forward) {
        Some(forward) => through_the_proxy(forward),
        None => not_configured(),
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct LoginParams {
    pub rd: Option<String>,
}

/// `GET /auth/login`: to the provider, with a new state, nonce and PKCE
/// verifier in the signed `UC_LOGIN` cookie.
pub async fn login(State(state): State<AppState>, Query(params): Query<LoginParams>) -> Response {
    let Some((oidc, security)) = oidc_sign_in(&state) else {
        return no_oidc_sign_in(&state);
    };
    let endpoints = match oidc.endpoints().await {
        Ok(endpoints) => endpoints,
        Err(e) => {
            let reason = format!("cannot read the provider's discovery document: {e}");
            return back_to_login(Failure::Provider, &reason);
        }
    };
    let login = LoginState {
        state: random(),
        nonce: random(),
        verifier: random(),
        rd: local_path(params.rd.as_deref()),
    };
    let signed = match security.sign_login_state(&login, LOGIN_TTL) {
        Ok(signed) => signed,
        Err(e) => return back_to_login(Failure::Server, e.message()),
    };
    let query = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", &oidc.client_id),
        ("redirect_uri", &oidc.redirect_uri()),
        ("scope", &oidc.scopes),
        ("state", &login.state),
        ("nonce", &login.nonce),
        ("code_challenge", &challenge(&login.verifier)),
        ("code_challenge_method", "S256"),
    ])
    .unwrap_or_default();
    let mut response = Redirect::to(&with_query(&endpoints.authorization, &query)).into_response();
    if let Some(cookie) = login_cookie(oidc, &signed, LOGIN_TTL) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

#[derive(Debug, Default, Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// `GET /auth/callback`: the code for an ID token, the ID token for the
/// `UC_TOKEN` cookie of the user it names; then back to `rd`.
pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    headers: HeaderMap,
) -> Response {
    let Some((oidc, security)) = oidc_sign_in(&state) else {
        return no_oidc_sign_in(&state);
    };
    let signed = cookie(&headers, LOGIN_COOKIE);
    // Back to the login page; the `UC_LOGIN` cookie, if any, expires.
    let failed = |failure: Failure, reason: String| {
        let mut response = back_to_login(failure, &reason);
        if signed.is_some()
            && let Some(cleared) = login_cookie(oidc, "", Duration::ZERO)
        {
            response.headers_mut().insert(header::SET_COOKIE, cleared);
        }
        response
    };
    if let Some(error) = params.error {
        let description = params.error_description.unwrap_or_default();
        return failed(
            Failure::Provider,
            format!("the identity provider answered {error} {description}"),
        );
    }
    let login = signed
        .as_deref()
        .and_then(|signed| security.verify_login_state::<LoginState>(signed));
    let Some(login) = login else {
        return failed(
            Failure::Expired,
            "no valid sign-in state: it took too long, or it did not start here".into(),
        );
    };
    if params.state.as_deref() != Some(login.state.as_str()) {
        return failed(Failure::Expired, "the state does not match".into());
    }
    let Some(code) = params.code.filter(|c| !c.is_empty()) else {
        return failed(
            Failure::Provider,
            "the identity provider answered without a code".into(),
        );
    };
    let id_token = match oidc.exchange(&code, &login.verifier).await {
        Ok(id_token) => id_token,
        Err(e) => {
            return failed(
                Failure::Token,
                format!("the identity provider did not accept the code: {e}"),
            );
        }
    };

    // The ID token: signed by the issuer, current, for this client and
    // this sign-in (OpenID Connect Core §3.1.3.7).
    let decoded = match idp::decode(&id_token) {
        Ok(decoded) => decoded,
        Err(e) => return failed(Failure::Token, format!("invalid ID token: {e}")),
    };
    if let Err(e) = state
        .auth
        .providers
        .verify(&id_token, &decoded, &oidc.issuer)
        .await
    {
        return failed(Failure::Token, e.message().to_string());
    }
    let claims = &decoded.claims;
    let azp = claims.get("azp").and_then(Value::as_str);
    if !idp::audiences(claims).contains(&oidc.client_id)
        || azp.is_some_and(|azp| azp != oidc.client_id)
    {
        return failed(Failure::Token, "the ID token is for another client".into());
    }
    if claims.get("nonce").and_then(Value::as_str) != Some(login.nonce.as_str()) {
        return failed(Failure::Token, "the nonce does not match".into());
    }

    // The user that the token exchange would sign in.
    let principal = match idp::resolve_principal(&state, idp::ID_TOKEN, claims).await {
        Ok(principal) => principal,
        Err(e) if e.code() == ErrorCode::InvalidArgument => {
            let email = claims
                .get("email")
                .or_else(|| claims.get("sub"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let reason = match idp::email_unverified(claims) {
                true => format!("the identity provider has not verified the email {email}"),
                false => format!(
                    "no active user has the email {email}. An admin adds users (SCIM, `uc user create`)."
                ),
            };
            return failed(Failure::User, reason);
        }
        Err(e) => return failed(Failure::Server, format!("cannot find the user: {e}")),
    };
    let settings = &state.auth;
    let token = match security.create_access_token(&principal, settings.access_token_timeout) {
        Ok(token) => token,
        Err(e) => return failed(Failure::Server, e.message().to_string()),
    };
    tracing::info!(%principal, "signed in to the web UI");
    let mut response = Redirect::to(&login.rd).into_response();
    let cookies = response.headers_mut();
    if let Ok(cookie) = token_cookie(
        &token,
        settings.effective_cookie_timeout(),
        settings.secure_cookies(),
    ) {
        cookies.append(header::SET_COOKIE, cookie);
    }
    if let Some(cleared) = login_cookie(oidc, "", Duration::ZERO) {
        cookies.append(header::SET_COOKIE, cleared);
    }
    response
}

/// `GET /auth/logout`: expire the `UC_TOKEN` cookie, then back to `/`, on to
/// the end of the provider's session (`oidc_end_session`), or to the auth
/// proxy's sign-out.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let signed_in = cookie(&headers, UC_TOKEN_COOKIE).is_some();
    let to = match state.auth.sign_in.as_ref() {
        // Only with the cookie, which is `SameSite=Strict`: a link on
        // another site does not end the provider's session.
        Some(SignIn::Oidc(oidc)) if oidc.end_session && signed_in => oidc.end_session_url().await,
        Some(SignIn::Forward(forward)) => forward.logout_url.clone(),
        _ => None,
    };
    let mut response = Redirect::to(to.as_deref().unwrap_or("/")).into_response();
    if signed_in && let Ok(cleared) = token_cookie("", Duration::ZERO, state.auth.secure_cookies())
    {
        response.headers_mut().insert(header::SET_COOKIE, cleared);
    }
    response
}

/// `GET /auth/mode`: how the web UI signs in (`auth.mode`, or `off`), and
/// where its "Log out" goes; null hides it. It needs no sign-in.
pub async fn mode(State(state): State<AppState>) -> Response {
    let (mode, logout_url) = match state.auth.sign_in.as_ref() {
        Some(SignIn::Oidc(_)) => ("oidc", Some(LOGOUT)),
        // Without the proxy's sign-out, the proxy signs the user in again
        // at once.
        Some(SignIn::Forward(forward)) => ("forward", forward.logout_url.as_ref().map(|_| LOGOUT)),
        // No one signs in, and everyone may do everything.
        None if !state.authorization_enabled => ("off", None),
        // It expires the `UC_TOKEN` cookie of an earlier sign-in.
        None => ("none", Some(LOGOUT)),
    };
    let body = Json(serde_json::json!({ "mode": mode, "logout_url": logout_url }));
    ([(header::CACHE_CONTROL, "no-store")], body).into_response()
}

/// The routes of the web sign-in, outside the API.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/mode", get(mode))
        .route("/auth/login", get(login))
        .route(CALLBACK, get(callback))
        .route(LOGOUT, get(logout))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sign-in of the `[auth]` table with the lines `auth`.
    fn sign_in(auth: &str) -> Result<Option<SignIn>, String> {
        let settings = settings::Settings::from_toml(&format!("[auth]\n{auth}"))?;
        SignIn::new(&settings.auth)
    }

    fn settings(auth: &str) -> Result<Option<OidcSignIn>, String> {
        sign_in(auth).map(|s| {
            s.map(|s| match s {
                SignIn::Oidc(oidc) => oidc,
                SignIn::Forward(_) => panic!("not oidc"),
            })
        })
    }

    fn forward(auth: &str) -> Result<ForwardSignIn, String> {
        sign_in(&format!("mode = \"forward\"\n{auth}")).map(|s| match s {
            Some(SignIn::Forward(forward)) => forward,
            _ => panic!("not forward"),
        })
    }

    #[test]
    fn reads_the_forward_settings() {
        let missing = forward("").err().unwrap();
        assert!(missing.starts_with(
            "auth.forward_trusted_proxies (UC_AUTH_FORWARD_TRUSTED_PROXIES) is required"
        ));
        assert!(
            forward(r#"forward_trusted_proxies = "10.0.0.0/8, proxy""#)
                .err()
                .unwrap()
                .contains("'proxy'")
        );
        let proxy =
            forward(r#"forward_trusted_proxies = ["10.42.0.0/16", "127.0.0.1", "::1"]"#).unwrap();
        assert_eq!(proxy.trusted_proxies.len(), 3);
        assert_eq!(
            (proxy.email_header.as_str(), proxy.user_header.as_str()),
            ("Remote-Email", "Remote-User")
        );
        assert!(proxy.logout_url.is_none());
        let custom = forward(
            r#"
            forward_trusted_proxies = "10.0.0.1"
            forward_email_header = "X-Forwarded-Email"
            forward_user_header = "X-Forwarded-User"
            forward_logout_url = "/oauth2/sign_out"
            "#,
        )
        .unwrap();
        assert_eq!(custom.email_header, "X-Forwarded-Email");
        assert_eq!(custom.logout_url.as_deref(), Some("/oauth2/sign_out"));
    }

    #[test]
    fn trusts_only_the_proxies() {
        let proxy = forward(r#"forward_trusted_proxies = "10.42.0.0/16,127.0.0.1""#).unwrap();
        let ip = |text: &str| text.parse::<IpAddr>().unwrap();
        assert!(proxy.trusts(ip("10.42.7.1")));
        assert!(proxy.trusts(ip("127.0.0.1")));
        // IPv4 on a dual-stack listener.
        assert!(proxy.trusts(ip("::ffff:10.42.7.1")));
        assert!(!proxy.trusts(ip("10.43.0.1")));
        assert!(!proxy.trusts(ip("127.0.0.2")));
        assert!(!proxy.trusts(ip("::1")));
    }

    #[test]
    fn the_user_is_the_email_header_else_the_user_header() {
        let proxy = forward(r#"forward_trusted_proxies = "127.0.0.1""#).unwrap();
        let headers = |pairs: &[(&'static str, &'static str)]| {
            let mut map = HeaderMap::new();
            for (name, value) in pairs {
                map.insert(*name, HeaderValue::from_static(value));
            }
            map
        };
        assert_eq!(proxy.user(&headers(&[])), None);
        assert_eq!(proxy.user(&headers(&[("remote-email", " ")])), None);
        assert_eq!(
            proxy
                .user(&headers(&[("remote-user", "alice@example.com")]))
                .as_deref(),
            Some("alice@example.com")
        );
        assert_eq!(
            proxy
                .user(&headers(&[
                    ("remote-user", "alice"),
                    ("remote-email", "alice@example.com")
                ]))
                .as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn reads_the_auth_settings() {
        assert!(settings("").unwrap().is_none());
        assert!(settings(r#"mode = "none""#).unwrap().is_none());
        assert!(
            settings(r#"mode = "saml""#)
                .err()
                .unwrap()
                .contains("unknown variant")
        );
        let missing = settings(r#"mode = "oidc""#).err().unwrap();
        assert_eq!(
            missing,
            "auth.public_url (UC_AUTH_PUBLIC_URL) is required with auth.mode = \"oidc\""
        );
        let all = |public_url: &str| {
            format!(
                "mode = \"oidc\"\npublic_url = \"{public_url}\"\n\
                 oidc_issuer = \"https://idp.example.com\"\noidc_client_id = \"uc\""
            )
        };
        let oidc = settings(&all("https://uc.example.com/")).unwrap().unwrap();
        assert_eq!(oidc.redirect_uri(), "https://uc.example.com/auth/callback");
        assert_eq!(oidc.scopes, DEFAULT_SCOPES);
        assert!(oidc.client_secret.is_none() && oidc.secure());
        assert!(!oidc.end_session);
        let end_session = format!("{}\noidc_end_session = true", all("https://uc.example.com"));
        assert!(settings(&end_session).unwrap().unwrap().end_session);
        assert!(settings(&all("uc.example.com")).is_err());
        // Browsers keep no `Secure` cookie from a plain-http page.
        let secure_cookies =
            |public_url: &str| sign_in(&all(public_url)).unwrap().unwrap().secure_cookies();
        assert!(secure_cookies("https://uc.example.com"));
        assert!(!secure_cookies("http://uc.lan:8080"));
        let http = token_cookie("t", Duration::from_secs(60), false).unwrap();
        assert_eq!(
            http,
            "UC_TOKEN=t; Max-Age=60; Path=/; HttpOnly; SameSite=Strict"
        );
    }

    #[test]
    fn returns_only_to_paths_of_this_server() {
        assert_eq!(local_path(Some("/data/unity?x=1")), "/data/unity?x=1");
        for elsewhere in [
            None,
            Some(""),
            Some("data"),
            Some("//evil.example"),
            Some("/\\evil.example"),
            Some("https://evil.example/"),
            Some("/x\r\nSet-Cookie: a=b"),
        ] {
            assert_eq!(local_path(elsewhere), "/", "{elsewhere:?}");
        }
    }

    #[test]
    fn pkce_challenge_is_rfc_7636s() {
        // RFC 7636, appendix B.
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert_ne!(random(), random());
        assert_eq!(random().len(), 43);
    }
}
