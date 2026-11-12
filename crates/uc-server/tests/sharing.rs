//! Sharing: shares and recipients, and what a recipient reads with the
//! Delta Sharing and Open Sharing protocols.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, assert_status, json_body, req_json, seeded};

const DELTA: &str = "/api/v1/delta-sharing";
const OPEN: &str = "/api/1.0/opensharing";

/// A protocol request: `path` is the whole path, with the protocol prefix.
async fn call(app: &Router, method: Method, path: &str, token: Option<&str>) -> Response {
    send(app, method, path, token, json!({})).await
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> Response {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let request = request
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn get_json(app: &Router, path: &str, token: &str) -> Value {
    let resp = call(app, Method::GET, path, Some(token)).await;
    json_body(assert_status(resp, StatusCode::OK, path).await).await
}

/// The lines of an NDJSON answer.
async fn lines(resp: Response) -> Vec<Value> {
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (String::from_utf8(body.to_vec()).unwrap().lines())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

async fn admin(app: &Router, method: Method, path: &str, body: Value) -> Value {
    let resp = (app.clone().oneshot(req_json(method, path, body)).await).unwrap();
    json_body(assert_status(resp, StatusCode::OK, path).await).await
}

/// The path of the activation link of the newest token of `recipient`.
fn activation(recipient: &Value) -> String {
    let tokens = recipient["tokens"].as_array().unwrap();
    let url = tokens.last().unwrap()["activation_url"].as_str().unwrap();
    let at = url.find("/api/").unwrap_or_else(|| panic!("{recipient}"));
    url[at..].to_string()
}

/// The profile that the activation link at `path` gives.
async fn activate(app: &Router, path: &str) -> Value {
    let resp = call(app, Method::GET, path, None).await;
    json_body(assert_status(resp, StatusCode::OK, path).await).await
}

/// Let `recipient` read `share`, or with `remove` not any more.
async fn permission(app: &Router, share: &str, recipient: &str, change: &str) -> Value {
    let path = format!("/shares/{share}/permissions");
    let changes = json!({ "changes": [{ "principal": recipient, change: ["SELECT"] }] });
    admin(app, Method::PATCH, &path, changes).await
}

/// A recipient with the shares `shares`; its token, from its activation
/// link.
async fn recipient(app: &Router, name: &str, shares: &[&str]) -> String {
    let created = admin(app, Method::POST, "/recipients", json!({ "name": name })).await;
    for share in shares {
        permission(app, share, name, "add").await;
    }
    let profile = activate(app, &activation(&created)).await;
    profile["bearerToken"].as_str().unwrap().to_string()
}

/// The change `action` of the share `share` with the object `object`.
fn update(action: &str, object: Value) -> Value {
    json!({ "updates": [{ "action": action, "data_object": object }] })
}

/// A Delta table of two versions in `dir`: version 1 adds one file.
fn delta_table(dir: &std::path::Path) {
    let log = dir.join("_delta_log");
    std::fs::create_dir_all(&log).unwrap();
    let schema =
        r#"{"type":"struct","fields":[{"name":"id","type":"long","nullable":true,"metadata":{}}]}"#;
    let v0 = [
        json!({ "protocol": { "minReaderVersion": 1, "minWriterVersion": 2 } }),
        json!({ "metaData": {
            "id": "5fba94ed-9794-4965-ba6e-6ee3c0d22af9",
            "format": { "provider": "parquet", "options": {} },
            "schemaString": schema,
            "partitionColumns": [],
            "configuration": {},
            "createdTime": 1700000000000i64,
        } }),
    ];
    let v1 = [json!({ "add": {
        "path": "part-0.parquet",
        "partitionValues": {},
        "size": 100,
        "modificationTime": 1700000000000i64,
        "dataChange": true,
        "stats": r#"{"numRecords":3,"minValues":{"id":1},"maxValues":{"id":3},"nullCount":{"id":0}}"#,
    } })];
    let commit = |actions: &[Value]| {
        (actions.iter().map(Value::to_string).collect::<Vec<_>>()).join("\n") + "\n"
    };
    std::fs::write(log.join("00000000000000000000.json"), commit(&v0)).unwrap();
    std::fs::write(log.join("00000000000000000001.json"), commit(&v1)).unwrap();
}

#[rstest]
#[tokio::test]
async fn a_recipient_reads_a_shared_table(#[future(awt)] seeded: Router) {
    let app = seeded;
    let dir = tempfile::tempdir().unwrap();
    delta_table(dir.path());
    let location = format!("file://{}", dir.path().display());
    let table = json!({
        "name": "events", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": location,
    });
    admin(&app, Method::POST, "/tables", table).await;
    admin(
        &app,
        Method::POST,
        "/shares",
        json!({ "name": "analytics" }),
    )
    .await;
    let object = json!({
        "name": "main.default.events", "data_object_type": "TABLE", "shared_as": "public.events",
    });
    let objects = update("ADD", object);
    let share = admin(&app, Method::PATCH, "/shares/analytics", objects).await;
    assert_eq!(share["objects"][0]["data_object_type"], "TABLE");
    assert_eq!(share["objects"][0]["shared_as"], "public.events");
    let token = recipient(&app, "partner-x", &["analytics"]).await;
    assert!(token.starts_with("dsr_"));

    // Both protocols serve the tables.
    for prefix in [DELTA, OPEN] {
        let shares = get_json(&app, &format!("{prefix}/shares"), &token).await;
        assert_eq!(shares["items"][0]["name"], "analytics");
        let schemas = get_json(&app, &format!("{prefix}/shares/analytics/schemas"), &token).await;
        assert_eq!(schemas["items"][0]["name"], "public");
        let tables = get_json(
            &app,
            &format!("{prefix}/shares/analytics/all-tables"),
            &token,
        )
        .await;
        assert_eq!(tables["items"][0]["name"], "events");
        assert_eq!(tables["items"][0]["schema"], "public");

        let table = format!("{prefix}/shares/analytics/schemas/public/tables/events");
        let resp = call(&app, Method::GET, &format!("{table}/version"), Some(&token)).await;
        let resp = assert_status(resp, StatusCode::OK, "version").await;
        assert_eq!(resp.headers()["delta-table-version"], "1");

        let resp = call(
            &app,
            Method::GET,
            &format!("{table}/metadata"),
            Some(&token),
        )
        .await;
        let resp = assert_status(resp, StatusCode::OK, "metadata").await;
        assert_eq!(resp.headers()["delta-table-version"], "1");
        let content_type = resp.headers()["content-type"].to_str().unwrap().to_string();
        assert!(content_type.contains("application/x-ndjson"));
        let metadata = lines(resp).await;
        assert_eq!(metadata.len(), 2, "{metadata:?}");
        assert_eq!(metadata[0]["protocol"]["minReaderVersion"], 1);
        assert!(
            (metadata[1]["metaData"]["schemaString"].as_str().unwrap()).contains("\"id\""),
            "{metadata:?}"
        );

        let resp = call(&app, Method::POST, &format!("{table}/query"), Some(&token)).await;
        let query = lines(assert_status(resp, StatusCode::OK, "query").await).await;
        assert_eq!(query.len(), 3, "{query:?}");
        let file = &query[2]["file"];
        assert_eq!(file["url"], format!("{location}/part-0.parquet"));
        assert_eq!(file["size"], 100);
        assert!(file["stats"].as_str().unwrap().contains("numRecords"));
    }
}

/// The local Delta table `main.default.events` in `dir`, in the share
/// `analytics` as `public.events` with the settings `history`.
async fn shared_events(app: &Router, dir: &std::path::Path, history: Value) -> Value {
    delta_table(dir);
    let table = json!({
        "name": "events", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("file://{}", dir.display()),
    });
    admin(app, Method::POST, "/tables", table).await;
    admin(app, Method::POST, "/shares", json!({ "name": "analytics" })).await;
    share_events(app, "ADD", history).await
}

/// The table `main.default.events` as `public.events`, with `history`.
fn events(history: Value) -> Value {
    let mut object = json!({
        "name": "main.default.events", "data_object_type": "TABLE", "shared_as": "public.events",
    });
    object
        .as_object_mut()
        .unwrap()
        .extend(history.as_object().unwrap().clone());
    object
}

async fn share_events(app: &Router, action: &str, history: Value) -> Value {
    let objects = update(action, events(history));
    admin(app, Method::PATCH, "/shares/analytics", objects).await
}

/// The status of a query of the shared table `events` at `version`.
async fn query_at(app: &Router, token: &str, version: i64) -> StatusCode {
    let path = format!("{DELTA}/shares/analytics/schemas/public/tables/events/query");
    let body = json!({ "version": version });
    send(app, Method::POST, &path, Some(token), body)
        .await
        .status()
}

#[rstest]
#[tokio::test]
async fn history_is_shared_when_the_share_says_so(#[future(awt)] seeded: Router) {
    let app = seeded;
    let dir = tempfile::tempdir().unwrap();
    let share = shared_events(&app, dir.path(), json!({})).await;
    assert_eq!(
        share["objects"][0]["history_data_sharing_status"],
        "DISABLED"
    );
    let token = recipient(&app, "partner-x", &["analytics"]).await;

    // Without history, the latest version only.
    let latest = format!("{DELTA}/shares/analytics/schemas/public/tables/events/query");
    let resp = call(&app, Method::POST, &latest, Some(&token)).await;
    assert_status(resp, StatusCode::OK, "latest").await;
    assert_eq!(query_at(&app, &token, 0).await, StatusCode::BAD_REQUEST);

    // An update replaces the settings of the object.
    let history = json!({ "history_data_sharing_status": "ENABLED" });
    let share = share_events(&app, "UPDATE", history).await;
    assert_eq!(share["objects"].as_array().unwrap().len(), 1);
    assert_eq!(query_at(&app, &token, 0).await, StatusCode::OK);
    assert_eq!(query_at(&app, &token, 1).await, StatusCode::OK);

    // From version 1 on.
    let history = json!({ "history_data_sharing_status": "ENABLED", "start_version": 1 });
    share_events(&app, "UPDATE", history).await;
    assert_eq!(query_at(&app, &token, 0).await, StatusCode::BAD_REQUEST);
    assert_eq!(query_at(&app, &token, 1).await, StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn history_settings_are_checked(#[future(awt)] seeded: Router) {
    let app = seeded;
    admin(
        &app,
        Method::POST,
        "/shares",
        json!({ "name": "analytics" }),
    )
    .await;
    let refused = [
        (json!({ "start_version": 1 }), "start_version needs"),
        (
            json!({ "history_data_sharing_status": "ENABLED", "start_version": -1 }),
            "cannot be negative",
        ),
        (
            json!({ "data_object_type": "VOLUME", "history_data_sharing_status": "ENABLED" }),
            "is not a table",
        ),
        (
            json!({ "data_object_type": "MODEL" }),
            "data_object_type must be",
        ),
        (json!({ "shared_as": "events" }), "must be <schema>.<name>"),
        (
            json!({ "cdf_enabled": true }),
            "cdf_enabled is not supported",
        ),
    ];
    for (history, message) in refused {
        let request = req_json(
            Method::PATCH,
            "/shares/analytics",
            update("ADD", events(history.clone())),
        );
        let resp = app.clone().oneshot(request).await.unwrap();
        let resp = assert_status(resp, StatusCode::BAD_REQUEST, "refused").await;
        // The answer says what is wrong, as the other routes of the API do.
        let error = json_body(resp).await;
        assert_eq!(error["error_code"], "INVALID_ARGUMENT", "{history}");
        assert!(
            error["message"].as_str().unwrap().contains(message),
            "{error}"
        );
    }
}

#[rstest]
#[tokio::test]
async fn a_recipient_reads_a_shared_volume_and_skill(#[future(awt)] seeded: Router) {
    let app = seeded;
    let dir = tempfile::tempdir().unwrap();
    let skill = "---\nname: lookup\ndescription: Looks up an order.\nlicense: Apache-2.0\n---\n\n# Lookup\n";
    std::fs::write(dir.path().join("SKILL.md"), skill).unwrap();
    let volume = json!({
        "name": "lookup", "catalog_name": "main", "schema_name": "default",
        "volume_type": "EXTERNAL",
        "storage_location": format!("file://{}", dir.path().display()),
    });
    admin(&app, Method::POST, "/volumes", volume).await;
    admin(&app, Method::POST, "/shares", json!({ "name": "tools" })).await;
    // A volume is in a share once per type.
    let object = |kind: &str, name: &str| {
        json!({
            "name": "main.default.lookup",
            "data_object_type": kind,
            "string_shared_as": format!("public.{name}"),
        })
    };
    let change = |action: &str, kind: &str, name: &str| json!({ "action": action, "data_object": object(kind, name) });
    let objects = json!({ "updates": [
        change("ADD", "VOLUME", "first"),
        change("ADD", "AGENT_SKILL", "lookup"),
        change("UPDATE", "VOLUME", "files"),
    ] });
    let share = admin(&app, Method::PATCH, "/shares/tools", objects).await;
    assert_eq!(share["objects"].as_array().unwrap().len(), 2);
    assert_eq!(share["objects"][0]["string_shared_as"], "public.files");
    let again = req_json(
        Method::PATCH,
        "/shares/tools",
        update("ADD", object("VOLUME", "again")),
    );
    let resp = app.clone().oneshot(again).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let token = recipient(&app, "partner-x", &["tools"]).await;

    let share = format!("{OPEN}/shares/tools");
    let volumes = get_json(&app, &format!("{share}/all-volumes"), &token).await;
    assert_eq!(volumes["items"].as_array().unwrap().len(), 1, "{volumes}");
    assert_eq!(volumes["items"][0]["name"], "files");
    let volume = get_json(
        &app,
        &format!("{share}/schemas/public/volumes/files"),
        &token,
    )
    .await;
    assert_eq!(volume["name"], "files");

    let skills = get_json(&app, &format!("{share}/all-skills"), &token).await;
    assert_eq!(skills["items"].as_array().unwrap().len(), 1, "{skills}");
    let skill = get_json(
        &app,
        &format!("{share}/schemas/public/skills/lookup"),
        &token,
    )
    .await;
    assert_eq!(skill["name"], "lookup");
    assert_eq!(skill["description"], "Looks up an order.");

    // Neither is a table, and the Delta Sharing protocol has tables only.
    let tables = get_json(&app, &format!("{share}/all-tables"), &token).await;
    assert!(
        tables["items"].as_array().is_none_or(Vec::is_empty),
        "{tables}"
    );
    let path = format!("{DELTA}/shares/tools/all-volumes");
    let resp = call(&app, Method::GET, &path, Some(&token)).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn the_protocols_need_a_recipient_token(#[future(awt)] app: Router) {
    for prefix in [DELTA, OPEN] {
        let path = format!("{prefix}/shares");
        let resp = call(&app, Method::GET, &path, None).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let resp = call(&app, Method::GET, &path, Some("dsr_unknown")).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

#[rstest]
#[tokio::test]
async fn a_share_that_was_not_granted_is_not_found(#[future(awt)] app: Router) {
    admin(&app, Method::POST, "/shares", json!({ "name": "secret" })).await;
    let token = recipient(&app, "noone", &[]).await;

    let shares = get_json(&app, &format!("{DELTA}/shares"), &token).await;
    assert!(
        shares["items"].as_array().is_none_or(Vec::is_empty),
        "{shares}"
    );
    // The answer for a share that does not exist is the same.
    for share in ["secret", "missing"] {
        let path = format!("{DELTA}/shares/{share}/schemas");
        let resp = call(&app, Method::GET, &path, Some(&token)).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{share}");
    }
}

#[tokio::test]
async fn a_recipient_token_signs_in_on_the_protocols_only() {
    let (app, metastore) = common::signed_in_app().await;
    let recipient = uc_metastore::NewRecipient {
        name: "partner-x".into(),
        ..Default::default()
    };
    let sharing = metastore.sharing();
    let recipient = sharing.create_recipient(recipient, None).await.unwrap();
    let code = recipient.tokens[0].activation_code.clone().unwrap();
    let token = sharing.activate(&code).await.unwrap().bearer_token;

    for prefix in [DELTA, OPEN] {
        let resp = call(&app, Method::GET, &format!("{prefix}/shares"), Some(&token)).await;
        assert_status(resp, StatusCode::OK, "list shares").await;
    }
    // The UC API does not take it.
    let path = format!("{}/catalogs", common::BASE);
    let resp = call(&app, Method::GET, &path, Some(&token)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[rstest]
#[tokio::test]
async fn an_activation_link_gives_the_profile_once(#[future(awt)] app: Router) {
    let body =
        json!({ "name": "partner-x", "properties_kvpairs": { "properties": { "team": "x" } } });
    let created = admin(&app, Method::POST, "/recipients", body).await;
    assert_eq!(created["authentication_type"], "TOKEN");
    assert_eq!(created["properties_kvpairs"]["properties"]["team"], "x");
    // The admin gets the link, and no token.
    assert!(!created.to_string().contains("dsr_"), "{created}");
    let link = activation(&created);
    assert!(link.starts_with("/api/1.0/opensharing/activations/"));

    let profile = activate(&app, &link).await;
    assert_eq!(profile["shareCredentialsVersion"], 1);
    assert!(
        profile["endpoint"].as_str().unwrap().ends_with(DELTA),
        "{profile}"
    );
    let token = profile["bearerToken"].as_str().unwrap();
    get_json(&app, &format!("{DELTA}/shares"), token).await;

    // The link is used up, and the recipient shows none.
    let resp = call(&app, Method::GET, &link, None).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let found = admin(&app, Method::GET, "/recipients/partner-x", json!({})).await;
    assert_eq!(found["tokens"].as_array().unwrap().len(), 1);
    assert!(
        found["tokens"][0].get("activation_url").is_none(),
        "{found}"
    );
}

#[rstest]
#[tokio::test]
async fn a_rotated_token_can_overlap_the_new_one(#[future(awt)] app: Router) {
    let created = admin(
        &app,
        Method::POST,
        "/recipients",
        json!({ "name": "partner-x" }),
    )
    .await;
    let first = activate(&app, &activation(&created)).await;
    let first = first["bearerToken"].as_str().unwrap();
    let shares = format!("{DELTA}/shares");

    // With an overlap, both tokens work.
    let rotate = "/recipients/partner-x/rotate-token";
    let body = json!({ "existing_token_expire_in_seconds": 3600 });
    let rotated = admin(&app, Method::POST, rotate, body).await;
    assert_eq!(rotated["tokens"].as_array().unwrap().len(), 2, "{rotated}");
    // The models write 64-bit numbers as text, as proto3 JSON does.
    let end = rotated["tokens"][0]["expiration_time"].as_str().unwrap();
    assert!(end.parse::<i64>().unwrap() > 0);
    let second = activate(&app, &activation(&rotated)).await;
    let second = second["bearerToken"].as_str().unwrap();
    assert_ne!(first, second);
    for token in [first, second] {
        get_json(&app, &shares, token).await;
    }

    // Without one, the older tokens end at once.
    let rotated = admin(&app, Method::POST, rotate, json!({})).await;
    assert_eq!(rotated["tokens"].as_array().unwrap().len(), 1, "{rotated}");
    for token in [first, second] {
        let resp = call(&app, Method::GET, &shares, Some(token)).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
    let third = activate(&app, &activation(&rotated)).await;
    get_json(&app, &shares, third["bearerToken"].as_str().unwrap()).await;
}

#[rstest]
#[tokio::test]
async fn permissions_grant_a_share_to_recipients(#[future(awt)] app: Router) {
    admin(
        &app,
        Method::POST,
        "/shares",
        json!({ "name": "analytics" }),
    )
    .await;
    let token = recipient(&app, "partner-x", &[]).await;
    let shares = format!("{DELTA}/shares");
    let names = |listed: &Value| listed["items"].as_array().map_or(0, Vec::len);
    assert_eq!(names(&get_json(&app, &shares, &token).await), 0);

    let granted = permission(&app, "analytics", "partner-x", "add").await;
    let expected = json!([{ "principal": "partner-x", "privileges": ["SELECT"] }]);
    assert_eq!(granted["privilege_assignments"], expected);
    let read = admin(
        &app,
        Method::GET,
        "/shares/analytics/permissions",
        json!({}),
    )
    .await;
    assert_eq!(read["privilege_assignments"], expected);
    assert_eq!(names(&get_json(&app, &shares, &token).await), 1);

    // A share with the same name later is another share.
    let resp = (app.clone())
        .oneshot(req_json(Method::DELETE, "/shares/analytics", json!({})))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "delete").await;
    admin(
        &app,
        Method::POST,
        "/shares",
        json!({ "name": "analytics" }),
    )
    .await;
    assert_eq!(names(&get_json(&app, &shares, &token).await), 0);

    permission(&app, "analytics", "partner-x", "add").await;
    let revoked = permission(&app, "analytics", "partner-x", "remove").await;
    assert!(revoked.get("privilege_assignments").is_none(), "{revoked}");
    assert_eq!(names(&get_json(&app, &shares, &token).await), 0);

    // A recipient has SELECT only, and an unknown recipient is not found.
    let path = "/shares/analytics/permissions";
    let other = json!({ "changes": [{ "principal": "partner-x", "add": ["MODIFY"] }] });
    let resp = (app
        .clone()
        .oneshot(req_json(Method::PATCH, path, other))
        .await)
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let unknown = json!({ "changes": [{ "principal": "nobody", "add": ["SELECT"] }] });
    let resp = (app
        .clone()
        .oneshot(req_json(Method::PATCH, path, unknown))
        .await)
        .unwrap();
    let error = json_body(assert_status(resp, StatusCode::NOT_FOUND, "unknown").await).await;
    assert_eq!(error["message"], "recipient 'nobody' not found");
}

#[rstest]
#[tokio::test]
async fn shares_are_listed_without_their_objects(#[future(awt)] app: Router) {
    admin(
        &app,
        Method::POST,
        "/shares",
        json!({ "name": "analytics" }),
    )
    .await;
    let object = json!({ "name": "main.default.events", "data_object_type": "TABLE" });
    let share = admin(
        &app,
        Method::PATCH,
        "/shares/analytics",
        update("ADD", object),
    )
    .await;
    // Without `shared_as`, the schema and name of the catalog.
    assert_eq!(share["objects"][0]["shared_as"], "default.events");

    let listed = admin(&app, Method::GET, "/shares", json!({})).await;
    assert_eq!(listed["shares"][0]["name"], "analytics");
    assert!(listed["shares"][0].get("objects").is_none(), "{listed}");
    let bare = admin(&app, Method::GET, "/shares/analytics", json!({})).await;
    assert!(bare.get("objects").is_none(), "{bare}");
    let path = "/shares/analytics?include_shared_data=true";
    let full = admin(&app, Method::GET, path, json!({})).await;
    assert_eq!(full["objects"][0]["name"], "main.default.events");

    let renamed = json!({ "new_name": "reports", "comment": "weekly" });
    let share = admin(&app, Method::PATCH, "/shares/analytics", renamed).await;
    assert_eq!(
        (share["name"].as_str(), share["comment"].as_str()),
        (Some("reports"), Some("weekly"))
    );
}

#[rstest]
#[tokio::test]
async fn a_bad_request_body_gets_the_error_body_of_the_api(#[future(awt)] app: Router) {
    let path = format!("{}/shares", common::BASE);
    for body in ["{", r#"{"name": 1}"#] {
        let request = Request::builder()
            .method(Method::POST)
            .uri(&path)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(request).await.unwrap();
        let error = json_body(assert_status(resp, StatusCode::BAD_REQUEST, body).await).await;
        assert_eq!(error["error_code"], "INVALID_ARGUMENT", "{body}");
        assert!(
            error["message"].as_str().unwrap().contains("JSON"),
            "{error}"
        );
    }
}
