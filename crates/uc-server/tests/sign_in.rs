//! Sign-in to the web UI (`/auth/login`, `/auth/callback`, `/auth/logout`)
//! against an OpenID Connect provider served here: discovery, its keys, and
//! a token endpoint that checks the client secret and the PKCE verifier.
//! Failed sign-ins go back to the UI's login page; with `oidc_end_session`,
//! logout ends the provider's session too. Then sign-in through an auth
//! proxy in front (`auth.mode = "forward"`), and `/auth/mode`.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Form, Json, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{EncodingKey, Header};
use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use uc_metastore::{DynMetastore, MetastoreConfig, SqlMetastore};
use uc_server::{AppState, authn::idp::AuthSettings, authn::sign_in::SignIn, build_app};

use common::{active_user, json_body, security};

const PUBLIC_URL: &str = "https://uc.example.com";
const CLIENT: &str = "uc";
const SECRET: &str = "s3cret";

/// The codes the provider will take: the PKCE challenge each needs, and
/// the ID token it gives.
type Codes = Arc<Mutex<HashMap<String, (String, String)>>>;

struct Provider {
    issuer: String,
    key: EncodingKey,
    codes: Codes,
}

impl Provider {
    async fn start() -> Self {
        Self::serve(true).await
    }

    /// A provider whose discovery document has no end-session endpoint.
    async fn without_end_session() -> Self {
        Self::serve(false).await
    }

    async fn serve(end_session: bool) -> Self {
        let rsa = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let b64 = |bytes: &[u8]| URL_SAFE_NO_PAD.encode(bytes);
        let jwks = json!({ "keys": [{ "kty": "RSA", "kid": "rsa", "use": "sig", "alg": "RS256",
            "n": b64(&rsa.n().to_bytes_be()), "e": b64(&rsa.e().to_bytes_be()) }] });
        let mut config = json!({
            "issuer": issuer,
            "jwks_uri": format!("{issuer}/jwks"),
            "authorization_endpoint": format!("{issuer}/authorize"),
            "token_endpoint": format!("{issuer}/token"),
        });
        if end_session {
            // With a query of its own, as some providers have.
            config["end_session_endpoint"] = json!(format!("{issuer}/logout?realm=uc"));
        }
        let codes: Codes = Arc::default();
        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || async move { Json(config) }),
            )
            .route("/jwks", get(move || async move { Json(jwks) }))
            .route("/token", post(token))
            .with_state(codes.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            issuer,
            key: EncodingKey::from_rsa_der(rsa.to_pkcs1_der().unwrap().as_bytes()),
            codes,
        }
    }

    /// An ID token with these claims over the defaults.
    fn id_token(&self, extra: Value) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut claims = json!({ "iss": self.issuer, "aud": CLIENT, "iat": now, "exp": now + 600 });
        claims
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("rsa".into());
        jsonwebtoken::encode(&header, &claims, &self.key).unwrap()
    }

    /// `code` gives `id_token` for the verifier of `challenge`.
    fn accept(&self, code: &str, challenge: &str, id_token: String) {
        self.codes
            .lock()
            .unwrap()
            .insert(code.into(), (challenge.into(), id_token));
    }
}

/// The token endpoint: the client's secret (HTTP Basic), the code, the
/// redirect URI and the PKCE verifier.
async fn token(
    State(codes): State<Codes>,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let refused = |error: &str| (StatusCode::BAD_REQUEST, Json(json!({ "error": error })));
    let basic = format!(
        "Basic {}",
        URL_SAFE_NO_PAD.encode(format!("{CLIENT}:{SECRET}"))
    );
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim_end_matches('='));
    if authorization != Some(basic.trim_end_matches('=')) {
        return refused("invalid_client").into_response();
    }
    if form.get("grant_type").map(String::as_str) != Some("authorization_code")
        || form.get("redirect_uri").map(String::as_str)
            != Some(&format!("{PUBLIC_URL}/auth/callback"))
    {
        return refused("invalid_request").into_response();
    }
    let code = form.get("code").cloned().unwrap_or_default();
    let Some((challenge, id_token)) = codes.lock().unwrap().remove(&code) else {
        return refused("invalid_grant").into_response();
    };
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    if URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != challenge {
        return refused("invalid_grant").into_response();
    }
    Json(json!({ "id_token": id_token, "access_token": "at", "token_type": "Bearer" }))
        .into_response()
}

