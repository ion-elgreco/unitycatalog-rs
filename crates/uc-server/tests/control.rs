//! Sign-in through the control plane, after upstream `AuthServiceTest` and
//! `AuthDecoratorTest`: token exchange against an identity provider served
//! here (OIDC discovery and JWKS), UC access tokens, the `UC_TOKEN` cookie,
//! sign-out, and sign-in required when authorization is on.

mod common;

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_metastore::{DynMetastore, MetastoreConfig, SqlMetastore};
use uc_server::{AppState, authn::idp::AuthSettings, build_app};

use common::{BASE, assert_error, idp::Idp, json_body, security};

const TOKENS: &str = "/api/1.0/unity-control/auth/tokens";
const LOGOUT: &str = "/api/1.0/unity-control/auth/logout";
const ID_TOKEN: &str = "urn:ietf:params:oauth:token-type:id_token";
const ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";

/// An app with authorization on that takes `idp`'s tokens at the exchange,
/// as the `[[auth.issuers]]` block `idp` with the issuer pattern `issuer`
/// (`$IDP` is the provider; `""` is no block at all) and the audience
/// `audience`; and its metastore.
async fn app(idp: &Idp, issuer: &str, audience: &str) -> (Router, DynMetastore) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let dyn_metastore: DynMetastore = metastore.clone();
    let issuers = match issuer {
        "" => Vec::new(),
        pattern => vec![uc_common::settings::Issuer {
            name: "idp".into(),
            issuer: pattern.replace("$IDP", &idp.issuer),
            audiences: vec![audience.to_string()],
            ..Default::default()
        }],
    };
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        issuers,
        access_token_timeout: Duration::from_secs(3600),
        cookie_timeout: Duration::from_secs(600),
        ..Default::default()
    });
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security())
        .with_auth_settings(settings);
    (build_app(state), dyn_metastore)
}

async fn user(metastore: &DynMetastore, email: &str, external_id: Option<&str>, active: bool) {
    metastore
        .scim()
        .create_user(uc_api::ScimUser {
            schemas: vec![],
            user_name: email.to_string(),
            emails: Some(vec![uc_api::ScimEmail {
                value: email.to_string(),
                primary: Some(true),
                r#type: None,
            }]),
            external_id: external_id.map(str::to_string),
            active: Some(active),
            ..Default::default()
        })
        .await
        .unwrap();
}

