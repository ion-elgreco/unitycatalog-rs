mod common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, control, control_json, req_json};

const USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
const GROUP_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
const PATCH_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
const ERROR_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:Error";

/// An answer: its status, headers and JSON body (`null` when empty).
struct Answer {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Value,
}

async fn send(app: &Router, request: Request<Body>) -> Answer {
    let resp = app.clone().oneshot(request).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = match bytes.is_empty() {
        true => Value::Null,
        false => serde_json::from_slice(&bytes).expect("body is json"),
    };
    Answer {
        status,
        headers,
        body,
    }
}

async fn get(app: &Router, path: &str) -> Answer {
    send(app, control(Method::GET, path)).await
}

async fn call(app: &Router, method: Method, path: &str, body: Value) -> Answer {
    send(app, control_json(method, path, body)).await
}

async fn patch(app: &Router, path: &str, operations: Value) -> Answer {
    call(
        app,
        Method::PATCH,
        path,
        json!({ "schemas": [PATCH_SCHEMA], "Operations": operations }),
    )
    .await
}

impl Answer {
    /// The body, after checking the status.
    fn ok(self, expected: StatusCode) -> Value {
        assert_eq!(self.status, expected, "body: {}", self.body);
        self.body
    }

    /// The SCIM error, after checking its status and `scimType`.
    fn scim_error(self, expected: StatusCode, scim_type: Option<&str>) -> Value {
        assert_eq!(self.status, expected, "body: {}", self.body);
        assert_eq!(
            self.headers["content-type"], "application/scim+json",
            "{}",
            self.body
        );
        assert_eq!(self.body["schemas"], json!([ERROR_SCHEMA]), "{}", self.body);
        assert_eq!(self.body["status"], expected.as_u16().to_string());
        assert_eq!(
            self.body["scimType"].as_str(),
            scim_type,
            "scimType in {}",
            self.body
        );
        self.body
    }
}

/// Create a user with the email `email` (its user name too) and return its
/// id.
async fn create(app: &Router, email: &str) -> String {
    let body = json!({ "schemas": [USER_SCHEMA], "userName": email,
        "emails": [{ "value": email, "primary": true }] });
    let user = call(app, Method::POST, "/scim2/Users", body)
        .await
        .ok(StatusCode::CREATED);
    user["id"].as_str().unwrap().to_string()
}

/// The `userName` of each user that `query` lists.
async fn user_names(app: &Router, query: &str) -> Vec<String> {
    let list = get(app, &format!("/scim2/Users{query}"))
        .await
        .ok(StatusCode::OK);
    list["Resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["userName"].as_str().unwrap().to_string())
        .collect()
}

/// The ids of the members of `group`.
fn member_ids(group: &Value) -> Vec<&str> {
    group["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["value"].as_str().unwrap())
        .collect()
}

fn encoded(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn filter(f: &str) -> String {
    format!("?filter={}", encoded(f))
}

#[rstest]
#[tokio::test]
async fn create_get_delete_user(#[future(awt)] app: Router) {
    let body = json!({
        "schemas": [USER_SCHEMA],
        "userName": "alice",
        "name": { "givenName": "Alice", "familyName": "Doe" },
        "displayName": "Alice",
        "emails": [{ "value": "alice@example.com", "type": "work", "primary": true }],
        "externalId": "ext-1",
        "active": false,
    });
    let created = call(&app, Method::POST, "/scim2/Users", body).await;
    assert_eq!(created.headers["content-type"], "application/scim+json");
    let location = created.headers["location"].to_str().unwrap().to_string();
    let user = created.ok(StatusCode::CREATED);
    let id = user["id"].as_str().unwrap().to_string();
    // The user as the client sent it.
    assert_eq!(user["schemas"], json!([USER_SCHEMA]));
    assert_eq!(user["userName"], "alice");
    assert_eq!(
        user["name"],
        json!({ "givenName": "Alice", "familyName": "Doe" })
    );
    assert_eq!(user["active"], false);
    assert_eq!(
        user["emails"],
        json!([{ "value": "alice@example.com", "type": "work", "primary": true }])
    );
    assert!(user.get("photos").is_none());
    assert_eq!(user["meta"]["resourceType"], "User");
    assert!(user["meta"]["created"].as_str().unwrap().ends_with('Z'));
    assert_eq!(user["meta"]["location"], location);
    assert!(location.ends_with(&format!("/api/1.0/unity-control/scim2/Users/{id}")));

    assert_eq!(
        get(&app, &format!("/scim2/Users/{id}"))
            .await
            .ok(StatusCode::OK),
        user
    );
    let list = get(&app, "/scim2/Users").await.ok(StatusCode::OK);
    assert_eq!(list["totalResults"], 1);
    assert!(list.get("meta").is_none());

    // A deleted user is gone, so the same user can come back.
    let deleted = send(&app, control(Method::DELETE, &format!("/scim2/Users/{id}"))).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    get(&app, &format!("/scim2/Users/{id}"))
        .await
        .scim_error(StatusCode::NOT_FOUND, None);
    create(&app, "alice@example.com").await;
    send(&app, control(Method::DELETE, "/scim2/Users/nope"))
        .await
        .scim_error(StatusCode::NOT_FOUND, None);
}

/// A user without an email is named by its user name; one without a user
/// name by its email.
#[rstest]
#[tokio::test]
async fn a_user_needs_a_user_name_or_an_email(#[future(awt)] app: Router) {
    call(
        &app,
        Method::POST,
        "/scim2/Users",
        json!({ "schemas": [USER_SCHEMA] }),
    )
    .await
    .scim_error(StatusCode::BAD_REQUEST, Some("invalidValue"));
    let user = call(
        &app,
        Method::POST,
        "/scim2/Users",
        json!({ "userName": "svc-etl" }),
    )
    .await
    .ok(StatusCode::CREATED);
    assert!(user.get("emails").is_none());
    let user = call(
        &app,
        Method::POST,
        "/scim2/Users",
        json!({ "emails": [{ "value": "bob@example.com" }] }),
    )
    .await
    .ok(StatusCode::CREATED);
    assert_eq!(user["userName"], "bob@example.com");
}

#[rstest]
#[tokio::test]
async fn the_same_user_name_principal_or_external_id_is_a_conflict(#[future(awt)] app: Router) {
    let body = json!({ "userName": "bob", "externalId": "ext-bob",
        "emails": [{ "value": "bob@example.com", "primary": true }] });
    call(&app, Method::POST, "/scim2/Users", body)
        .await
        .ok(StatusCode::CREATED);
    for (body, detail) in [
        (json!({ "userName": "BOB" }), "User already exists: BOB"),
        (
            json!({ "userName": "robert", "emails": [{ "value": "bob@example.com" }] }),
            "User already exists: bob@example.com",
        ),
        (
            json!({ "userName": "dave", "externalId": "ext-bob" }),
            "User already exists with external id: ext-bob",
        ),
        (
            json!({ "userName": "dave", "externalId": "EXT-BOB" }),
            "User already exists with external id: EXT-BOB",
        ),
    ] {
        let err = call(&app, Method::POST, "/scim2/Users", body)
            .await
            .scim_error(StatusCode::CONFLICT, Some("uniqueness"));
        assert_eq!(err["detail"], detail);
        assert_eq!(err["error_code"], "ALREADY_EXISTS");
    }
    call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "Writers" }),
    )
    .await
    .ok(StatusCode::CREATED);
    let err = call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "writers" }),
    )
    .await
    .scim_error(StatusCode::CONFLICT, Some("uniqueness"));
    assert_eq!(err["detail"], "Group already exists: writers");
}

