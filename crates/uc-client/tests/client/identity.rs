//! Sign-in with the identity of the environment (`uc_client::identity`):
//! an OAuth client with the client-credentials grant, an AWS identity's
//! signed requests, the tokens of `UC_TOKEN` and `UC_TOKEN_FILE`, and what
//! the server says it takes.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::get;
use url::Url;

use uc_client::error::Error;
use uc_client::{Client, identity};
use uc_server::authn::idp::AuthSettings;
use uc_server::authn::security::{ADMIN, initialize_admin};
use uc_server::{AppState, build_app};

use crate::common::{
    IDP_CLIENT, IDP_SECRET, Idp, STS_KEYS, Sts, listen, metastore, security, serve, serve_on, token,
};

/// The URL `url` without its `/`, as a block's audience.
fn audience(url: &Url) -> String {
    url.as_str().trim_end_matches('/').to_string()
}

/// A server with authorization on that takes `idp`'s tokens for the
/// audience `uc`, or its own URL, on every request and makes the users they
/// name; its root URL.
async fn server_with_direct_idp(idp: &Idp) -> Url {
    let (listener, url) = listen().await;
    let metastore = metastore().await;
    initialize_admin(&metastore).await.unwrap();
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        issuers: vec![uc_common::settings::Issuer {
            name: "idp".into(),
            issuer: idp.issuer.clone(),
            audiences: vec!["uc".into(), audience(&url)],
            direct: true,
            create_users: true,
            ..Default::default()
        }],
        access_token_timeout: Duration::from_secs(3600),
        ..Default::default()
    });
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security())
        .with_auth_settings(settings);
    serve_on(listener, build_app(state));
    url
}

fn env(vars: &[(&str, &str)]) -> identity::Env {
    vars.iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

/// The user names of the server at `url`, as the admin sees them.
async fn user_names(url: &Url) -> Vec<String> {
    Client::new_with_token(url.clone(), token(ADMIN))
        .list_users()
        .await
        .unwrap()
        .resources
        .into_iter()
        .map(|user| user.user_name)
        .collect()
}

#[tokio::test]
async fn an_oauth_client_signs_in_with_its_credentials() {
    let idp = Idp::start().await;
    let url = server_with_direct_idp(&idp).await;
    let provider =
        identity::client_credentials(&idp.token_url(), IDP_CLIENT, IDP_SECRET, None, Some("uc"))
            .unwrap();
    let client = Client::new_with_bearer(url.clone(), provider).unwrap();
    client.list_catalogs().await.unwrap();
    // The server made the user of the client, named by the issuer and the
    // client.
    assert!(user_names(&url).await.contains(&"idp:etl".to_string()));

    // A wrong secret fails the call before any request.
    let wrong =
        identity::client_credentials(&idp.token_url(), IDP_CLIENT, "nope", None, Some("uc"))
            .unwrap();
    let error = Client::new_with_bearer(url.clone(), wrong)
        .unwrap()
        .list_catalogs()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Transport(_)), "{error:?}");
    assert!(error.to_string().starts_with("request failed: "), "{error}");
}

