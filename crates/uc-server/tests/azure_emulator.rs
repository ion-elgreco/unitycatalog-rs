//! Azure credentials end to end, against Floci AZ (`floci/floci-az`), a
//! local Azure emulator with ADLS Gen2: the server signs in at its Entra ID
//! or instance metadata service, gets a user delegation key from its blob
//! service and signs a directory SAS. The emulator checks the SAS: its
//! signature, path and permissions. Skipped without `UC_TEST_FLOCI_AZ_URL`,
//! such as `http://127.0.0.1:4577`.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{
    AppState, build_app,
    storage::vendor::{CloudVendor, Endpoints, StorageConfig},
};

use common::{assert_status, json_body, req_json};

/// The emulator's account with a hierarchical namespace.
const ACCOUNT: &str = "devstoreaccount1";
/// The emulator's tenant and app. It takes any client secret.
const TENANT: &str = "00000000-0000-0000-0000-000000000002";
const CLIENT: &str = "11111111-1111-1111-1111-111111111111";

/// The emulator's URL, if the tests run against one.
fn emulator() -> Option<String> {
    let url = std::env::var("UC_TEST_FLOCI_AZ_URL").ok()?;
    let url = url.trim().trim_end_matches('/');
    (!url.is_empty()).then(|| url.to_string())
}

/// The app with the emulator in place of Azure.
async fn app(url: &str) -> Router {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let endpoints = Endpoints {
        azure_login: Some(url.to_string()),
        azure_blob: Some(format!("{url}/{ACCOUNT}")),
        azure_msi: Some(format!("{url}/metadata/identity/oauth2/token")),
        ..Default::default()
    };
    let vendor = CloudVendor::new(StorageConfig::default()).with_endpoints(endpoints);
    build_app(AppState::new(metastore).with_vendor(vendor))
}

async fn post(app: &Router, path: &str, body: Value) -> Value {
    let resp = app
        .clone()
        .oneshot(req_json(Method::POST, path, body))
        .await
        .unwrap();
    json_body(assert_status(resp, StatusCode::OK, path).await).await
}

/// A new container, and the URL of its folder `lake` under an external
/// location whose credential has `identity`.
async fn lake(url: &str, app: &Router, identity: Value) -> (String, String) {
    let container = format!("uc-{}", uuid::Uuid::new_v4().simple());
    let made = reqwest::Client::new()
        .put(format!("{url}/{ACCOUNT}/{container}?restype=container"))
        .header("x-ms-version", "2021-08-06")
        .send()
        .await
        .unwrap();
    assert!(made.status().is_success(), "container: {}", made.status());
    let mut credential = json!({ "name": container, "purpose": "STORAGE" });
    credential
        .as_object_mut()
        .unwrap()
        .extend(identity.as_object().unwrap().clone());
    post(app, "/credentials", credential).await;
    let location = format!("abfss://{container}@{ACCOUNT}.dfs.core.windows.net/lake");
    let body = json!({ "name": container, "url": location, "credential_name": container });
    post(app, "/external-locations", body).await;
    (container, location)
}

/// The SAS that the server vends for `url`.
async fn vend(app: &Router, url: &str, operation: &str) -> String {
    let body = json!({ "url": url, "operation": operation });
    let credentials = post(app, "/temporary-path-credentials", body).await;
    credentials["azure_user_delegation_sas"]["sas_token"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The blob at `path` of `container`, through `sas`.
fn blob(url: &str, container: &str, path: &str, sas: &str) -> String {
    format!("{url}/{ACCOUNT}/{container}/{path}?{sas}")
}

async fn put(url: &str, container: &str, path: &str, sas: &str) -> StatusCode {
    reqwest::Client::new()
        .put(blob(url, container, path, sas))
        .header("x-ms-blob-type", "BlockBlob")
        .body("data")
        .send()
        .await
        .unwrap()
        .status()
}

async fn get(url: &str, container: &str, path: &str, sas: &str) -> (StatusCode, String) {
    let resp = reqwest::get(blob(url, container, path, sas)).await.unwrap();
    (resp.status(), resp.text().await.unwrap())
}

/// A service principal's SAS reaches its path, and only with the vended
/// permissions.
#[tokio::test]
async fn a_service_principal_vends_a_sas_for_its_path() {
    let Some(url) = emulator() else { return };
    let app = app(&url).await;
    let principal = json!({ "azure_service_principal": {
        "directory_id": TENANT, "application_id": CLIENT, "client_secret": "secret" } });
    let (container, location) = lake(&url, &app, principal).await;

    let sas = vend(&app, &format!("{location}/t1"), "PATH_READ_WRITE").await;
    assert_eq!(
        put(&url, &container, "lake/t1/part-0", &sas).await,
        StatusCode::CREATED
    );
    assert_eq!(
        get(&url, &container, "lake/t1/part-0", &sas).await,
        (StatusCode::OK, "data".to_string())
    );
    // Not beside the path.
    assert_eq!(
        put(&url, &container, "lake/t2/part-0", &sas).await,
        StatusCode::FORBIDDEN
    );
    // A read SAS reads, and does not write.
    let read = vend(&app, &format!("{location}/t1"), "PATH_READ").await;
    assert_eq!(
        get(&url, &container, "lake/t1/part-0", &read).await.0,
        StatusCode::OK
    );
    assert_eq!(
        put(&url, &container, "lake/t1/part-1", &read).await,
        StatusCode::FORBIDDEN
    );
    // A changed signature fails.
    let forged = sas.replace("sp=rwdl", "sp=rwdlc");
    assert_eq!(
        put(&url, &container, "lake/t1/part-2", &forged).await,
        StatusCode::FORBIDDEN
    );
}

/// Make the ARM resource `id` of the emulator's subscription.
async fn arm(url: &str, id: &str, api_version: &str) {
    let made = reqwest::Client::new()
        .put(format!("{url}{id}?api-version={api_version}"))
        .json(&json!({ "location": "westeurope" }))
        .send()
        .await
        .unwrap();
    assert!(made.status().is_success(), "{id}: {}", made.status());
}

/// The server's own identity, and a user-assigned identity by its resource
/// ID, from the instance metadata service.
#[tokio::test]
async fn managed_identities_vend_a_sas() {
    let Some(url) = emulator() else { return };
    let app = app(&url).await;
    let group = "/subscriptions/00000000-0000-0000-0000-000000000001/resourceGroups/uc";
    arm(&url, group, "2021-04-01").await;
    let id = format!("{group}/providers/Microsoft.ManagedIdentity/userAssignedIdentities/uc");
    arm(&url, &id, "2023-01-31").await;
    for identity in [
        json!({ "azure_managed_identity": {} }),
        json!({ "azure_managed_identity": { "managed_identity_id": id } }),
    ] {
        let (container, location) = lake(&url, &app, identity).await;
        let sas = vend(&app, &format!("{location}/t1"), "PATH_READ_WRITE").await;
        assert_eq!(
            put(&url, &container, "lake/t1/part-0", &sas).await,
            StatusCode::CREATED
        );
        assert_eq!(
            put(&url, &container, "other/part-0", &sas).await,
            StatusCode::FORBIDDEN
        );
    }
}
