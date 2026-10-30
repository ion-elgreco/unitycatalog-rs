//! Sign-in with the tokens of identity providers on every request: the
//! `[[auth.issuers]]` blocks with `direct = true`, as a machine with the
//! identity of its environment signs in, whichever provider gave it.
//!
//! The server has no code per provider: a block names the claims, and the
//! fake provider here issues whatever claims a test asks for. The tokens
//! come in three shapes. Entra's (`oid`, `appid`, `idtyp`, `roles`) walks
//! every step of naming the caller, so most tests use it. Google's
//! (`email`, `email_verified`, `azp`) and a Kubernetes cluster's
//! (`sub = system:serviceaccount:…`) use a block with the default claims.

mod common;

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_api::{PermissionsChange, Privilege, ScimGroup, ScimUser, UpdatePermissions};
use uc_common::{SecurableType, settings::Issuer};
use uc_metastore::{DynMetastore, MetastoreConfig, SqlMetastore};
use uc_server::{AppState, authn::idp::AuthSettings, build_app};

use common::{BASE, assert_error, idp::Idp, json_body, security};

const ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";

/// The block of `idp` as an Entra tenant: tokens for `api://uc` are taken
/// on every request, and missing users are made.
fn entra(idp: &Idp) -> Issuer {
    Issuer {
        name: "entra".into(),
        issuer: idp.issuer.clone(),
        audiences: vec!["api://uc".into()],
        direct: true,
        principal_claims: vec!["email".into(), "preferred_username".into(), "oid".into()],
        client_claims: vec!["azp".into(), "appid".into()],
        groups_claim: Some("roles".into()),
        create_users: true,
        display_name: Some("{app_displayname}".into()),
    }
}

/// The block of `idp` as Google: ID tokens for the audience `uc`, the
/// default claims (`email`, else `sub`), and missing users made.
fn google(idp: &Idp) -> Issuer {
    Issuer {
        name: "google".into(),
        issuer: idp.issuer.clone(),
        audiences: vec!["uc".into()],
        direct: true,
        create_users: true,
        ..Default::default()
    }
}

/// The block of `idp` as a Kubernetes cluster's issuer: projected
/// service-account tokens for the cluster's audience, named by `sub`.
fn kubernetes(idp: &Idp) -> Issuer {
    Issuer {
        name: "k8s".into(),
        issuer: idp.issuer.clone(),
        audiences: vec!["https://kubernetes.default.svc".into()],
        direct: true,
        create_users: true,
        ..Default::default()
    }
}

/// An app with authorization on and these issuer blocks, and its
/// metastore. Accepted tokens are kept `ttl`, else the default.
async fn app(issuers: Vec<Issuer>, ttl: Option<Duration>) -> (Router, DynMetastore) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let dyn_metastore: DynMetastore = metastore.clone();
    let mut settings = AuthSettings::new(&uc_common::settings::Auth {
        issuers,
        ..Default::default()
    });
    settings.validate().unwrap();
    if let Some(ttl) = ttl {
        settings = settings.with_accepted_ttl(ttl);
    }
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security())
        .with_auth_settings(settings);
    (build_app(state), dyn_metastore)
}

/// The claims of an Entra application's own token for `api://uc`, with
/// `extra` over them: the ETL app, object id `7f3e`.
fn app_claims(extra: Value) -> Value {
    let mut claims = json!({
        "aud": "api://uc", "oid": "7f3e", "appid": "etl-app", "azp": "etl-app",
        "idtyp": "app", "app_displayname": "ETL",
    });
    claims
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    claims
}

fn app_token(idp: &Idp, extra: Value) -> String {
    idp.token(app_claims(extra))
}

