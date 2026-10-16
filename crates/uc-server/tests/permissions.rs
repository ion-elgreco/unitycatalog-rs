mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req, req_json, seeded};

#[rstest]
#[tokio::test]
async fn empty_permissions_returns_empty_list(#[future(awt)] seeded: Router) {
    let resp = seeded
        .oneshot(req(Method::GET, "/permissions/catalog/main"))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "get").await).await;
    assert_eq!(body["privilege_assignments"].as_array().unwrap().len(), 0);
}

#[rstest]
#[tokio::test]
async fn add_then_get_then_remove_privileges(#[future(awt)] seeded: Router) {
    let app = seeded;
    common::create_user(&app, "alice@example.com").await;
    common::create_user(&app, "data-eng").await;
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/permissions/catalog/main",
            json!({
                "changes": [
                    {
                        "principal": "alice@example.com",
                        "add": ["USE CATALOG", "CREATE SCHEMA"],
                        "remove": [],
                    },
                    {
                        "principal": "data-eng",
                        "add": ["SELECT", "MODIFY"],
                        "remove": [],
                    },
                ],
            }),
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "add").await).await;
    let assignments = body["privilege_assignments"].as_array().unwrap();
    assert_eq!(assignments.len(), 2);

    // Filter by principal.
    let resp = app
        .clone()
        .oneshot(req(
            Method::GET,
            "/permissions/catalog/main?principal=alice@example.com",
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "filtered").await).await;
    let assignments = body["privilege_assignments"].as_array().unwrap();
    assert_eq!(assignments.len(), 1);
    let privs: Vec<&str> = assignments[0]["privileges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert!(privs.contains(&"USE CATALOG"));
    assert!(privs.contains(&"CREATE SCHEMA"));

    // Remove one privilege.
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/permissions/catalog/main",
            json!({
                "changes": [{
                    "principal": "alice@example.com",
                    "add": [],
                    "remove": ["CREATE SCHEMA"],
                }],
            }),
        ))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "remove").await;

    let resp = app
        .oneshot(req(
            Method::GET,
            "/permissions/catalog/main?principal=alice@example.com",
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "after-remove").await).await;
    let privs: Vec<&str> = body["privilege_assignments"][0]["privileges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(privs, vec!["USE CATALOG"]);
}

/// As upstream's server, a change may leave out `add` or `remove`.
#[rstest]
#[tokio::test]
async fn changes_may_leave_out_add_or_remove(#[future(awt)] seeded: Router) {
    common::create_user(&seeded, "alice@example.com").await;
    let change = |body| {
        seeded
            .clone()
            .oneshot(req_json(Method::PATCH, "/permissions/catalog/main", body))
    };
    let resp = change(
        json!({ "changes": [{ "principal": "alice@example.com", "add": ["USE CATALOG"] }] }),
    )
    .await
    .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "add only").await).await;
    assert_eq!(
        body["privilege_assignments"][0]["privileges"],
        json!(["USE CATALOG"])
    );
    let resp = change(
        json!({ "changes": [{ "principal": "alice@example.com", "remove": ["USE CATALOG"] }] }),
    )
    .await
    .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "remove only").await).await;
    assert!(body["privilege_assignments"].as_array().unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn removing_all_privileges_drops_the_assignment(#[future(awt)] seeded: Router) {
    let app = seeded;
    common::create_user(&app, "p1").await;
    let resp = app.clone().oneshot(req_json(Method::POST, "/tables", json!({
        "name": "t1", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA", "storage_location": "s3://lake/t1",
    }))).await.unwrap();
    assert_status(resp, StatusCode::OK, "table").await;
    app.clone()
        .oneshot(req_json(
            Method::PATCH,
            "/permissions/table/main.default.t1",
            json!({
                "changes": [{
                    "principal": "p1", "add": ["SELECT"], "remove": [],
                }],
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/permissions/table/main.default.t1",
            json!({
                "changes": [{
                    "principal": "p1", "add": [], "remove": ["SELECT"],
                }],
            }),
        ))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "remove all").await;

    let resp = app
        .oneshot(req(Method::GET, "/permissions/table/main.default.t1"))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "get").await).await;
    assert_eq!(body["privilege_assignments"].as_array().unwrap().len(), 0);
}

#[rstest]
#[tokio::test]
async fn metastore_summary_returns_stable_id(#[future(awt)] app: Router) {
    let r1 = app
        .clone()
        .oneshot(req(Method::GET, "/metastore_summary"))
        .await
        .unwrap();
    let id1 = json_body(assert_status(r1, StatusCode::OK, "first").await).await;
    assert!(id1["metastore_id"].is_string());

    let r2 = app
        .oneshot(req(Method::GET, "/metastore_summary"))
        .await
        .unwrap();
    let id2 = json_body(assert_status(r2, StatusCode::OK, "second").await).await;
    assert_eq!(
        id1["metastore_id"], id2["metastore_id"],
        "id should persist"
    );
}

