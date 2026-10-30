//! Sign-in with the identity of the environment (`uc_client::identity`):
//! an OAuth client with the client-credentials grant, the tokens of
//! `UC_TOKEN` and `UC_TOKEN_FILE`, and what the server says it takes.

use std::collections::HashMap;
use std::time::Duration;

use axum::Router;
use url::Url;

use uc_client::error::Error;
use uc_client::{Client, identity};
use uc_server::authn::idp::AuthSettings;
use uc_server::authn::security::{ADMIN, initialize_admin};
use uc_server::{AppState, build_app};

use crate::common::{IDP_CLIENT, IDP_SECRET, Idp, metastore, security, serve, token};

/// A server with authorization on that takes `idp`'s tokens for the
/// audience `uc` on every request and makes the users they name; its root
/// URL.
async fn server_with_direct_idp(idp: &Idp) -> Url {
    let metastore = metastore().await;
    initialize_admin(&metastore).await.unwrap();
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        issuers: vec![uc_common::settings::Issuer {
            name: "idp".into(),
            issuer: idp.issuer.clone(),
            audiences: vec!["uc".into()],
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
    serve(build_app(state)).await
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
    let source =
        identity::client_credentials(&idp.token_url(), IDP_CLIENT, IDP_SECRET, None, Some("uc"))
            .unwrap();
    let client = Client::new_with_token_source(url.clone(), source);
    client.list_catalogs().await.unwrap();
    // The server made the user of the client, named by the issuer and the
    // client.
    assert!(user_names(&url).await.contains(&"idp:etl".to_string()));

    // A wrong secret fails the call before any request.
    let wrong =
        identity::client_credentials(&idp.token_url(), IDP_CLIENT, "nope", None, Some("uc"))
            .unwrap();
    let error = Client::new_with_token_source(url.clone(), wrong)
        .list_catalogs()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Token(_)), "{error:?}");
    assert!(
        error.to_string().starts_with("no bearer token: "),
        "{error}"
    );
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
            audiences: vec!["uc".into()],
            direct: true,
        }]
    );
    assert_eq!(resource.authorization_servers, [idp.issuer.as_str()]);
    // A server without the endpoint says nothing.
    let bare = serve(Router::new()).await;
    assert!(identity::metadata(&bare).await.unwrap().is_none());

    // Nothing in the environment: no source.
    let none = identity::detect_in(&HashMap::new(), &url, false, false)
        .await
        .unwrap();
    assert!(none.is_none());

    // An OAuth client in the environment, with the audience of the server's
    // issuer.
    let oauth = env(&[
        ("UC_OAUTH_TOKEN_URL", &idp.token_url()),
        ("UC_OAUTH_CLIENT_ID", IDP_CLIENT),
        ("UC_OAUTH_CLIENT_SECRET", IDP_SECRET),
    ]);
    let source = identity::detect_in(&oauth, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    Client::new_with_token_source(url.clone(), source)
        .list_catalogs()
        .await
        .unwrap();
    assert!(user_names(&url).await.contains(&"idp:etl".to_string()));

    // A fixed token, and a token file.
    let fixed = env(&[("UC_TOKEN", &token(ADMIN))]);
    let source = identity::detect_in(&fixed, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    Client::new_with_token_source(url.clone(), source)
        .list_catalogs()
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("token");
    std::fs::write(&file, token(ADMIN)).unwrap();
    let from_file = env(&[("UC_TOKEN_FILE", file.to_str().unwrap())]);
    let source = identity::detect_in(&from_file, &url, false, false)
        .await
        .unwrap()
        .unwrap();
    Client::new_with_token_source(url, source)
        .list_catalogs()
        .await
        .unwrap();
}
