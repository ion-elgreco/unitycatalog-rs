//! Cloud credential vending, after upstream's vendor tests: the vendor of
//! the path's scheme, external-location credentials, per-bucket
//! configuration, and the requests each cloud receives (served here).

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::IntoResponse,
};
use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey};
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{
    AppState, build_app,
    storage::vendor::{CloudVendor, Endpoints, StorageConfig},
};

use common::{assert_error, assert_status, json_body, req_json};

/// A request a mock cloud received.
#[derive(Debug, Clone)]
struct Seen {
    path: String,
    headers: HeaderMap,
    body: String,
}

impl Seen {
    fn form(&self) -> HashMap<String, String> {
        serde_urlencoded::from_str(&self.body).unwrap()
    }
}

type Log = Arc<Mutex<Vec<Seen>>>;

/// A mock cloud endpoint answering every request with `answer`.
async fn mock(content_type: &'static str, answer: String) -> (String, Log) {
    let log: Log = Arc::default();
    let seen = log.clone();
    let app = Router::new().fallback(move |uri: Uri, headers: HeaderMap, body: Bytes| {
        let seen = seen.clone();
        let answer = answer.clone();
        async move {
            let path = uri
                .path_and_query()
                .map(|p| p.to_string())
                .unwrap_or_default();
            seen.lock().unwrap().push(Seen {
                path,
                headers,
                body: String::from_utf8_lossy(&body).into_owned(),
            });
            ([("content-type", content_type)], answer).into_response()
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, log)
}

/// The app with the credentials of the settings `settings` (uc.toml).
async fn app(settings: &str, endpoints: Endpoints) -> Router {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let settings = uc_common::settings::Settings::from_toml(settings).unwrap();
    let config = StorageConfig::from_settings(&settings).unwrap();
    let state =
        AppState::new(metastore).with_vendor(CloudVendor::new(config).with_endpoints(endpoints));
    build_app(state)
}

async fn post(app: &Router, path: &str, body: Value) -> axum::response::Response {
    app.clone()
        .oneshot(req_json(Method::POST, path, body))
        .await
        .unwrap()
}

async fn vend(app: &Router, url: &str, operation: &str) -> axum::response::Response {
    post(
        app,
        "/temporary-path-credentials",
        json!({ "url": url, "operation": operation }),
    )
    .await
}

async fn ok(resp: axum::response::Response) -> Value {
    json_body(assert_status(resp, StatusCode::OK, "vend").await).await
}

/// The AWS role of a credential.
fn aws_role() -> Value {
    json!({ "aws_iam_role": { "role_arn": "arn:aws:iam::123456789012:role/data" } })
}

/// Credential `name` with `identity`, such as [`aws_role`], and external
/// location `name` at `url`.
async fn external_location(app: &Router, name: &str, url: &str, identity: Value) -> Value {
    let mut body = json!({ "name": name, "purpose": "STORAGE" });
    body.as_object_mut()
        .unwrap()
        .extend(identity.as_object().unwrap().clone());
    let credential = json_body(
        assert_status(
            post(app, "/credentials", body).await,
            StatusCode::OK,
            "credential",
        )
        .await,
    )
    .await;
    let location = json!({ "name": name, "url": url, "credential_name": name });
    assert_status(
        post(app, "/external-locations", location).await,
        StatusCode::OK,
        "location",
    )
    .await;
    credential
}

const ASSUME_ROLE: &str = r#"<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
  <AssumeRoleResult>
    <AssumedRoleUser><AssumedRoleId>AROA:uc-x</AssumedRoleId><Arn>arn:aws:sts::1:assumed-role/data/uc-x</Arn></AssumedRoleUser>
    <Credentials>
      <AccessKeyId>ASIAMOCK</AccessKeyId>
      <SecretAccessKey>mock-secret</SecretAccessKey>
      <SessionToken>mock-session</SessionToken>
      <Expiration>2030-01-01T00:00:00Z</Expiration>
    </Credentials>
  </AssumeRoleResult>
  <ResponseMetadata><RequestId>1</RequestId></ResponseMetadata>
</AssumeRoleResponse>"#;

fn sts_policy(seen: &Seen) -> Value {
    serde_json::from_str(&seen.form()["Policy"]).unwrap()
}

/// Upstream `AwsCredentialVendor`: under an external location the server's
/// identity assumes the credential's role with its external id.
#[tokio::test]
async fn s3_under_an_external_location_assumes_its_role() {
    let (sts, log) = mock("text/xml", ASSUME_ROLE.to_string()).await;
    let master = r#"
        [identity.aws]
        region = "us-west-2"
        access_key = "AKMASTER"
        secret_key = "master-secret"
    "#;
    let app = app(
        master,
        Endpoints {
            aws_sts: Some(sts),
            ..Default::default()
        },
    )
    .await;
    let credential = external_location(&app, "el", "s3://bucket/data", aws_role()).await;

    let body = ok(vend(&app, "s3://bucket/data/t1", "PATH_READ").await).await;
    assert_eq!(
        body["aws_temp_credentials"],
        json!({
        "access_key_id": "ASIAMOCK", "secret_access_key": "mock-secret", "session_token": "mock-session" })
    );
    assert_eq!(body["expiration_time"], 1_893_456_000_000_i64);
    assert_eq!(body["url"], "s3://bucket/data/t1");

    let seen = log.lock().unwrap()[0].clone();
    let form = seen.form();
    assert_eq!(form["Action"], "AssumeRole");
    assert_eq!(form["RoleArn"], "arn:aws:iam::123456789012:role/data");
    assert_eq!(
        form["ExternalId"],
        credential["aws_iam_role"]["external_id"].as_str().unwrap()
    );
    assert_eq!(form["DurationSeconds"], "3600");
    assert!(form["RoleSessionName"].starts_with("uc-"));
    assert!(
        seen.headers["authorization"]
            .to_str()
            .unwrap()
            .contains("Credential=AKMASTER/")
    );
    let policy = sts_policy(&seen);
    assert_eq!(policy["Statement"][0]["Action"], json!(["s3:GetObject"]));
    assert_eq!(
        policy["Statement"][0]["Resource"],
        json!([
            "arn:aws:s3:::bucket/data/t1/*",
            "arn:aws:s3:::bucket/data/t1"
        ])
    );

    // Writes widen the policy.
    ok(vend(&app, "s3://bucket/data/t1", "PATH_READ_WRITE").await).await;
    let policy = sts_policy(&log.lock().unwrap()[1]);
    assert!(
        policy["Statement"][0]["Action"]
            .as_array()
            .unwrap()
            .contains(&json!("s3:PutObject"))
    );
}

/// `identity.aws.endpoint`: an S3-compatible store, such as RustFS. AssumeRole goes
/// to it, signed for `us-east-1`, and its session policy has no KMS
/// statement, which such a store does not know.
#[tokio::test]
async fn s3_compatible_store_gets_sts_calls_without_kms() {
    let (sts, log) = mock("text/xml", ASSUME_ROLE.to_string()).await;
    let settings = format!(
        r#"
        [identity.aws]
        endpoint = "{sts}/"
        access_key = "AKSTORE"
        secret_key = "store-secret"
    "#
    );
    let app = app(&settings, Endpoints::default()).await;
    external_location(&app, "el", "s3://bucket/data", aws_role()).await;

    let body = ok(vend(&app, "s3://bucket/data/t1", "PATH_READ_WRITE").await).await;
    assert_eq!(body["aws_temp_credentials"]["access_key_id"], "ASIAMOCK");
    let seen = log.lock().unwrap()[0].clone();
    assert_eq!(seen.form()["Action"], "AssumeRole");
    let authorization = seen.headers["authorization"].to_str().unwrap();
    assert!(
        authorization.contains("Credential=AKSTORE/"),
        "{authorization}"
    );
    assert!(authorization.contains("/us-east-1/sts/"), "{authorization}");
    let policy = sts_policy(&seen);
    assert_eq!(policy["Statement"].as_array().unwrap().len(), 2);
    assert!(!policy.to_string().contains("kms:"), "{policy}");

    let settings =
        uc_common::settings::Settings::from_toml("[identity.aws]\nendpoint = \"localhost:9000\"")
            .unwrap();
    let error = StorageConfig::from_settings(&settings).unwrap_err();
    assert!(
        error.starts_with("Invalid value for identity.aws.endpoint (UC_IDENTITY_AWS_ENDPOINT)"),
        "{error}"
    );
}

/// Without an external location, the longest `[locations.<name>]` that
/// covers the path: the server's identity assumes its role, or with
/// `aws_iam_role_arn = true` the server's own role.
#[tokio::test]
async fn s3_without_an_external_location_uses_the_settings_location() {
    let (sts, log) = mock("text/xml", ASSUME_ROLE.to_string()).await;
    let settings = r#"
        [identity.aws]
        region = "eu-west-1"
        access_key = "AKSERVER"
        secret_key = "server-secret"
        arn = "arn:aws:iam::1:role/uc-server"

        [locations.own]
        url = "s3://own-bucket"
        aws_iam_role_arn = true

        [locations.bucket]
        url = "s3://role-bucket/"
        aws_iam_role_arn = "arn:aws:iam::1:role/bucket"

        [locations.tables]
        url = "s3://role-bucket/tables"
        aws_iam_role_arn = "arn:aws:iam::1:role/tables"
    "#;
    let app = app(
        settings,
        Endpoints {
            aws_sts: Some(sts),
            ..Default::default()
        },
    )
    .await;

    let body = ok(vend(&app, "s3://role-bucket/t", "PATH_READ").await).await;
    assert_eq!(body["aws_temp_credentials"]["access_key_id"], "ASIAMOCK");
    let seen = log.lock().unwrap()[0].clone();
    assert_eq!(seen.form()["RoleArn"], "arn:aws:iam::1:role/bucket");
    assert!(!seen.form().contains_key("ExternalId"));
    let authorization = seen.headers["authorization"].to_str().unwrap().to_string();
    assert!(
        authorization.contains("Credential=AKSERVER/") && authorization.contains("/eu-west-1/sts/"),
        "{authorization}"
    );

    ok(vend(&app, "s3://role-bucket/tables/t", "PATH_READ").await).await;
    let seen = log.lock().unwrap()[1].clone();
    assert_eq!(seen.form()["RoleArn"], "arn:aws:iam::1:role/tables");

    ok(vend(&app, "s3://own-bucket/t", "PATH_READ").await).await;
    let seen = log.lock().unwrap()[2].clone();
    assert_eq!(seen.form()["RoleArn"], "arn:aws:iam::1:role/uc-server");

    let err = assert_error(
        vend(&app, "s3://unknown/t", "PATH_READ").await,
        StatusCode::BAD_REQUEST,
        "FAILED_PRECONDITION",
    )
    .await;
    assert_eq!(
        err["message"],
        "No external location and no [locations.<name>] of the settings covers s3://unknown/t."
    );
}

/// A user delegation key, as the blob service answers it.
const DELEGATION_KEY: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?><UserDelegationKey>\
    <SignedOid>oid</SignedOid><SignedTid>tid</SignedTid><SignedStart>2030-01-01T00:00:00Z</SignedStart>\
    <SignedExpiry>2030-01-01T01:00:00Z</SignedExpiry><SignedService>b</SignedService>\
    <SignedVersion>2021-08-06</SignedVersion><Value>c2VjcmV0</Value></UserDelegationKey>";

/// Upstream `AzureCredentialVendor`: the location's service principal gets
/// a user delegation key and signs a directory SAS for the path.
#[tokio::test]
async fn azure_signs_a_user_delegation_sas() {
    let (login, logins) = mock(
        "application/json",
        json!({ "access_token": "aad-token", "token_type": "Bearer", "expires_in": 3599 })
            .to_string(),
    )
    .await;
    let (blob, keys) = mock("application/xml", DELEGATION_KEY.to_string()).await;
    let settings = r#"
        [locations.acct]
        url = "abfss://container@acct.dfs.core.windows.net"
        azure_directory_id = "tenant"
        azure_application_id = "client"
        azure_client_secret = "secret"
    "#;
    let endpoints = Endpoints {
        azure_login: Some(login),
        azure_blob: Some(blob),
        ..Default::default()
    };
    let app = app(settings, endpoints).await;

    let body = ok(vend(
        &app,
        "abfss://container@acct.dfs.core.windows.net/tables/t1",
        "PATH_READ_WRITE",
    )
    .await)
    .await;
    let sas = body["azure_user_delegation_sas"]["sas_token"]
        .as_str()
        .unwrap()
        .to_string();
    let params: HashMap<String, String> = serde_urlencoded::from_str(&sas).unwrap();
    assert_eq!(
        (
            params["sp"].as_str(),
            params["sr"].as_str(),
            params["sdd"].as_str()
        ),
        ("rwdl", "d", "2")
    );
    assert_eq!(
        (
            params["skoid"].as_str(),
            params["sktid"].as_str(),
            params["skv"].as_str()
        ),
        ("oid", "tid", "2021-08-06")
    );
    let delegation_key = uc_server::storage::vendor::azure::DelegationKey {
        signed_oid: "oid".into(),
        signed_tid: "tid".into(),
        signed_start: "2030-01-01T00:00:00Z".into(),
        signed_expiry: "2030-01-01T01:00:00Z".into(),
        signed_service: "b".into(),
        signed_version: "2021-08-06".into(),
        value: "c2VjcmV0".into(),
    };
    let expected = uc_server::storage::vendor::azure::user_delegation_sas(
        &delegation_key,
        "acct",
        "container",
        "tables/t1",
        "rwdl",
        &params["st"],
        &params["se"],
    )
    .unwrap();
    assert_eq!(sas, expected);
    let expiry = chrono::DateTime::parse_from_rfc3339(&params["se"])
        .unwrap()
        .timestamp_millis();
    assert_eq!(body["expiration_time"], expiry);

    let signed_in = logins.lock().unwrap()[0].clone();
    assert_eq!(signed_in.path, "/tenant/oauth2/v2.0/token");
    assert_eq!(signed_in.form()["grant_type"], "client_credentials");
    assert_eq!(
        signed_in.form()["scope"],
        "https://storage.azure.com/.default"
    );
    let asked = keys.lock().unwrap()[0].clone();
    assert_eq!(asked.path, "/?restype=service&comp=userdelegationkey");
    assert_eq!(asked.headers["authorization"], "Bearer aad-token");
    assert_eq!(asked.headers["x-ms-version"], "2021-08-06");
    assert!(
        asked
            .body
            .contains(&format!("<Start>{}</Start>", params["st"]))
    );

    // Reads list and read.
    let body = ok(vend(
        &app,
        "abfss://container@acct.dfs.core.windows.net/tables/t1",
        "PATH_READ",
    )
    .await)
    .await;
    assert!(
        body["azure_user_delegation_sas"]["sas_token"]
            .as_str()
            .unwrap()
            .contains("sp=rl&")
    );
    let err = assert_error(
        vend(&app, "abfss://c@other.dfs.core.windows.net/x", "PATH_READ").await,
        StatusCode::BAD_REQUEST,
        "FAILED_PRECONDITION",
    )
    .await;
    assert_eq!(
        err["message"],
        "No external location and no [locations.<name>] of the settings covers \
         abfss://c@other.dfs.core.windows.net/x."
    );

    // Under an external location, its credential's service principal signs
    // in.
    let principal = json!({ "azure_service_principal": {
        "directory_id": "el-tenant", "application_id": "el-client", "client_secret": "el-secret" } });
    let url = "abfss://c@acct.dfs.core.windows.net/el";
    let credential = external_location(&app, "el", url, principal).await;
    assert_eq!(
        credential["azure_service_principal"],
        json!({ "directory_id": "el-tenant", "application_id": "el-client" })
    );
    let body = ok(vend(&app, &format!("{url}/x"), "PATH_READ").await).await;
    let sas = body["azure_user_delegation_sas"]["sas_token"]
        .as_str()
        .unwrap();
    assert!(sas.contains("sp=rl&"), "{sas}");
    let signed_in = logins.lock().unwrap().last().unwrap().clone();
    assert_eq!(signed_in.path, "/el-tenant/oauth2/v2.0/token");
    let form = signed_in.form();
    assert_eq!(
        (form["client_id"].as_str(), form["client_secret"].as_str()),
        ("el-client", "el-secret")
    );
    // A location of another account: the credential reaches any.
    let other = "abfss://c@other.dfs.core.windows.net/el";
    let location = json!({ "name": "other", "url": other, "credential_name": "el" });
    assert_status(
        post(&app, "/external-locations", location).await,
        StatusCode::OK,
        "location",
    )
    .await;
    ok(vend(&app, &format!("{other}/x"), "PATH_READ").await).await;
}

/// A managed identity is a user-assigned one by its resource ID, from the
/// instance metadata service, or else the server's own identity:
/// `[identity.azure]`, else the identity of the environment. A credential or
/// a `[locations.<name>]` names it.
#[tokio::test]
async fn azure_managed_identities_sign_user_delegation_sas() {
    let token = json!({ "access_token": "msi-token", "expires_on": "4102444800" });
    let (msi, tokens) = mock("application/json", token.to_string()).await;
    let (blob, keys) = mock("application/xml", DELEGATION_KEY.to_string()).await;
    let endpoints = Endpoints {
        azure_msi: Some(format!("{msi}/metadata/identity/oauth2/token")),
        azure_blob: Some(blob),
        ..Default::default()
    };
    let settings = r#"
        [locations.lake]
        url = "abfss://c@lake.dfs.core.windows.net"
        azure_managed_identity = "/subscriptions/s/lake"
    "#;
    let app = app(settings, endpoints.clone()).await;
    let id = "/subscriptions/s/resourceGroups/g/providers/Microsoft.ManagedIdentity/userAssignedIdentities/uc";
    let url = "abfss://c@acct.dfs.core.windows.net/mi";
    let identity = json!({ "azure_managed_identity": { "managed_identity_id": id } });
    let credential = external_location(&app, "mi", url, identity).await;
    assert_eq!(
        credential["azure_managed_identity"],
        json!({ "managed_identity_id": id })
    );

    let body = ok(vend(&app, &format!("{url}/t"), "PATH_READ_WRITE").await).await;
    let sas = body["azure_user_delegation_sas"]["sas_token"]
        .as_str()
        .unwrap();
    assert!(sas.contains("sp=rwdl&"), "{sas}");
    let asked = tokens.lock().unwrap()[0].clone();
    let (path, query) = asked.path.split_once('?').unwrap();
    assert_eq!(path, "/metadata/identity/oauth2/token");
    let query: HashMap<String, String> = serde_urlencoded::from_str(query).unwrap();
    assert_eq!(query["resource"], "https://storage.azure.com");
    assert_eq!(query["msi_res_id"], id);
    assert_eq!(asked.headers["metadata"], "true");
    assert_eq!(
        keys.lock().unwrap()[0].headers["authorization"],
        "Bearer msi-token"
    );

    // Without a resource ID: the environment's identity, here the instance
    // metadata service's default one.
    let url = "abfss://c@acct.dfs.core.windows.net/own";
    external_location(&app, "own", url, json!({ "azure_managed_identity": {} })).await;
    ok(vend(&app, &format!("{url}/t"), "PATH_READ").await).await;
    let asked = tokens.lock().unwrap().last().unwrap().clone();
    assert!(!asked.path.contains("msi_res_id"), "{}", asked.path);

    // Without an external location: the location of the settings.
    let url = "abfss://c@lake.dfs.core.windows.net/tables/t";
    ok(vend(&app, url, "PATH_READ").await).await;
    let asked = tokens.lock().unwrap().last().unwrap().clone();
    assert!(
        asked
            .path
            .contains("msi_res_id=%2Fsubscriptions%2Fs%2Flake"),
        "{}",
        asked.path
    );

    // With `[identity.azure]`, the server's own identity is that one.
    let settings = r#"
        [identity.azure]
        managed_identity_id = "/subscriptions/s/server"

        [locations.lake]
        url = "abfss://c@lake.dfs.core.windows.net"
        azure_managed_identity = true
    "#;
    let server = crate::app(settings, endpoints).await;
    ok(vend(
        &server,
        "abfss://c@lake.dfs.core.windows.net/t",
        "PATH_READ",
    )
    .await)
    .await;
    let asked = tokens.lock().unwrap().last().unwrap().clone();
    assert!(
        asked
            .path
            .contains("msi_res_id=%2Fsubscriptions%2Fs%2Fserver"),
        "{}",
        asked.path
    );
}

/// Upstream `GcpCredentialVendor`: the token of the server's identity, a
/// service account key of `[identity.gcp]`, downscoped at Google STS to the
/// path.
#[tokio::test]
async fn gcp_downscopes_the_server_token() {
    let (token_uri, sign_ins) = mock(
        "application/json",
        json!({ "access_token": "source", "expires_in": 3600 }).to_string(),
    )
    .await;
    let (sts, exchanges) = mock("application/json", json!({ "access_token": "downscoped", "expires_in": 1800,
        "issued_token_type": "urn:ietf:params:oauth:token-type:access_token", "token_type": "Bearer" }).to_string()).await;
    let private_key = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
    let key_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        key_file.path(),
        json!({
            "type": "service_account",
            "client_email": "uc@project.iam.gserviceaccount.com",
            "private_key_id": "kid",
            "private_key": private_key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF).unwrap().as_str(),
            "token_uri": format!("{token_uri}/token"),
        })
        .to_string(),
    )
    .unwrap();
    let key_path = key_file.path().to_string_lossy().into_owned();
    let settings = format!(
        r#"
        [identity.gcp]
        json_key_file = "{key_path}"

        [locations.gbucket]
        url = "gs://gbucket"
        gcp_service_account = true
        "#
    );
    let app = app(
        &settings,
        Endpoints {
            gcp_sts: Some(format!("{sts}/v1/token")),
            ..Default::default()
        },
    )
    .await;

    let before = uc_common::now_epoch_millis();
    let body = ok(vend(&app, "gs://gbucket/tables/t1", "PATH_READ").await).await;
    assert_eq!(body["gcp_oauth_token"]["oauth_token"], "downscoped");
    let expires = body["expiration_time"].as_i64().unwrap();
    assert!(expires >= before + 1_800_000 && expires <= uc_common::now_epoch_millis() + 1_800_000);

    let sign_in = sign_ins.lock().unwrap()[0].clone();
    assert_eq!(
        sign_in.form()["grant_type"],
        "urn:ietf:params:oauth:grant-type:jwt-bearer"
    );
    let assertion = sign_in.form()["assertion"].clone();
    let header = jsonwebtoken::decode_header(&assertion).unwrap();
    assert_eq!(
        (header.alg, header.kid.as_deref()),
        (jsonwebtoken::Algorithm::RS256, Some("kid"))
    );
    let claims: Value = {
        use base64::Engine;
        let payload = assertion.split('.').nth(1).unwrap();
        serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(payload)
                .unwrap(),
        )
        .unwrap()
    };
    assert_eq!(claims["iss"], "uc@project.iam.gserviceaccount.com");
    assert_eq!(
        claims["scope"],
        "https://www.googleapis.com/auth/cloud-platform"
    );

    let exchange = exchanges.lock().unwrap()[0].clone();
    let form = exchange.form();
    assert_eq!(exchange.path, "/v1/token");
    assert_eq!(
        form["grant_type"],
        "urn:ietf:params:oauth:grant-type:token-exchange"
    );
    assert_eq!(form["subject_token"], "source");
    let options: Value = serde_json::from_str(&form["options"]).unwrap();
    assert_eq!(
        options,
        json!({ "accessBoundary": { "accessBoundaryRules": [{
        "availableResource": "//storage.googleapis.com/projects/_/buckets/gbucket",
        "availablePermissions": ["inRole:roles/storage.objectViewer"],
        // The path and the objects under it, not those of `tables/t10`.
        "availabilityCondition": { "expression": "resource.name == \"projects/_/buckets/gbucket/objects/tables/t1\" || \
            resource.name.startsWith(\"projects/_/buckets/gbucket/objects/tables/t1/\") || \
            api.getAttribute('storage.googleapis.com/objectListPrefix', '') == \"tables/t1\" || \
            api.getAttribute('storage.googleapis.com/objectListPrefix', '').startsWith(\"tables/t1/\")" },
    }] } })
    );
    ok(vend(&app, "gs://gbucket/tables/t1", "PATH_READ_WRITE").await).await;
    let options: Value =
        serde_json::from_str(&exchanges.lock().unwrap()[1].form()["options"]).unwrap();
    assert_eq!(
        options["accessBoundary"]["accessBoundaryRules"][0]["availablePermissions"],
        json!([
            "inRole:roles/storage.objectViewer",
            "inRole:roles/storage.legacyBucketWriter"
        ])
    );

    let err = assert_error(
        vend(&app, "gs://nope/x", "PATH_READ").await,
        StatusCode::BAD_REQUEST,
        "FAILED_PRECONDITION",
    )
    .await;
    assert_eq!(
        err["message"],
        "No external location and no [locations.<name>] of the settings covers gs://nope/x."
    );
}

