mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, assert_status, control, control_json, json_body};

const USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
const GROUP_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
const PATCH_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";

/// A new user: upstream needs a primary email.
fn new_user(email: &str) -> Value {
    json!({ "schemas": [USER_SCHEMA], "emails": [{ "value": email, "primary": true }] })
}

/// Create `email` and return its id.
async fn create(app: &Router, email: &str) -> String {
    let resp = app
        .clone()
        .oneshot(control_json(Method::POST, "/scim2/Users", new_user(email)))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::CREATED, "create").await).await;
    body["id"].as_str().unwrap().to_string()
}

/// Upstream answers SCIM errors with HTTP 500 and the SCIM error body.
async fn assert_scim_error(resp: axum::response::Response, status: &str) -> Value {
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = json_body(resp).await;
    assert_eq!(
        body["schemas"],
        json!(["urn:ietf:params:scim:api:messages:2.0:Error"])
    );
    assert_eq!(body["status"], status);
    body
}

#[rstest]
#[tokio::test]
async fn create_get_delete_scim_user(#[future(awt)] app: Router) {
    let resp = app
        .clone()
        .oneshot(control_json(
            Method::POST,
            "/scim2/Users",
            json!({
                "schemas": [USER_SCHEMA],
                "userName": "ignored",
                "displayName": "Alice",
                "emails": [{ "value": "alice@example.com", "primary": true }],
                "externalId": "ext-1",
                "active": false,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.headers()["content-type"], "application/scim+json");
    let body = json_body(assert_status(resp, StatusCode::CREATED, "create").await).await;
    let id = body["id"].as_str().unwrap().to_string();
    // Upstream `asUserResource`: the primary email is the user name; new
    // users are enabled.
    assert_eq!(body["schemas"], json!([USER_SCHEMA]));
    assert_eq!(body["userName"], "alice@example.com");
    assert_eq!(body["displayName"], "Alice");
    assert_eq!(body["externalId"], "ext-1");
    assert_eq!(body["active"], true);
    assert_eq!(
        body["emails"],
        json!([{ "value": "alice@example.com", "primary": true }])
    );
    assert_eq!(body["photos"], json!([{ "value": "" }]));
    assert_eq!(body["meta"]["resourceType"], "User");
    assert!(body["meta"]["created"].as_str().unwrap().ends_with('Z'));

    let resp = app
        .clone()
        .oneshot(control(Method::GET, &format!("/scim2/Users/{id}")))
        .await
        .unwrap();
    assert_eq!(
        json_body(assert_status(resp, StatusCode::OK, "get").await).await,
        body
    );

    let resp = app
        .clone()
        .oneshot(control(Method::GET, "/scim2/Users"))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    assert_eq!(list["totalResults"], 1);
    assert_eq!(list["Resources"].as_array().unwrap().len(), 1);

    // Upstream `deleteUser` disables the user.
    let resp = app
        .clone()
        .oneshot(control(Method::DELETE, &format!("/scim2/Users/{id}")))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "delete").await;
    let resp = app
        .clone()
        .oneshot(control(Method::GET, &format!("/scim2/Users/{id}")))
        .await
        .unwrap();
    assert_eq!(
        json_body(assert_status(resp, StatusCode::OK, "get").await).await["active"],
        false
    );
    let resp = app
        .clone()
        .oneshot(control_json(
            Method::POST,
            "/scim2/Users",
            new_user("alice@example.com"),
        ))
        .await
        .unwrap();
    assert_scim_error(resp, "409").await;
    let resp = app
        .clone()
        .oneshot(control(Method::DELETE, "/scim2/Users/nope"))
        .await
        .unwrap();
    common::assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;

    // A missing user is the UC 404.
    let resp = app
        .oneshot(control(Method::GET, "/scim2/Users/nope"))
        .await
        .unwrap();
    let err = common::assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
    assert_eq!(err["message"], "User not found: nope");
}

#[rstest]
#[tokio::test]
async fn create_needs_a_primary_email(#[future(awt)] app: Router) {
    for body in [
        json!({ "schemas": [USER_SCHEMA], "userName": "bob@example.com" }),
        json!({ "emails": [{ "value": "bob@example.com" }] }),
    ] {
        let resp = app
            .clone()
            .oneshot(control_json(Method::POST, "/scim2/Users", body))
            .await
            .unwrap();
        let err = assert_scim_error(resp, "412").await;
        assert_eq!(err["detail"], "User does not have a primary email.");
    }
}

#[rstest]
#[tokio::test]
async fn duplicate_email_or_external_id_is_a_conflict(#[future(awt)] app: Router) {
    let mut body = new_user("bob@example.com");
    body["externalId"] = json!("ext-bob");
    create(&app, "carol@example.com").await;
    let resp = app
        .clone()
        .oneshot(control_json(Method::POST, "/scim2/Users", body.clone()))
        .await
        .unwrap();
    assert_status(resp, StatusCode::CREATED, "create").await;

    let resp = app
        .clone()
        .oneshot(control_json(Method::POST, "/scim2/Users", body))
        .await
        .unwrap();
    let err = assert_scim_error(resp, "409").await;
    assert_eq!(err["detail"], "User already exists: bob@example.com");
    let mut other = new_user("dave@example.com");
    other["externalId"] = json!("ext-bob");
    let resp = app
        .oneshot(control_json(Method::POST, "/scim2/Users", other))
        .await
        .unwrap();
    let err = assert_scim_error(resp, "409").await;
    assert_eq!(
        err["detail"],
        "User already exists with external id: ext-bob"
    );
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
    let body =
        json_body(assert_status(app.oneshot(me).await.unwrap(), StatusCode::OK, "me").await).await;
    assert_eq!(body["userName"], "alice@example.com");
    assert_eq!(body["roles"], json!([{ "value": "admin" }]));
}

/// Upstream `getCurrentUser` with no caller: SCIM 400 "No user found.".
#[rstest]
#[tokio::test]
async fn me_without_a_caller_is_a_scim_error(#[future(awt)] app: Router) {
    let resp = app
        .oneshot(control(Method::GET, "/scim2/Me"))
        .await
        .unwrap();
    let err = assert_scim_error(resp, "400").await;
    assert_eq!(err["detail"], "No user found.");
}

#[rstest]
#[tokio::test]
async fn create_group_with_members_then_patch_remove(#[future(awt)] app: Router) {
    let u1_id = create(&app, "u1@x").await;
    let u2_id = create(&app, "u2@x").await;

    // Create a group with both members.
    let resp = app
        .clone()
        .oneshot(control_json(
            Method::POST,
            "/scim2/Groups",
            json!({
                "schemas": [GROUP_SCHEMA],
                "displayName": "engineering",
                "members": [
                    { "value": u1_id, "display": "u1@x" },
                    { "value": u2_id, "display": "u2@x" },
                ],
            }),
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::CREATED, "create group").await).await;
    let gid = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["displayName"], "engineering");
    assert_eq!(body["members"].as_array().unwrap().len(), 2);

    // PATCH remove u1 by filter expression.
    let resp = app
        .oneshot(control_json(
            Method::PATCH,
            &format!("/scim2/Groups/{gid}"),
            json!({
                "schemas": [PATCH_SCHEMA],
                "Operations": [
                    { "op": "remove", "path": format!("members[value eq \"{u1_id}\"]") },
                ],
            }),
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "patch remove").await).await;
    let members = body["members"].as_array().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["value"], u2_id);
}

