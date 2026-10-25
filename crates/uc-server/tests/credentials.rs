mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req, req_json};

#[rstest]
#[tokio::test]
async fn create_get_list_update_delete_credential(#[future(awt)] app: Router) {
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({
                "name": "aws-prod",
                "purpose": "STORAGE",
                "aws_iam_role": { "role_arn": "arn:aws:iam::123:role/uc" },
                "comment": "prod",
            }),
        ))
        .await
        .unwrap();
    let info = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(info["name"], "aws-prod");
    assert_eq!(info["purpose"], "STORAGE");
    assert_eq!(info["aws_iam_role"]["role_arn"], "arn:aws:iam::123:role/uc");
    assert!(info["id"].is_string());

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/credentials/aws-prod"))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "get").await;

    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/credentials/aws-prod",
            json!({ "comment": "edited" }),
        ))
        .await
        .unwrap();
    let updated = json_body(assert_status(resp, StatusCode::OK, "patch").await).await;
    assert_eq!(updated["comment"], "edited");

    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/credentials"))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    assert_eq!(list["credentials"].as_array().unwrap().len(), 1);

    let resp = app
        .oneshot(req(Method::DELETE, "/credentials/aws-prod"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

/// The filter runs in the query, so a page holds `max_results` matches.
#[rstest]
#[tokio::test]
async fn purpose_filter_on_list(#[future(awt)] app: Router) {
    for (name, purpose) in [
        ("a-sp", "SERVICE_PRINCIPAL"),
        ("b-storage", "STORAGE"),
        ("c-storage", "STORAGE"),
    ] {
        let resp = app.clone().oneshot(req_json(Method::POST, "/credentials", json!({
            "name": name, "purpose": purpose, "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" },
        }))).await.unwrap();
        assert_status(resp, StatusCode::OK, name).await;
    }

    let resp = app
        .oneshot(req(
            Method::GET,
            "/credentials?purpose=STORAGE&max_results=1",
        ))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    let arr = list["credentials"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "b-storage");
    assert!(list["next_page_token"].is_string());
}

/// Upstream `CredentialDAO`: an AWS IAM role is required, and each role
/// gets a generated external id.
#[rstest]
#[tokio::test]
async fn aws_roles_carry_an_external_id(#[future(awt)] app: Router) {
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({ "name": "none", "purpose": "STORAGE" }),
        ))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Unknown credential type");

    let resp = app.clone().oneshot(req_json(Method::POST, "/credentials", json!({
        "name": "c", "purpose": "STORAGE", "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/a" },
    }))).await.unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    let first_id = created["aws_iam_role"]["external_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(first_id.len(), 36);

    // A new role replaces the old one and gets a new external id.
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/credentials/c",
            json!({ "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/b" } }),
        ))
        .await
        .unwrap();
    let updated = json_body(assert_status(resp, StatusCode::OK, "patch").await).await;
    assert_eq!(updated["aws_iam_role"]["role_arn"], "arn:aws:iam::1:role/b");
    assert_ne!(updated["aws_iam_role"]["external_id"], first_id.as_str());

    let resp = app
        .oneshot(req(Method::GET, "/credentials/c"))
        .await
        .unwrap();
    let got = json_body(assert_status(resp, StatusCode::OK, "get").await).await;
    assert_eq!(got["aws_iam_role"], updated["aws_iam_role"]);
}

#[tokio::test]
async fn aws_roles_show_the_server_role() {
    let metastore = uc_metastore::SqlMetastore::connect(uc_metastore::MetastoreConfig::for_tests())
        .await
        .unwrap();
    let state =
        uc_server::AppState::new(metastore).with_aws_identity_arn("arn:aws:iam::9:role/uc-master");
    let app = uc_server::build_app(state);
    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({
                "name": "c", "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/a" },
            }),
        ))
        .await
        .unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(
        created["aws_iam_role"]["unity_catalog_iam_arn"],
        "arn:aws:iam::9:role/uc-master"
    );
}