/// A JSON key of a new service account, whose token endpoint is `token_uri`.
fn service_account_key(email: &str, token_uri: &str) -> Value {
    let private_key = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
    json!({
        "type": "service_account",
        "client_email": email,
        "private_key_id": "kid",
        "private_key": private_key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF).unwrap().as_str(),
        "token_uri": format!("{token_uri}/token"),
    })
}

/// The `iss` claim of a JWT bearer grant.
fn issuer(seen: &Seen) -> Value {
    use base64::Engine;
    let assertion = seen.form()["assertion"].clone();
    let payload = assertion.split('.').nth(1).unwrap();
    let claims: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .unwrap(),
    )
    .unwrap();
    claims["iss"].clone()
}

/// A GCP credential's token, downscoped at STS: of its service account key,
/// or of the server's own identity, which impersonates the credential's
/// service account. A `[locations.<name>]` holds the same identities.
#[tokio::test]
async fn gcp_credentials_downscope_their_identity() {
    let token = |value: &str| json!({ "access_token": value, "expires_in": 3600 }).to_string();
    let (metadata, own_tokens) = mock("application/json", token("own")).await;
    let (token_uri, sign_ins) = mock("application/json", token("keyed")).await;
    let impersonated =
        json!({ "accessToken": "impersonated", "expireTime": "2030-01-01T00:00:00Z" });
    let (iam, impersonations) = mock("application/json", impersonated.to_string()).await;
    let (sts, exchanges) = mock("application/json", token("downscoped")).await;
    let key_file = tempfile::NamedTempFile::new().unwrap();
    let file_key = service_account_key("file@p.iam.gserviceaccount.com", &token_uri);
    std::fs::write(key_file.path(), file_key.to_string()).unwrap();
    let settings = format!(
        r#"
        [locations.lake]
        url = "gs://lake"
        gcp_service_account = "lake@p.iam.gserviceaccount.com"

        [locations.keyed]
        url = "gs://keyed"
        gcp_service_account_key = "{}"
        "#,
        key_file.path().display()
    );
    let endpoints = Endpoints {
        gcp_sts: Some(format!("{sts}/v1/token")),
        gcp_metadata: Some(metadata),
        gcp_iam: Some(iam),
        ..Default::default()
    };
    let app = app(&settings, endpoints).await;
    let subject = |n: usize| exchanges.lock().unwrap()[n].form()["subject_token"].clone();

    // A service account that the server's own identity impersonates.
    let email = "data@p.iam.gserviceaccount.com";
    let identity = json!({ "gcp_service_account": { "email": email } });
    let credential = external_location(&app, "sa", "gs://gb/sa", identity).await;
    assert_eq!(credential["gcp_service_account"], json!({ "email": email }));
    let body = ok(vend(&app, "gs://gb/sa/t1", "PATH_READ").await).await;
    assert_eq!(body["gcp_oauth_token"]["oauth_token"], "downscoped");
    let own = own_tokens.lock().unwrap()[0].clone();
    assert!(
        own.path
            .starts_with("/computeMetadata/v1/instance/service-accounts/default/token?"),
        "{}",
        own.path
    );
    assert_eq!(own.headers["metadata-flavor"], "Google");
    let impersonation = impersonations.lock().unwrap()[0].clone();
    assert_eq!(
        impersonation.path,
        format!("/v1/projects/-/serviceAccounts/{email}:generateAccessToken")
    );
    assert_eq!(impersonation.headers["authorization"], "Bearer own");
    assert_eq!(
        serde_json::from_str::<Value>(&impersonation.body).unwrap(),
        json!({ "scope": ["https://www.googleapis.com/auth/cloud-platform"] })
    );
    assert_eq!(subject(0), "impersonated");
    let options: Value =
        serde_json::from_str(&exchanges.lock().unwrap()[0].form()["options"]).unwrap();
    assert_eq!(
        options["accessBoundary"]["accessBoundaryRules"][0]["availableResource"],
        "//storage.googleapis.com/projects/_/buckets/gb"
    );

    // Without an email: the server's own identity.
    let identity = json!({ "gcp_service_account": {} });
    let credential = external_location(&app, "own", "gs://gb/own", identity).await;
    assert_eq!(credential["gcp_service_account"], json!({}));
    ok(vend(&app, "gs://gb/own/t1", "PATH_READ").await).await;
    assert_eq!(subject(1), "own");
    assert_eq!(impersonations.lock().unwrap().len(), 1);

    // A service account key: its own token, by a JWT bearer grant.
    let key = service_account_key("key@p.iam.gserviceaccount.com", &token_uri);
    let identity = json!({ "gcp_service_account_key": { "json_key": key.to_string() } });
    let credential = external_location(&app, "key", "gs://gb/key", identity).await;
    assert_eq!(
        credential["gcp_service_account_key"],
        json!({ "email": "key@p.iam.gserviceaccount.com", "private_key_id": "kid" })
    );
    ok(vend(&app, "gs://gb/key/t1", "PATH_READ_WRITE").await).await;
    let sign_in = sign_ins.lock().unwrap()[0].clone();
    assert_eq!(sign_in.path, "/token");
    assert_eq!(issuer(&sign_in), "key@p.iam.gserviceaccount.com");
    assert_eq!(subject(2), "keyed");

    // Without an external location: the location of the settings.
    ok(vend(&app, "gs://lake/tables/t1", "PATH_READ").await).await;
    let impersonation = impersonations.lock().unwrap()[1].clone();
    assert!(
        impersonation
            .path
            .contains("/serviceAccounts/lake@p.iam.gserviceaccount.com:"),
        "{}",
        impersonation.path
    );
    assert_eq!(subject(3), "impersonated");
    ok(vend(&app, "gs://keyed/t1", "PATH_READ").await).await;
    assert_eq!(
        issuer(&sign_ins.lock().unwrap()[1]),
        "file@p.iam.gserviceaccount.com"
    );
    assert_eq!(subject(4), "keyed");
}