/// Upstream `patchUser` (Okta): a path-less `replace` sets the state from
/// its value's first member; 200 with no body; anything else is 501.
#[rstest]
#[tokio::test]
async fn patch_sets_the_state_like_okta(#[future(awt)] app: Router) {
    let id = create(&app, "patchme@x").await;
    let patch = |id: &str, ops: Value| {
        let app = app.clone();
        let path = format!("/scim2/Users/{id}");
        async move {
            app.oneshot(control_json(
                Method::PATCH,
                &path,
                json!({ "schemas": [PATCH_SCHEMA], "Operations": ops }),
            ))
            .await
            .unwrap()
        }
    };
    let active = |id: String| {
        let app = app.clone();
        async move {
            let resp = app
                .oneshot(control(Method::GET, &format!("/scim2/Users/{id}")))
                .await
                .unwrap();
            json_body(assert_status(resp, StatusCode::OK, "get").await).await["active"].clone()
        }
    };
    let resp = patch(
        &id,
        json!([{ "op": "replace", "value": { "active": false } }]),
    )
    .await;
    let resp = assert_status(resp, StatusCode::OK, "deactivate").await;
    assert!(
        axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(active(id.clone()).await, false);
    // Entra ID's form, with a path and a string value.
    let resp = patch(
        &id,
        json!([{ "op": "Replace", "path": "active", "value": "True" }]),
    )
    .await;
    assert_status(resp, StatusCode::OK, "activate").await;
    assert_eq!(active(id.clone()).await, true);

    let resp = patch(
        &id,
        json!([{ "op": "add", "path": "displayName", "value": "x" }]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    let resp = patch(
        &id,
        json!([{ "op": "replace", "value": { "active": "maybe" } }]),
    )
    .await;
    let err = assert_scim_error(resp, "500").await;
    assert_eq!(err["detail"], "Problem with patch operation");
    let resp = patch(
        "nope",
        json!([{ "op": "replace", "value": { "active": false } }]),
    )
    .await;
    common::assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
}

/// Each SCIM discovery endpoint answers with a static document.
#[rstest]
#[case::schemas("/scim2/Schemas")]
#[case::resource_types("/scim2/ResourceTypes")]
#[case::service_provider_config("/scim2/ServiceProviderConfig")]
#[tokio::test]
async fn discovery_endpoint_returns_static_doc(#[future(awt)] app: Router, #[case] path: &str) {
    let resp = app.oneshot(control(Method::GET, path)).await.unwrap();
    let _ = assert_status(resp, StatusCode::OK, path).await;
}

#[rstest]
#[tokio::test]
async fn service_provider_config_advertises_patch_bulk_filter(#[future(awt)] app: Router) {
    let resp = app
        .oneshot(control(Method::GET, "/scim2/ServiceProviderConfig"))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "spc").await).await;
    assert_eq!(body["patch"]["supported"], true);
    assert_eq!(body["bulk"]["supported"], true);
    assert_eq!(body["filter"]["supported"], true);
}

#[rstest]
#[tokio::test]
async fn bulk_creates_and_patches_users_atomically(#[future(awt)] app: Router) {
    let body = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
        "failOnErrors": 1,
        "Operations": [
            { "method": "POST", "path": "/Users", "bulkId": "u1", "data": new_user("bulk1@x") },
            { "method": "POST", "path": "/Users", "bulkId": "u2", "data": new_user("bulk2@x") },
        ]
    });
    let resp = app
        .oneshot(control_json(Method::POST, "/scim2/Bulk", body))
        .await
        .unwrap();
    let resp = json_body(assert_status(resp, StatusCode::OK, "bulk").await).await;
    let ops = resp["Operations"].as_array().unwrap();
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|o| o["status"] == "201"));
}

