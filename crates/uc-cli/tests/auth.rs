//! `uc auth login`: an identity token, given or got in the browser, is
//! exchanged for a Unity Catalog access token. An identity provider is
//! served here: OIDC discovery, its JWK set and an OAuth token endpoint.

use std::net::SocketAddr;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use jsonwebtoken::{EncodingKey, Header};
use rsa::RsaPrivateKey;
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, Command};

use uc_metastore::{DynMetastore, MetastoreConfig, SqlMetastore};
use uc_server::authn::idp::AuthSettings;
use uc_server::authn::security::SecurityContext;
use uc_server::{AppState, build_app};

const CLIENT_ID: &str = "uc-cli";
const CLIENT_SECRET: &str = "s3cret";

/// An identity provider with one RSA signing key.
struct Idp {
    issuer: String,
    key: EncodingKey,
}

impl Idp {
    async fn start() -> Arc<Self> {
        let rsa = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let jwks = json!({ "keys": [{
            "kty": "RSA", "kid": "rsa", "use": "sig", "alg": "RS256",
            "n": URL_SAFE_NO_PAD.encode(rsa.n().to_bytes_be()),
            "e": URL_SAFE_NO_PAD.encode(rsa.e().to_bytes_be()),
        }] });
        let config = json!({ "issuer": issuer, "jwks_uri": format!("{issuer}/jwks") });
        let idp = Arc::new(Self {
            issuer,
            key: EncodingKey::from_rsa_der(rsa.to_pkcs1_der().unwrap().as_bytes()),
        });
        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || async move { Json(config) }),
            )
            .route("/jwks", get(move || async move { Json(jwks) }))
            .route("/token", post(token_endpoint))
            .with_state(idp.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        idp
    }

    /// An identity token for `email`, for audience `uc`.
    fn token(&self, email: &str) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = json!({
            "iss": self.issuer, "aud": "uc", "iat": now, "exp": now + 600,
            "sub": email, "email": email,
        });
        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("rsa".into());
        jsonwebtoken::encode(&header, &claims, &self.key).unwrap()
    }
}

/// The OAuth token endpoint: the code `ada` gives Ada's identity token,
/// `no-id` a response without one; other codes are refused.
async fn token_endpoint(
    State(idp): State<Arc<Idp>>,
    headers: HeaderMap,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let basic = format!(
        "Basic {}",
        STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"))
    );
    if headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        != Some(&basic)
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid_client" })),
        );
    }
    let redirect = form.get("redirect_uri").map(String::as_str).unwrap_or("");
    if form.get("grant_type").map(String::as_str) != Some("authorization_code")
        || !redirect.starts_with("http://localhost:")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_request" })),
        );
    }
    match form.get("code").map(String::as_str) {
        Some("ada") => (
            StatusCode::OK,
            Json(json!({ "id_token": idp.token("ada@example.com"), "token_type": "Bearer" })),
        ),
        Some("no-id") => (StatusCode::OK, Json(json!({ "access_token": "opaque" }))),
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_grant" })),
        ),
    }
}

/// A server with authorization on that takes `idp`'s tokens; its URL.
/// Ada is a user.
async fn server(idp: &Idp) -> String {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database");
    let users: DynMetastore = metastore.clone();
    users
        .scim()
        .create_user(uc_api::ScimUser {
            user_name: "ada@example.com".into(),
            emails: Some(vec![uc_api::ScimEmail {
                value: "ada@example.com".into(),
                primary: Some(true),
                r#type: None,
            }]),
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    let security = Arc::new(SecurityContext::generate().unwrap());
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        allowed_issuers: vec![idp.issuer.clone()],
        audiences: vec!["uc".into()],
        access_token_timeout: Duration::from_secs(3600),
        cookie_timeout: Duration::from_secs(600),
        ..Default::default()
    });
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security)
        .with_auth_settings(settings);
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    url
}

