//! OPA as the engine of privileges (`auth.engine = "opa"`), after
//! upstream's `SdkOpaAccessControlTest`: OPA decides every privilege,
//! `OWNER` and the admin; grants do not count; a failed decision is 503.

mod common;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{assert_status, json_body, signed};
use uc_common::settings::Settings;
use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{
    AppState,
    authz::{EngineSettings, opa::Opa},
    build_app,
};

/// The user whom the policy allows everything.
const ADMIN: &str = "admin@example.com";

/// A grant of the policy: principal, privilege, securable type and name
/// (empty for the metastore).
type Grant = (String, String, String, String);

/// An OPA with the policy of upstream's `opa-example.rego`: everything for
/// [`ADMIN`], else what its own grants hold. It keeps each input.
#[derive(Clone, Default)]
struct MockOpa {
    grants: Arc<Mutex<HashSet<Grant>>>,
    inputs: Arc<Mutex<Vec<Value>>>,
    down: Arc<Mutex<bool>>,
}

impl MockOpa {
    /// The OPA and its decision URL.
    async fn start() -> (Self, String) {
        let opa = Self::default();
        let app = Router::new()
            .route("/v1/data/uc/authz", post(decide))
            .with_state(opa.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (opa, format!("http://{address}/v1/data/uc/authz"))
    }

    fn grant(&self, principal: &str, privilege: &str, kind: &str, name: &str) {
        self.grants.lock().unwrap().insert((
            principal.into(),
            privilege.into(),
            kind.into(),
            name.into(),
        ));
    }

    fn revoke(&self, principal: &str, privilege: &str, kind: &str, name: &str) {
        self.grants.lock().unwrap().remove(&(
            principal.into(),
            privilege.into(),
            kind.into(),
            name.into(),
        ));
    }

    fn inputs(&self) -> Vec<Value> {
        self.inputs.lock().unwrap().clone()
    }

    fn clear(&self) {
        self.inputs.lock().unwrap().clear();
    }

    fn set_down(&self, down: bool) {
        *self.down.lock().unwrap() = down;
    }
}

async fn decide(State(opa): State<MockOpa>, Json(body): Json<Value>) -> Response {
    if *opa.down.lock().unwrap() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let input = body["input"].clone();
    opa.inputs.lock().unwrap().push(input.clone());
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    let email = text(&input["principal"]["email"]);
    let grant = (
        email.clone(),
        text(&input["privilege"]),
        text(&input["resource"]["type"]),
        text(&input["resource"]["name"]),
    );
    let allow = email == ADMIN || opa.grants.lock().unwrap().contains(&grant);
    Json(json!({ "result": { "allow": allow } })).into_response()
}

/// The admin, alice and bob, with OPA deciding.
struct World {
    app: Router,
    opa: MockOpa,
    admin: String,
    alice: String,
    bob: String,
}

impl World {
    async fn new() -> Self {
        let (opa, url) = MockOpa::start().await;
        let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
            .await
            .unwrap();
        let dyn_metastore: uc_metastore::DynMetastore = metastore.clone();
        let state = AppState::new(metastore)
            .with_authz(true)
            .with_security(common::security())
            .with_opa(Opa::new(&url, Duration::from_secs(2)).unwrap());
        let admin = common::active_user(&dyn_metastore, ADMIN).await;
        let alice = common::active_user(&dyn_metastore, "alice@example.com").await;
        let bob = common::active_user(&dyn_metastore, "bob@example.com").await;
        Self {
            app: build_app(state),
            opa,
            admin,
            alice,
            bob,
        }
    }

    async fn call(&self, token: &str, method: Method, path: &str, body: Option<Value>) -> Response {
        self.app
            .clone()
            .oneshot(signed(method, path, body, token))
            .await
            .unwrap()
    }

    async fn status(&self, token: &str, method: Method, path: &str) -> StatusCode {
        self.call(token, method, path, None).await.status()
    }

    async fn ok(&self, token: &str, method: Method, path: &str, body: Option<Value>) -> Value {
        let response = self.call(token, method, path, body).await;
        json_body(assert_status(response, StatusCode::OK, path).await).await
    }