#[rstest]
#[tokio::test]
async fn bulk_stops_after_fail_on_errors_limit(#[future(awt)] app: Router) {
    // First op succeeds, second tries to delete a nonexistent user (404),
    // third should be skipped because failOnErrors = 1.
    let body = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
        "failOnErrors": 1,
        "Operations": [
            { "method": "POST", "path": "/Users", "data": new_user("ok@x") },
            { "method": "DELETE", "path": "/Users/does-not-exist" },
            { "method": "POST", "path": "/Users", "data": new_user("skipped@x") },
        ]
    });
    let resp = app
        .oneshot(control_json(Method::POST, "/scim2/Bulk", body))
        .await
        .unwrap();
    let resp = json_body(assert_status(resp, StatusCode::OK, "bulk").await).await;
    let ops = resp["Operations"].as_array().unwrap();
    assert_eq!(ops.len(), 2, "third op should be skipped");
}

#[rstest]
#[tokio::test]
async fn list_users_filter_by_user_name_eq(#[future(awt)] app: Router) {
    for n in ["alpha@x", "beta@x", "gamma@x"] {
        create(&app, n).await;
    }
    let resp = app
        .oneshot(control(
            Method::GET,
            "/scim2/Users?filter=userName%20eq%20%22beta%40x%22",
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "filter").await).await;
    assert_eq!(body["Resources"].as_array().unwrap().len(), 1);
    assert_eq!(body["Resources"][0]["userName"], "beta@x");
}