/// An app with authorization on and sign-in with `provider` (or with an
/// issuer at `issuer`), and its metastore.
async fn app(issuer: &str) -> (Router, DynMetastore) {
    app_with(&oidc(issuer)).await
}

/// The `[auth]` settings of the sign-in with the provider at `issuer`.
fn oidc(issuer: &str) -> String {
    format!(
        r#"
        mode = "oidc"
        public_url = "{PUBLIC_URL}"
        oidc_issuer = "{issuer}"
        oidc_client_id = "{CLIENT}"
        oidc_client_secret = "{SECRET}"
        "#
    )
}

/// An app with authorization on and the sign-in of the `[auth]` settings
/// `auth` (uc.toml), and its metastore.
async fn app_with(auth: &str) -> (Router, DynMetastore) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let dyn_metastore: DynMetastore = metastore.clone();
    let settings = uc_common::settings::Settings::from_toml(&format!("[auth]\n{auth}")).unwrap();
    let sign_in = SignIn::new(&settings.auth).unwrap();
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        access_token_timeout: Duration::from_secs(3600),
        cookie_timeout: Duration::from_secs(600),
        ..Default::default()
    })
    .with_sign_in(sign_in);
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security())
        .with_auth_settings(settings);
    (build_app(state), dyn_metastore)
}

fn get_with_cookie(uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut request = Request::get(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    request.body(Body::empty()).unwrap()
}

async fn text(response: Response) -> String {
    String::from_utf8(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

fn location(response: &Response) -> String {
    response.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string()
}

/// The `Set-Cookie` values, by cookie name.
fn cookies(response: &Response) -> HashMap<String, String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|v| {
            let (name, rest) = v.split_once('=')?;
            Some((name.to_string(), rest.to_string()))
        })
        .collect()
}

/// A started sign-in: the query of the provider's URL and the `UC_LOGIN`
/// cookie.
struct Started {
    query: HashMap<String, String>,
    cookie: String,
}

async fn start(app: &Router, rd: &str) -> Started {
    let response = app
        .clone()
        .oneshot(get_with_cookie(&format!("/auth/login?rd={rd}"), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let url = location(&response);
    let query = serde_urlencoded::from_str(url.split_once('?').unwrap().1).unwrap();
    let login = cookies(&response).remove("UC_LOGIN").unwrap();
    assert!(login.contains("Path=/auth/callback; HttpOnly; SameSite=Lax; Secure"));
    let cookie = format!("UC_LOGIN={}", login.split(';').next().unwrap());
    Started { query, cookie }
}

async fn callback(app: &Router, code: &str, state: &str, cookie: Option<&str>) -> Response {
    let uri = format!("/auth/callback?code={code}&state={state}");
    app.clone()
        .oneshot(get_with_cookie(&uri, cookie))
        .await
        .unwrap()
}

/// A failed sign-in: back to the web UI's login page with the error `code`.
/// The `UC_LOGIN` cookie expires if the browser sent one.
fn assert_failed(response: &Response, code: &str, sent_login_cookie: bool) {
    assert_eq!(response.status(), StatusCode::SEE_OTHER, "{code}");
    assert_eq!(location(response), format!("/login?error={code}"));
    let set = cookies(response);
    match sent_login_cookie {
        true => assert!(set["UC_LOGIN"].starts_with("; Max-Age=0"), "{set:?}"),
        false => assert!(set.is_empty(), "{set:?}"),
    }
}

#[tokio::test]
async fn signs_in_with_the_provider_and_sets_the_uc_token_cookie() {
    let provider = Provider::start().await;
    let (app, metastore) = app(&provider.issuer).await;
    active_user(&metastore, "alice@example.com").await;

    let started = start(&app, "/data/unity").await;
    let q = &started.query;
    assert_eq!(q["response_type"], "code");
    assert_eq!(q["client_id"], CLIENT);
    assert_eq!(q["redirect_uri"], format!("{PUBLIC_URL}/auth/callback"));
    assert_eq!(q["scope"], "openid email profile");
    assert_eq!(q["code_challenge_method"], "S256");
    let id_token = provider.id_token(json!({ "email": "alice@example.com", "nonce": q["nonce"] }));
    provider.accept("good", &q["code_challenge"], id_token);

    let response = callback(&app, "good", &q["state"], Some(&started.cookie)).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), "/data/unity");
    let set = cookies(&response);
    assert!(set["UC_LOGIN"].starts_with("; Max-Age=0"), "{set:?}");
    let uc_token = set["UC_TOKEN"].split(';').next().unwrap().to_string();
    assert!(set["UC_TOKEN"].contains("HttpOnly; SameSite=Strict; Secure"));

    // The cookie signs Alice in to the API, as the token exchange's does.
    let me = app
        .clone()
        .oneshot(get_with_cookie(
            "/api/1.0/unity-control/scim2/Me",
            Some(&format!("UC_TOKEN={uc_token}")),
        ))
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(json_body(me).await["userName"], "alice@example.com");

    // A code is good once.
    let again = callback(&app, "good", &q["state"], Some(&started.cookie)).await;
    assert_failed(&again, "token", true);

    // Logout expires the cookie; the provider's session stays.
    let out = logout(&app, Some(&format!("UC_TOKEN={uc_token}"))).await;
    assert_eq!(out.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&out), "/");
    assert!(cookies(&out)["UC_TOKEN"].starts_with("; Max-Age=0"));
}