#[rstest]
#[tokio::test]
async fn cannot_delete_credential_bound_to_external_location_without_force(
    #[future(awt)] app: Router,
) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({
                "name": "c1", "purpose": "STORAGE",
                "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" },
            }),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "loc1", "url": "s3://b/p", "credential_name": "c1",
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, "/credentials/c1"))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        "Credential still used by external location 'loc1'"
    );

    let resp = app
        .oneshot(req(Method::DELETE, "/credentials/c1?force=true"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(req_json(method, path, body))
        .await
        .unwrap()
}

/// An Azure service principal: answers hold its tenant and application,
/// never its secret, which the database keeps.
#[rstest]
#[tokio::test]
async fn azure_service_principals_keep_their_secret(#[future(awt)] app: Router) {
    let principal =
        json!({ "directory_id": "tenant", "application_id": "app", "client_secret": "s3cret" });
    let resp = send(
        &app,
        Method::POST,
        "/credentials",
        json!({ "name": "sp", "azure_service_principal": principal }),
    )
    .await;
    let info = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(
        info["azure_service_principal"],
        json!({ "directory_id": "tenant", "application_id": "app" })
    );
    assert!(info.get("aws_iam_role").is_none());
    for resp in [
        app.clone()
            .oneshot(req(Method::GET, "/credentials/sp"))
            .await
            .unwrap(),
        app.clone()
            .oneshot(req(Method::GET, "/credentials"))
            .await
            .unwrap(),
    ] {
        let text = json_body(assert_status(resp, StatusCode::OK, "read").await)
            .await
            .to_string();
        assert!(!text.contains("s3cret"), "{text}");
    }
    // Each field is required.
    for field in ["directory_id", "application_id", "client_secret"] {
        let mut principal = principal.clone();
        principal[field] = json!("");
        let resp = send(
            &app,
            Method::POST,
            "/credentials",
            json!({ "name": "bad", "azure_service_principal": principal }),
        )
        .await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], format!("{field} must not be empty"));
    }
}

/// A GCP service account key: answers hold its email and key ID, never the
/// key, which the database keeps.
#[rstest]
#[tokio::test]
async fn gcp_service_account_keys_keep_their_key(#[future(awt)] app: Router) {
    let key = json!({
        "type": "service_account",
        "client_email": "uc@p.iam.gserviceaccount.com",
        "private_key_id": "kid",
        "private_key": "-----BEGIN PRIVATE KEY-----\nPRIVATE\n-----END PRIVATE KEY-----\n",
    });
    let body =
        |key: String| json!({ "name": "key", "gcp_service_account_key": { "json_key": key } });
    let resp = send(&app, Method::POST, "/credentials", body(key.to_string())).await;
    let info = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(
        info["gcp_service_account_key"],
        json!({ "email": "uc@p.iam.gserviceaccount.com", "private_key_id": "kid" })
    );
    for path in ["/credentials/key", "/credentials"] {
        let resp = app.clone().oneshot(req(Method::GET, path)).await.unwrap();
        let text = json_body(assert_status(resp, StatusCode::OK, "read").await)
            .await
            .to_string();
        assert!(!text.contains("PRIVATE"), "{text}");
    }
    let mut user = key.clone();
    user["type"] = json!("authorized_user");
    let mut keyless = key.clone();
    keyless["private_key"] = json!("");
    for (key, reason) in [
        (
            "{".to_string(),
            "EOF while parsing an object at line 1 column 1",
        ),
        (user.to_string(), "its type is not service_account"),
        (keyless.to_string(), "it has no private_key"),
    ] {
        let resp = send(&app, Method::POST, "/credentials", body(key)).await;
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(
            err["message"],
            format!("json_key is not a service account key: {reason}.")
        );
    }
}

