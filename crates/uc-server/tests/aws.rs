//! Sign-in with AWS identities: a signed `sts:GetCallerIdentity` request as
//! the bearer value, which the server sends to STS (`[[auth.issuers]]` with
//! `kind = "aws"`). A fake STS answers here with the ARN of each access key.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use serde_json::json;
use tower::ServiceExt;

use uc_api::ScimUser;
use uc_common::settings::{Issuer, IssuerKind};
use uc_metastore::{DynMetastore, MetastoreConfig, SqlMetastore};
use uc_server::{AppState, authn::idp::AuthSettings, build_app};

use common::{BASE, assert_error, json_body, security, sts::Sts};

const ACCOUNT: &str = "123456789012";
const SERVER_ID: &str = "https://uc.example.com";
const ETL: &str = "aws:arn:aws:iam::123456789012:role/etl";

/// The block of `sts`: this account's identities, for this server, made
/// users at their first request.
fn aws(sts: &Sts) -> Issuer {
    Issuer {
        name: "aws".into(),
        kind: IssuerKind::Aws,
        issuer: sts.endpoint.clone(),
        audiences: vec![SERVER_ID.into()],
        accounts: vec![ACCOUNT.into()],
        create_users: true,
        ..Default::default()
    }
}

/// STS with the role `etl` (two sessions), a user of the account, and a
/// role of another account.
async fn sts() -> Sts {
    let sts = Sts::start().await;
    sts.identity(
        "ASIAETL1",
        "arn:aws:sts::123456789012:assumed-role/etl/i-0a1",
    );
    sts.identity(
        "ASIAETL2",
        "arn:aws:sts::123456789012:assumed-role/etl/i-0b2",
    );
    sts.identity("AKIAALICE", "arn:aws:iam::123456789012:user/alice");
    sts.identity(
        "ASIAOTHER",
        "arn:aws:sts::999999999999:assumed-role/etl/i-0c3",
    );
    sts
}

async fn app(issuers: Vec<Issuer>) -> (Router, DynMetastore) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let dyn_metastore: DynMetastore = metastore.clone();
    let settings = AuthSettings::new(&uc_common::settings::Auth {
        issuers,
        ..Default::default()
    });
    settings.validate().unwrap();
    let state = AppState::new(metastore)
        .with_authz(true)
        .with_security(security())
        .with_auth_settings(settings);
    (build_app(state), dyn_metastore)
}