/// The server ignores the attributes that a client may not write, however
/// they look.
#[rstest]
#[tokio::test]
async fn a_new_resource_may_have_any_read_only_attributes(#[future(awt)] app: Router) {
    let body = json!({ "userName": "ro@x", "id": 7, "meta": { "created": "x" },
        "roles": ["admin"] });
    let user = call(&app, Method::POST, "/scim2/Users", body)
        .await
        .ok(StatusCode::CREATED);
    assert!(user["id"].is_string());
    assert!(user.get("roles").is_none());
    let body = json!({ "displayName": "ro", "meta": { "created": "x" } });
    call(&app, Method::POST, "/scim2/Groups", body)
        .await
        .ok(StatusCode::CREATED);
}

/// With authorization off, the checks let every caller do what the
/// metastore admin does: `/Me` names the role `admin`.
#[tokio::test]
async fn me_names_the_admin_role_with_authorization_off() {
    let (app, metastore) = common::app_with_metastore().await;
    let token = common::active_user(&metastore, "alice@example.com").await;
    let mut me = control(Method::GET, "/scim2/Me");
    me.headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    let body = send(&app, me).await.ok(StatusCode::OK);
    assert_eq!(body["userName"], "alice@example.com");
    assert_eq!(body["roles"], json!([{ "value": "admin" }]));
}

/// Upstream `getCurrentUser` with no caller: 400 "No user found.".
#[rstest]
#[tokio::test]
async fn me_without_a_caller_is_a_scim_error(#[future(awt)] app: Router) {
    let err = get(&app, "/scim2/Me")
        .await
        .scim_error(StatusCode::BAD_REQUEST, None);
    assert_eq!(err["detail"], "No user found.");
}

