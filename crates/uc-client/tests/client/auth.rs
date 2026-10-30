//! Sign-in: personal access tokens and the client credentials grant, the
//! token exchange with an identity provider, the sign-in mode, and logout.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{Json, Router, routing::get};
use futures::TryStreamExt;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::json;
use url::Url;

use uc_client::Client;
use uc_server::authn::idp::AuthSettings;
use uc_server::authn::security::{ADMIN, initialize_admin};
use uc_server::{AppState, build_app};

use crate::common::{client, email, metastore, rejected, security, serve, server_with_keys, token};

const ALICE: &str = "alice@example.com";
const TOKEN_EXCHANGE: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";
const ID_TOKEN: &str = "urn:ietf:params:oauth:token-type:id_token";

/// Personal access tokens belong to a signed-in user, also with
/// authorization off. The client sends its token with each request.
#[tokio::test]
async fn personal_access_tokens() {
    let url = server_with_keys().await;
    let anonymous = Client::new_unauthenticated(url.clone());
    let alice = Client::new_with_token(url.clone(), token(ALICE));
    rejected(anonymous.create_token("ci").await, 401, "UNAUTHENTICATED");
    rejected(anonymous.list_tokens().await, 401, "UNAUTHENTICATED");

    let expires_at = now_ms() + 3_600_000;
    let created = alice
        .create_token("ci")
        .with_expires_at(expires_at)
        .await
        .unwrap();
    assert_eq!(created.name, "ci");
    assert_eq!(created.owner, ALICE);
    assert_eq!(created.expires_at, Some(expires_at));
    let secret = created.token_value.clone().expect("the token, once");
    assert!(secret.starts_with(&created.token_prefix));

    // The new token signs in as well; lists leave its value out.
    let with_pat = Client::new_with_token(url.clone(), &secret);
    let listed = with_pat.list_tokens().await.unwrap().tokens;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(listed[0].token_value, None);
    alice.create_token("other").await.unwrap();
    let first = alice.list_tokens().with_max_results(1).await.unwrap();
    let second = alice
        .list_tokens()
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_ne!(first.tokens[0].id, second.tokens[0].id);
    let all: Vec<_> = alice
        .list_tokens()
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(all.len(), 2);

    rejected(
        anonymous.token(&created.id).delete().await,
        401,
        "UNAUTHENTICATED",
    );
    alice.token(&created.id).delete().await.unwrap();
    rejected(alice.token(&created.id).delete().await, 404, "NOT_FOUND");
    assert_eq!(alice.list_tokens().await.unwrap().tokens.len(), 1);
}

/// The client credentials grant takes a form: the secret of a personal
/// access token, and its id.
#[tokio::test]
async fn client_credentials() {
    let url = server_with_keys().await;
    let alice = Client::new_with_token(url.clone(), token(ALICE));
    let pat = alice
        .create_token("ci")
        .with_expires_at(now_ms() + 600_000)
        .await
        .unwrap();
    let secret = pat.token_value.unwrap();
    let anonymous = Client::new_unauthenticated(url);
    let granted = anonymous
        .request_access_token("client_credentials", &secret)
        .with_client_id(pat.id.clone())
        .with_scope("all-apis".to_string())
        .await
        .unwrap();
    assert_eq!(granted.access_token, secret);
    assert_eq!(granted.token_type, "Bearer");
    assert_eq!(granted.scope.as_deref(), Some("all-apis"));
    assert!((1..=600).contains(&granted.expires_in), "{granted:?}");
    let plain = anonymous
        .request_access_token("client_credentials", &secret)
        .await
        .unwrap();
    assert_eq!((plain.expires_in, plain.scope), (3600, None));

    let error = rejected(
        anonymous
            .request_access_token("client_credentials", &secret)
            .with_client_id("not-this-one".to_string())
            .await,
        401,
        "invalid_client",
    );
    assert_eq!(error.message, "client_id does not match secret");
    rejected(
        anonymous.request_access_token("password", &secret).await,
        400,
        "unsupported_grant_type",
    );
}

/// The sign-in mode of a server without sign-in, and logout, which only
/// expires a cookie that the client does not have.
#[tokio::test]
async fn auth_mode_and_logout() {
    let uc = client().await;
    let mode = uc.get_auth_mode().await.unwrap();
    assert_eq!(mode.mode, "off");
    assert_eq!(mode.logout_url, None);
    uc.logout().await.unwrap();
}