async fn logout(app: &Router, cookie: Option<&str>) -> Response {
    app.clone()
        .oneshot(get_with_cookie("/auth/logout", cookie))
        .await
        .unwrap()
}

#[tokio::test]
async fn with_oidc_end_session_logout_ends_the_providers_session() {
    let ending = |issuer: &str| format!("{}\noidc_end_session = true", oidc(issuer));
    let provider = Provider::start().await;
    let (app, _) = app_with(&ending(&provider.issuer)).await;

    // Logout expires the cookie, then ends the provider's session, which
    // returns the browser to the web UI.
    let out = logout(&app, Some("UC_TOKEN=t")).await;
    assert_eq!(out.status(), StatusCode::SEE_OTHER);
    let end_session = location(&out);
    let (endpoint, query) = end_session.split_once('?').unwrap();
    assert_eq!(endpoint, format!("{}/logout", provider.issuer));
    let query: Vec<(String, String)> = serde_urlencoded::from_str(query).unwrap();
    let pair = |key: &str, value: &str| (key.to_string(), value.to_string());
    assert_eq!(
        query,
        [
            pair("realm", "uc"),
            pair("client_id", CLIENT),
            pair("post_logout_redirect_uri", &format!("{PUBLIC_URL}/")),
        ]
    );
    assert!(cookies(&out)["UC_TOKEN"].starts_with("; Max-Age=0"));

    // Without the cookie, such as from a link on another site, the
    // provider's session stays.
    let link = logout(&app, None).await;
    assert_eq!(location(&link), "/");
    assert!(cookies(&link).is_empty());

    // A provider without an end-session endpoint, or one that does not
    // answer: back to the web UI.
    let without = Provider::without_end_session().await;
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let unreachable = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    for issuer in [without.issuer.as_str(), unreachable.as_str()] {
        let (app, _) = app_with(&ending(issuer)).await;
        let out = logout(&app, Some("UC_TOKEN=t")).await;
        assert_eq!(location(&out), "/", "{issuer}");
        assert!(cookies(&out)["UC_TOKEN"].starts_with("; Max-Age=0"));
    }
}