    async fn create_catalog(&self, name: &str) -> Value {
        let body = json!({ "name": name });
        self.ok(&self.admin, Method::POST, "/catalogs", Some(body))
            .await
    }

    /// A SCIM request signed in with `token`.
    async fn scim(&self, token: &str, method: Method, path: &str, body: Option<Value>) -> Response {
        let mut request = match body {
            Some(body) => common::control_json(method, path, body),
            None => common::control(method, path),
        };
        request
            .headers_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
        self.app.clone().oneshot(request).await.unwrap()
    }

    /// The SCIM id of the user `email`.
    async fn user_id(&self, email: &str) -> String {
        let path = format!("/scim2/Users?filter=userName%20eq%20%22{email}%22");
        let found = self.scim(&self.admin, Method::GET, &path, None).await;
        let found = json_body(assert_status(found, StatusCode::OK, "find").await).await;
        found["Resources"][0]["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn opa_decides_and_grants_do_not_count() {
    let w = World::new().await;
    w.create_catalog("c").await;
    // A grant gives nothing: OPA does not know it.
    let grant = json!({ "changes": [{
        "principal": "alice@example.com", "add": ["USE CATALOG"], "remove": [],
    }] });
    w.ok(
        &w.admin,
        Method::PATCH,
        "/permissions/catalog/c",
        Some(grant),
    )
    .await;
    assert_eq!(
        w.status(&w.alice, Method::GET, "/catalogs/c").await,
        StatusCode::FORBIDDEN
    );
    w.opa
        .grant("alice@example.com", "USE_CATALOG", "CATALOG", "c");
    assert_eq!(
        w.status(&w.alice, Method::GET, "/catalogs/c").await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn the_input_is_upstreams_with_ours_on_top() {
    let w = World::new().await;
    let catalog = w.create_catalog("c").await;
    let schema = json!({ "name": "s", "catalog_name": "c" });
    let schema = w.ok(&w.admin, Method::POST, "/schemas", Some(schema)).await;
    // With `USE_CATALOG`, the rule goes on to the schema.
    w.opa
        .grant("alice@example.com", "USE_CATALOG", "CATALOG", "c");
    w.opa.clear();
    w.status(&w.alice, Method::GET, "/schemas/c.s").await;
    let inputs = w.opa.inputs();
    let on_schema = (inputs.iter())
        .find(|i| i["privilege"] == "USE_SCHEMA")
        .unwrap_or_else(|| panic!("no USE_SCHEMA question in {inputs:#?}"));
    let principal = &on_schema["principal"];
    assert_eq!(principal["email"], "alice@example.com");
    assert!(principal["id"].as_str().is_some_and(|id| !id.is_empty()));
    assert_eq!(principal["groups"], json!([]));
    assert_eq!(
        on_schema["resource"],
        json!({
            "id": schema["schema_id"], "type": "SCHEMA", "name": "s", "owner": ADMIN,
            "parents": [{ "id": catalog["id"], "type": "CATALOG", "name": "c" }],
        })
    );
    assert_eq!(on_schema["context"]["auth_method"], "server-token");
    // The admin is a question too: `OWNER` on the metastore.
    assert!(
        inputs.iter().any(|i| i["privilege"] == "OWNER"
            && i["resource"]["type"] == "METASTORE"
            && i["resource"]["parents"] == json!([])),
        "{inputs:#?}"
    );
    // Each question once per request.
    let mut asked: Vec<String> = (inputs.iter())
        .map(|i| format!("{} {}", i["privilege"], i["resource"]["id"]))
        .collect();
    let all = asked.len();
    asked.sort();
    asked.dedup();
    assert_eq!(asked.len(), all, "{asked:#?}");
}

#[tokio::test]
async fn owners_are_whom_opa_names() {
    let w = World::new().await;
    w.create_catalog("c").await;
    // The admin made `c`, but bob owns it for OPA.
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/catalogs/c").await,
        StatusCode::FORBIDDEN
    );
    w.opa.grant("bob@example.com", "OWNER", "CATALOG", "c");
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/catalogs/c").await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn lists_hold_what_opa_allows() {
    let w = World::new().await;
    w.create_catalog("c1").await;
    w.create_catalog("c2").await;
    w.opa
        .grant("alice@example.com", "USE_CATALOG", "CATALOG", "c1");
    let listed = w.ok(&w.alice, Method::GET, "/catalogs", None).await;
    let names: Vec<&str> = (listed["catalogs"].as_array().unwrap().iter())
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert_eq!(names, ["c1"]);
}

#[tokio::test]
async fn a_failed_decision_is_503() {
    let w = World::new().await;
    w.create_catalog("c").await;
    w.opa.set_down(true);
    for path in ["/catalogs/c", "/catalogs"] {
        let response = w.call(&w.alice, Method::GET, path, None).await;
        let error =
            common::assert_error(response, StatusCode::SERVICE_UNAVAILABLE, "UNAVAILABLE").await;
        let message = error["message"].as_str().unwrap();
        assert!(
            message.starts_with(
                "Could not get an authorization decision from OPA: OPA answered HTTP 500"
            ),
            "{message}"
        );
    }
    let response = w.scim(&w.alice, Method::GET, "/scim2/Users", None).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn the_admin_is_whom_opa_names() {
    let w = World::new().await;
    let me = w.scim(&w.alice, Method::GET, "/scim2/Me", None).await;
    let me = json_body(assert_status(me, StatusCode::OK, "me").await).await;
    assert_eq!(me.get("roles"), None);
    assert_eq!(
        w.scim(&w.alice, Method::GET, "/scim2/Users", None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    w.opa.grant("alice@example.com", "OWNER", "METASTORE", "");
    assert_eq!(
        w.scim(&w.alice, Method::GET, "/scim2/Users", None)
            .await
            .status(),
        StatusCode::OK
    );
    let me = w.scim(&w.alice, Method::GET, "/scim2/Me", None).await;
    let me = json_body(assert_status(me, StatusCode::OK, "me").await).await;
    assert_eq!(me["roles"], json!([{ "value": "admin" }]));
}

#[tokio::test]
async fn scim_keeps_the_user_whom_opa_names_admin() {
    let w = World::new().await;
    let bob = w.user_id("bob@example.com").await;
    let path = format!("/scim2/Users/{bob}");
    w.opa.grant("bob@example.com", "OWNER", "METASTORE", "");
    let refused = w.scim(&w.admin, Method::DELETE, &path, None).await;
    let refused = assert_status(refused, StatusCode::BAD_REQUEST, "delete").await;
    assert_eq!(
        json_body(refused).await["detail"],
        "The metastore owner can't be deleted or deactivated: bob@example.com"
    );
    let deactivate = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [{ "op": "replace", "path": "active", "value": false }],
    });
    let refused = w
        .scim(&w.admin, Method::PATCH, &path, Some(deactivate.clone()))
        .await;
    assert_status(refused, StatusCode::BAD_REQUEST, "deactivate").await;
    // A change that keeps the user active asks nothing.
    let rename = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [{ "op": "replace", "path": "displayName", "value": "Bob" }],
    });
    let renamed = w.scim(&w.admin, Method::PATCH, &path, Some(rename)).await;
    assert_status(renamed, StatusCode::OK, "rename").await;
    w.opa.revoke("bob@example.com", "OWNER", "METASTORE", "");
    let deactivated = w
        .scim(&w.admin, Method::PATCH, &path, Some(deactivate))
        .await;
    assert_status(deactivated, StatusCode::OK, "deactivate").await;
    let deleted = w.scim(&w.admin, Method::DELETE, &path, None).await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
}

fn engine(toml: &str) -> Result<EngineSettings, String> {
    EngineSettings::new(&Settings::from_toml(toml).unwrap().auth)
}

#[test]
fn the_engine_takes_only_its_own_settings() {
    assert!(matches!(engine(""), Ok(EngineSettings::Grants)));
    let opa = "[auth]\nenabled = true\nengine = \"opa\"\nopa_url = \"http://opa:8181/v1/data/uc/authz\"\n";
    assert!(matches!(engine(opa), Ok(EngineSettings::Opa(_))));
    let cedar = "[auth]\nenabled = true\nengine = \"cedar\"\ncedar_policies = [\"a.cedar\"]\n";
    assert!(matches!(engine(cedar), Ok(EngineSettings::Cedar(p)) if p.len() == 1));
    for (toml, expected) in [
        (
            "[auth]\nengine = \"opa\"\nopa_url = \"http://opa/v1/data/uc\"\n",
            "needs auth.enabled",
        ),
        (
            "[auth]\nenabled = true\nengine = \"opa\"\n",
            "needs auth.opa_url",
        ),
        (
            "[auth]\nenabled = true\nopa_url = \"http://opa/v1/data/uc\"\n",
            "auth.opa_url (UC_AUTH_OPA_URL) takes effect only with",
        ),
        (
            "[auth]\nenabled = true\ncedar_policies = [\"a.cedar\"]\n",
            "auth.cedar_policies (UC_AUTH_CEDAR_POLICIES) takes effect only with",
        ),
        (
            "[auth]\nenabled = true\nengine = \"opa\"\nopa_url = \"http://opa/v1/data/uc\"\nadmin_principals = [\"a@example.com\"]\n",
            "OPA decides who the admin is",
        ),
        (
            "[auth]\nenabled = true\nengine = \"opa\"\nopa_url = \"http://user:secret@opa/v1/data/uc\"\n",
            "without credentials",
        ),
    ] {
        let error = engine(toml)
            .err()
            .unwrap_or_else(|| panic!("no error for {toml}"));
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("secret"), "{error}");
    }
}

/// A real OPA with upstream's `etc/conf/opa-example.rego`, unchanged
/// (`tests/fixtures/opa`), at `UC_TEST_OPA_URL`, such as
/// `http://127.0.0.1:18181` (`just test-opa`); skipped without it.
#[tokio::test]
async fn upstreams_example_policy_works_unchanged() {
    let Some(base) = std::env::var("UC_TEST_OPA_URL")
        .ok()
        .filter(|url| !url.is_empty())
    else {
        eprintln!("skipped: UC_TEST_OPA_URL is not set");
        return;
    };
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let dyn_metastore: uc_metastore::DynMetastore = metastore.clone();
    uc_server::authn::security::initialize_admin(&dyn_metastore)
        .await
        .unwrap();
    let alice = common::active_user(&dyn_metastore, "alice@example.com").await;
    let url = format!("{base}/v1/data/uc/authz");
    let app = build_app(
        AppState::new(metastore)
            .with_authz(true)
            .with_security(common::security())
            .with_opa(Opa::new(&url, Duration::from_secs(5)).unwrap()),
    );
    let call = |token: String, method: Method, path: &str, body: Option<Value>| {
        let request = signed(method, path, body, &token);
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap() }
    };
    // The policy's own data: no grants yet.
    let opa = reqwest::Client::new();
    let put_grants = async |grants: Value| {
        let response = (opa.put(format!("{base}/v1/data/grants")))
            .json(&grants)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
    };
    put_grants(json!([])).await;

    // The server's own `admin` may do anything.
    let created = call(
        common::admin_token(),
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c" })),
    )
    .await;
    let catalog = json_body(assert_status(created, StatusCode::OK, "create").await).await;
    let read = || call(alice.clone(), Method::GET, "/catalogs/c", None);
    assert_eq!(read().await.status(), StatusCode::FORBIDDEN);

    put_grants(json!([{
        "principal": "alice@example.com", "privilege": "USE_CATALOG",
        "type": "CATALOG", "id": catalog["id"],
    }]))
    .await;
    assert_eq!(read().await.status(), StatusCode::OK);
    let listed = call(alice.clone(), Method::GET, "/catalogs", None).await;
    let listed = json_body(assert_status(listed, StatusCode::OK, "list").await).await;
    assert_eq!(listed["catalogs"][0]["name"], "c");
    put_grants(json!([])).await;
}