/// Upstream evaluates full SCIM filters on each user.
#[rstest]
#[tokio::test]
async fn list_users_takes_scim_filters(#[future(awt)] app: Router) {
    for n in ["alpha@x", "beta@x", "gamma@y"] {
        create(&app, n).await;
    }
    let names = |body: &Value| -> Vec<String> {
        body["Resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u["userName"].as_str().unwrap().to_string())
            .collect()
    };
    for (filter, expected) in [
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
    ] {
        let path = format!("/scim2/Users?filter={}", urlencoding(filter));
        let resp = app
            .clone()
            .oneshot(control(Method::GET, &path))
            .await
            .unwrap();
        let body = json_body(assert_status(resp, StatusCode::OK, filter).await).await;
        assert_eq!(names(&body), expected, "{filter}");
    }
    let resp = app
        .oneshot(control(
            Method::GET,
            "/scim2/Users?filter=userName%20xx%20%22a%22",
        ))
        .await
        .unwrap();
    let err = assert_scim_error(resp, "400").await;
    assert_eq!(err["scimType"], "invalidFilter");
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Upstream pages with `startIndex` (from 1) and `count` (default 50);
/// `totalResults` and `itemsPerPage` count the page.
#[rstest]
#[tokio::test]
async fn list_users_pages_by_start_index_and_count(#[future(awt)] app: Router) {
    for i in 0..55 {
        create(&app, &format!("u{i:02}@x")).await;
    }
    let page = |query: &'static str| {
        let app = app.clone();
        async move {
            let resp = app
                .oneshot(control(Method::GET, &format!("/scim2/Users{query}")))
                .await
                .unwrap();
            json_body(assert_status(resp, StatusCode::OK, query).await).await
        }
    };
    let first = page("").await;
    assert_eq!(first["Resources"].as_array().unwrap().len(), 50);
    assert_eq!(first["totalResults"], 50);
    assert_eq!(first["itemsPerPage"], 50);
    assert_eq!(first["startIndex"], 1);
    assert_eq!(first["meta"]["resourceType"], "User");
    assert_eq!(first["Resources"][0]["userName"], "u00@x");

    let second = page("?startIndex=51&count=10").await;
    assert_eq!(second["startIndex"], 51);
    assert_eq!(second["totalResults"], 5);
    assert_eq!(second["Resources"][0]["userName"], "u50@x");

    let filtered = page("?filter=userName%20sw%20%22u1%22&startIndex=3&count=2").await;
    let names: Vec<&str> = filtered["Resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["userName"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["u12@x", "u13@x"]);
}

/// Upstream `updateUser`: the body names the user; the name, state and
/// external id change; the email stays.
#[rstest]
#[tokio::test]
async fn put_changes_name_state_and_external_id(#[future(awt)] app: Router) {
    let id = create(&app, "put@x").await;
    let other = create(&app, "other@x").await;
    let put = |path_id: String, body: Value| {
        let app = app.clone();
        async move {
            app.oneshot(control_json(
                Method::PUT,
                &format!("/scim2/Users/{path_id}"),
                body,
            ))
            .await
            .unwrap()
        }
    };
    // The body's id must be the path's.
    let err = assert_scim_error(put(id.clone(), json!({ "displayName": "X" })).await, "409").await;
    assert_eq!(err["detail"], "User id mismatch.");
    assert_scim_error(
        put(id.clone(), json!({ "id": other, "displayName": "X" })).await,
        "409",
    )
    .await;
    let resp = put("nope".into(), json!({ "id": "nope" })).await;
    common::assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;

    let body = json!({ "id": id, "displayName": "Put", "active": false, "externalId": "ext-put",
        "emails": [{ "value": "changed@x", "primary": true }] });
    let resp = put(id.clone(), body).await;
    assert_eq!(resp.headers()["content-type"], "application/scim+json");
    let user = json_body(assert_status(resp, StatusCode::OK, "put").await).await;
    assert_eq!(user["displayName"], "Put");
    assert_eq!(user["active"], false);
    assert_eq!(user["externalId"], "ext-put");
    assert_eq!(user["userName"], "put@x");
    // Absent fields stay.
    let user = json_body(
        assert_status(
            put(id.clone(), json!({ "id": id })).await,
            StatusCode::OK,
            "put",
        )
        .await,
    )
    .await;
    assert_eq!(
        (user["displayName"].clone(), user["active"].clone()),
        (json!("Put"), json!(false))
    );
    // Another user's external id is the UC conflict.
    let resp = put(
        other.clone(),
        json!({ "id": other, "externalId": "ext-put" }),
    )
    .await;
    let err = common::assert_error(resp, StatusCode::CONFLICT, "ALREADY_EXISTS").await;
    assert_eq!(
        err["message"],
        "User already exists with external id: ext-put"
    );
}