/// `uc --server url` with no settings and no sign-ins from this machine,
/// and a browser that writes the page it opens to `opened.txt`.
fn uc(dir: &Path, url: &str) -> Command {
    let settings = dir.join("empty.toml");
    std::fs::write(&settings, "").unwrap();
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for opener in ["open", "xdg-open"] {
        let script = bin.join(opener);
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s' \"$1\" > \"$UC_TEST_OPENED\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_uc"));
    command
        .args(["--server", url])
        .env_remove("UC_CLI_SERVER")
        .env_remove("UC_CLI_TOKEN")
        .env("UC_CONFIG", settings)
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("PATH", path)
        .env("UC_TEST_OPENED", dir.join("opened.txt"))
        .current_dir(dir)
        .kill_on_drop(true);
    command
}

async fn run(command: &mut Command) -> (i32, String, String) {
    let output = command.output().await.unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn an_identity_token_is_exchanged_for_an_access_token() {
    let dir = TempDir::new().unwrap();
    let idp = Idp::start().await;
    let url = server(&idp).await;

    let token = idp.token("ada@example.com");
    let (code, out, err) = run(uc(dir.path(), &url).args([
        "auth",
        "login",
        "--identity_token",
        &token,
        "--output",
        "json",
    ]))
    .await;
    assert_eq!(code, 0, "{err}");
    let login: Value = serde_json::from_str(out.trim()).unwrap();
    let keys: Vec<&String> = login.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["access_token"]);
    let access = login["access_token"].as_str().unwrap();

    // The access token signs Ada in.
    let (code, out, err) = run(uc(dir.path(), &url).args([
        "token",
        "create",
        "--name",
        "mine",
        "--auth_token",
        access,
        "--output",
        "json",
    ]))
    .await;
    assert_eq!(code, 0, "{err}");
    let pat: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(pat["owner"], "ada@example.com");

    // Someone who is not a user is refused.
    let stranger = idp.token("eve@example.com");
    let (code, _, err) =
        run(uc(dir.path(), &url).args(["auth", "login", "--identity_token", &stranger])).await;
    assert_eq!(code, 1);
    assert!(err.contains("Error authenticating - "), "{err}");
    assert!(err.contains("User not allowed"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn later_commands_use_the_sign_in() {
    let dir = TempDir::new().unwrap();
    let idp = Idp::start().await;
    let url = server(&idp).await;
    let tokens = dir.path().join("state/uc/tokens.json");
    let login = format!("uc --server {url} auth login");

    // Without a sign-in, a command says how to sign in.
    let (code, _, err) = run(uc(dir.path(), &url).args(["catalog", "list"])).await;
    assert_eq!(code, 1);
    assert!(
        err.contains("listCatalogs call failed with: 401 - "),
        "{err}"
    );
    assert!(
        err.contains(&format!("{url} needs a sign-in. Sign in with: {login}\n")),
        "{err}"
    );

    let token = idp.token("ada@example.com");
    let (code, out, err) = run(uc(dir.path(), &url).args([
        "auth",
        "login",
        "--identity_token",
        &token,
        "--output",
        "json",
    ]))
    .await;
    assert_eq!(code, 0, "{err}");
    assert!(err.contains(&format!("Signed in to {url} until ")), "{err}");
    let login_out: Value = serde_json::from_str(out.trim()).unwrap();
    let access = login_out["access_token"].as_str().unwrap();
    let kept: Value = serde_json::from_str(&std::fs::read_to_string(&tokens).unwrap()).unwrap();
    assert_eq!(kept, json!({ url.clone(): access }));

    // The next commands send it.
    let (code, out, err) =
        run(uc(dir.path(), &url).args(["token", "create", "--name", "mine", "--output", "json"]))
            .await;
    assert_eq!(code, 0, "{err}");
    let pat: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(pat["owner"], "ada@example.com");

    // A token of the settings wins, and a refused one is named.
    let (code, _, err) = run(uc(dir.path(), &url)
        .args(["catalog", "list"])
        .env("UC_CLI_TOKEN", "not-valid"))
    .await;
    assert_eq!(code, 1);
    assert!(
        err.contains(&format!(
            "{url} does not accept the token of cli.token (UC_CLI_TOKEN). Without it, uc uses \
             the sign-in of: {login}\n"
        )),
        "{err}"
    );
    let (code, _, err) = run(uc(dir.path(), &url)
        .args(["auth", "login", "--identity_token", &token])
        .env("UC_CLI_TOKEN", "not-valid"))
    .await;
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains(
            "While cli.token (UC_CLI_TOKEN) is set, uc sends its token, not this sign-in."
        ),
        "{err}"
    );

    // A sign-in that the server does not accept anymore, such as one of an
    // earlier signing key.
    std::fs::write(&tokens, json!({ url.clone(): "not-valid" }).to_string()).unwrap();
    let (code, _, err) = run(uc(dir.path(), &url).args(["catalog", "list"])).await;
    assert_eq!(code, 1);
    assert!(
        err.contains(&format!(
            "Your sign-in to {url} is not valid anymore. Sign in again with: {login}\n"
        )),
        "{err}"
    );

    // `auth logout` removes it, and prints only its message.
    let (code, out, err) = run(uc(dir.path(), &url).args(["auth", "logout"])).await;
    assert_eq!(code, 0, "{err}");
    assert!(err.contains(&format!("Signed out of {url}.")), "{err}");
    assert_eq!(out, "");
    assert_eq!(std::fs::read_to_string(&tokens).unwrap().trim(), "{}");
    let logout = ["auth", "logout", "--output", "json"];
    let (code, out, err) = run(uc(dir.path(), &url).args(logout)).await;
    assert_eq!(code, 0, "{err}");
    assert!(err.contains(&format!("Not signed in to {url}.")), "{err}");
    assert_eq!(out, "");
    let (code, _, err) = run(uc(dir.path(), &url).args(["catalog", "list"])).await;
    assert_eq!(code, 1);
    assert!(err.contains(&format!("{url} needs a sign-in.")), "{err}");
}

/// The settings of a browser sign-in at `idp`.
fn browser<'a>(command: &'a mut Command, idp: &Idp) -> &'a mut Command {
    command
        .env(
            "UC_CLI_OAUTH_AUTHORIZATION_URL",
            format!("{}/authorize", idp.issuer),
        )
        .env("UC_CLI_OAUTH_TOKEN_URL", format!("{}/token", idp.issuer))
        .env("UC_CLI_OAUTH_CLIENT_ID", CLIENT_ID)
        .env("UC_CLI_OAUTH_CLIENT_SECRET", CLIENT_SECRET)
}

/// `uc auth login`, waiting for the browser.
struct Login {
    child: Child,
    port: u16,
    /// The state of its authorization URL.
    state: String,
    /// Its messages so far, and the rest of its standard error.
    err: String,
    lines: Lines<BufReader<ChildStderr>>,
}

/// Run `uc auth login` until it waits for the browser: its messages, on
/// standard error, give the port and the URL to open.
async fn start_login(mut command: Command) -> Login {
    let mut child = command
        .args(["auth", "login", "--output", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stderr.take().unwrap()).lines();
    let mut err = String::new();
    let (mut port, mut state) = (None, None);
    while state.is_none() {
        let line = lines.next_line().await.unwrap().expect("the URL to open");
        if let Some(listening) = line.strip_prefix("Listening on port: ") {
            port = Some(listening.parse().unwrap());
        }
        if let Some((_, query)) = line.split_once('?') {
            let mut params = url::form_urlencoded::parse(query.as_bytes());
            state = params
                .find(|(key, _)| key == "state")
                .map(|(_, s)| s.into_owned());
        }
        err.push_str(&line);
        err.push('\n');
    }
    Login {
        child,
        port: port.unwrap_or_else(|| panic!("no port: {err}")),
        state: state.unwrap(),
        err,
        lines,
    }
}

/// The rest of the run; exit code, stdout, stderr.
async fn finish(login: Login) -> (i32, String, String) {
    let Login {
        mut child,
        mut err,
        mut lines,
        ..
    } = login;
    while let Some(line) = lines.next_line().await.unwrap() {
        err.push_str(&line);
        err.push('\n');
    }
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .await
        .unwrap();
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("uc auth login stops")
        .unwrap();
    (status.code().unwrap_or(-1), out, err)
}

/// The browser's redirect back to the CLI.
async fn redirect(port: u16, query: &str) -> (u16, String) {
    let response = reqwest::get(format!("http://localhost:{port}/{query}"))
        .await
        .unwrap();
    (response.status().as_u16(), response.text().await.unwrap())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn signs_in_in_the_browser() {
    let dir = TempDir::new().unwrap();
    let idp = Idp::start().await;
    let url = server(&idp).await;

    let mut command = uc(dir.path(), &url);
    browser(&mut command, &idp);
    let login = start_login(command).await;
    let (port, state) = (login.port, login.state.clone());

    // The browser opens the provider's authorization page.
    let opened = dir.path().join("opened.txt");
    let start = Instant::now();
    while !opened.exists() {
        assert!(start.elapsed() < Duration::from_secs(10), "no browser");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let page = std::fs::read_to_string(&opened).unwrap();
    let (base, query) = page.split_once('?').unwrap();
    assert_eq!(base, format!("{}/authorize", idp.issuer));
    let params: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();
    assert_eq!(params["client_id"], CLIENT_ID);
    assert_eq!(params["redirect_uri"], format!("http://localhost:{port}"));
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["scope"], "openid profile email");
    assert_eq!(params["state"].len(), 32);
    assert_eq!(params["state"], state);

    // Only this machine reaches the redirect.
    if let Some(ip) = lan_address() {
        assert!(
            std::net::TcpStream::connect((ip, port)).is_err(),
            "{ip}:{port} answers"
        );
    }
    // Other requests of the browser, such as a favicon, are not the answer;
    // nor is a code without this sign-in's state, from another page.
    assert_eq!(redirect(port, "favicon.ico").await, (404, String::new()));
    assert_eq!(redirect(port, "?code=eve").await, (404, String::new()));
    assert_eq!(
        redirect(port, "?code=eve&state=forged").await,
        (404, String::new())
    );
    assert_eq!(
        redirect(port, &format!("?code=ada&state={state}")).await,
        (200, "User validated with identity provider.".to_string())
    );

    // The messages go to stderr; stdout has only the result.
    let (code, out, err) = finish(login).await;
    assert_eq!(code, 0, "{err}");
    assert!(err.contains(&format!("{}/authorize?", idp.issuer)), "{err}");
    assert!(err.contains("Received authentication response.\n"), "{err}");
    assert!(err.contains("Received token response.\n"), "{err}");
    assert!(err.contains(&format!("Signed in to {url} until ")), "{err}");
    let login: Value = serde_json::from_str(out.trim()).unwrap();
    assert!(login["access_token"].as_str().unwrap().len() > 20, "{out}");
}

/// An address of this machine that is not loopback, if it has one.
fn lan_address() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // Sends nothing: it only picks the address of the route.
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn browser_sign_in_failures() {
    let dir = TempDir::new().unwrap();
    let idp = Idp::start().await;
    let url = "http://127.0.0.1:9";

    // Both provider URLs are needed.
    let (code, _, err) = run(uc(dir.path(), url).args(["auth", "login"])).await;
    assert_eq!(code, 1);
    assert!(
        err.contains("cli.oauth_authorization_url (UC_CLI_OAUTH_AUTHORIZATION_URL) is not set"),
        "{err}"
    );
    let (code, _, err) = run(uc(dir.path(), url)
        .args(["auth", "login"])
        .env("UC_CLI_OAUTH_AUTHORIZATION_URL", "http://idp/authorize"))
    .await;
    assert_eq!(code, 1);
    assert!(
        err.contains("cli.oauth_token_url (UC_CLI_OAUTH_TOKEN_URL) is not set"),
        "{err}"
    );

    // A code the provider refuses, on the port of the settings.
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut command = uc(dir.path(), url);
    browser(&mut command, &idp).env("UC_CLI_OAUTH_REDIRECT_PORT", port.to_string());
    let login = start_login(command).await;
    assert_eq!(login.port, port);
    redirect(port, &format!("?code=stolen&state={}", login.state)).await;
    let (code, out, err) = finish(login).await;
    assert_eq!(code, 1);
    assert!(!out.contains("invalid_grant"), "{out}");
    assert!(err.contains("Error authenticating - "), "{err}");
    assert!(err.contains("invalid_grant"), "{err}");

    // A token response without an identity token.
    let mut command = uc(dir.path(), url);
    browser(&mut command, &idp);
    let login = start_login(command).await;
    redirect(login.port, &format!("?code=no-id&state={}", login.state)).await;
    let (code, _, err) = finish(login).await;
    assert_eq!(code, 1);
    assert!(err.contains("The token response has no id_token"), "{err}");

    // The wrong client secret.
    let mut command = uc(dir.path(), url);
    browser(&mut command, &idp).env("UC_CLI_OAUTH_CLIENT_SECRET", "wrong");
    let login = start_login(command).await;
    redirect(login.port, &format!("?code=ada&state={}", login.state)).await;
    let (code, _, err) = finish(login).await;
    assert_eq!(code, 1);
    assert!(err.contains("invalid_client"), "{err}");
}