/// A GCP service account: the server's own identity, or the email of a
/// service account that it impersonates.
#[rstest]
#[tokio::test]
async fn gcp_service_accounts_name_an_email(#[future(awt)] app: Router) {
    for (name, account) in [
        ("own", json!({})),
        ("sa", json!({ "email": "uc@p.iam.gserviceaccount.com" })),
    ] {
        let resp = send(
            &app,
            Method::POST,
            "/credentials",
            json!({ "name": name, "gcp_service_account": account }),
        )
        .await;
        let info = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
        assert_eq!(info["gcp_service_account"], account);
    }
    let resp = send(
        &app,
        Method::PATCH,
        "/credentials/own",
        json!({ "gcp_service_account": { "email": "../x?y" } }),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Not a service account email: ../x?y");
}

/// A credential holds one identity; a new one replaces it.
#[rstest]
#[tokio::test]
async fn credentials_hold_one_identity(#[future(awt)] app: Router) {
    let resp = send(
        &app,
        Method::POST,
        "/credentials",
        json!({ "name": "none" }),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Unknown credential type");
    let two = json!({
        "name": "two",
        "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" },
        "azure_managed_identity": {},
    });
    let resp = send(&app, Method::POST, "/credentials", two).await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        "Give only one of aws_iam_role, azure_service_principal, azure_managed_identity, \
         gcp_service_account and gcp_service_account_key."
    );

    let resp = send(
        &app,
        Method::POST,
        "/credentials",
        json!({ "name": "mi", "azure_managed_identity": {} }),
    )
    .await;
    let info = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(info["azure_managed_identity"], json!({}));
    let resp = send(
        &app,
        Method::PATCH,
        "/credentials/mi",
        json!({ "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } }),
    )
    .await;
    let info = json_body(assert_status(resp, StatusCode::OK, "update").await).await;
    assert_eq!(info["aws_iam_role"]["role_arn"], "arn:aws:iam::1:role/x");
    assert!(info["aws_iam_role"]["external_id"].is_string());
    assert!(info.get("azure_managed_identity").is_none());
}

/// A location's credential must reach its storage: S3 needs an AWS role,
/// ADLS an Azure identity, GCS a GCP identity. Local paths take any.
#[rstest]
#[tokio::test]
async fn external_locations_need_a_credential_of_their_cloud(#[future(awt)] app: Router) {
    for body in [
        json!({ "name": "aws", "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } }),
        json!({ "name": "azure", "azure_managed_identity": {} }),
        json!({ "name": "gcp", "gcp_service_account": {} }),
    ] {
        let resp = send(&app, Method::POST, "/credentials", body).await;
        assert_status(resp, StatusCode::OK, "credential").await;
    }
    let location = |name: &str, url: &str, credential: &str| json!({ "name": name, "url": url, "credential_name": credential });
    let adls = "abfss://c@acct.dfs.core.windows.net/lake";
    let resp = send(
        &app,
        Method::POST,
        "/external-locations",
        location("el", adls, "aws"),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        format!(
            "Credential aws cannot reach {adls}: it needs an Azure service principal or managed identity."
        )
    );
    let resp = send(
        &app,
        Method::POST,
        "/external-locations",
        location("gs", "gs://bucket/lake", "azure"),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        "Credential azure cannot reach gs://bucket/lake: it needs a GCP service account or \
         service account key."
    );
    for (name, url, credential) in [
        ("el", adls, "azure"),
        ("gs", "gs://bucket/lake", "gcp"),
        ("local", "file:///tmp/uc-lake", "aws"),
    ] {
        let resp = send(
            &app,
            Method::POST,
            "/external-locations",
            location(name, url, credential),
        )
        .await;
        assert_status(resp, StatusCode::OK, "location").await;
    }
    // Neither a new URL nor a new credential may break the pair.
    let resp = send(
        &app,
        Method::PATCH,
        "/external-locations/el",
        json!({ "url": "s3://bucket/lake" }),
    )
    .await;
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    let resp = send(
        &app,
        Method::PATCH,
        "/external-locations/el",
        json!({ "credential_name": "aws" }),
    )
    .await;
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    // Nor a new identity of the credential.
    let resp = send(
        &app,
        Method::PATCH,
        "/credentials/azure",
        json!({ "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } }),
    )
    .await;
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        format!(
            "Credential azure cannot reach {adls}: it needs an Azure service principal or managed identity."
        )
    );
}