/// The requests of Microsoft's guide to Entra ID provisioning
/// ("Develop a SCIM endpoint"), in the order Entra sends them.
#[rstest]
#[tokio::test]
async fn entra_id_provisions_users_and_groups(#[future(awt)] app: Router) {
    // Test Connection: a random user and group, which are not found.
    let none = get(
        &app,
        &format!("/scim2/Users{}", filter(r#"userName eq "c3f1""#)),
    )
    .await;
    let none = none.ok(StatusCode::OK);
    assert_eq!(
        (none["totalResults"].clone(), none["Resources"].clone()),
        (json!(0), json!([]))
    );
    let path = format!(
        "/scim2/Groups?excludedAttributes=members&filter={}",
        encoded(r#"displayName eq "c3f1""#)
    );
    assert_eq!(get(&app, &path).await.ok(StatusCode::OK)["totalResults"], 0);

    // A user whose user principal name is not its email.
    let upn = "alice@corp.onmicrosoft.com";
    let entra_user = json!({
        "schemas": [USER_SCHEMA, "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"],
        "externalId": "0a21f0f2-8d2a-4f8e-bf98-7363c4aed4ef",
        "userName": upn,
        "active": true,
        "emails": [{ "primary": true, "type": "work", "value": "alice@corp.example" }],
        "meta": { "resourceType": "User" },
        "name": { "formatted": "Alice Doe", "familyName": "Doe", "givenName": "Alice" },
        "roles": [],
    });
    let user = call(&app, Method::POST, "/scim2/Users", entra_user.clone())
        .await
        .ok(StatusCode::CREATED);
    let id = user["id"].as_str().unwrap().to_string();
    assert_eq!(user["userName"], upn);
    call(&app, Method::POST, "/scim2/Users", entra_user)
        .await
        .scim_error(StatusCode::CONFLICT, Some("uniqueness"));
    for f in [
        format!(r#"userName eq "{upn}""#),
        r#"externalId eq "0a21f0f2-8d2a-4f8e-bf98-7363c4aed4ef""#.to_string(),
        r#"emails[type eq "work"].value eq "alice@corp.example""#.to_string(),
    ] {
        assert_eq!(user_names(&app, &filter(&f)).await, [upn], "{f}");
    }

    let user_path = format!("/scim2/Users/{id}");
    let user = patch(
        &app,
        &user_path,
        json!([
            { "op": "Replace", "path": "emails[type eq \"work\"].value", "value": "alice.doe@corp.example" },
            { "op": "Replace", "path": "name.familyName", "value": "Smith" },
        ]),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(user["emails"][0]["value"], "alice.doe@corp.example");
    assert_eq!(user["name"]["familyName"], "Smith");
    let new_upn = "alice.smith@corp.onmicrosoft.com";
    patch(
        &app,
        &user_path,
        json!([{ "op": "Replace", "path": "userName", "value": new_upn }]),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(
        user_names(&app, &filter(&format!(r#"userName eq "{new_upn}""#))).await,
        [new_upn]
    );

    // A disabled user is still found.
    patch(
        &app,
        &user_path,
        json!([{ "op": "Replace", "path": "active", "value": false }]),
    )
    .await
    .ok(StatusCode::OK);
    let list = get(
        &app,
        &format!(
            "/scim2/Users{}",
            filter(&format!(r#"userName eq "{new_upn}""#))
        ),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(list["Resources"][0]["active"], false);
    // Entra's older form: a string.
    let user = patch(
        &app,
        &user_path,
        json!([{ "op": "Replace", "path": "active", "value": "True" }]),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(user["active"], true);

    // A group, created empty, then members added and removed.
    let group = call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({
            "schemas": [GROUP_SCHEMA, "http://schemas.microsoft.com/2006/11/ResourceManagement/ADSCIM/2.0/Group"],
            "externalId": "8aa1a0c0-c4c3-4bc0-b4a5-2ef676900159",
            "displayName": "Writers",
            "meta": { "resourceType": "Group" },
        }),
    )
    .await
    .ok(StatusCode::CREATED);
    let gid = group["id"].as_str().unwrap().to_string();
    assert_eq!(group["externalId"], "8aa1a0c0-c4c3-4bc0-b4a5-2ef676900159");
    let group_path = format!("/scim2/Groups/{gid}");
    let member = json!([{ "$ref": null, "value": id }]);
    patch(
        &app,
        &group_path,
        json!([{ "op": "Add", "path": "members", "value": member }]),
    )
    .await
    .ok(StatusCode::OK);
    let group = get(&app, &format!("{group_path}?excludedAttributes=members")).await;
    assert!(group.ok(StatusCode::OK).get("members").is_none());
    let membership = format!(
        "/scim2/Groups?excludedAttributes=members&filter={}",
        encoded(&format!(r#"id eq "{gid}" and members eq "{id}""#))
    );
    assert_eq!(
        get(&app, &membership).await.ok(StatusCode::OK)["totalResults"],
        1
    );
    patch(
        &app,
        &group_path,
        json!([{ "op": "Remove", "path": "members", "value": member }]),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(
        get(&app, &membership).await.ok(StatusCode::OK)["totalResults"],
        0
    );
    patch(
        &app,
        &group_path,
        json!([{ "op": "Replace", "path": "displayName", "value": "Writers 2" }]),
    )
    .await
    .ok(StatusCode::OK);
    let renamed = format!("/scim2/Groups{}", filter(r#"displayName eq "Writers 2""#));
    assert_eq!(
        get(&app, &renamed).await.ok(StatusCode::OK)["totalResults"],
        1
    );

    for path in [&group_path, &user_path] {
        assert_eq!(
            send(&app, control(Method::DELETE, path)).await.status,
            StatusCode::NO_CONTENT
        );
    }
}

/// authentik updates users with PUT, renames groups with a `replace`
/// without a path, and finds a user by the `userName` it sent after a 409.
#[rstest]
#[tokio::test]
async fn authentik_provisions_users_and_groups(#[future(awt)] app: Router) {
    let sent = json!({
        "userName": "alice-1",
        "name": { "formatted": "Alice Liddell", "familyName": "Liddell", "givenName": "Alice" },
        "displayName": "Alice Liddell",
        "active": true,
        "emails": [{ "value": "alice@example.com", "type": "other", "primary": true }],
        "schemas": [USER_SCHEMA],
        "externalId": "fd878ab8",
    });
    let user = call(&app, Method::POST, "/scim2/Users", sent.clone())
        .await
        .ok(StatusCode::CREATED);
    let id = user["id"].as_str().unwrap().to_string();
    // The answer holds what authentik sent, so it sends no more updates.
    for key in [
        "userName",
        "name",
        "displayName",
        "active",
        "emails",
        "externalId",
    ] {
        assert_eq!(user[key], sent[key], "{key}");
    }
    call(&app, Method::POST, "/scim2/Users", sent.clone())
        .await
        .scim_error(StatusCode::CONFLICT, Some("uniqueness"));
    assert_eq!(
        user_names(&app, &filter(r#"userName eq "alice-1""#)).await,
        ["alice-1"]
    );

    let mut changed = sent.clone();
    changed["id"] = json!(id);
    changed["name"] = json!({ "formatted": "Alice H", "familyName": "H", "givenName": "Alice" });
    changed["emails"] =
        json!([{ "value": "alice.h@example.com", "type": "other", "primary": true }]);
    let user = call(
        &app,
        Method::PUT,
        &format!("/scim2/Users/{id}"),
        changed.clone(),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(
        (user["name"].clone(), user["emails"].clone()),
        (changed["name"].clone(), changed["emails"].clone())
    );

    let group = call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "readers", "schemas": [GROUP_SCHEMA], "externalId": "2a22" }),
    )
    .await
    .ok(StatusCode::CREATED);
    let group_path = format!("/scim2/Groups/{}", group["id"].as_str().unwrap());
    patch(
        &app,
        &group_path,
        json!([{ "op": "add", "path": "members", "value": [{ "value": id }] }]),
    )
    .await
    .ok(StatusCode::OK);
    let group = patch(
        &app,
        &group_path,
        json!([{ "op": "replace", "value": { "displayName": "readers-2",
            "schemas": [GROUP_SCHEMA], "externalId": "2a22" } }]),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(group["displayName"], "readers-2");
    assert_eq!(member_ids(&group), [id.as_str()]);
    let url = group["members"][0]["$ref"].as_str().unwrap();
    assert!(url.ends_with(&format!("/scim2/Users/{id}")), "{url}");
    let group = patch(
        &app,
        &group_path,
        json!([{ "op": "remove", "path": format!("members[value eq \"{id}\"]") }]),
    )
    .await
    .ok(StatusCode::OK);
    assert!(group.get("members").is_none());
}

/// Make the catalog `c1` with the schema `s1` and a staging table as the
/// user of `token`: the user owns them. The id of the staging table.
async fn owned_objects(app: &Router, token: &str) -> String {
    let as_user = |path: &str, body: Value| common::signed(Method::POST, path, Some(body), token);
    let body = json!({ "name": "c1", "storage_root": common::temp_storage_root() });
    send(app, as_user("/catalogs", body))
        .await
        .ok(StatusCode::OK);
    let body = json!({ "name": "s1", "catalog_name": "c1" });
    send(app, as_user("/schemas", body))
        .await
        .ok(StatusCode::OK);
    let body = json!({ "name": "t1", "catalog_name": "c1", "schema_name": "s1" });
    let staging = send(app, as_user("/staging-tables", body))
        .await
        .ok(StatusCode::OK);
    staging["id"].as_str().unwrap().to_string()
}

/// The owner of the staging table `id`: its creator.
async fn staging_owner(metastore: &uc_metastore::DynMetastore, id: &str) -> Option<String> {
    let lineage = metastore
        .permissions()
        .lineage_by_id(uc_common::SecurableType::Table, id)
        .await
        .unwrap();
    lineage.object.unwrap().owner
}

/// Grants and owners name a user by its principal: when its email changes,
/// they move to the new one.
#[tokio::test]
async fn a_new_email_moves_the_grants_and_owned_objects() {
    let (app, metastore) = common::app_with_metastore().await;
    let token = common::active_user(&metastore, "old@example.com").await;
    let staging = owned_objects(&app, &token).await;
    let catalog = send(&app, common::req(Method::GET, "/catalogs/c1")).await;
    assert_eq!(catalog.ok(StatusCode::OK)["owner"], "old@example.com");
    let grant = json!({ "changes": [{ "principal": "old@example.com", "add": ["USE CATALOG"] }] });
    send(
        &app,
        req_json(Method::PATCH, "/permissions/catalog/c1", grant),
    )
    .await
    .ok(StatusCode::OK);

    let user = metastore
        .scim()
        .find_user_by_principal("old@example.com")
        .await
        .unwrap()
        .unwrap();
    patch(
        &app,
        &format!("/scim2/Users/{}", user.id.unwrap()),
        json!([{ "op": "replace", "path": "emails[primary eq true].value", "value": "new@example.com" }]),
    )
    .await
    .ok(StatusCode::OK);

    let catalog = send(&app, common::req(Method::GET, "/catalogs/c1")).await;
    assert_eq!(catalog.ok(StatusCode::OK)["owner"], "new@example.com");
    let grants = send(&app, common::req(Method::GET, "/permissions/catalog/c1")).await;
    assert_eq!(
        grants.ok(StatusCode::OK)["privilege_assignments"],
        json!([{ "principal": "new@example.com", "privileges": ["USE CATALOG"] }])
    );
    assert_eq!(
        staging_owner(&metastore, &staging).await.as_deref(),
        Some("new@example.com")
    );
}

/// A deleted user's objects lose their owner: a later user with the same
/// email gets none of them.
#[tokio::test]
async fn a_deleted_user_leaves_no_objects_to_a_new_user_of_its_email() {
    let (app, metastore) = common::app_with_metastore().await;
    let token = common::active_user(&metastore, "gone@example.com").await;
    let staging = owned_objects(&app, &token).await;
    let user = metastore
        .scim()
        .find_user_by_principal("gone@example.com")
        .await
        .unwrap()
        .unwrap();
    let path = format!("/scim2/Users/{}", user.id.unwrap());
    send(&app, control(Method::DELETE, &path))
        .await
        .ok(StatusCode::NO_CONTENT);
    create(&app, "gone@example.com").await;

    for path in ["/catalogs/c1", "/schemas/c1.s1"] {
        let object = send(&app, common::req(Method::GET, path))
            .await
            .ok(StatusCode::OK);
        assert!(object["owner"].is_null(), "{path}: {object}");
    }
    assert_eq!(staging_owner(&metastore, &staging).await, None);
}

#[rstest]
#[tokio::test]
async fn put_replaces_what_the_body_has(#[future(awt)] app: Router) {
    let id = create(&app, "put@example.com").await;
    let other = create(&app, "other@example.com").await;
    let path = format!("/scim2/Users/{id}");
    call(
        &app,
        Method::PUT,
        &path,
        json!({ "id": other, "userName": "put@example.com" }),
    )
    .await
    .scim_error(StatusCode::BAD_REQUEST, Some("mutability"));
    // The body has the required attributes; a patch keeps them.
    call(&app, Method::PUT, &path, json!({ "displayName": "X" }))
        .await
        .scim_error(StatusCode::BAD_REQUEST, Some("invalidValue"));
    patch(&app, &path, json!([{ "op": "remove", "path": "userName" }]))
        .await
        .scim_error(StatusCode::BAD_REQUEST, Some("mutability"));
    call(
        &app,
        Method::PUT,
        "/scim2/Users/nope",
        json!({ "userName": "nope" }),
    )
    .await
    .scim_error(StatusCode::NOT_FOUND, None);

    let body = json!({ "displayName": "Put", "active": false, "externalId": "ext-put",
        "userName": "put", "emails": [{ "value": "changed@example.com", "primary": true }] });
    let user = call(&app, Method::PUT, &path, body)
        .await
        .ok(StatusCode::OK);
    assert_eq!(user["displayName"], "Put");
    assert_eq!(user["active"], false);
    assert_eq!(user["userName"], "put");
    assert_eq!(user["emails"][0]["value"], "changed@example.com");
    // Absent attributes stay; `null` removes one.
    let user = call(
        &app,
        Method::PUT,
        &path,
        json!({ "id": id, "userName": "put", "displayName": null }),
    )
    .await
    .ok(StatusCode::OK);
    assert!(user.get("displayName").is_none());
    assert_eq!(
        (user["active"].clone(), user["externalId"].clone()),
        (json!(false), json!("ext-put"))
    );
    call(
        &app,
        Method::PUT,
        &format!("/scim2/Users/{other}"),
        json!({ "userName": "other@example.com", "externalId": "ext-put" }),
    )
    .await
    .scim_error(StatusCode::CONFLICT, Some("uniqueness"));
}

#[rstest]
#[tokio::test]
async fn requests_take_attribute_names_in_any_case(#[future(awt)] app: Router) {
    let user = call(
        &app,
        Method::POST,
        "/scim2/Users",
        json!({ "UserName": "ryan", "Active": "True", "DisplayName": "Ryan",
            "Emails": [{ "Primary": true, "Type": "work", "Value": "ryan@example.com" }] }),
    )
    .await
    .ok(StatusCode::CREATED);
    assert_eq!(user["userName"], "ryan");
    assert_eq!(user["active"], true);
    assert_eq!(
        user["emails"],
        json!([{ "value": "ryan@example.com", "primary": true, "type": "work" }])
    );
    let id = user["id"].as_str().unwrap();
    let user = call(
        &app,
        Method::PATCH,
        &format!("/scim2/Users/{id}"),
        json!({ "operations": [{ "OP": "Replace", "Path": "DISPLAYNAME", "Value": "R" }] }),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(user["displayName"], "R");
}

#[rstest]
#[tokio::test]
async fn answers_pick_attributes(#[future(awt)] app: Router) {
    let id = create(&app, "pick@example.com").await;
    let user = get(
        &app,
        &format!("/scim2/Users/{id}?attributes=userName,emails.value"),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(
        user,
        json!({ "schemas": [USER_SCHEMA], "id": id, "userName": "pick@example.com",
            "emails": [{ "value": "pick@example.com" }] })
    );
    let list = get(&app, "/scim2/Users?excludedAttributes=emails,meta")
        .await
        .ok(StatusCode::OK);
    let user = &list["Resources"][0];
    assert!(user.get("emails").is_none() && user.get("meta").is_none());
    assert_eq!(user["userName"], "pick@example.com");

    // A created resource has its Location without its `meta`.
    let created = call(
        &app,
        Method::POST,
        "/scim2/Users?excludedAttributes=meta",
        json!({ "userName": "located" }),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert!(created.body.get("meta").is_none(), "{}", created.body);
    let id = created.body["id"].as_str().unwrap();
    assert_eq!(
        created.headers["location"],
        format!(
            "http://localhost{}/scim2/Users/{id}",
            uc_common::CONTROL_BASE
        )
    );
}

#[rstest]
#[tokio::test]
async fn search_takes_a_search_request(#[future(awt)] app: Router) {
    for n in ["alpha@x.com", "beta@x.com"] {
        create(&app, n).await;
    }
    let list = call(
        &app,
        Method::POST,
        "/scim2/Users/.search",
        json!({ "schemas": ["urn:ietf:params:scim:api:messages:2.0:SearchRequest"],
            "filter": "userName sw \"b\"", "attributes": ["userName"], "startIndex": 1, "count": 10 }),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(list["totalResults"], 1);
    assert_eq!(list["Resources"][0]["userName"], "beta@x.com");
    assert!(list["Resources"][0].get("emails").is_none());
}

#[rstest]
#[tokio::test]
async fn list_users_takes_scim_filters(#[future(awt)] app: Router) {
    for n in ["alpha@x", "beta@x", "gamma@y"] {
        create(&app, n).await;
    }
    for (f, expected) in [
        (
            r#"userName sw "A" or userName ew "@y""#,
            vec!["alpha@x", "gamma@y"],
        ),
        (
            r#"emails[value co "et"] and active eq true"#,
            vec!["beta@x"],
        ),
        (r#"not (userName ew "@x")"#, vec!["gamma@y"]),
        (
            r#"meta.created gt "2000-01-01T00:00:00Z" and externalId pr"#,
            vec![],
        ),
        // Found by index, then checked against the whole filter.
        (r#"userName eq "ALPHA@X""#, vec!["alpha@x"]),
        (r#"userName eq "beta@x" and active eq false"#, vec![]),
        (
            r#"userName eq "beta@x" or userName eq "gamma@y""#,
            vec!["beta@x", "gamma@y"],
        ),
    ] {
        assert_eq!(user_names(&app, &filter(f)).await, expected, "{f}");
    }
    get(
        &app,
        &format!("/scim2/Users{}", filter(r#"userName xx "a""#)),
    )
    .await
    .scim_error(StatusCode::BAD_REQUEST, Some("invalidFilter"));
}

/// `totalResults` counts the matches, `itemsPerPage` the page.
#[rstest]
#[tokio::test]
async fn list_users_pages_by_start_index_and_count(#[future(awt)] app: Router) {
    for i in 0..55 {
        create(&app, &format!("u{i:02}@x")).await;
    }
    let first = get(&app, "/scim2/Users").await.ok(StatusCode::OK);
    assert_eq!(first["Resources"].as_array().unwrap().len(), 50);
    assert_eq!(
        (
            first["totalResults"].clone(),
            first["itemsPerPage"].clone(),
            first["startIndex"].clone()
        ),
        (json!(55), json!(50), json!(1))
    );
    assert_eq!(first["Resources"][0]["userName"], "u00@x");
    let second = get(&app, "/scim2/Users?startIndex=51&count=10")
        .await
        .ok(StatusCode::OK);
    assert_eq!(
        (
            second["totalResults"].clone(),
            second["itemsPerPage"].clone()
        ),
        (json!(55), json!(5))
    );
    assert_eq!(second["Resources"][0]["userName"], "u50@x");
    let filtered = format!("{}&startIndex=3&count=2", filter(r#"userName sw "u1""#));
    assert_eq!(user_names(&app, &filtered).await, ["u12@x", "u13@x"]);
    let none = get(&app, "/scim2/Users?count=0").await.ok(StatusCode::OK);
    assert_eq!(
        (none["totalResults"].clone(), none["Resources"].clone()),
        (json!(55), json!([]))
    );
}

#[rstest]
#[tokio::test]
async fn a_group_member_must_be_a_user(#[future(awt)] app: Router) {
    let body = json!({ "displayName": "g", "members": [{ "value": "nobody" }] });
    call(&app, Method::POST, "/scim2/Groups", body)
        .await
        .scim_error(StatusCode::BAD_REQUEST, Some("invalidValue"));
    let group = call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "g" }),
    )
    .await
    .ok(StatusCode::CREATED);
    call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "g" }),
    )
    .await
    .scim_error(StatusCode::CONFLICT, Some("uniqueness"));
    let path = format!("/scim2/Groups/{}", group["id"].as_str().unwrap());
    patch(
        &app,
        &path,
        json!([{ "op": "add", "path": "members", "value": [{ "value": "nobody" }] }]),
    )
    .await
    .scim_error(StatusCode::BAD_REQUEST, Some("invalidValue"));
}

/// A deleted user leaves its groups.
#[rstest]
#[tokio::test]
async fn a_deleted_user_leaves_its_groups(#[future(awt)] app: Router) {
    let u1 = create(&app, "u1@x").await;
    let u2 = create(&app, "u2@x").await;
    let group = call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "eng", "members": [{ "value": u1 }, { "value": u2 }] }),
    )
    .await
    .ok(StatusCode::CREATED);
    send(&app, control(Method::DELETE, &format!("/scim2/Users/{u1}"))).await;
    let group = get(
        &app,
        &format!("/scim2/Groups/{}", group["id"].as_str().unwrap()),
    )
    .await
    .ok(StatusCode::OK);
    assert_eq!(member_ids(&group), [u2.as_str()]);
}

#[rstest]
#[tokio::test]
async fn errors_are_scim_errors(#[future(awt)] app: Router) {
    get(&app, "/scim2/Nope")
        .await
        .scim_error(StatusCode::NOT_FOUND, None);
    get(&app, "/scim2/Users/a/b")
        .await
        .scim_error(StatusCode::NOT_FOUND, None);
    send(&app, control(Method::DELETE, "/scim2/Users"))
        .await
        .scim_error(StatusCode::METHOD_NOT_ALLOWED, None);
    let malformed = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/scim2/Users", common::CONTROL))
        .header("content-type", "application/scim+json")
        .body(Body::from("{"))
        .unwrap();
    send(&app, malformed)
        .await
        .scim_error(StatusCode::BAD_REQUEST, Some("invalidSyntax"));
    let id = create(&app, "p@x").await;
    patch(
        &app,
        &format!("/scim2/Users/{id}"),
        json!([{ "op": "remove" }]),
    )
    .await
    .scim_error(StatusCode::BAD_REQUEST, Some("noTarget"));
    // The UC clients read the UC code and message.
    let err = get(&app, "/scim2/Users/gone")
        .await
        .scim_error(StatusCode::NOT_FOUND, None);
    assert_eq!(err["error_code"], "NOT_FOUND");
    assert_eq!(err["message"], "User not found: gone");
}

#[rstest]
#[tokio::test]
async fn discovery_documents(#[future(awt)] app: Router) {
    for path in ["/scim2/Schemas", "/scim2/ResourceTypes"] {
        let list = get(&app, path).await.ok(StatusCode::OK);
        assert_eq!(
            list["schemas"],
            json!(["urn:ietf:params:scim:api:messages:2.0:ListResponse"])
        );
        assert_eq!(list["totalResults"], 2);
    }
    let schema = get(&app, &format!("/scim2/Schemas/{USER_SCHEMA}"))
        .await
        .ok(StatusCode::OK);
    let names: Vec<&str> = schema["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "userName",
            "name",
            "displayName",
            "active",
            "emails",
            "photos",
            "roles"
        ]
    );
    assert_eq!(
        get(&app, "/scim2/ResourceTypes/Group")
            .await
            .ok(StatusCode::OK)["endpoint"],
        "/Groups"
    );
    get(&app, "/scim2/Schemas/urn:nope")
        .await
        .scim_error(StatusCode::NOT_FOUND, None);
    get(&app, "/scim2/ResourceTypes/Nope")
        .await
        .scim_error(StatusCode::NOT_FOUND, None);
    let config = get(&app, "/scim2/ServiceProviderConfig")
        .await
        .ok(StatusCode::OK);
    assert_eq!(
        (
            config["patch"]["supported"].clone(),
            config["filter"]["supported"].clone()
        ),
        (json!(true), json!(true))
    );
}

#[rstest]
#[tokio::test]
async fn bulk_runs_each_operation(#[future(awt)] app: Router) {
    let new_user = |email: &str| json!({ "userName": email });
    let body = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
        "failOnErrors": 1,
        "Operations": [
            { "method": "POST", "path": "/Users", "bulkId": "u1", "data": new_user("bulk1@x") },
            { "method": "DELETE", "path": "/Users/does-not-exist" },
            { "method": "POST", "path": "/Users", "data": new_user("skipped@x") },
        ]
    });
    let resp = call(&app, Method::POST, "/scim2/Bulk", body)
        .await
        .ok(StatusCode::OK);
    let ops = resp["Operations"].as_array().unwrap();
    assert_eq!(ops.len(), 2, "the third runs after one failure");
    assert_eq!(ops[0]["status"], "201");
    assert_eq!(ops[0]["bulkId"], "u1");
    assert!(
        ops[0]["location"]
            .as_str()
            .unwrap()
            .contains("/scim2/Users/")
    );
    assert_eq!(ops[1]["status"], "404");
    assert_eq!(ops[1]["response"]["schemas"], json!([ERROR_SCHEMA]));
}

/// `bulkId:<id>` names the resource of an earlier operation.
#[rstest]
#[tokio::test]
async fn bulk_resolves_bulk_ids(#[future(awt)] app: Router) {
    let body = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
        "Operations": [
            { "method": "POST", "path": "/Users", "bulkId": "u1", "data": { "userName": "b1@x" } },
            { "method": "POST", "path": "/Groups", "bulkId": "g1",
              "data": { "displayName": "bulk", "members": [{ "value": "bulkId:u1" }] } },
            { "method": "PATCH", "path": "/Users/bulkId:u1", "data": { "Operations": [
                { "op": "replace", "path": "displayName", "value": "B1" } ] } },
            { "method": "POST", "path": "/Groups",
              "data": { "displayName": "other", "members": [{ "value": "bulkId:nope" }] } },
        ]
    });
    let resp = call(&app, Method::POST, "/scim2/Bulk", body)
        .await
        .ok(StatusCode::OK);
    let ops = resp["Operations"].as_array().unwrap();
    let statuses: Vec<&str> = ops.iter().map(|o| o["status"].as_str().unwrap()).collect();
    assert_eq!(statuses, ["201", "201", "200", "409"], "{resp}");
    let user = ops[0]["location"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap();
    let group = ops[1]["location"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap();
    let group = get(&app, &format!("/scim2/Groups/{group}"))
        .await
        .ok(StatusCode::OK);
    assert_eq!(member_ids(&group), [user]);
    let user = get(&app, &format!("/scim2/Users/{user}"))
        .await
        .ok(StatusCode::OK);
    assert_eq!(user["displayName"], "B1");
}

/// Changes of one group at the same time each stay: a change that another
/// request beat runs again on the newer group.
#[tokio::test]
async fn changes_at_the_same_time_each_stay() {
    let (app, metastore) = common::app_with_metastore().await;
    let mut users = Vec::new();
    for email in ["c1@x.com", "c2@x.com", "c3@x.com"] {
        users.push(create(&app, email).await);
    }
    let group = call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "g" }),
    )
    .await
    .ok(StatusCode::CREATED);
    let id = group["id"].as_str().unwrap();
    let path = format!("/scim2/Groups/{id}");
    let add = |user: &str| {
        patch(
            &app,
            &path,
            json!([{ "op": "add", "path": "members", "value": [{ "value": user }] }]),
        )
    };
    let (a, b, c) = tokio::join!(add(&users[0]), add(&users[1]), add(&users[2]));
    for answer in [a, b, c] {
        answer.ok(StatusCode::OK);
    }
    let group = get(&app, &path).await.ok(StatusCode::OK);
    let mut members = member_ids(&group);
    members.sort_unstable();
    let mut expected: Vec<&str> = users.iter().map(String::as_str).collect();
    expected.sort_unstable();
    assert_eq!(members, expected);

    // A change runs on the stored group, and its error changes nothing.
    let scim = metastore.scim();
    scim.update_group(id, &mut |current| {
        Ok(uc_api::ScimGroup {
            display_name: "h".into(),
            ..current.clone()
        })
    })
    .await
    .unwrap();
    let failed = scim
        .update_group(id, &mut |_| Err(uc_common::Error::invalid_argument("no")))
        .await;
    assert_eq!(failed.unwrap_err().message(), "no");
    let group = scim.get_group(id).await.unwrap();
    assert_eq!(group.display_name, "h");
    assert_eq!(group.members.unwrap().len(), 3);
}

/// User names and emails are kept without spaces around them, and so is
/// the principal.
#[tokio::test]
async fn user_names_and_emails_are_trimmed() {
    let (app, metastore) = common::app_with_metastore().await;
    let scim = metastore.scim();
    let user = call(
        &app,
        Method::POST,
        "/scim2/Users",
        json!({ "userName": " svc " }),
    )
    .await
    .ok(StatusCode::CREATED);
    assert_eq!(user["userName"], "svc");
    assert!(scim.find_user_by_principal("svc").await.unwrap().is_some());
    let body = json!({ "userName": "m", "emails": [{ "value": " m@x.com ", "primary": true }] });
    let user = call(&app, Method::POST, "/scim2/Users", body)
        .await
        .ok(StatusCode::CREATED);
    assert_eq!(user["emails"][0]["value"], "m@x.com");
    assert!(
        scim.find_user_by_principal("m@x.com")
            .await
            .unwrap()
            .is_some()
    );
}

/// `admin` is the server's own user, which `uc-server token` signs in: no
/// other user takes the name in any case, and the user keeps it.
#[tokio::test]
async fn the_name_admin_belongs_to_the_servers_own_user() {
    let (app, metastore) = common::app_with_metastore().await;
    for body in [
        json!({ "userName": "Admin" }),
        json!({ "userName": "ops", "emails": [{ "value": "ADMIN" }] }),
    ] {
        call(&app, Method::POST, "/scim2/Users", body)
            .await
            .scim_error(StatusCode::CONFLICT, Some("uniqueness"));
    }
    let other = create(&app, "ops@example.com").await;
    let rename = |name: &str| json!([{ "op": "replace", "path": "userName", "value": name }]);
    patch(&app, &format!("/scim2/Users/{other}"), rename("admin"))
        .await
        .scim_error(StatusCode::CONFLICT, Some("uniqueness"));

    uc_server::authn::security::initialize_admin(&metastore)
        .await
        .unwrap();
    let admin = metastore
        .scim()
        .find_user_by_principal("admin")
        .await
        .unwrap()
        .unwrap();
    let path = format!("/scim2/Users/{}", admin.id.unwrap());
    patch(&app, &path, rename("root"))
        .await
        .scim_error(StatusCode::BAD_REQUEST, Some("mutability"));
    let disable = json!([{ "op": "replace", "path": "active", "value": false }]);
    patch(&app, &path, disable)
        .await
        .scim_error(StatusCode::BAD_REQUEST, Some("mutability"));
    send(&app, control(Method::DELETE, &path))
        .await
        .scim_error(StatusCode::FORBIDDEN, None);
    let display = json!([{ "op": "replace", "path": "displayName", "value": "Root" }]);
    patch(&app, &path, display).await.ok(StatusCode::OK);
}

/// A grant names a user by principal, else a group by name, else a user by
/// user name in any case.
#[rstest]
#[tokio::test]
async fn a_grant_to_the_name_of_a_group_goes_to_the_group(#[future(awt)] app: Router) {
    call(
        &app,
        Method::POST,
        "/scim2/Groups",
        json!({ "displayName": "ops" }),
    )
    .await
    .ok(StatusCode::CREATED);
    let bot =
        json!({ "userName": "Ops", "emails": [{ "value": "bot@example.com", "primary": true }] });
    call(&app, Method::POST, "/scim2/Users", bot)
        .await
        .ok(StatusCode::CREATED);
    let body = json!({ "name": "c1", "storage_root": common::temp_storage_root() });
    send(&app, req_json(Method::POST, "/catalogs", body))
        .await
        .ok(StatusCode::OK);
    for (name, principal) in [("ops", "ops"), ("OPS", "bot@example.com")] {
        let grant = json!({ "changes": [{ "principal": name, "add": ["USE CATALOG"] }] });
        let grants = send(
            &app,
            req_json(Method::PATCH, "/permissions/catalog/c1", grant),
        )
        .await
        .ok(StatusCode::OK);
        assert_eq!(
            grants["privilege_assignments"],
            json!([{ "principal": principal, "privileges": ["USE CATALOG"] }]),
            "{name}"
        );
    }
}