async fn get(app: &Router, token: &str) -> Response {
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("{BASE}/catalogs"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn refused(app: &Router, token: &str) {
    let err = assert_error(
        get(app, token).await,
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
    )
    .await;
    assert_eq!(err["message"], "Invalid access token.");
}

#[tokio::test]
async fn a_role_signs_in_with_a_signed_request() {
    let sts = sts().await;
    let (app, metastore) = app(vec![aws(&sts)]).await;
    let token = sts.token("ASIAETL1", SERVER_ID);
    assert_eq!(get(&app, &token).await.status(), StatusCode::OK);

    // The user is the role, not the session, with the role's ARN.
    let made = metastore
        .scim()
        .get_user_by_name(ETL)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (made.principal(), made.external_id.as_deref()),
        (ETL, Some(ETL))
    );
    // The token is kept: STS is asked once for it.
    assert_eq!(get(&app, &token).await.status(), StatusCode::OK);
    assert_eq!(sts.calls(), 1);
    // Another session of the role is the same user.
    let other = sts.token("ASIAETL2", SERVER_ID);
    assert_eq!(get(&app, &other).await.status(), StatusCode::OK);
    assert_eq!(metastore.scim().list_users(None).await.unwrap().len(), 1);
    // An IAM user is named by its own ARN.
    let alice = sts.token("AKIAALICE", SERVER_ID);
    assert_eq!(get(&app, &alice).await.status(), StatusCode::OK);
    assert!(
        (metastore.scim())
            .get_user_by_name("aws:arn:aws:iam::123456789012:user/alice")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn requests_that_do_not_check_out_are_refused() {
    let sts = sts().await;
    let (app, _) = app(vec![aws(&sts)]).await;

    // Refused before STS is asked: signed for another server, the server
    // id not signed, another endpoint, too old, not GetCallerIdentity.
    refused(&app, &sts.token("ASIAETL1", "https://other.example.com")).await;
    let mut request = sts.request("ASIAETL1", SERVER_ID);
    request.server_id_signed = false;
    refused(&app, &request.token()).await;
    let mut request = sts.request("ASIAETL1", SERVER_ID);
    request.url = "http://169.254.169.254/".into();
    refused(&app, &request.token()).await;
    let mut request = sts.request("ASIAETL1", SERVER_ID);
    request.url = format!("{}/?Action=GetCallerIdentity", sts.endpoint);
    refused(&app, &request.token()).await;
    let mut request = sts.request("ASIAETL1", SERVER_ID);
    request.date -= chrono::Duration::minutes(20);
    refused(&app, &request.token()).await;
    let mut request = sts.request("ASIAETL1", SERVER_ID);
    request.body = "Action=GetSessionToken&Version=2011-06-15".into();
    refused(&app, &request.token()).await;
    refused(&app, "uc-aws-v1.not-json").await;
    assert_eq!(sts.calls(), 0);

    // STS refuses the signature; another account's role.
    refused(&app, &sts.token("ASIAUNKNOWN", SERVER_ID)).await;
    refused(&app, &sts.token("ASIAOTHER", SERVER_ID)).await;
    assert_eq!(sts.calls(), 2);
}

#[tokio::test]
async fn blocks_of_one_server_id_differ_by_account() {
    let sts = sts().await;
    let other = Issuer {
        name: "aws-other".into(),
        accounts: vec!["999999999999".into()],
        ..aws(&sts)
    };
    let (app, metastore) = app(vec![aws(&sts), other]).await;
    for key in ["ASIAETL1", "ASIAOTHER"] {
        assert_eq!(
            get(&app, &sts.token(key, SERVER_ID)).await.status(),
            StatusCode::OK,
            "{key}"
        );
    }
    let scim = metastore.scim();
    for name in [ETL, "aws-other:arn:aws:iam::999999999999:role/etl"] {
        assert!(
            scim.get_user_by_name(name).await.unwrap().is_some(),
            "{name}"
        );
    }
}

#[tokio::test]
async fn a_missing_user_is_refused_unless_made_or_known() {
    let sts = sts().await;
    let (app, metastore) = app(vec![Issuer {
        create_users: false,
        ..aws(&sts)
    }])
    .await;
    let token = sts.token("ASIAETL1", SERVER_ID);
    let err = assert_error(
        get(&app, &token).await,
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
    )
    .await;
    assert_eq!(err["message"], "User not allowed");

    // A user whose external id is the block's name and the role's ARN.
    (metastore.scim())
        .create_user(ScimUser {
            user_name: "etl@example.com".into(),
            emails: Some(vec![uc_api::ScimEmail {
                value: "etl@example.com".into(),
                primary: Some(true),
                r#type: None,
                display: None,
            }]),
            external_id: Some(ETL.into()),
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    let token = sts.token("ASIAETL2", SERVER_ID);
    let request = Request::builder()
        .uri("/api/1.0/unity-control/scim2/Me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(request).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["userName"], "etl@example.com");
}

#[tokio::test]
async fn the_server_tells_its_server_id() {
    let sts = sts().await;
    let (app, _) = app(vec![Issuer {
        issuer: String::new(),
        ..aws(&sts)
    }])
    .await;
    let request = Request::builder()
        .uri("/.well-known/oauth-protected-resource")
        .body(Body::empty())
        .unwrap();
    let body = json_body(app.clone().oneshot(request).await.unwrap()).await;
    // Not an OAuth authorization server.
    assert_eq!(body["authorization_servers"], json!([]));
    assert_eq!(
        body["issuers"],
        json!([{ "name": "aws", "kind": "aws", "issuer": "",
                 "audiences": [SERVER_ID], "direct": true }])
    );
}

#[test]
fn blocks_are_checked() {
    let error = |issuer: Issuer| {
        AuthSettings::new(&uc_common::settings::Auth {
            issuers: vec![issuer],
            ..Default::default()
        })
        .validate()
        .unwrap_err()
    };
    let block = Issuer {
        name: "aws".into(),
        kind: IssuerKind::Aws,
        audiences: vec![SERVER_ID.into()],
        accounts: vec![ACCOUNT.into()],
        ..Default::default()
    };
    assert!(
        error(Issuer {
            accounts: vec![],
            ..block.clone()
        })
        .contains("[0].accounts: ''")
    );
    assert!(
        error(Issuer {
            accounts: vec!["1234".into()],
            ..block.clone()
        })
        .contains("12-digit")
    );
    assert!(
        error(Issuer {
            audiences: vec!["*".into()],
            ..block.clone()
        })
        .contains("[0].audiences: '*'")
    );
    assert!(
        error(Issuer {
            groups_claim: Some("roles".into()),
            ..block.clone()
        })
        .contains("carry no groups")
    );
}