/// Upstream `PermissionService`: grants name existing securables and users,
/// a PATCH answers for its own principals only, and grants follow the
/// securable through renames and go away with it.
#[rstest]
#[tokio::test]
async fn grants_follow_upstream_rules(#[future(awt)] seeded: Router) {
    let app = seeded;
    let patch = |path: &str, principal: &str, add: &[&str]| {
        app.clone().oneshot(req_json(
            Method::PATCH,
            path,
            json!({
                "changes": [{ "principal": principal, "add": add, "remove": [] }],
            }),
        ))
    };

    let err = assert_error(
        patch("/permissions/catalog/nope", "x", &["USE CATALOG"])
            .await
            .unwrap(),
        StatusCode::NOT_FOUND,
        "CATALOG_NOT_FOUND",
    )
    .await;
    assert_eq!(err["message"], "Catalog not found: nope");
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/permissions/table/main.default.nope"))
        .await
        .unwrap();
    assert_error(resp, StatusCode::NOT_FOUND, "TABLE_NOT_FOUND").await;
    let err = assert_error(
        patch(
            "/permissions/catalog/main",
            "ghost@example.com",
            &["USE CATALOG"],
        )
        .await
        .unwrap(),
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;
    assert_eq!(err["message"], "User not found: ghost@example.com");

    // A principal whose name differs only in punctuation is another principal.
    common::create_user(&app, "john.doe@x.com").await;
    common::create_user(&app, "john_doe@x.com").await;
    let resp = patch(
        "/permissions/catalog/main",
        "john.doe@x.com",
        &["USE CATALOG"],
    )
    .await
    .unwrap();
    assert_status(resp, StatusCode::OK, "grant 1").await;
    let resp = patch(
        "/permissions/catalog/main",
        "john_doe@x.com",
        &["CREATE SCHEMA"],
    )
    .await
    .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "grant 2").await).await;
    assert_eq!(
        body["privilege_assignments"],
        json!([
            { "principal": "john_doe@x.com", "privileges": ["CREATE SCHEMA"] },
        ]),
        "a PATCH answers for its own principals only"
    );
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/permissions/catalog/main"))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "get").await).await;
    assert_eq!(body["privilege_assignments"].as_array().unwrap().len(), 2);

    // The metastore has one id, whatever name the path uses.
    let resp = patch(
        "/permissions/metastore/metastore",
        "john.doe@x.com",
        &["CREATE CATALOG"],
    )
    .await
    .unwrap();
    assert_status(resp, StatusCode::OK, "metastore").await;
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/permissions/metastore/anything"))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "metastore get").await).await;
    assert_eq!(
        body["privilege_assignments"][0]["privileges"],
        json!(["CREATE CATALOG"])
    );

    // Renames keep grants; deletes drop them.
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/catalogs/main",
            json!({ "new_name": "renamed" }),
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "rename").await;
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/permissions/catalog/renamed"))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "after rename").await).await;
    assert_eq!(body["privilege_assignments"].as_array().unwrap().len(), 2);
    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, "/catalogs/renamed?force=true"))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "drop").await;
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "renamed" }),
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "recreate").await;
    let resp = app
        .oneshot(req(Method::GET, "/permissions/catalog/renamed"))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "recreated").await).await;
    assert_eq!(body["privilege_assignments"], json!([]));
}

/// Deleting a securable, also through a cascade, deletes its grants.
#[rstest]
#[tokio::test]
async fn grants_go_with_their_securable(#[future(awt)] app: Router) {
    common::create_user(&app, "alice@example.com").await;
    let call = |method, path: &str, body| {
        let request = match body {
            Some(body) => req_json(method, path, body),
            None => req(method, path),
        };
        app.clone().oneshot(request)
    };
    call(Method::POST, "/catalogs", Some(json!({ "name": "gc" })))
        .await
        .unwrap();
    call(
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "gc" })),
    )
    .await
    .unwrap();
    for (path, privilege) in [
        ("/permissions/catalog/gc", "USE CATALOG"),
        ("/permissions/schema/gc.s", "USE SCHEMA"),
    ] {
        let change =
            json!({ "changes": [{ "principal": "alice@example.com", "add": [privilege] }] });
        let resp = call(Method::PATCH, path, Some(change)).await.unwrap();
        assert_status(resp, StatusCode::OK, "grant").await;
    }
    let grants = || async {
        let resp = app
            .clone()
            .oneshot(common::as_admin(req(Method::GET, "/admin/backup")))
            .await
            .unwrap();
        let snapshot = json_body(assert_status(resp, StatusCode::OK, "backup").await).await;
        snapshot["tables"]["permission"]
            .as_array()
            .map_or(0, Vec::len)
    };
    assert_eq!(grants().await, 2);
    let resp = call(Method::DELETE, "/catalogs/gc?force=true", None)
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "delete").await;
    assert_eq!(grants().await, 0);
}