fn form(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A token exchange for `subject_token` of `subject_type`.
fn exchange_form(subject_token: &str, subject_type: &str) -> String {
    form(&[
        (
            "grant_type",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ),
        ("requested_token_type", ACCESS_TOKEN),
        ("subject_token_type", subject_type),
        ("subject_token", subject_token),
    ])
}

async fn post_form(app: &Router, path: &str, body: String) -> axum::response::Response {
    let request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn exchange(
    app: &Router,
    subject_token: &str,
    subject_type: &str,
) -> axum::response::Response {
    post_form(app, TOKENS, exchange_form(subject_token, subject_type)).await
}

/// `GET path` under the API base with these headers.
async fn api_get(
    app: &Router,
    path: &str,
    headers: &[(header::HeaderName, String)],
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(Method::GET)
        .uri(format!("{BASE}{path}"));
    for (name, value) in headers {
        request = request.header(name, value);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

fn bearer(token: &str) -> Vec<(header::HeaderName, String)> {
    vec![(header::AUTHORIZATION, format!("Bearer {token}"))]
}

#[tokio::test]
async fn token_exchange_issues_uc_access_tokens() {
    let idp = Idp::start().await;
    let (app, metastore) = app(&idp, "$IDP", "uc").await;
    user(&metastore, "alice@example.com", None, true).await;

    let resp = exchange(
        &app,
        &idp.token(json!({ "sub": "alice-id", "email": "alice@example.com" })),
        ID_TOKEN,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().get(header::SET_COOKIE).is_none());
    let body = json_body(resp).await;
    assert_eq!(
        (
            body["issued_token_type"].as_str(),
            body["token_type"].as_str(),
            body["expires_in"].as_i64()
        ),
        (Some(ACCESS_TOKEN), Some("Bearer"), Some(3600))
    );
    let token = body["access_token"].as_str().unwrap().to_string();
    let claims = security().verify(&token).unwrap();
    assert_eq!(
        (
            claims.sub.as_str(),
            claims.iss.as_str(),
            claims.token_type.as_str()
        ),
        ("alice@example.com", "internal", "ACCESS")
    );

    // The UC token signs in.
    assert_eq!(
        api_get(&app, "/catalogs", &bearer(&token)).await.status(),
        StatusCode::OK
    );

    // ES256 and ES512 tokens are accepted too.
    for token in [
        idp.es256(json!({ "email": "alice@example.com" })),
        idp.es512(json!({ "email": "alice@example.com" })),
    ] {
        assert_eq!(
            exchange(&app, &token, ID_TOKEN).await.status(),
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn token_exchange_requests_are_checked() {
    let idp = Idp::start().await;
    let (app, metastore) = app(&idp, "$IDP", "uc").await;
    user(&metastore, "alice@example.com", None, true).await;
    let good = idp.token(json!({ "email": "alice@example.com" }));
    let failing = |body: String, status: StatusCode, code: &'static str, message: String| {
        let app = app.clone();
        async move {
            let err = assert_error(post_form(&app, TOKENS, body).await, status, code).await;
            assert_eq!(err["message"], message);
        }
    };
    let bad = StatusCode::BAD_REQUEST;
    failing(
        form(&[("grant_type", "client_credentials")]),
        bad,
        "INVALID_ARGUMENT",
        "Unsupported grant type: client_credentials".into(),
    )
    .await;
    failing(
        form(&[(
            "grant_type",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        )]),
        bad,
        "INVALID_ARGUMENT",
        "Unsupported requested token type: null".into(),
    )
    .await;
    failing(
        form(&[
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("requested_token_type", ACCESS_TOKEN),
        ]),
        bad,
        "INVALID_ARGUMENT",
        "Subject token type is required but was not specified".into(),
    )
    .await;
    let mut with_actor = exchange_form(&good, ID_TOKEN);
    with_actor.push_str("&actor_token_type=x");
    failing(
        with_actor,
        bad,
        "INVALID_ARGUMENT",
        "Actor tokens not currently supported".into(),
    )
    .await;

    let unauthenticated = StatusCode::UNAUTHORIZED;
    let err = assert_error(
        exchange(&app, "not-a-jwt", ID_TOKEN).await,
        unauthenticated,
        "UNAUTHENTICATED",
    )
    .await;
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .starts_with("Invalid token: "),
        "{err}"
    );
    let foreign =
        idp.token(json!({ "iss": "https://evil.example.com", "email": "alice@example.com" }));
    let err = assert_error(
        exchange(&app, &foreign, ID_TOKEN).await,
        unauthenticated,
        "UNAUTHENTICATED",
    )
    .await;
    assert_eq!(err["message"], "Invalid issuer");
    let mut tampered = good.clone();
    tampered.replace_range(tampered.len() - 4.., "AAAA");
    let err = assert_error(
        exchange(&app, &tampered, ID_TOKEN).await,
        unauthenticated,
        "UNAUTHENTICATED",
    )
    .await;
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .starts_with("Token verification failed: "),
        "{err}"
    );
    let expired = idp.token(json!({ "email": "alice@example.com", "exp": 1 }));
    assert_error(
        exchange(&app, &expired, ID_TOKEN).await,
        unauthenticated,
        "UNAUTHENTICATED",
    )
    .await;
    let err = assert_error(
        exchange(
            &app,
            &idp.token(json!({ "email": "alice@example.com", "aud": "other" })),
            ID_TOKEN,
        )
        .await,
        unauthenticated,
        "UNAUTHENTICATED",
    )
    .await;
    assert_eq!(err["message"], "Invalid audience");

    // Only known, active users, and never the bootstrap admin.
    user(&metastore, "bob@example.com", None, false).await;
    user(&metastore, "admin", None, true).await;
    for email in ["nobody@example.com", "bob@example.com", "admin"] {
        let err = assert_error(
            exchange(&app, &idp.token(json!({ "email": email })), ID_TOKEN).await,
            bad,
            "INVALID_ARGUMENT",
        )
        .await;
        assert_eq!(err["message"], "User not allowed", "{email}");
    }
    // An email that the provider has not verified names no user.
    for verified in [json!(false), json!("false")] {
        let claims = json!({ "email": "alice@example.com", "email_verified": verified });
        let err = assert_error(
            exchange(&app, &idp.token(claims), ID_TOKEN).await,
            bad,
            "INVALID_ARGUMENT",
        )
        .await;
        assert_eq!(err["message"], "User not allowed", "{verified}");
    }
    let verified = json!({ "email": "alice@example.com", "email_verified": true });
    assert_eq!(
        exchange(&app, &idp.token(verified), ID_TOKEN)
            .await
            .status(),
        StatusCode::OK
    );

    // Settings: no issuers; a wildcard issuer with any audience.
    let (no_issuers, _) = self::app(&idp, "", "uc").await;
    let err = assert_error(
        exchange(&no_issuers, &good, ID_TOKEN).await,
        bad,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(
        err["message"],
        "No issuers configured. Add the identity provider as [[auth.issuers]] in uc.toml"
    );
    let (any_audience, metastore) = self::app(&idp, "http://127.0.0.1:*", "*").await;
    user(&metastore, "alice@example.com", None, true).await;
    let other_audience = idp.token(json!({ "email": "alice@example.com", "aud": "other" }));
    assert_eq!(
        exchange(&any_audience, &other_audience, ID_TOKEN)
            .await
            .status(),
        StatusCode::OK
    );

    // Authorization off: no token exchange.
    let off = build_app(AppState::new(
        SqlMetastore::connect(MetastoreConfig::for_tests())
            .await
            .unwrap(),
    ));
    let err = assert_error(
        exchange(&off, &good, ID_TOKEN).await,
        bad,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(err["message"], "Authorization is disabled");
}

#[tokio::test]
async fn oauth_clients_resolve_by_external_id() {
    let idp = Idp::start().await;
    let (app, metastore) = app(&idp, "$IDP", "uc").await;
    user(&metastore, "svc@example.com", Some("client-123"), true).await;
    user(&metastore, "admin", Some("admin-client"), true).await;
    // An access token without `email` names its client first.
    let resp = exchange(
        &app,
        &idp.token(json!({ "sub": "x", "azp": "client-123" })),
        ACCESS_TOKEN,
    )
    .await;
    let token = json_body(resp).await["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(security().verify(&token).unwrap().sub, "svc@example.com");
    let resp = exchange(
        &app,
        &idp.token(json!({ "sub": "x", "client_id": "client-123" })),
        ACCESS_TOKEN,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    // An id token never does, and the admin cannot be reached that way.
    let resp = exchange(
        &app,
        &idp.token(json!({ "sub": "x", "azp": "client-123" })),
        ID_TOKEN,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let resp = exchange(
        &app,
        &idp.token(json!({ "sub": "x", "azp": "admin-client" })),
        ACCESS_TOKEN,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cookie_sign_in_and_sign_out() {
    let idp = Idp::start().await;
    let (app, metastore) = app(&idp, "$IDP", "uc").await;
    user(&metastore, "alice@example.com", None, true).await;
    let resp = post_form(
        &app,
        &format!("{TOKENS}?ext=cookie"),
        exchange_form(
            &idp.token(json!({ "email": "alice@example.com" })),
            ID_TOKEN,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let cookie = resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .to_string();
    // The cookie lasts no longer than the token.
    assert!(
        cookie.starts_with("UC_TOKEN=ey")
            && cookie.contains("Max-Age=600")
            && cookie.contains("Secure"),
        "{cookie}"
    );
    let value = cookie.split(';').next().unwrap().to_string();
    assert_eq!(
        api_get(
            &app,
            "/catalogs",
            &[(header::COOKIE, format!("theme=dark; {value}"))]
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        post_form(&app, &format!("{TOKENS}?ext=jar"), String::new())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );

    // Sign-out expires the cookie.
    let request = Request::builder()
        .method(Method::POST)
        .uri(LOGOUT)
        .header(header::COOKIE, value.clone())
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(request).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let expired = resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        expired.starts_with("UC_TOKEN=;") && expired.contains("Max-Age=0"),
        "{expired}"
    );
    assert_eq!(json_body(resp).await, json!({}));
    // Without a cookie there is nothing to expire; and it needs a sign-in.
    let token = security()
        .create_access_token("alice@example.com", Duration::from_secs(60))
        .unwrap();
    let request = Request::builder()
        .method(Method::POST)
        .uri(LOGOUT)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(request).await.unwrap();
    assert!(resp.headers().get(header::SET_COOKIE).is_none());
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(LOGOUT)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// The upstream React UI's sign-in, in the calls its `AuthProvider` makes:
/// `/scim2/Me` answers 401 until the ID token is exchanged with
/// `ext=cookie`; then the cookie names the user to the UI and the API;
/// logout expires it.
#[tokio::test]
async fn the_upstream_ui_signs_in_with_a_cookie() {
    let idp = Idp::start().await;
    let (app, metastore) = app(&idp, "$IDP", "uc").await;
    metastore
        .scim()
        .create_user(uc_api::ScimUser {
            user_name: "alice@example.com".into(),
            display_name: Some("Alice Liddell".into()),
            emails: Some(vec![uc_api::ScimEmail {
                value: "alice@example.com".into(),
                primary: Some(true),
                r#type: None,
            }]),
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    let me = |cookie: Option<String>| {
        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/api/1.0/unity-control/scim2/Me");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        app.clone().oneshot(request.body(Body::empty()).unwrap())
    };
    assert_eq!(me(None).await.unwrap().status(), StatusCode::UNAUTHORIZED);

    let resp = post_form(
        &app,
        &format!("{TOKENS}?ext=cookie"),
        exchange_form(
            &idp.token(json!({ "email": "alice@example.com" })),
            ID_TOKEN,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let cookie = resp.headers()[header::SET_COOKIE].to_str().unwrap();
    let cookie = cookie.split(';').next().unwrap().to_string();

    let resp = me(Some(cookie.clone())).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let user = json_body(resp).await;
    assert_eq!(user["displayName"], "Alice Liddell");
    assert_eq!(user["emails"][0]["value"], "alice@example.com");
    assert_eq!(
        api_get(&app, "/catalogs", &[(header::COOKIE, cookie.clone())])
            .await
            .status(),
        StatusCode::OK
    );

    let request = Request::builder()
        .method(Method::POST)
        .uri(LOGOUT)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(request).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let expired = resp.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(expired.starts_with("UC_TOKEN=;"), "{expired}");
    assert_eq!(me(None).await.unwrap().status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sign_in_is_required_when_authorization_is_on() {
    let idp = Idp::start().await;
    let (app, metastore) = app(&idp, "$IDP", "uc").await;
    user(&metastore, "alice@example.com", None, true).await;
    user(&metastore, "bob@example.com", None, false).await;

    let err = assert_error(
        api_get(&app, "/catalogs", &[]).await,
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
    )
    .await;
    assert_eq!(err["message"], "No authorization found.");
    let err = assert_error(
        api_get(&app, "/catalogs", &bearer("garbage")).await,
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
    )
    .await;
    assert_eq!(err["message"], "Invalid access token.");
    // The tokens of an issuer without `direct = true` must be exchanged
    // first; so must those of an unknown issuer.
    for iss in [idp.issuer.clone(), "https://other.example.com".to_string()] {
        let raw = idp.token(json!({ "iss": iss, "email": "alice@example.com" }));
        let err = assert_error(
            api_get(&app, "/catalogs", &bearer(&raw)).await,
            StatusCode::UNAUTHORIZED,
            "UNAUTHENTICATED",
        )
        .await;
        assert_eq!(err["message"], "Invalid access token.");
    }
    // Only active users.
    for (email, status) in [
        ("alice@example.com", StatusCode::OK),
        ("bob@example.com", StatusCode::FORBIDDEN),
        ("carol@example.com", StatusCode::FORBIDDEN),
    ] {
        let token = security()
            .create_access_token(email, Duration::from_secs(60))
            .unwrap();
        let resp = api_get(&app, "/catalogs", &bearer(&token)).await;
        assert_eq!(resp.status(), status, "{email}");
        if status == StatusCode::FORBIDDEN {
            assert_eq!(
                json_body(resp).await["message"],
                format!("User not allowed: {email}")
            );
        }
    }
    // Probes need no sign-in.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn tokens_are_optional_when_authorization_is_off() {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let app = build_app(AppState::new(metastore).with_security(security()));
    assert_eq!(
        api_get(&app, "/catalogs", &[]).await.status(),
        StatusCode::OK
    );
    // Upstream ignores tokens then; one that is valid still names the caller.
    assert_eq!(
        api_get(&app, "/catalogs", &bearer("garbage"))
            .await
            .status(),
        StatusCode::OK
    );
    let token = security()
        .create_access_token("alice@example.com", Duration::from_secs(60))
        .unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("{BASE}/catalogs"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "name": "c" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(json_body(resp).await["owner"], "alice@example.com");
    // An anonymous caller is recorded as nobody (upstream: null).
    let resp = app
        .clone()
        .oneshot(common::req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "nobodys" }),
        ))
        .await
        .unwrap();
    let body = json_body(resp).await;
    for field in ["owner", "created_by", "updated_by"] {
        assert!(
            body.get(field).is_none_or(Value::is_null),
            "{field}: {body}"
        );
    }
}