/// A location that needs the server's own identity, when the server has
/// none: a 400 that names the settings to set.
#[tokio::test]
async fn a_missing_server_identity_names_its_settings() {
    // Nothing listens on the metadata server's port.
    let metadata = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let settings = r#"
        [locations.lake]
        url = "gs://lake"
        gcp_service_account = true
    "#;
    let endpoints = Endpoints {
        gcp_metadata: Some(metadata),
        ..Default::default()
    };
    let app = app(settings, endpoints).await;
    let err = assert_error(
        vend(&app, "gs://lake/t", "PATH_READ").await,
        StatusCode::BAD_REQUEST,
        "FAILED_PRECONDITION",
    )
    .await;
    let message = err["message"].as_str().unwrap();
    assert!(
        message.starts_with(
            "The server could not get its GCP identity: set [identity.gcp] \
             (UC_IDENTITY_GCP_JSON_KEY_FILE)"
        ),
        "{message}"
    );
}

/// Upstream `CloudCredentialVendor`: local paths vend nothing.
#[tokio::test]
async fn local_paths_vend_nothing() {
    let app = app("", Endpoints::default()).await;
    let root = common::temp_storage_root();
    let body = ok(vend(&app, &format!("{root}/t"), "PATH_READ").await).await;
    assert_eq!(body, json!({ "url": format!("{root}/t") }));
}