async fn get(app: &Router, path: &str, token: &str) -> Response {
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("{BASE}{path}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn post(app: &Router, path: &str, token: &str, body: Value) -> Response {
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("{BASE}{path}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

/// A token exchange of the access token `token`.
async fn exchange(app: &Router, token: &str) -> Response {
    let body = serde_urlencoded::to_string([
        (
            "grant_type",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ),
        ("requested_token_type", ACCESS_TOKEN),
        ("subject_token_type", ACCESS_TOKEN),
        ("subject_token", token),
    ])
    .unwrap();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/1.0/unity-control/auth/tokens")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn refused(app: &Router, token: &str, status: StatusCode, code: &str, message: &str) {
    let err = assert_error(get(app, "/catalogs", token).await, status, code).await;
    assert_eq!(err["message"], message);
}

/// An active user `email`, with the client id `external_id`.
async fn user(metastore: &DynMetastore, email: &str, external_id: Option<&str>) {
    metastore
        .scim()
        .create_user(ScimUser {
            user_name: email.to_string(),
            emails: Some(vec![uc_api::ScimEmail {
                value: email.to_string(),
                primary: Some(true),
                r#type: None,
            }]),
            external_id: external_id.map(str::to_string),
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
}

/// `CREATE CATALOG` for `principal`, a user or a group.
async fn may_create_catalogs(metastore: &DynMetastore, principal: &str) {
    metastore
        .permissions()
        .update(
            SecurableType::Metastore,
            "metastore",
            UpdatePermissions {
                changes: vec![PermissionsChange {
                    principal: principal.into(),
                    add: vec![Privilege::CreateCatalog],
                    remove: Vec::new(),
                }],
            },
        )
        .await
        .unwrap();
}

/// The owner of the catalog that `token` makes.
async fn owner_of_a_new_catalog(app: &Router, token: &str, name: &str) -> Value {
    let resp = post(app, "/catalogs", token, json!({ "name": name })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    json_body(resp).await["owner"].clone()
}

#[tokio::test]
async fn an_application_signs_in_with_its_own_token() {
    let idp = Idp::start().await;
    let (app, metastore) = app(vec![entra(&idp)], None).await;
    let token = app_token(&idp, json!({}));
    assert_eq!(
        get(&app, "/catalogs", &token).await.status(),
        StatusCode::OK
    );

    // The user was made at the first request: named by the issuer and the
    // application's object id, with its client id and display name. The
    // store keeps the principal as the email too, as it does for `admin`.
    let scim = metastore.scim();
    let made = scim.get_user_by_name("entra:7f3e").await.unwrap().unwrap();
    assert_eq!(
        (
            made.principal(),
            made.display_name.as_deref(),
            made.external_id.as_deref(),
            made.is_active()
        ),
        ("entra:7f3e", Some("ETL"), Some("etl-app"), true)
    );
    // `/scim2/Me` is that user.
    let request = Request::builder()
        .uri("/api/1.0/unity-control/scim2/Me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(request).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["userName"], "entra:7f3e");

    // Grants go to that name, and what it makes, it owns.
    may_create_catalogs(&metastore, "entra:7f3e").await;
    assert_eq!(
        owner_of_a_new_catalog(&app, &token, "etl").await,
        "entra:7f3e"
    );
    // The provider was asked for its keys once.
    assert_eq!(idp.fetches(), 1);
}

#[tokio::test]
async fn a_missing_user_is_refused_unless_made_or_known_by_its_client_id() {
    let idp = Idp::start().await;
    let (app, metastore) = app(
        vec![Issuer {
            create_users: false,
            ..entra(&idp)
        }],
        None,
    )
    .await;
    let token = app_token(&idp, json!({}));
    refused(
        &app,
        &token,
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
        "User not allowed",
    )
    .await;
    assert!(
        metastore
            .scim()
            .get_user_by_name("entra:7f3e")
            .await
            .unwrap()
            .is_none()
    );

    // A user whose `externalId` is the application's client id is the
    // application.
    user(&metastore, "svc@example.com", Some("etl-app")).await;
    assert_eq!(
        get(&app, "/catalogs", &token).await.status(),
        StatusCode::OK
    );
    may_create_catalogs(&metastore, "svc@example.com").await;
    assert_eq!(
        owner_of_a_new_catalog(&app, &token, "etl").await,
        "svc@example.com"
    );
    // A disabled user is refused.
    user(&metastore, "old@example.com", Some("old-app")).await;
    let scim = metastore.scim();
    let old = scim
        .get_user_by_name("old@example.com")
        .await
        .unwrap()
        .unwrap();
    scim.update_user(
        old.id.as_deref().unwrap(),
        ScimUser {
            active: Some(false),
            ..old.clone()
        },
    )
    .await
    .unwrap();
    let old_token = app_token(
        &idp,
        json!({ "oid": "0ld", "appid": "old-app", "azp": "old-app" }),
    );
    refused(
        &app,
        &old_token,
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
        "User not allowed",
    )
    .await;
}

#[tokio::test]
async fn a_person_is_named_by_email_and_never_by_the_tools_client_id() {
    let idp = Idp::start().await;
    let (app, metastore) = app(
        vec![Issuer {
            create_users: false,
            ..entra(&idp)
        }],
        None,
    )
    .await;
    user(&metastore, "alice@example.com", None).await;
    user(&metastore, "cli@example.com", Some("azure-cli")).await;
    may_create_catalogs(&metastore, "alice@example.com").await;
    may_create_catalogs(&metastore, "cli@example.com").await;

    // Alice's token from `az`: `azp` is the Azure CLI, not Alice.
    let alice = idp.token(json!({
        "aud": "api://uc", "preferred_username": "alice@example.com", "oid": "a1",
        "azp": "azure-cli",
    }));
    assert_eq!(
        owner_of_a_new_catalog(&app, &alice, "alices").await,
        "alice@example.com"
    );
    // Someone unknown with the same tool is nobody, not the CLI's user.
    let nobody = idp.token(json!({
        "aud": "api://uc", "preferred_username": "nobody@example.com", "oid": "n1",
        "azp": "azure-cli",
    }));
    refused(
        &app,
        &nobody,
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
        "User not allowed",
    )
    .await;
    // An email that the provider has not verified names nobody.
    let unverified = idp.token(json!({
        "aud": "api://uc", "email": "alice@example.com", "email_verified": false, "oid": "a1",
    }));
    refused(
        &app,
        &unverified,
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
        "User not allowed",
    )
    .await;
}

#[tokio::test]
async fn tokens_that_do_not_check_out_are_refused() {
    let idp = Idp::start().await;
    let (app, _) = app(vec![entra(&idp)], None).await;
    let now = Idp::now() as i64;
    let invalid = |token: String| {
        let app = app.clone();
        async move {
            refused(
                &app,
                &token,
                StatusCode::UNAUTHORIZED,
                "UNAUTHENTICATED",
                "Invalid access token.",
            )
            .await;
        }
    };
    // Another audience, an unknown issuer, expired, not yet valid, tampered.
    invalid(app_token(&idp, json!({ "aud": "uc" }))).await;
    invalid(app_token(
        &idp,
        json!({ "iss": "https://evil.example.com" }),
    ))
    .await;
    invalid(app_token(&idp, json!({ "exp": now - 120 }))).await;
    invalid(app_token(&idp, json!({ "nbf": now + 120 }))).await;
    let mut tampered = app_token(&idp, json!({}));
    tampered.replace_range(tampered.len() - 4.., "AAAA");
    invalid(tampered).await;
    // 60 seconds of clock skew pass, for every algorithm.
    for token in [
        app_token(&idp, json!({ "exp": now - 30 })),
        app_token(&idp, json!({ "nbf": now + 30 })),
        idp.es256(app_claims(json!({ "exp": now - 30 }))),
        idp.es512(app_claims(json!({ "exp": now - 30 }))),
        idp.es512(app_claims(json!({ "nbf": now + 30 }))),
    ] {
        assert_eq!(
            get(&app, "/catalogs", &token).await.status(),
            StatusCode::OK
        );
    }
    invalid(idp.es512(app_claims(json!({ "exp": now - 120 })))).await;
    // A key the provider does not publish: the token is refused. The keys
    // were fetched less than a minute ago, so they are not fetched again
    // for it.
    let fetches = idp.fetches();
    invalid(idp.token_with_kid("gone", app_claims(json!({})))).await;
    invalid(idp.token_with_kid("gone-too", app_claims(json!({})))).await;
    assert_eq!(idp.fetches(), fetches);
}

#[tokio::test]
async fn exchange_only_issuers_are_refused_on_the_api() {
    let idp = Idp::start().await;
    let (app, metastore) = app(
        vec![Issuer {
            direct: false,
            ..entra(&idp)
        }],
        None,
    )
    .await;
    user(&metastore, "svc@example.com", Some("etl-app")).await;
    let token = app_token(&idp, json!({}));
    refused(
        &app,
        &token,
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
        "Invalid access token.",
    )
    .await;
    // The exchange takes it, and names the application's user.
    let resp = exchange(&app, &token).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let own = json_body(resp).await["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(security().verify(&own).unwrap().sub, "svc@example.com");
    assert_eq!(get(&app, "/catalogs", &own).await.status(), StatusCode::OK);
}

#[tokio::test]
async fn each_issuer_has_its_own_audience() {
    let (a, b) = (Idp::start().await, Idp::start().await);
    let block = |name: &str, idp: &Idp| Issuer {
        name: name.into(),
        issuer: idp.issuer.clone(),
        audiences: vec![format!("uc-{name}")],
        direct: true,
        create_users: true,
        ..Default::default()
    };
    let (app, metastore) = app(vec![block("a", &a), block("b", &b)], None).await;
    // Each provider's token, for its own audience; `sub` names the user
    // with the provider's name.
    for (idp, aud) in [(&a, "uc-a"), (&b, "uc-b")] {
        let token = idp.token(json!({ "aud": aud, "sub": "x" }));
        assert_eq!(
            get(&app, "/catalogs", &token).await.status(),
            StatusCode::OK
        );
    }
    let scim = metastore.scim();
    for name in ["a:x", "b:x"] {
        assert!(
            scim.get_user_by_name(name).await.unwrap().is_some(),
            "{name}"
        );
    }
    // A token of one with the audience of the other is refused, on the
    // API and at the exchange.
    for (idp, aud) in [(&a, "uc-b"), (&b, "uc-a")] {
        let token = idp.token(json!({ "aud": aud, "sub": "x" }));
        refused(
            &app,
            &token,
            StatusCode::UNAUTHORIZED,
            "UNAUTHENTICATED",
            "Invalid access token.",
        )
        .await;
        let err = assert_error(
            exchange(&app, &token).await,
            StatusCode::UNAUTHORIZED,
            "UNAUTHENTICATED",
        )
        .await;
        assert_eq!(err["message"], "Invalid audience");
    }
}

#[tokio::test]
async fn the_groups_of_a_token_count_when_the_group_exists() {
    let idp = Idp::start().await;
    let (app, metastore) = app(vec![entra(&idp)], None).await;
    metastore
        .scim()
        .create_group(ScimGroup {
            display_name: "writers".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    user(&metastore, "alice@example.com", None).await;
    may_create_catalogs(&metastore, "writers").await;
    may_create_catalogs(&metastore, "alice@example.com").await;

    // A role that is a group of the server gives its grants.
    let writer = app_token(&idp, json!({ "roles": ["writers"] }));
    assert_eq!(
        owner_of_a_new_catalog(&app, &writer, "written").await,
        "entra:7f3e"
    );
    // A role that no group has counts for nothing; nor does a user's name.
    for (oid, roles) in [
        ("r1", json!(["readers"])),
        ("i1", json!(["alice@example.com"])),
    ] {
        let token = app_token(&idp, json!({ "oid": oid, "roles": roles }));
        let resp = post(&app, "/catalogs", &token, json!({ "name": oid })).await;
        let err = assert_error(resp, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
        assert_eq!(err["message"], "Access denied.");
    }
}

#[tokio::test]
async fn accepted_tokens_are_kept_until_the_cap() {
    let idp = Idp::start().await;
    let (app, metastore) = app(vec![entra(&idp)], Some(Duration::from_secs(1))).await;
    let token = app_token(&idp, json!({}));
    assert_eq!(
        get(&app, "/catalogs", &token).await.status(),
        StatusCode::OK
    );

    // The user is disabled: the token passes until the cap, then not.
    let scim = metastore.scim();
    let made = scim.get_user_by_name("entra:7f3e").await.unwrap().unwrap();
    scim.update_user(
        made.id.as_deref().unwrap(),
        ScimUser {
            active: Some(false),
            ..made.clone()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        get(&app, "/catalogs", &token).await.status(),
        StatusCode::OK
    );
    tokio::time::sleep(Duration::from_millis(1300)).await;
    refused(
        &app,
        &token,
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
        "User not allowed",
    )
    .await;
}

#[tokio::test]
async fn the_server_tells_what_it_takes() {
    let idp = Idp::start().await;
    let ferriskey = Issuer {
        name: "ferriskey".into(),
        issuer: "https://*.example.com".into(),
        audiences: vec!["unitycatalog".into()],
        ..Default::default()
    };
    let (app, _) = app(vec![entra(&idp), ferriskey], None).await;
    // No sign-in needed.
    let request = Request::builder()
        .uri("/.well-known/oauth-protected-resource")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(request).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["authorization_servers"], json!([idp.issuer]));
    assert_eq!(body["bearer_methods_supported"], json!(["header"]));
    assert_eq!(
        body["issuers"],
        json!([
            { "name": "entra", "issuer": idp.issuer, "audiences": ["api://uc"], "direct": true },
            { "name": "ferriskey", "issuer": "https://*.example.com",
              "audiences": ["unitycatalog"], "direct": false },
        ])
    );
    // Without a public URL, no resource.
    assert!(body.get("resource").is_none());
}

#[tokio::test]
async fn a_google_service_account_signs_in_with_an_id_token() {
    let idp = Idp::start().await;
    let (app, metastore) = app(vec![google(&idp)], None).await;
    // Google names the account by its email; `azp` is the OAuth client,
    // which the default block does not read.
    let token = idp.token(json!({
        "aud": "uc", "sub": "104692", "email": "etl@project.iam.gserviceaccount.com",
        "email_verified": true, "azp": "104692",
    }));
    assert_eq!(
        get(&app, "/catalogs", &token).await.status(),
        StatusCode::OK
    );
    let made = metastore
        .scim()
        .get_user_by_name("etl@project.iam.gserviceaccount.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (made.principal(), made.external_id.as_deref()),
        ("etl@project.iam.gserviceaccount.com", None)
    );
    may_create_catalogs(&metastore, "etl@project.iam.gserviceaccount.com").await;
    assert_eq!(
        owner_of_a_new_catalog(&app, &token, "etl").await,
        "etl@project.iam.gserviceaccount.com"
    );
    // An unknown audience is refused: a token for another Google client.
    refused(
        &app,
        &idp.token(json!({ "aud": "other", "email": "etl@project.iam.gserviceaccount.com" })),
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
        "Invalid access token.",
    )
    .await;
}

#[tokio::test]
async fn a_kubernetes_service_account_signs_in_with_its_projected_token() {
    let idp = Idp::start().await;
    let (app, metastore) = app(vec![kubernetes(&idp)], None).await;
    // A projected token: `sub` is the service account, with no email.
    let token = idp.token(json!({
        "aud": "https://kubernetes.default.svc", "sub": "system:serviceaccount:data:etl",
        "kubernetes.io": { "namespace": "data", "serviceaccount": { "name": "etl" } },
    }));
    assert_eq!(
        get(&app, "/catalogs", &token).await.status(),
        StatusCode::OK
    );
    // Named by the block and the subject.
    let principal = "k8s:system:serviceaccount:data:etl";
    assert!(
        metastore
            .scim()
            .get_user_by_name(principal)
            .await
            .unwrap()
            .is_some()
    );
    may_create_catalogs(&metastore, principal).await;
    assert_eq!(owner_of_a_new_catalog(&app, &token, "etl").await, principal);
}