/// The token exchange takes a form: an ID token of an identity provider
/// that the server trusts gives a UC access token, which signs in.
#[tokio::test]
async fn token_exchange() {
    let idp = Idp::start().await;
    let url = server_with_idp(&idp).await;
    let anonymous = Client::new_unauthenticated(url.clone());
    let mode = anonymous.get_auth_mode().await.unwrap();
    assert_eq!(mode.mode, "none");
    assert_eq!(mode.logout_url.as_deref(), Some("/auth/logout"));

    let exchanged = anonymous
        .exchange_token(TOKEN_EXCHANGE, ACCESS_TOKEN, idp.id_token(ALICE), ID_TOKEN)
        .with_scope("all-apis".to_string())
        .await
        .unwrap();
    assert_eq!(exchanged.issued_token_type, ACCESS_TOKEN);
    assert_eq!(exchanged.token_type, "Bearer");
    assert_eq!(exchanged.expires_in, 3600);
    let alice = Client::new_with_token(url, exchanged.access_token);
    assert_eq!(alice.get_me().await.unwrap().user_name, ALICE);
    rejected(anonymous.get_me().await, 401, "UNAUTHENTICATED");
}

/// The server reads each field of the form of a token exchange.
#[tokio::test]
async fn token_exchanges_that_the_server_refuses() {
    let idp = Idp::start().await;
    let anonymous = Client::new_unauthenticated(server_with_idp(&idp).await);
    let refused = [
        (
            anonymous.exchange_token(
                "client_credentials",
                ACCESS_TOKEN,
                idp.id_token(ALICE),
                ID_TOKEN,
            ),
            "Unsupported grant type: client_credentials",
        ),
        (
            anonymous.exchange_token(TOKEN_EXCHANGE, ID_TOKEN, idp.id_token(ALICE), ID_TOKEN),
            "Unsupported requested token type: urn:ietf:params:oauth:token-type:id_token",
        ),
        (
            anonymous
                .exchange_token(TOKEN_EXCHANGE, ACCESS_TOKEN, idp.id_token(ALICE), ID_TOKEN)
                .with_actor_token("actor".to_string())
                .with_actor_token_type(ACCESS_TOKEN.to_string()),
            "Actor tokens not currently supported",
        ),
    ];
    for (exchange, message) in refused {
        let error = rejected(exchange.await, 400, "INVALID_ARGUMENT");
        assert_eq!(error.message, message);
    }
    rejected(
        anonymous
            .exchange_token(TOKEN_EXCHANGE, ACCESS_TOKEN, "not-a-token", ID_TOKEN)
            .await,
        401,
        "UNAUTHENTICATED",
    );
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// A server with authorization on that trusts the ID tokens of `idp` for
/// the audience `uc`, with the users `admin` and alice; its root URL.
async fn server_with_idp(idp: &Idp) -> Url {
    let metastore = metastore().await;
    initialize_admin(&metastore).await.unwrap();
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        issuers: vec![uc_common::settings::Issuer {
            name: "idp".into(),
            issuer: idp.issuer.clone(),
            audiences: vec!["uc".into()],
            ..Default::default()
        }],
        access_token_timeout: Duration::from_secs(3600),
        ..Default::default()
    });
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security())
        .with_auth_settings(settings);
    let url = serve(build_app(state)).await;
    Client::new_with_token(url.clone(), token(ADMIN))
        .create_user()
        .with_emails([email(ALICE)])
        .await
        .unwrap();
    url
}

/// The P-256 key of [`Idp`], a key for these tests only, in PKCS #8.
const IDP_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg9bV2OxfT2NY4FhNp
tB1eTLqKzvnAKPsbWpjnyDSUIL2hRANCAASTdscsBJf35ZbWbx/g//Vy1ff3ouV+
+6X+vzSzOilBukWUMgs5eB7pEhpv4eu45k1jCFMdVSMjBNVRdkK0afBc
-----END PRIVATE KEY-----";

/// An identity provider in this process: OIDC discovery and the public
/// key of [`IDP_KEY`] (its `x` and `y`), which signs its ID tokens.
struct Idp {
    issuer: String,
    key: EncodingKey,
}

impl Idp {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let config = json!({ "issuer": issuer, "jwks_uri": format!("{issuer}/jwks") });
        let jwks = json!({ "keys": [{
            "kty": "EC", "kid": "p256", "use": "sig", "alg": "ES256", "crv": "P-256",
            "x": "k3bHLASX9-WW1m8f4P_1ctX396Llfvul_r80szopQbo",
            "y": "RZQyCzl4HukSGm_h67jmTWMIUx1VIyME1VF2QrRp8Fw",
        }] });
        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || async move { Json(config) }),
            )
            .route("/jwks", get(move || async move { Json(jwks) }));
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            issuer,
            key: EncodingKey::from_ec_pem(IDP_KEY.as_bytes()).unwrap(),
        }
    }

    /// An ID token of the user `email`, for the audience `uc`.
    fn id_token(&self, email: &str) -> String {
        let now = now_ms() / 1000;
        let claims = json!({
            "iss": self.issuer, "aud": "uc", "sub": email, "email": email,
            "iat": now, "exp": now + 600,
        });
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some("p256".into());
        jsonwebtoken::encode(&header, &claims, &self.key).unwrap()
    }
}
