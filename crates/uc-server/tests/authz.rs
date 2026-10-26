//! Authorization with sign-in, after upstream's access-control tests:
//! ownership, inherited grants and the per-endpoint rules.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{assert_status, json_body, signed};

/// Users: the admin (metastore owner), alice and bob.
struct World {
    app: Router,
    metastore: uc_metastore::DynMetastore,
    admin: String,
    alice: String,
    bob: String,
}

impl World {
    async fn new() -> Self {
        let (app, metastore) = common::signed_in_app().await;
        let admin = common::active_user(&metastore, "admin@example.com").await;
        metastore
            .permissions()
            .set_metastore_owner("admin@example.com")
            .await
            .unwrap();
        let alice = common::active_user(&metastore, "alice@example.com").await;
        let bob = common::active_user(&metastore, "bob@example.com").await;
        Self {
            app,
            metastore,
            admin,
            alice,
            bob,
        }
    }

    async fn call(
        &self,
        token: &str,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> axum::response::Response {
        self.app
            .clone()
            .oneshot(signed(method, path, body, token))
            .await
            .unwrap()
    }

    /// A SCIM request of the admin.
    async fn scim(&self, method: Method, path: &str, body: Value) -> axum::response::Response {
        let mut request = common::control_json(method, path, body);
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {}", self.admin).parse().unwrap(),
        );
        self.app.clone().oneshot(request).await.unwrap()
    }

    /// A control-plane GET signed in with `token`.
    async fn control(&self, token: &str, path: &str) -> StatusCode {
        let mut request = common::control(Method::GET, path);
        request
            .headers_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
        self.app.clone().oneshot(request).await.unwrap().status()
    }

