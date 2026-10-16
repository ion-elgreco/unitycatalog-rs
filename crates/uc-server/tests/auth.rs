mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{BASE, app, assert_status, json_body, req, req_bearer, req_json};

#[rstest]
#[tokio::test]
async fn anonymous_request_still_works_when_no_token_supplied(#[future(awt)] app: Router) {
    let resp = app
        .oneshot(req(Method::GET, "/metastore_summary"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_token_returns_401_when_sign_in_is_required() {
    let (app, _) = common::signed_in_app().await;
    let resp = app
        .oneshot(req_bearer(
            Method::GET,
            "/metastore_summary",
            "definitely-not-a-real-token",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_pat_returns_plaintext_then_can_authenticate_with_it() {
    let (app, metastore) = common::signed_in_app().await;
    let session = common::active_user(&metastore, "alice@example.com").await;
    let mut create = req_json(Method::POST, "/tokens", json!({ "name": "ci-bot" }));
    create.headers_mut().insert(
        "authorization",
        format!("Bearer {session}").parse().unwrap(),
    );
    let resp = app.clone().oneshot(create).await.unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(created["name"], "ci-bot");
    let token = created["token_value"]
        .as_str()
        .expect("token_value present")
        .to_string();
    let prefix = created["token_prefix"].as_str().unwrap();
    assert_eq!(prefix.len(), 8);

    // List PATs - should not include the secret.
    let resp = app
        .clone()
        .oneshot(req_bearer(Method::GET, "/tokens", &token))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    let tokens = list["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(
        tokens[0]["token_value"].is_null(),
        "token_value must not be returned by list"
    );

    // Using the token as Bearer should resolve a principal (200, not 401).
    let resp = app
        .oneshot(req_bearer(Method::GET, "/metastore_summary", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn oauth_token_endpoint_validates_pat_as_client_credentials(#[future(awt)] app: Router) {
    // Create a PAT to use as the client secret.
    let resp = app
        .clone()
        .oneshot(signed_in(
            req_json(Method::POST, "/tokens", json!({ "name": "oauth-test" })),
            "alice@example.com",
        ))
        .await
        .unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    let id = created["id"].as_str().unwrap().to_string();
    let secret = created["token_value"].as_str().unwrap().to_string();

    // The PAT's id and secret in a form; the access token is the secret.
    let body = format!(
        "grant_type=client_credentials&client_id={id}&client_secret={}",
        urlencode(&secret),
    );
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri(format!("{BASE}/oauth/token"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "oauth").await).await;
    assert_eq!(body["access_token"], secret);
    assert_eq!(body["token_type"], "Bearer");

    // HTTP Basic client authentication (RFC 6749 §2.3.1), as upstream's
    // clients send it.
    use base64::Engine as _;
    let basic =
        base64::engine::general_purpose::STANDARD.encode(format!("{id}:{}", urlencode(&secret)));
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri(format!("{BASE}/oauth/token"))
        .header("content-type", "application/x-www-form-urlencoded")
        .header("authorization", format!("Basic {basic}"))
        .body(axum::body::Body::from("grant_type=client_credentials"))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "oauth basic").await).await;
    assert_eq!(body["access_token"], secret);

    // A client_id that names another token → 401.
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri(format!("{BASE}/oauth/token"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(format!(
            "grant_type=client_credentials&client_id=not-this-one&client_secret={}",
            urlencode(&secret)
        )))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Bogus secret → 401 invalid_client.
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri(format!("{BASE}/oauth/token"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(
            "grant_type=client_credentials&client_secret=bogus",
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Wrong grant_type → 400 unsupported_grant_type.
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri(format!("{BASE}/oauth/token"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(
            "grant_type=password&client_secret=x",
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// `request` with a UC access token for `user`.
fn signed_in(
    mut request: axum::http::Request<axum::body::Body>,
    user: &str,
) -> axum::http::Request<axum::body::Body> {
    let token = common::security()
        .create_access_token(user, std::time::Duration::from_secs(60))
        .unwrap();
    request
        .headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    request
}

/// Tokens belong to a signed-in user, also with authorization off; only
/// their owner or an admin revokes them.
#[rstest]
#[tokio::test]
async fn tokens_need_a_signed_in_owner(#[future(awt)] app: Router) {
    for request in [
        req_json(Method::POST, "/tokens", json!({ "name": "x" })),
        req(Method::GET, "/tokens"),
    ] {
        let resp = app.clone().oneshot(request).await.unwrap();
        common::assert_error(resp, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED").await;
    }
    let resp = app
        .clone()
        .oneshot(signed_in(
            req_json(Method::POST, "/tokens", json!({ "name": "alice's" })),
            "alice@example.com",
        ))
        .await
        .unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(created["owner"], "alice@example.com");
    let path = format!("/tokens/{}", created["id"].as_str().unwrap());

    // Bob sees none of Alice's tokens and cannot revoke them.
    let resp = app
        .clone()
        .oneshot(signed_in(req(Method::GET, "/tokens"), "bob@example.com"))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    assert!(list["tokens"].as_array().unwrap().is_empty());
    let resp = app
        .clone()
        .oneshot(signed_in(req(Method::DELETE, &path), "bob@example.com"))
        .await
        .unwrap();
    common::assert_error(resp, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, &path))
        .await
        .unwrap();
    common::assert_error(resp, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED").await;

    // The admin can.
    let resp = app
        .clone()
        .oneshot(common::as_admin(req(Method::DELETE, &path)))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "admin revoke").await;
    let resp = app
        .oneshot(signed_in(req(Method::DELETE, &path), "alice@example.com"))
        .await
        .unwrap();
    common::assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

#[tokio::test]
async fn revoked_token_no_longer_authenticates() {
    let (app, metastore) = common::signed_in_app().await;
    let session = common::active_user(&metastore, "alice@example.com").await;
    let mut create = req_json(Method::POST, "/tokens", json!({ "name": "tmp" }));
    create.headers_mut().insert(
        "authorization",
        format!("Bearer {session}").parse().unwrap(),
    );
    let resp = app.clone().oneshot(create).await.unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    let id = body["id"].as_str().unwrap().to_string();
    let token = body["token_value"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(req_bearer(
            Method::DELETE,
            &format!("/tokens/{id}"),
            &session,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(req_bearer(Method::GET, "/metastore_summary", &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
