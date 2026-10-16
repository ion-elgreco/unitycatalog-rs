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

/// Credential `cred` (an AWS role) and external location `el` at `url`.
async fn external_location(app: &Router, url: &str) -> Value {
    let credential = json_body(
        assert_status(
            post(
                app,
                "/credentials",
                json!({ "name": "cred", "purpose": "STORAGE",
                "aws_iam_role": { "role_arn": "arn:aws:iam::123456789012:role/data" } }),
            )
            .await,
            StatusCode::OK,
            "credential",
        )
        .await,
    )
    .await;
    let location = json!({ "name": "el", "url": url, "credential_name": "cred" });
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
        [aws]
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
    let credential = external_location(&app, "s3://bucket/data").await;

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

/// `aws.endpoint`: an S3-compatible store, such as RustFS. AssumeRole goes
/// to it, signed for `us-east-1`, and its session policy has no KMS
/// statement, which such a store does not know.
#[tokio::test]
async fn s3_compatible_store_gets_sts_calls_without_kms() {
    let (sts, log) = mock("text/xml", ASSUME_ROLE.to_string()).await;
    let settings = format!(
        r#"
        [aws]
        endpoint = "{sts}/"
        access_key = "AKSTORE"
        secret_key = "store-secret"
    "#
    );
    let app = app(&settings, Endpoints::default()).await;
    external_location(&app, "s3://bucket/data").await;

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
        uc_common::settings::Settings::from_toml("[aws]\nendpoint = \"localhost:9000\"").unwrap();
    let error = StorageConfig::from_settings(&settings).unwrap_err();
    assert!(
        error.starts_with("Invalid value for aws.endpoint (UC_AWS_ENDPOINT)"),
        "{error}"
    );
}

/// Without an external location, the bucket's `s3.*.N` entry: a preset
/// session, or its keys assuming its role.
#[tokio::test]
async fn s3_without_a_location_uses_the_bucket_configuration() {
    let (sts, log) = mock("text/xml", ASSUME_ROLE.to_string()).await;
    let settings = r#"
        [s3.static]
        bucket = "s3://static-bucket"
        access_key = "AKSTATIC"
        secret_key = "static-secret"
        session_token = "static-session"

        [s3.role]
        bucket = "s3://role-bucket/"
        region = "eu-west-1"
        role_arn = "arn:aws:iam::1:role/bucket"
        access_key = "AKBUCKET"
        secret_key = "bucket-secret"
    "#;
    let app = app(
        settings,
        Endpoints {
            aws_sts: Some(sts),
            ..Default::default()
        },
    )
    .await;

    let body = ok(vend(&app, "s3://static-bucket/t", "PATH_READ").await).await;
    assert_eq!(
        body["aws_temp_credentials"],
        json!({
        "access_key_id": "AKSTATIC", "secret_access_key": "static-secret", "session_token": "static-session" })
    );
    assert_eq!(body["expiration_time"], Value::Null);
    assert!(log.lock().unwrap().is_empty());

    let body = ok(vend(&app, "s3://role-bucket/t", "PATH_READ").await).await;
    assert_eq!(body["aws_temp_credentials"]["access_key_id"], "ASIAMOCK");
    let seen = log.lock().unwrap()[0].clone();
    assert_eq!(seen.form()["RoleArn"], "arn:aws:iam::1:role/bucket");
    assert!(!seen.form().contains_key("ExternalId"));
    let authorization = seen.headers["authorization"].to_str().unwrap().to_string();
    assert!(
        authorization.contains("Credential=AKBUCKET/") && authorization.contains("/eu-west-1/sts/"),
        "{authorization}"
    );

    let err = assert_error(
        vend(&app, "s3://unknown/t", "PATH_READ").await,
        StatusCode::BAD_REQUEST,
        "FAILED_PRECONDITION",
    )
    .await;
    assert_eq!(err["message"], "S3 bucket configuration not found.");
}

/// Upstream `AzureCredentialVendor`: the account's service principal gets a
/// user delegation key and signs a directory SAS for the path.
#[tokio::test]
async fn azure_signs_a_user_delegation_sas() {
    let (login, logins) = mock(
        "application/json",
        json!({ "access_token": "aad-token", "token_type": "Bearer", "expires_in": 3599 })
            .to_string(),
    )
    .await;
    let key = "<?xml version=\"1.0\" encoding=\"utf-8\"?><UserDelegationKey>\
        <SignedOid>oid</SignedOid><SignedTid>tid</SignedTid><SignedStart>2030-01-01T00:00:00Z</SignedStart>\
        <SignedExpiry>2030-01-01T01:00:00Z</SignedExpiry><SignedService>b</SignedService>\
        <SignedVersion>2021-08-06</SignedVersion><Value>c2VjcmV0</Value></UserDelegationKey>";
    let (blob, keys) = mock("application/xml", key.to_string()).await;
    let settings = r#"
        [adls.acct]
        account = "acct"
        tenant_id = "tenant"
        client_id = "client"
        client_secret = "secret"

        [adls.test]
        account = "testacct"
        tenant_id = "t"
        client_id = "c"
        client_secret = "s"
        test_mode = true
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
    // Test mode: upstream's fixed token.
    let body = ok(vend(
        &app,
        "abfss://c@testacct.dfs.core.windows.net/x",
        "PATH_READ",
    )
    .await)
    .await;
    assert_eq!(body["azure_user_delegation_sas"]["sas_token"], "t/c/s");
    assert_eq!(body["expiration_time"], 253_370_790_000_000_i64);

    let err = assert_error(
        vend(&app, "abfss://c@other.dfs.core.windows.net/x", "PATH_READ").await,
        StatusCode::BAD_REQUEST,
        "FAILED_PRECONDITION",
    )
    .await;
    assert_eq!(
        err["message"],
        "Azure storage account configuration not found."
    );
    external_location(&app, "abfss://c@acct.dfs.core.windows.net/el").await;
    let err = assert_error(
        vend(
            &app,
            "abfss://c@acct.dfs.core.windows.net/el/x",
            "PATH_READ",
        )
        .await,
        StatusCode::NOT_IMPLEMENTED,
        "UNIMPLEMENTED",
    )
    .await;
    assert_eq!(
        err["message"],
        "Storage credential/external location for Azure is not supported yet."
    );
}

/// Upstream `GcpCredentialVendor`: the bucket's service account token,
/// downscoped at Google STS to the path.
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
        [gcs.key]
        bucket = "gs://gbucket"
        json_key_file = "{key_path}"

        [gcs.test]
        bucket = "gs://tbucket"
        credential_generator = "io.unitycatalog.server.service.credential.gcp.TestingCredentialGenerator"
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
        "availabilityCondition": { "expression": "resource.name.startsWith(\"projects/_/buckets/gbucket/objects/tables/t1\") || \
            api.getAttribute('storage.googleapis.com/objectListPrefix', '').startsWith(\"tables/t1\")" },
    }] } })
    );
    ok(vend(&app, "gs://gbucket/tables/t1", "PATH_READ_WRITE").await).await;
    let options: Value =
        serde_json::from_str(&exchanges.lock().unwrap()[1].form()["options"]).unwrap();
    assert_eq!(
        options["accessBoundary"]["accessBoundaryRules"][0]["availablePermissions"],
        json!(["inRole:roles/storage.objectAdmin"])
    );

    // Upstream's testing generator.
    let body = ok(vend(&app, "gs://tbucket/x", "PATH_READ").await).await;
    assert_eq!(
        body["gcp_oauth_token"]["oauth_token"],
        "testing://gs://tbucket"
    );
    assert_eq!(body["expiration_time"], 253_370_790_000_000_i64);

    let err = assert_error(
        vend(&app, "gs://nope/x", "PATH_READ").await,
        StatusCode::BAD_REQUEST,
        "FAILED_PRECONDITION",
    )
    .await;
    assert_eq!(
        err["message"],
        "Unknown GCS storage configuration for gs://nope."
    );
    external_location(&app, "gs://gbucket/el").await;
    let err = assert_error(
        vend(&app, "gs://gbucket/el/x", "PATH_READ").await,
        StatusCode::NOT_IMPLEMENTED,
        "UNIMPLEMENTED",
    )
    .await;
    assert_eq!(
        err["message"],
        "Storage credential/external location for GCP is not supported yet."
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