    async fn status(
        &self,
        token: &str,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> StatusCode {
        self.call(token, method, path, body).await.status()
    }

    /// The status of a GET of catalog `c` signed in with `token`.
    async fn get_catalog(&self, token: &str) -> StatusCode {
        self.status(token, Method::GET, "/catalogs/c", None).await
    }

    async fn ok(&self, token: &str, method: Method, path: &str, body: Option<Value>) -> Value {
        json_body(
            assert_status(
                self.call(token, method, path, body).await,
                StatusCode::OK,
                path,
            )
            .await,
        )
        .await
    }

    async fn grant(&self, securable: &str, name: &str, principal: &str, privileges: &[&str]) {
        let body =
            json!({ "changes": [{ "principal": principal, "add": privileges, "remove": [] }] });
        self.ok(
            &self.admin,
            Method::PATCH,
            &format!("/permissions/{securable}/{name}"),
            Some(body),
        )
        .await;
    }

    fn names(body: &Value, key: &str) -> Vec<String> {
        body[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap().to_string())
            .collect()
    }

    /// Catalog `c` and schema `c.s` of the admin; `users` get USE CATALOG
    /// and USE SCHEMA.
    async fn catalog_and_schema(&self, users: &[&str]) {
        self.catalog_and_schema_at(users, None).await;
    }

    /// [`Self::catalog_and_schema`], the catalog storing at `storage_root`.
    async fn catalog_and_schema_at(&self, users: &[&str], storage_root: Option<String>) {
        let catalog = json!({ "name": "c", "storage_root": storage_root });
        self.ok(&self.admin, Method::POST, "/catalogs", Some(catalog))
            .await;
        self.ok(
            &self.admin,
            Method::POST,
            "/schemas",
            Some(json!({ "name": "s", "catalog_name": "c" })),
        )
        .await;
        for user in users {
            self.grant("catalog", "c", user, &["USE CATALOG"]).await;
            self.grant("schema", "c.s", user, &["USE SCHEMA"]).await;
        }
    }

    /// The admin's credential `cred` and external location `el` at `url`.
    async fn external_location(&self, url: &str) {
        self.ok(
            &self.admin,
            Method::POST,
            "/credentials",
            Some(json!({ "name": "cred", "purpose": "STORAGE",
            "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } })),
        )
        .await;
        let body = json!({ "name": "el", "url": url, "credential_name": "cred" });
        self.ok(&self.admin, Method::POST, "/external-locations", Some(body))
            .await;
    }
}

const ALICE: &str = "alice@example.com";
const BOB: &str = "bob@example.com";

#[tokio::test]
async fn catalog_rules() {
    let w = World::new().await;
    // Creating a catalog needs the admin or CREATE CATALOG on the metastore.
    let denied = w
        .call(
            &w.alice,
            Method::POST,
            "/catalogs",
            Some(json!({ "name": "c1" })),
        )
        .await;
    let err = common::assert_error(denied, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    assert_eq!(err["message"], "Access denied.");
    w.grant(
        "metastore",
        "metastore",
        "alice@example.com",
        &["CREATE CATALOG"],
    )
    .await;
    let created = w
        .ok(
            &w.alice,
            Method::POST,
            "/catalogs",
            Some(json!({ "name": "c1" })),
        )
        .await;
    assert_eq!(created["owner"], "alice@example.com");
    w.ok(
        &w.admin,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c2" })),
    )
    .await;

    // Reads: the admin, the owner, or USE CATALOG; lists keep only those.
    assert_eq!(
        w.status(&w.bob, Method::GET, "/catalogs/c1", None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        World::names(
            &w.ok(&w.bob, Method::GET, "/catalogs", None).await,
            "catalogs"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        World::names(
            &w.ok(&w.alice, Method::GET, "/catalogs", None).await,
            "catalogs"
        ),
        ["c1"]
    );
    assert_eq!(
        World::names(
            &w.ok(&w.admin, Method::GET, "/catalogs", None).await,
            "catalogs"
        ),
        ["c1", "c2"]
    );
    w.grant("catalog", "c2", "bob@example.com", &["USE CATALOG"])
        .await;
    w.ok(&w.bob, Method::GET, "/catalogs/c2", None).await;
    // A grant on the metastore counts for every catalog.
    w.grant(
        "metastore",
        "metastore",
        "bob@example.com",
        &["USE CATALOG"],
    )
    .await;
    w.ok(&w.bob, Method::GET, "/catalogs/c1", None).await;

    // Changes: only the owner; MODIFY is not enough. Deletes: owner or admin.
    w.grant("catalog", "c1", "bob@example.com", &["MODIFY"])
        .await;
    let patch = Some(json!({ "comment": "x" }));
    assert_eq!(
        w.status(&w.bob, Method::PATCH, "/catalogs/c1", patch.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.admin, Method::PATCH, "/catalogs/c1", patch.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.ok(&w.alice, Method::PATCH, "/catalogs/c1", patch).await;
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/catalogs/c1", None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.admin, Method::DELETE, "/catalogs/c1", None)
            .await,
        StatusCode::OK
    );
    // A missing catalog is 404, not 403.
    assert_eq!(
        w.status(&w.bob, Method::GET, "/catalogs/nope", None).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn schema_rules() {
    let w = World::new().await;
    w.ok(
        &w.admin,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c" })),
    )
    .await;
    // Creating a schema: catalog owner, or USE CATALOG + CREATE SCHEMA.
    let create = |name: &str| Some(json!({ "name": name, "catalog_name": "c" }));
    w.grant("catalog", "c", "alice@example.com", &["USE CATALOG"])
        .await;
    assert_eq!(
        w.status(&w.alice, Method::POST, "/schemas", create("s1"))
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("catalog", "c", "alice@example.com", &["CREATE SCHEMA"])
        .await;
    assert_eq!(
        w.ok(&w.alice, Method::POST, "/schemas", create("s1")).await["owner"],
        "alice@example.com"
    );
    w.ok(&w.admin, Method::POST, "/schemas", create("s2")).await;

    // Reads: USE CATALOG and the schema's OWNER or USE SCHEMA.
    let list = "/schemas?catalog_name=c";
    assert_eq!(
        World::names(&w.ok(&w.alice, Method::GET, list, None).await, "schemas"),
        ["s1"]
    );
    assert_eq!(
        w.status(&w.alice, Method::GET, "/schemas/c.s2", None).await,
        StatusCode::FORBIDDEN
    );
    // USE SCHEMA granted on the catalog counts for its schemas.
    w.grant("catalog", "c", "alice@example.com", &["USE SCHEMA"])
        .await;
    w.ok(&w.alice, Method::GET, "/schemas/c.s2", None).await;
    assert_eq!(
        World::names(&w.ok(&w.alice, Method::GET, list, None).await, "schemas"),
        ["s1", "s2"]
    );
    // Without USE CATALOG, schema grants do not help.
    w.grant("schema", "c.s2", "bob@example.com", &["USE SCHEMA"])
        .await;
    assert_eq!(
        w.status(&w.bob, Method::GET, "/schemas/c.s2", None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        World::names(&w.ok(&w.bob, Method::GET, list, None).await, "schemas"),
        Vec::<String>::new()
    );

    // Changes and deletes: catalog owner, or USE CATALOG + schema owner.
    let patch = Some(json!({ "comment": "x" }));
    assert_eq!(
        w.status(&w.alice, Method::PATCH, "/schemas/c.s2", patch.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.ok(&w.alice, Method::PATCH, "/schemas/c.s1", patch.clone())
        .await;
    w.ok(&w.admin, Method::PATCH, "/schemas/c.s2", patch).await;
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/schemas/c.s1", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.alice, Method::DELETE, "/schemas/c.s1", None)
            .await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn managed_storage_needs_the_external_location() {
    let w = World::new().await;
    let root = common::temp_storage_root();
    w.ok(
        &w.admin,
        Method::POST,
        "/credentials",
        Some(json!({ "name": "cred", "purpose": "STORAGE",
        "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } })),
    )
    .await;
    w.ok(
        &w.admin,
        Method::POST,
        "/external-locations",
        Some(json!({ "name": "el", "url": root, "credential_name": "cred" })),
    )
    .await;
    w.grant(
        "metastore",
        "metastore",
        "alice@example.com",
        &["CREATE CATALOG"],
    )
    .await;
    let body = Some(json!({ "name": "c", "storage_root": format!("{root}/c") }));
    assert_eq!(
        w.status(&w.alice, Method::POST, "/catalogs", body.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant(
        "external_location",
        "el",
        "alice@example.com",
        &["CREATE MANAGED STORAGE"],
    )
    .await;
    w.ok(&w.alice, Method::POST, "/catalogs", body).await;
    // Without an external location there is nowhere to put it.
    let elsewhere = Some(json!({ "name": "d", "storage_root": common::temp_storage_root() }));
    assert_eq!(
        w.status(&w.admin, Method::POST, "/catalogs", elsewhere)
            .await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn table_rules() {
    let w = World::new().await;
    w.ok(
        &w.admin,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c" })),
    )
    .await;
    w.ok(
        &w.admin,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "c" })),
    )
    .await;
    let table = |name: &str| {
        Some(
            json!({ "name": name, "catalog_name": "c", "schema_name": "s",
        "table_type": "EXTERNAL", "data_source_format": "DELTA", "storage_location": format!("s3://lake/{name}") }),
        )
    };
    // A path that no external location covers is refused, also for the admin.
    let resp = w.call(&w.admin, Method::POST, "/tables", table("t1")).await;
    let err = common::assert_error(resp, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    assert_eq!(err["message"], "No external location covers s3://lake/t1.");
    w.external_location("s3://lake").await;
    w.grant("external_location", "el", ALICE, &["CREATE EXTERNAL TABLE"])
        .await;
    // Creating: USE CATALOG, and USE SCHEMA + CREATE TABLE.
    for user in ["alice@example.com", "bob@example.com"] {
        w.grant("catalog", "c", user, &["USE CATALOG"]).await;
        w.grant("schema", "c.s", user, &["USE SCHEMA"]).await;
    }
    assert_eq!(
        w.status(&w.alice, Method::POST, "/tables", table("t1"))
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("schema", "c.s", "alice@example.com", &["CREATE TABLE"])
        .await;
    let created = w.ok(&w.alice, Method::POST, "/tables", table("t1")).await;
    assert_eq!(created["owner"], "alice@example.com");
    let table_id = created["table_id"].as_str().unwrap().to_string();

    // Reads need OWNER, SELECT or MODIFY on the table.
    assert_eq!(
        w.status(&w.bob, Method::GET, "/tables/c.s.t1", None).await,
        StatusCode::FORBIDDEN
    );
    let list = "/tables?catalog_name=c&schema_name=s";
    assert_eq!(
        World::names(&w.ok(&w.bob, Method::GET, list, None).await, "tables"),
        Vec::<String>::new()
    );
    assert_eq!(
        World::names(&w.ok(&w.alice, Method::GET, list, None).await, "tables"),
        ["t1"]
    );
    let delta = "/delta/v1/catalogs/c/schemas/s/tables/t1";
    let resp = w.call(&w.bob, Method::GET, delta, None).await;
    common::assert_delta_error(resp, StatusCode::FORBIDDEN, "PermissionDeniedException").await;
    w.grant("table", "c.s.t1", "bob@example.com", &["SELECT"])
        .await;
    w.ok(&w.bob, Method::GET, "/tables/c.s.t1", None).await;
    assert_eq!(
        World::names(&w.ok(&w.bob, Method::GET, list, None).await, "tables"),
        ["t1"]
    );

    // Credentials: READ with SELECT; READ_WRITE needs MODIFY too, or OWNER.
    let creds = |op: &str| Some(json!({ "table_id": table_id, "operation": op }));
    let vend = "/temporary-table-credentials";
    assert_ne!(
        w.status(&w.bob, Method::POST, vend, creds("READ")).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.bob, Method::POST, vend, creds("READ_WRITE"))
            .await,
        StatusCode::FORBIDDEN
    );
    assert_ne!(
        w.status(&w.alice, Method::POST, vend, creds("READ_WRITE"))
            .await,
        StatusCode::FORBIDDEN
    );

    // Deletes: the table owner (with the USE privileges), schema or catalog owner.
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/tables/c.s.t1", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.alice, Method::DELETE, "/tables/c.s.t1", None)
            .await,
        StatusCode::OK
    );
    // The catalog owner may delete any table in it.
    w.ok(&w.alice, Method::POST, "/tables", table("t2")).await;
    assert_eq!(
        w.status(&w.admin, Method::DELETE, "/tables/c.s.t2", None)
            .await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn volume_rules() {
    let w = World::new().await;
    w.catalog_and_schema(&[ALICE, BOB]).await;
    w.external_location("s3://el-bucket/data").await;
    let volume = |name: &str, location: &str| {
        Some(
            json!({ "name": name, "catalog_name": "c", "schema_name": "s",
        "volume_type": "EXTERNAL", "storage_location": location }),
        )
    };
    // Creating: USE CATALOG, and USE SCHEMA + CREATE VOLUME, and CREATE
    // EXTERNAL VOLUME on the external location; never inside another table
    // or volume.
    let v1 = volume("v1", "s3://el-bucket/data/v1");
    assert_eq!(
        w.status(&w.alice, Method::POST, "/volumes", v1.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("schema", "c.s", ALICE, &["CREATE VOLUME"]).await;
    assert_eq!(
        w.status(&w.alice, Method::POST, "/volumes", v1.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant(
        "external_location",
        "el",
        ALICE,
        &["CREATE EXTERNAL VOLUME"],
    )
    .await;
    let created = w.ok(&w.alice, Method::POST, "/volumes", v1).await;
    assert_eq!(created["owner"], ALICE);
    let volume_id = created["volume_id"].as_str().unwrap().to_string();
    w.ok(
        &w.alice,
        Method::POST,
        "/volumes",
        volume("v2", "s3://el-bucket/data/v2"),
    )
    .await;
    // A path that no external location covers is refused, also for the admin.
    let resp = w
        .call(
            &w.admin,
            Method::POST,
            "/volumes",
            volume("v3", "s3://lake/v3"),
        )
        .await;
    let err = common::assert_error(resp, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    assert_eq!(err["message"], "No external location covers s3://lake/v3.");
    let inside = volume("v3", "s3://el-bucket/data/v1/inner");
    assert_eq!(
        w.status(&w.alice, Method::POST, "/volumes", inside).await,
        StatusCode::FORBIDDEN
    );

    // Reads need OWNER or READ VOLUME on the volume.
    let list = "/volumes?catalog_name=c&schema_name=s";
    assert_eq!(
        w.status(&w.bob, Method::GET, "/volumes/c.s.v1", None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        World::names(&w.ok(&w.bob, Method::GET, list, None).await, "volumes"),
        Vec::<String>::new()
    );
    assert_eq!(
        World::names(&w.ok(&w.alice, Method::GET, list, None).await, "volumes"),
        ["v1", "v2"]
    );
    w.grant("volume", "c.s.v1", BOB, &["READ VOLUME"]).await;
    w.ok(&w.bob, Method::GET, "/volumes/c.s.v1", None).await;
    assert_eq!(
        World::names(&w.ok(&w.bob, Method::GET, list, None).await, "volumes"),
        ["v1"]
    );

    // Credentials: READ VOLUME reads; writes need OWNER.
    let creds = |op: &str| Some(json!({ "volume_id": volume_id, "operation": op }));
    let vend = "/temporary-volume-credentials";
    assert_ne!(
        w.status(&w.bob, Method::POST, vend, creds("READ_VOLUME"))
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.bob, Method::POST, vend, creds("WRITE_VOLUME"))
            .await,
        StatusCode::FORBIDDEN
    );
    assert_ne!(
        w.status(&w.alice, Method::POST, vend, creds("WRITE_VOLUME"))
            .await,
        StatusCode::FORBIDDEN
    );

    // Changes and deletes: the owner (with the USE privileges), schema or
    // catalog owner.
    let patch = Some(json!({ "comment": "x" }));
    assert_eq!(
        w.status(&w.bob, Method::PATCH, "/volumes/c.s.v1", patch.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.ok(&w.alice, Method::PATCH, "/volumes/c.s.v1", patch)
        .await;
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/volumes/c.s.v1", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.admin, Method::DELETE, "/volumes/c.s.v2", None)
            .await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn function_and_model_rules() {
    let w = World::new().await;
    // Managed models need a storage root, under an external location.
    let root = common::temp_storage_root();
    w.external_location(&root).await;
    w.catalog_and_schema_at(&[ALICE, BOB], Some(format!("{root}/c")))
        .await;
    let function = Some(json!({ "function_info": {
        "name": "f", "catalog_name": "c", "schema_name": "s", "routine_body": "EXTERNAL",
        "parameter_style": "S", "security_type": "DEFINER" } }));
    // Functions: USE SCHEMA + CREATE FUNCTION; reads need OWNER or EXECUTE.
    assert_eq!(
        w.status(&w.alice, Method::POST, "/functions", function.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("schema", "c.s", ALICE, &["CREATE FUNCTION"]).await;
    assert_eq!(
        w.ok(&w.alice, Method::POST, "/functions", function).await["owner"],
        ALICE
    );
    let functions = "/functions?catalog_name=c&schema_name=s";
    assert_eq!(
        w.status(&w.bob, Method::GET, "/functions/c.s.f", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        World::names(
            &w.ok(&w.bob, Method::GET, functions, None).await,
            "functions"
        ),
        Vec::<String>::new()
    );
    w.grant("function", "c.s.f", BOB, &["EXECUTE"]).await;
    w.ok(&w.bob, Method::GET, "/functions/c.s.f", None).await;
    assert_eq!(
        World::names(
            &w.ok(&w.bob, Method::GET, functions, None).await,
            "functions"
        ),
        ["f"]
    );
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/functions/c.s.f", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.alice, Method::DELETE, "/functions/c.s.f", None)
            .await,
        StatusCode::OK
    );

    // Models: CREATE MODEL or CREATE FUNCTION.
    let model = |name: &str| Some(json!({ "name": name, "catalog_name": "c", "schema_name": "s" }));
    w.ok(&w.alice, Method::POST, "/models", model("m1")).await;
    assert_eq!(
        w.status(&w.bob, Method::POST, "/models", model("m2")).await,
        StatusCode::FORBIDDEN
    );
    w.grant("schema", "c.s", BOB, &["CREATE MODEL"]).await;
    w.ok(&w.bob, Method::POST, "/models", model("m2")).await;
    // Reads, of models and their versions: OWNER or EXECUTE.
    let models = "/models?catalog_name=c&schema_name=s";
    assert_eq!(
        w.status(&w.bob, Method::GET, "/models/c.s.m1", None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        World::names(
            &w.ok(&w.bob, Method::GET, models, None).await,
            "registered_models"
        ),
        ["m2"]
    );
    w.grant("registered_model", "c.s.m1", BOB, &["EXECUTE"])
        .await;
    w.ok(&w.bob, Method::GET, "/models/c.s.m1", None).await;
    assert_eq!(
        World::names(
            &w.ok(&w.bob, Method::GET, models, None).await,
            "registered_models"
        ),
        ["m1", "m2"]
    );
    // New versions and changes: the model owner.
    let version = Some(
        json!({ "model_name": "m1", "catalog_name": "c", "schema_name": "s", "source": "s3://lake/m1/v1" }),
    );
    assert_eq!(
        w.status(&w.bob, Method::POST, "/models/versions", version.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.ok(&w.alice, Method::POST, "/models/versions", version)
        .await;
    w.ok(&w.bob, Method::GET, "/models/c.s.m1/versions/1", None)
        .await;
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/models/c.s.m1/versions/1", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.alice, Method::DELETE, "/models/c.s.m1/versions/1", None)
            .await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn credential_and_external_location_rules() {
    let w = World::new().await;
    let credential = |name: &str| {
        Some(json!({ "name": name, "purpose": "STORAGE",
        "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } }))
    };
    // Credentials: the admin or CREATE STORAGE CREDENTIAL creates.
    assert_eq!(
        w.status(&w.alice, Method::POST, "/credentials", credential("ca"))
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant(
        "metastore",
        "metastore",
        ALICE,
        &["CREATE STORAGE CREDENTIAL"],
    )
    .await;
    assert_eq!(
        w.ok(&w.alice, Method::POST, "/credentials", credential("ca"))
            .await["owner"],
        ALICE
    );
    w.ok(&w.admin, Method::POST, "/credentials", credential("cb"))
        .await;
    // Reads: the admin, the owner or CREATE EXTERNAL LOCATION on it.
    assert_eq!(
        w.status(&w.bob, Method::GET, "/credentials/ca", None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        World::names(
            &w.ok(&w.bob, Method::GET, "/credentials", None).await,
            "credentials"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        World::names(
            &w.ok(&w.alice, Method::GET, "/credentials", None).await,
            "credentials"
        ),
        ["ca"]
    );
    assert_eq!(
        World::names(
            &w.ok(&w.admin, Method::GET, "/credentials", None).await,
            "credentials"
        ),
        ["ca", "cb"]
    );
    w.grant("credential", "ca", BOB, &["CREATE EXTERNAL LOCATION"])
        .await;
    w.ok(&w.bob, Method::GET, "/credentials/ca", None).await;
    // Changes: the admin or the owner.
    let patch = Some(json!({ "comment": "x" }));
    assert_eq!(
        w.status(&w.bob, Method::PATCH, "/credentials/ca", patch.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.ok(&w.alice, Method::PATCH, "/credentials/ca", patch.clone())
        .await;

    // External locations: CREATE EXTERNAL LOCATION on the metastore and on
    // the credential.
    let location = |name: &str, url: &str, credential: &str| {
        Some(json!({ "name": name, "url": url, "credential_name": credential }))
    };
    let e1 = location("e1", "s3://b1/x", "ca");
    assert_eq!(
        w.status(&w.bob, Method::POST, "/external-locations", e1.clone())
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("metastore", "metastore", BOB, &["CREATE EXTERNAL LOCATION"])
        .await;
    assert_eq!(
        w.ok(&w.bob, Method::POST, "/external-locations", e1).await["owner"],
        BOB
    );
    let e2 = location("e2", "s3://b2/x", "cb");
    assert_eq!(
        w.status(&w.bob, Method::POST, "/external-locations", e2)
            .await,
        StatusCode::FORBIDDEN
    );
    // Reads: the admin, the owner, or a privilege on it.
    assert_eq!(
        w.status(&w.alice, Method::GET, "/external-locations/e1", None)
            .await,
        StatusCode::FORBIDDEN
    );
    let list = "/external-locations";
    assert_eq!(
        World::names(
            &w.ok(&w.alice, Method::GET, list, None).await,
            "external_locations"
        ),
        Vec::<String>::new()
    );
    w.grant("external_location", "e1", ALICE, &["READ FILES"])
        .await;
    w.ok(&w.alice, Method::GET, "/external-locations/e1", None)
        .await;
    assert_eq!(
        World::names(
            &w.ok(&w.alice, Method::GET, list, None).await,
            "external_locations"
        ),
        ["e1"]
    );
    // Changes: the owner, who must be able to use a new credential.
    w.ok(&w.bob, Method::PATCH, "/external-locations/e1", patch)
        .await;
    let to_cb = Some(json!({ "credential_name": "cb" }));
    assert_eq!(
        w.status(
            &w.bob,
            Method::PATCH,
            "/external-locations/e1",
            to_cb.clone()
        )
        .await,
        StatusCode::FORBIDDEN
    );
    w.ok(&w.admin, Method::PATCH, "/external-locations/e1", to_cb)
        .await;
    // Deletes: the admin or the owner.
    assert_eq!(
        w.status(&w.alice, Method::DELETE, "/external-locations/e1", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/external-locations/e1", None)
            .await,
        StatusCode::OK
    );
    assert_eq!(
        w.status(&w.bob, Method::DELETE, "/credentials/ca", None)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.alice, Method::DELETE, "/credentials/ca", None)
            .await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn path_credential_rules() {
    let w = World::new().await;
    w.catalog_and_schema(&[]).await;
    w.external_location("s3://el-bucket/data").await;
    let path = |url: &str, op: &str| Some(json!({ "url": url, "operation": op }));
    let vend = "/temporary-path-credentials";
    // Under an external location: READ FILES reads, WRITE FILES too writes,
    // CREATE EXTERNAL TABLE creates tables.
    let file = "s3://el-bucket/data/f";
    assert_eq!(
        w.status(&w.alice, Method::POST, vend, path(file, "PATH_READ"))
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("external_location", "el", ALICE, &["READ FILES"])
        .await;
    w.ok(&w.alice, Method::POST, vend, path(file, "PATH_READ"))
        .await;
    assert_eq!(
        w.status(&w.alice, Method::POST, vend, path(file, "PATH_READ_WRITE"))
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("external_location", "el", ALICE, &["WRITE FILES"])
        .await;
    w.ok(&w.alice, Method::POST, vend, path(file, "PATH_READ_WRITE"))
        .await;
    assert_eq!(
        w.status(
            &w.alice,
            Method::POST,
            vend,
            path(file, "PATH_CREATE_TABLE")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    w.grant("external_location", "el", ALICE, &["CREATE EXTERNAL TABLE"])
        .await;
    w.ok(
        &w.alice,
        Method::POST,
        vend,
        path(file, "PATH_CREATE_TABLE"),
    )
    .await;
    // Inside a table, the table decides.
    w.ok(&w.admin, Method::POST, "/tables", Some(json!({ "name": "t", "catalog_name": "c", "schema_name": "s",
        "table_type": "EXTERNAL", "data_source_format": "DELTA", "storage_location": "s3://el-bucket/data/t" }))).await;
    let in_table = "s3://el-bucket/data/t/part-0.parquet";
    assert_eq!(
        w.status(&w.alice, Method::POST, vend, path(in_table, "PATH_READ"))
            .await,
        StatusCode::FORBIDDEN
    );
    w.grant("catalog", "c", ALICE, &["USE CATALOG"]).await;
    w.grant("schema", "c.s", ALICE, &["USE SCHEMA"]).await;
    w.grant("table", "c.s.t", ALICE, &["SELECT"]).await;
    w.ok(&w.alice, Method::POST, vend, path(in_table, "PATH_READ"))
        .await;
    assert_eq!(
        w.status(
            &w.alice,
            Method::POST,
            vend,
            path(in_table, "PATH_CREATE_TABLE")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    // A path above an entity is refused.
    let above = w
        .call(
            &w.alice,
            Method::POST,
            vend,
            path("s3://el-bucket", "PATH_READ"),
        )
        .await;
    common::assert_error(above, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    // A path that no external location covers is refused, also for the admin.
    let uncovered = w
        .call(
            &w.admin,
            Method::POST,
            vend,
            path("s3://lake/t", "PATH_CREATE_TABLE"),
        )
        .await;
    let err = common::assert_error(uncovered, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    assert_eq!(err["message"], "No external location covers s3://lake/t.");
}

#[tokio::test]
async fn permission_rules() {
    let w = World::new().await;
    w.catalog_and_schema(&[]).await;
    w.grant("catalog", "c", ALICE, &["USE CATALOG", "CREATE SCHEMA"])
        .await;
    w.ok(
        &w.alice,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "mine", "catalog_name": "c" })),
    )
    .await;
    w.grant("schema", "c.s", BOB, &["USE SCHEMA"]).await;
    let principals = |body: &Value| -> Vec<String> {
        body["privilege_assignments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["principal"].as_str().unwrap().to_string())
            .collect()
    };
    // Others see their own grants only; the admin and owners up the tree
    // see all.
    let get = "/permissions/schema/c.s";
    assert_eq!(
        principals(&w.ok(&w.bob, Method::GET, get, None).await),
        [BOB]
    );
    assert_eq!(
        principals(&w.ok(&w.alice, Method::GET, get, None).await),
        Vec::<String>::new()
    );
    let bobs = format!("{get}?principal={BOB}");
    assert_eq!(
        principals(&w.ok(&w.alice, Method::GET, &bobs, None).await),
        Vec::<String>::new()
    );
    assert_eq!(
        principals(&w.ok(&w.admin, Method::GET, get, None).await),
        [BOB]
    );
    assert_eq!(
        principals(
            &w.ok(&w.admin, Method::GET, "/permissions/catalog/c", None)
                .await
        ),
        [ALICE]
    );

    // Changes: the metastore needs the admin.
    let change = |principal: &str, privilege: &str| {
        Some(json!({ "changes": [{ "principal": principal, "add": [privilege], "remove": [] }] }))
    };
    let metastore = "/permissions/metastore/metastore";
    assert_eq!(
        w.status(
            &w.alice,
            Method::PATCH,
            metastore,
            change(BOB, "CREATE CATALOG")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    // A schema: the catalog owner, or its owner with USE CATALOG.
    assert_eq!(
        w.status(&w.alice, Method::PATCH, get, change(BOB, "CREATE TABLE"))
            .await,
        StatusCode::FORBIDDEN
    );
    let mine = "/permissions/schema/c.mine";
    w.ok(&w.alice, Method::PATCH, mine, change(BOB, "USE SCHEMA"))
        .await;
    assert_eq!(
        principals(&w.ok(&w.alice, Method::GET, mine, None).await),
        [BOB]
    );
    assert_eq!(
        w.status(&w.bob, Method::PATCH, mine, change(BOB, "CREATE TABLE"))
            .await,
        StatusCode::FORBIDDEN
    );
    // Grants go to users that exist, named by user name or email.
    let ghost = w
        .call(
            &w.admin,
            Method::PATCH,
            mine,
            change("ghost@example.com", "USE SCHEMA"),
        )
        .await;
    common::assert_error(ghost, StatusCode::NOT_FOUND, "NOT_FOUND").await;
    // A missing securable is 404, not 403.
    assert_eq!(
        w.status(&w.bob, Method::GET, "/permissions/catalog/nope", None)
            .await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn admin_endpoints() {
    let w = World::new().await;
    // SCIM users are the admin's; anyone signed in sees themself.
    assert_eq!(
        w.control(&w.alice, "/scim2/Users").await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(w.control(&w.admin, "/scim2/Users").await, StatusCode::OK);
    let mut me = common::control(Method::GET, "/scim2/Me");
    me.headers_mut().insert(
        "authorization",
        format!("Bearer {}", w.alice).parse().unwrap(),
    );
    let me = json_body(
        assert_status(
            w.app.clone().oneshot(me).await.unwrap(),
            StatusCode::OK,
            "me",
        )
        .await,
    )
    .await;
    assert_eq!(me["userName"], ALICE);
    // Only the admin's names the role `admin`.
    assert!(me.get("roles").is_none(), "{me}");
    let mut admin_me = common::control(Method::GET, "/scim2/Me");
    admin_me.headers_mut().insert(
        "authorization",
        format!("Bearer {}", w.admin).parse().unwrap(),
    );
    let admin_me = json_body(
        assert_status(
            w.app.clone().oneshot(admin_me).await.unwrap(),
            StatusCode::OK,
            "admin's me",
        )
        .await,
    )
    .await;
    assert_eq!(admin_me["roles"], json!([{ "value": "admin" }]));
    // Not signed in: 401.
    let anonymous = w
        .app
        .clone()
        .oneshot(common::control(Method::GET, "/scim2/Me"))
        .await
        .unwrap();
    common::assert_error(anonymous, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED").await;
    // So are shares.
    assert_eq!(
        w.status(&w.alice, Method::GET, "/shares", None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        w.status(&w.admin, Method::GET, "/shares", None).await,
        StatusCode::OK
    );
}

/// Upstream `initializeAdmin`: the user `admin` of `uc-server token`
/// exists and owns the metastore.
#[tokio::test]
async fn admin_bootstrap() {
    let (app, metastore) = common::signed_in_app().await;
    uc_server::authn::security::initialize_admin(&metastore)
        .await
        .unwrap();
    uc_server::authn::security::initialize_admin(&metastore)
        .await
        .unwrap();
    assert_eq!(
        metastore
            .permissions()
            .metastore_owner()
            .await
            .unwrap()
            .as_deref(),
        Some("admin")
    );
    let admin = metastore
        .scim()
        .find_user_by_email("admin")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(admin.display_name.as_deref(), Some("Admin"));
    let token = common::admin_token();
    let resp = app
        .oneshot(signed(
            Method::POST,
            "/catalogs",
            Some(json!({ "name": "c" })),
            &token,
        ))
        .await
        .unwrap();
    let created =
        json_body(assert_status(resp, StatusCode::OK, "admin creates a catalog").await).await;
    assert_eq!(created["owner"], "admin");
}

/// `[[auth.users]]` and `auth.admin_principals`: each start makes the
/// missing users, with the missing metastore privileges of the list, in one
/// transaction. Users that exist stay as they are; disabled ones get no
/// privileges.
#[tokio::test]
async fn users_of_the_settings() {
    use uc_api::{PermissionsChange, Privilege, UpdatePermissions};
    use uc_common::SecurableType;
    use uc_server::authn::bootstrap;

    let (app, metastore) = common::signed_in_app().await;
    let settings = |text: &str| {
        let auth = uc_common::settings::Settings::from_toml(text).unwrap().auth;
        bootstrap::seed_users(&auth).unwrap()
    };
    let seeds = settings(
        r#"
        [auth]
        admin_principals = ["carol@example.com"]

        [[auth.users]]
        email = "alice@example.com"
        name = "Alice"
        privileges = ["CREATE CATALOG"]

        [[auth.users]]
        email = "etl@example.com"
        external_id = "etl-client"
        "#,
    );
    let report = bootstrap::seed(&metastore, &seeds).await.unwrap();
    assert_eq!((report.created, report.granted), (3, 1));
    let report = bootstrap::seed(&metastore, &seeds).await.unwrap();
    assert_eq!((report.created, report.granted), (0, 0));
    let scim = metastore.scim();
    let alice = scim.find_user_by_email("alice@example.com").await.unwrap();
    assert_eq!(alice.unwrap().display_name.as_deref(), Some("Alice"));
    let etl = scim.find_user_by_external_id("etl-client").await.unwrap();
    assert_eq!(etl.unwrap().user_name, "etl@example.com");
    assert!(
        scim.find_user_by_email("carol@example.com")
            .await
            .unwrap()
            .is_some()
    );

    // alice makes a catalog with her privilege.
    let alice = common::security()
        .create_access_token("alice@example.com", std::time::Duration::from_secs(60))
        .unwrap();
    let resp = app
        .clone()
        .oneshot(signed(
            Method::POST,
            "/catalogs",
            Some(json!({ "name": "seeded" })),
            &alice,
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "alice creates a catalog").await;

    // A revoked privilege of the list comes back at the next start.
    let revoke = UpdatePermissions {
        changes: vec![PermissionsChange {
            principal: "alice@example.com".into(),
            add: Vec::new(),
            remove: vec![Privilege::CreateCatalog],
        }],
    };
    let permissions = metastore.permissions();
    permissions
        .update(SecurableType::Metastore, "metastore", revoke)
        .await
        .unwrap();
    assert!(
        permissions
            .grants_of("alice@example.com")
            .await
            .unwrap()
            .is_empty()
    );
    let report = bootstrap::seed(&metastore, &seeds).await.unwrap();
    assert_eq!((report.created, report.granted), (0, 1));

    // A user that exists keeps its name; a disabled one gets nothing.
    scim.create_user(uc_api::ScimUser {
        user_name: "dave@example.com".into(),
        display_name: Some("Dave".into()),
        active: Some(false),
        ..Default::default()
    })
    .await
    .unwrap();
    let dave = settings(
        r#"
        [[auth.users]]
        email = "dave@example.com"
        name = "David"
        privileges = ["CREATE CATALOG"]
        "#,
    );
    let report = bootstrap::seed(&metastore, &dave).await.unwrap();
    assert_eq!((report.created, report.granted), (0, 0));
    let dave = scim.find_user_by_email("dave@example.com").await.unwrap();
    assert_eq!(dave.unwrap().display_name.as_deref(), Some("Dave"));

    // An entry never takes over another identity, and a failed start
    // changes nothing.
    for (text, message) in [
        (
            "[[auth.users]]\nemail = \"etl@example.com\"\nexternal_id = \"other\"",
            "User etl@example.com exists with an external id other than other",
        ),
        (
            "[[auth.users]]\nemail = \"erin@example.com\"\n\n\
             [[auth.users]]\nemail = \"eve@example.com\"\nexternal_id = \"etl-client\"",
            "The external id etl-client of eve@example.com belongs to etl@example.com",
        ),
    ] {
        let error = bootstrap::seed(&metastore, &settings(text))
            .await
            .unwrap_err();
        assert_eq!(error.message(), message);
    }
    assert!(
        scim.find_user_by_email("erin@example.com")
            .await
            .unwrap()
            .is_none()
    );
}

/// Upstream `deleteUser`: the user's grants go and its tokens stop working.
#[tokio::test]
async fn deleted_users_lose_grants_and_sign_in() {
    let w = World::new().await;
    w.catalog_and_schema(&[BOB]).await;
    w.ok(&w.bob, Method::GET, "/catalogs/c", None).await;
    let mut find = common::control(
        Method::GET,
        "/scim2/Users?filter=userName%20eq%20%22bob%40example.com%22",
    );
    find.headers_mut().insert(
        "authorization",
        format!("Bearer {}", w.admin).parse().unwrap(),
    );
    let found = json_body(
        assert_status(
            w.app.clone().oneshot(find).await.unwrap(),
            StatusCode::OK,
            "find",
        )
        .await,
    )
    .await;
    let id = found["Resources"][0]["id"].as_str().unwrap().to_string();
    let mut delete = common::control(Method::DELETE, &format!("/scim2/Users/{id}"));
    delete.headers_mut().insert(
        "authorization",
        format!("Bearer {}", w.admin).parse().unwrap(),
    );
    assert_status(
        w.app.clone().oneshot(delete).await.unwrap(),
        StatusCode::OK,
        "delete",
    )
    .await;

    let refused = w.call(&w.bob, Method::GET, "/catalogs/c", None).await;
    let err = common::assert_error(refused, StatusCode::FORBIDDEN, "PERMISSION_DENIED").await;
    assert_eq!(err["message"], "User not allowed: bob@example.com");
    let grants = w
        .ok(&w.admin, Method::GET, "/permissions/catalog/c", None)
        .await;
    assert_eq!(grants["privilege_assignments"], json!([]));
}

/// A grant to a group counts for its members. A renamed group keeps its
/// grants, and a deleted group's grants go. A user and a group cannot
/// share a name.
#[tokio::test]
async fn groups_give_their_grants_to_members() {
    const GROUP: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
    const PATCH: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
    let w = World::new().await;
    w.catalog_and_schema(&[]).await;
    let alice = w.metastore.scim().find_user_by_email(ALICE).await.unwrap();
    let alice = alice.unwrap().id.unwrap();
    let group = json!({ "schemas": [GROUP], "displayName": "data",
        "members": [{ "value": alice, "display": ALICE }] });
    let created = w.scim(Method::POST, "/scim2/Groups", group).await;
    let created = json_body(assert_status(created, StatusCode::CREATED, "group").await).await;
    let group = format!("/scim2/Groups/{}", created["id"].as_str().unwrap());
    let patch = |op: Value| json!({ "schemas": [PATCH], "Operations": [op] });

    assert_eq!(w.get_catalog(&w.alice).await, StatusCode::FORBIDDEN);
    w.grant("catalog", "c", "data", &["USE CATALOG"]).await;
    assert_eq!(w.get_catalog(&w.alice).await, StatusCode::OK);
    assert_eq!(w.get_catalog(&w.bob).await, StatusCode::FORBIDDEN);
    // CREATE SCHEMA of her own and USE CATALOG of the group.
    w.grant("catalog", "c", ALICE, &["CREATE SCHEMA"]).await;
    let schema = json!({ "name": "s2", "catalog_name": "c" });
    w.ok(&w.alice, Method::POST, "/schemas", Some(schema)).await;

    let rename = patch(json!({ "op": "replace", "path": "displayName", "value": "analysts" }));
    let renamed = w.scim(Method::PATCH, &group, rename).await;
    assert_status(renamed, StatusCode::OK, "rename").await;
    let grants = w
        .ok(&w.admin, Method::GET, "/permissions/catalog/c", None)
        .await;
    assert_eq!(
        grants["privilege_assignments"],
        json!([
            { "principal": ALICE, "privileges": ["CREATE SCHEMA"] },
            { "principal": "analysts", "privileges": ["USE CATALOG"] },
        ])
    );
    assert_eq!(w.get_catalog(&w.alice).await, StatusCode::OK);

    let path = format!("members[value eq \"{alice}\"]");
    let remove = patch(json!({ "op": "remove", "path": path }));
    let removed = w.scim(Method::PATCH, &group, remove).await;
    assert_status(removed, StatusCode::OK, "remove").await;
    assert_eq!(w.get_catalog(&w.alice).await, StatusCode::FORBIDDEN);
    let add = patch(json!({ "op": "add", "path": "members", "value": [{ "value": alice }] }));
    let added = w.scim(Method::PATCH, &group, add).await;
    assert_status(added, StatusCode::OK, "add").await;
    assert_eq!(w.get_catalog(&w.alice).await, StatusCode::OK);

    let deleted = w.scim(Method::DELETE, &group, Value::Null).await;
    assert_status(deleted, StatusCode::NO_CONTENT, "delete").await;
    assert_eq!(w.get_catalog(&w.alice).await, StatusCode::FORBIDDEN);
    let grants = w
        .ok(&w.admin, Method::GET, "/permissions/catalog/c", None)
        .await;
    assert_eq!(
        grants["privilege_assignments"],
        json!([{ "principal": ALICE, "privileges": ["CREATE SCHEMA"] }])
    );

    // Grants need a user or a group of that name.
    let body = json!({ "changes": [
        { "principal": "analysts", "add": ["USE CATALOG"], "remove": [] },
    ] });
    let path = "/permissions/catalog/c";
    let missing = w.call(&w.admin, Method::PATCH, path, Some(body)).await;
    common::assert_error(missing, StatusCode::NOT_FOUND, "NOT_FOUND").await;
    let taken = json!({ "schemas": [GROUP], "displayName": ALICE });
    let taken = w.scim(Method::POST, "/scim2/Groups", taken).await;
    common::assert_error(taken, StatusCode::CONFLICT, "ALREADY_EXISTS").await;
    let ops = json!({ "schemas": [GROUP], "displayName": "ops@example.com" });
    let ops = w.scim(Method::POST, "/scim2/Groups", ops).await;
    assert_status(ops, StatusCode::CREATED, "ops").await;
    let user = uc_api::ScimUser {
        user_name: "ops@example.com".to_string(),
        ..Default::default()
    };
    let err = w.metastore.scim().create_user(user).await.unwrap_err();
    assert_eq!(err.code(), uc_common::ErrorCode::AlreadyExists);
}