#[tokio::test]
async fn refuses_sign_ins_that_do_not_check_out() {
    let provider = Provider::start().await;
    let (app, metastore) = app(&provider.issuer).await;
    active_user(&metastore, "alice@example.com").await;
    let get = |uri: String, cookie: Option<&str>| {
        let request = get_with_cookie(&uri, cookie);
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap() }
    };

    // Not started here, a forged sign-in state, or a state of another sign-in.
    let started = start(&app, "/").await;
    let q = &started.query;
    assert_failed(
        &callback(&app, "x", &q["state"], None).await,
        "expired",
        false,
    );
    let forged = Some("UC_LOGIN=forged");
    assert_failed(
        &callback(&app, "x", &q["state"], forged).await,
        "expired",
        true,
    );
    let cookie = Some(started.cookie.as_str());
    assert_failed(&callback(&app, "x", "other", cookie).await, "expired", true);
    // The provider refused, or answered without a code.
    let state = &q["state"];
    let refused = get(
        format!("/auth/callback?error=access_denied&state={state}"),
        cookie,
    )
    .await;
    assert_failed(&refused, "provider", true);
    let no_code = get(format!("/auth/callback?state={state}"), cookie).await;
    assert_failed(&no_code, "provider", true);
    // The code with another verifier, or an ID token that is no JWT.
    let token = provider.id_token(json!({ "email": "alice@example.com", "nonce": q["nonce"] }));
    provider.accept("pkce", "another challenge", token);
    assert_failed(&callback(&app, "pkce", state, cookie).await, "token", true);
    provider.accept("garbled", &q["code_challenge"], "not a JWT".into());
    assert_failed(
        &callback(&app, "garbled", state, cookie).await,
        "token",
        true,
    );

    // ID tokens of another sign-in, another client, or an unknown user.
    let cases = [
        (
            json!({ "email": "alice@example.com", "nonce": "other" }),
            "token",
        ),
        (
            json!({ "email": "alice@example.com", "nonce": q["nonce"], "aud": "other" }),
            "token",
        ),
        (
            json!({ "email": "bob@example.com", "nonce": q["nonce"] }),
            "user",
        ),
    ];
    for (i, (claims, error)) in cases.into_iter().enumerate() {
        let code = format!("case{i}");
        provider.accept(&code, &q["code_challenge"], provider.id_token(claims));
        assert_failed(&callback(&app, &code, state, cookie).await, error, true);
    }

    // The signed sign-in state is no access token.
    let login_token = started.cookie.trim_start_matches("UC_LOGIN=");
    assert!(security().verify(login_token).is_err());
    let catalogs = app
        .clone()
        .oneshot(common::req_bearer(
            axum::http::Method::GET,
            "/catalogs",
            login_token,
        ))
        .await
        .unwrap();
    // Its issuer is neither the server's nor an `[[auth.issuers]]` block:
    // refused as any such token is.
    assert_eq!(catalogs.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sign_in_needs_its_settings_and_a_reachable_provider() {
    // No auth.mode: no sign-in.
    let (plain, _) = common::signed_in_app().await;
    let response = plain
        .oneshot(get_with_cookie("/auth/login", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(text(response).await.contains("auth.mode = \"oidc\""));

    // A provider that does not answer.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let issuer = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let (app, _) = app(&issuer).await;
    let response = app
        .oneshot(get_with_cookie("/auth/login", None))
        .await
        .unwrap();
    assert_failed(&response, "provider", false);
}

/// `GET /auth/mode` of `app`, without a sign-in.
async fn mode(app: &Router) -> Value {
    let response = app
        .clone()
        .oneshot(get_with_cookie("/auth/mode", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    json_body(response).await
}

#[tokio::test]
async fn mode_tells_the_ui_how_it_signs_in_and_where_log_out_goes() {
    let (oidc, _) = app("https://idp.example.com").await;
    assert_eq!(
        mode(&oidc).await,
        json!({ "mode": "oidc", "logout_url": "/auth/logout" })
    );

    // Behind a proxy, "Log out" needs the proxy's sign-out: else the proxy
    // signs the user in again at once.
    let proxy = "mode = \"forward\"\nforward_trusted_proxies = \"10.0.0.0/8\"";
    let (forward, _) = app_with(&format!(
        "{proxy}\nforward_logout_url = \"/oauth2/sign_out\""
    ))
    .await;
    assert_eq!(
        mode(&forward).await,
        json!({ "mode": "forward", "logout_url": "/auth/logout" })
    );
    let (forward, _) = app_with(proxy).await;
    assert_eq!(
        mode(&forward).await,
        json!({ "mode": "forward", "logout_url": null })
    );

    // Without a sign-in, "Log out" expires the cookie of an earlier one.
    let (none, metastore) = app_with("").await;
    assert_eq!(
        mode(&none).await,
        json!({ "mode": "none", "logout_url": "/auth/logout" })
    );
    let token = active_user(&metastore, "alice@example.com").await;
    let out = none
        .oneshot(get_with_cookie(
            "/auth/logout",
            Some(&format!("UC_TOKEN={token}")),
        ))
        .await
        .unwrap();
    assert_eq!(location(&out), "/");
    assert!(cookies(&out)["UC_TOKEN"].starts_with("; Max-Age=0"));

    // With authorization off too, no one signs in: the UI asks no one who
    // is signed in, and has no "Log out".
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let off = build_app(AppState::new(metastore));
    assert_eq!(
        mode(&off).await,
        json!({ "mode": "off", "logout_url": null })
    );
}

/// A request from `peer`, with these headers.
fn from(peer: &str, uri: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut request = Request::get(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut request = request.body(Body::empty()).unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        peer.parse::<std::net::SocketAddr>().unwrap(),
    ));
    request
}

#[tokio::test]
async fn behind_an_auth_proxy_its_headers_name_the_user_from_its_addresses_only() {
    let (app, metastore) = app_with(
        r#"
        mode = "forward"
        forward_trusted_proxies = "10.0.0.0/8"
        forward_logout_url = "/oauth2/sign_out"
        "#,
    )
    .await;
    let token = active_user(&metastore, "alice@example.com").await;
    let me = "/api/1.0/unity-control/scim2/Me";
    let call = |request: Request<Body>| {
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap() }
    };
    let signed_in_as = |response: Response, user: &'static str| async move {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await["userName"], user);
    };
    let alice = ("Remote-Email", "alice@example.com");

    // From the proxy: its headers, the email else the user, whatever token
    // comes along; IPv4 on a dual-stack listener too.
    signed_in_as(
        call(from("10.1.2.3:5000", me, &[alice])).await,
        "alice@example.com",
    )
    .await;
    signed_in_as(
        call(from(
            "10.1.2.3:5000",
            me,
            &[("Remote-User", "alice@example.com")],
        ))
        .await,
        "alice@example.com",
    )
    .await;
    signed_in_as(
        call(from(
            "10.1.2.3:5000",
            me,
            &[alice, ("Authorization", "Bearer idp-token")],
        ))
        .await,
        "alice@example.com",
    )
    .await;
    signed_in_as(
        call(from("[::ffff:10.1.2.3]:5000", me, &[alice])).await,
        "alice@example.com",
    )
    .await;
    // A user the server does not have.
    common::assert_error(
        call(from(
            "10.1.2.3:5000",
            me,
            &[("Remote-Email", "bob@example.com")],
        ))
        .await,
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
    )
    .await;
    // Never the bootstrap admin, though it exists: a proxy user may have
    // its name.
    active_user(&metastore, "admin").await;
    for header in ["Remote-User", "Remote-Email"] {
        let error = common::assert_error(
            call(from("10.1.2.3:5000", me, &[(header, "admin")])).await,
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
        )
        .await;
        assert_eq!(error["message"], "User not allowed: admin");
    }

    // From elsewhere the headers do not count; tokens still do.
    common::assert_error(
        call(from("192.168.1.5:5000", me, &[alice])).await,
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
    )
    .await;
    common::assert_error(
        call(from("10.1.2.3:5000", me, &[])).await,
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
    )
    .await;
    let bearer = format!("Bearer {token}");
    signed_in_as(
        call(from("192.168.1.5:5000", me, &[("Authorization", &bearer)])).await,
        "alice@example.com",
    )
    .await;

    // The proxy signs in; logout goes to its sign-out.
    let login = call(from("10.1.2.3:5000", "/auth/login", &[])).await;
    assert_eq!(login.status(), StatusCode::NOT_FOUND);
    assert!(text(login).await.contains("auth proxy"));
    let logout = call(from("10.1.2.3:5000", "/auth/logout", &[alice])).await;
    assert_eq!(logout.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&logout), "/oauth2/sign_out");
}