#[tokio::test]
async fn the_environment_signs_in_at_what_the_server_takes() {
    let idp = Idp::start().await;
    let url = server_with_direct_idp(&idp).await;

    // The server says what it takes.
    let resource = identity::metadata(&url).await.unwrap().unwrap();
    assert_eq!(
        resource.issuers,
        [identity::Issuer {
            name: "idp".into(),
            issuer: idp.issuer.clone(),
            audiences: vec!["uc".into(), audience(&url)],
            direct: true,
            kind: "oidc".into(),
        }]
    );
    assert_eq!(resource.authorization_servers, [idp.issuer.as_str()]);
    // A server without the endpoint says nothing.
    let bare = serve(Router::new()).await;
    assert!(identity::metadata(&bare).await.unwrap().is_none());
    // A server under a path, without its last `/`.
    let state = AppState::new(metastore().await);
    let nested = Router::new().nest(
        "/uc",
        uc_server::authn::resource::router().with_state(state),
    );
    let below = Url::parse(&format!("{}uc", serve(nested).await)).unwrap();
    assert!(identity::metadata(&below).await.unwrap().is_some());

    // Nothing in the environment: no signer.
    let none = identity::detect_in(&HashMap::new(), &url, false, false)
        .await
        .unwrap();
    assert!(none.is_none());

    // An OAuth client in the environment, with the audience of the server's
    // issuer that is the server's URL.
    let oauth = env(&[
        ("UC_OAUTH_TOKEN_URL", &idp.token_url()),
        ("UC_OAUTH_CLIENT_ID", IDP_CLIENT),
        ("UC_OAUTH_CLIENT_SECRET", IDP_SECRET),
    ]);
    let signer = identity::detect_in(&oauth, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    Client::new_with_signer(url.clone(), signer)
        .unwrap()
        .list_catalogs()
        .await
        .unwrap();
    assert!(user_names(&url).await.contains(&"idp:etl".to_string()));

    // A fixed token, and a token file.
    let fixed = env(&[("UC_TOKEN", &token(ADMIN))]);
    let signer = identity::detect_in(&fixed, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    Client::new_with_signer(url.clone(), signer)
        .unwrap()
        .list_catalogs()
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("token");
    std::fs::write(&file, token(ADMIN)).unwrap();
    let from_file = env(&[("UC_TOKEN_FILE", file.to_str().unwrap())]);
    let signer = identity::detect_in(&from_file, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    Client::new_with_signer(url, signer)
        .unwrap()
        .list_catalogs()
        .await
        .unwrap();
}

/// A server whose metadata answers 503 until `up`, then names an AWS block
/// for its URL, and whose catalogs say how a request signed in; its root
/// URL.
async fn server_that_comes_back(up: Arc<AtomicBool>) -> Url {
    let (listener, url) = listen().await;
    let block = serde_json::json!({ "issuers": [{
        "name": "aws", "kind": "aws", "issuer": "", "audiences": [audience(&url)], "direct": true,
    }]});
    let metadata = get(move |State(up): State<Arc<AtomicBool>>| async move {
        match up.load(Ordering::SeqCst) {
            true => (StatusCode::OK, block.to_string()),
            false => (StatusCode::SERVICE_UNAVAILABLE, String::new()),
        }
    });
    let catalogs = get(|headers: HeaderMap| async move {
        let signed = (headers.get(header::AUTHORIZATION))
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("Bearer uc-aws-v1."));
        let name = if signed { "signed" } else { "none" };
        format!(r#"{{"catalogs":[{{"name":"{name}"}}]}}"#)
    });
    let app = Router::new()
        .route("/.well-known/oauth-protected-resource", metadata)
        .route("/api/2.1/unity-catalog/catalogs", catalogs)
        .with_state(up);
    serve_on(listener, app);
    url
}

#[tokio::test]
async fn a_client_made_while_the_server_is_away_signs_in_when_it_is_back() {
    let up = Arc::new(AtomicBool::new(false));
    let url = server_that_comes_back(up.clone()).await;
    let keys = env(&[
        ("AWS_ACCESS_KEY_ID", "AKIAEXAMPLE"),
        ("AWS_SECRET_ACCESS_KEY", "secret"),
    ]);
    let signer = identity::detect_in(&keys, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    let client = Client::new_with_signer(url, signer).unwrap();
    let name =
        |page: uc_client::models::catalogs::v1::ListCatalogsResponse| page.catalogs[0].name.clone();
    // Until the server says what it takes, the requests go without a token.
    assert_eq!(name(client.list_catalogs().await.unwrap()), "none");
    // Then with the identity of its AWS block.
    up.store(true, Ordering::SeqCst);
    assert_eq!(name(client.list_catalogs().await.unwrap()), "signed");
}

/// The server id of the AWS tests' server.
const SERVER_ID: &str = "https://uc.example.com";

/// A server that takes the AWS identities of the account of [`Sts`],
/// signed for [`SERVER_ID`] or its own URL, and makes the users they name;
/// its root URL.
async fn server_with_aws(sts: &Sts) -> Url {
    let (listener, url) = listen().await;
    let metastore = metastore().await;
    initialize_admin(&metastore).await.unwrap();
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        issuers: vec![uc_common::settings::Issuer {
            name: "aws".into(),
            kind: uc_common::settings::IssuerKind::Aws,
            issuer: sts.endpoint.clone(),
            audiences: vec![SERVER_ID.into(), audience(&url)],
            accounts: vec!["123456789012".into()],
            create_users: true,
            ..Default::default()
        }],
        ..Default::default()
    });
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security())
        .with_auth_settings(settings);
    serve_on(listener, build_app(state));
    url
}

#[tokio::test]
async fn an_aws_role_signs_in_with_a_signed_request() {
    let sts = Sts::start().await;
    let url = server_with_aws(&sts).await;
    let (key_id, secret, _) = STS_KEYS[0];
    let keys = env(&[
        ("AWS_ACCESS_KEY_ID", key_id),
        ("AWS_SECRET_ACCESS_KEY", secret),
        ("AWS_SESSION_TOKEN", "session"),
    ]);

    // The environment's keys at the server's AWS block: its STS endpoint
    // comes from the server, and the server id is the server's URL. STS takes
    // the signature of the request that the server forwards.
    let signer = identity::detect_in(&keys, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    Client::new_with_signer(url.clone(), signer)
        .unwrap()
        .list_catalogs()
        .await
        .unwrap();
    assert!(
        user_names(&url)
            .await
            .contains(&"aws:arn:aws:iam::123456789012:role/etl".to_string())
    );

    // A wrong secret: STS refuses the signature, and so does the server.
    let wrong = env(&[
        ("AWS_ACCESS_KEY_ID", key_id),
        ("AWS_SECRET_ACCESS_KEY", "nope"),
    ]);
    let provider = identity::aws_in(&wrong, SERVER_ID, Some(&sts.endpoint)).unwrap();
    let error = Client::new_with_bearer(url.clone(), provider)
        .unwrap()
        .list_catalogs()
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("Invalid access token"),
        "{error}"
    );
    // Signed for another server: refused before STS is asked.
    let provider =
        identity::aws_in(&keys, "https://other.example.com", Some(&sts.endpoint)).unwrap();
    assert!(
        Client::new_with_bearer(url, provider)
            .unwrap()
            .list_catalogs()
            .await
            .is_err()
    );
}
