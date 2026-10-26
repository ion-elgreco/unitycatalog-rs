//! The Files API: the files of volumes, and the files of tables,
//! read-only.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use http_body_util::BodyExt;
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{assert_error, assert_status, json_body, req_json, seeded};
use uc_common::FILES_BASE;

/// A Files API request: `path` under [`FILES_BASE`], with `body`, signed in
/// with `token` if any.
fn fs(method: Method, path: &str, body: &[u8], token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("{FILES_BASE}{path}"))
        .body(Body::from(body.to_vec()))
        .unwrap();
    if let Some(token) = token {
        let value = format!("Bearer {token}").parse().unwrap();
        request.headers_mut().insert(header::AUTHORIZATION, value);
    }
    request
}

async fn call(app: &Router, method: Method, path: &str, body: &[u8]) -> Response {
    app.clone()
        .oneshot(fs(method, path, body, None))
        .await
        .unwrap()
}

async fn status(app: &Router, method: Method, path: &str, body: &[u8]) -> StatusCode {
    call(app, method, path, body).await.status()
}

async fn bytes(response: Response) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

/// The names of the entries of a listing, directories with a `/`.
fn names(listing: &Value) -> Vec<String> {
    listing["contents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let name = entry["name"].as_str().unwrap();
            match entry["is_directory"].as_bool().unwrap() {
                true => format!("{name}/"),
                false => name.to_string(),
            }
        })
        .collect()
}

async fn list(app: &Router, path: &str) -> Value {
    let response = call(app, Method::GET, &format!("/directories{path}"), b"").await;
    json_body(assert_status(response, StatusCode::OK, path).await).await
}

#[rstest]
#[tokio::test]
async fn volume_files(#[future(awt)] seeded: Router) {
    let app = seeded;
    let volume = json!({
        "name": "v", "catalog_name": "main", "schema_name": "default", "volume_type": "MANAGED",
    });
    let resp = app
        .clone()
        .oneshot(req_json(Method::POST, "/volumes", volume))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "volume").await;
    let v = "/Volumes/main/default/v";
    assert_eq!(list(&app, v).await, json!({ "contents": [] }));

    // Uploads: an existing file only with `overwrite`.
    let b = format!("/files{v}/a/b.txt");
    assert_eq!(
        status(&app, Method::PUT, &b, b"hello world").await,
        StatusCode::NO_CONTENT
    );
    let resp = call(&app, Method::PUT, &b, b"hello").await;
    assert_error(resp, StatusCode::CONFLICT, "ALREADY_EXISTS").await;
    let overwrite = format!("{b}?overwrite=true");
    assert_eq!(
        status(&app, Method::PUT, &overwrite, b"hello there!").await,
        StatusCode::NO_CONTENT
    );
    let top = format!("/files{v}/top.txt");
    assert_eq!(
        status(&app, Method::PUT, &top, b"x").await,
        StatusCode::NO_CONTENT
    );

    // Listings: directories first, then files; pages.
    let root = list(&app, v).await;
    assert_eq!(names(&root), ["a/", "top.txt"]);
    assert_eq!(root["contents"][0]["path"], format!("{v}/a/"));
    assert_eq!(root["contents"][1]["path"], format!("{v}/top.txt"));
    assert_eq!(root["contents"][1]["file_size"], 1);
    assert!(root["contents"][1]["last_modified"].as_i64().unwrap() > 0);
    assert!(root["contents"][0].get("file_size").is_none());
    let first = list(&app, &format!("{v}?page_size=1")).await;
    assert_eq!(names(&first), ["a/"]);
    let token = first["next_page_token"].as_str().unwrap();
    let second = list(&app, &format!("{v}?page_size=1&page_token={token}")).await;
    assert_eq!(names(&second), ["top.txt"]);
    assert!(second.get("next_page_token").is_none());
    assert_eq!(names(&list(&app, &format!("{v}/a/")).await), ["b.txt"]);
    let missing = call(&app, Method::GET, &format!("/directories{v}/nope"), b"").await;
    assert_error(missing, StatusCode::NOT_FOUND, "NOT_FOUND").await;

    // Reads: metadata, the whole file, and ranges.
    let head = call(&app, Method::HEAD, &b, b"").await;
    let head = assert_status(head, StatusCode::OK, "head").await;
    assert_eq!(head.headers()[header::CONTENT_LENGTH], "12");
    assert_eq!(
        head.headers()[header::CONTENT_TYPE],
        "application/octet-stream"
    );
    assert!(head.headers().contains_key(header::LAST_MODIFIED));
    let get = call(&app, Method::GET, &b, b"").await;
    assert_eq!(
        bytes(assert_status(get, StatusCode::OK, "get").await).await,
        b"hello there!"
    );
    let mut ranged = fs(Method::GET, &b, b"", None);
    ranged
        .headers_mut()
        .insert(header::RANGE, "bytes=6-10".parse().unwrap());
    let part = app.clone().oneshot(ranged).await.unwrap();
    let part = assert_status(part, StatusCode::PARTIAL_CONTENT, "range").await;
    assert_eq!(part.headers()[header::CONTENT_RANGE], "bytes 6-10/12");
    assert_eq!(bytes(part).await, b"there");
    let mut outside = fs(Method::GET, &b, b"", None);
    outside
        .headers_mut()
        .insert(header::RANGE, "bytes=50-".parse().unwrap());
    let outside = app.clone().oneshot(outside).await.unwrap();
    assert_eq!(outside.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(outside.headers()[header::CONTENT_RANGE], "bytes */12");
    let resp = call(&app, Method::GET, &format!("/files{v}/nope.txt"), b"").await;
    assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;

    // Directories: local storage has empty ones.
    let dir = |path: &str| format!("/directories{v}/{path}");
    assert_eq!(
        status(&app, Method::HEAD, &dir("a"), b"").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, Method::HEAD, &dir(""), b"").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, Method::HEAD, &dir("nope"), b"").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status(&app, Method::PUT, &dir("new/deeper"), b"").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        status(&app, Method::HEAD, &dir("new/deeper"), b"").await,
        StatusCode::OK
    );
    assert_eq!(
        list(&app, &format!("{v}/new/deeper")).await,
        json!({ "contents": [] })
    );
    let resp = call(&app, Method::PUT, &dir("top.txt"), b"").await;
    assert_error(resp, StatusCode::CONFLICT, "ALREADY_EXISTS").await;

    // Deletes: files, then empty directories.
    let resp = call(&app, Method::DELETE, &dir("a"), b"").await;
    assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
    assert_eq!(
        status(&app, Method::DELETE, &b, b"").await,
        StatusCode::NO_CONTENT
    );
    let resp = call(&app, Method::DELETE, &b, b"").await;
    assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
    assert_eq!(
        status(&app, Method::DELETE, &dir("a"), b"").await,
        StatusCode::NO_CONTENT
    );
    let resp = call(&app, Method::DELETE, &dir("a"), b"").await;
    assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
    let resp = call(&app, Method::DELETE, &dir(""), b"").await;
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(names(&list(&app, v).await), ["new/", "top.txt"]);

    // With `recursive`, a directory and what it holds; never the volume.
    for file in ["new/deeper/x.txt", "new/y.txt"] {
        let path = format!("/files{v}/{file}");
        assert_eq!(
            status(&app, Method::PUT, &path, b"x").await,
            StatusCode::NO_CONTENT
        );
    }
    let resp = call(&app, Method::DELETE, &dir("new"), b"").await;
    assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
    let recursive = format!("{}?recursive=true", dir("new"));
    assert_eq!(
        status(&app, Method::DELETE, &recursive, b"").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        status(&app, Method::HEAD, &dir("new"), b"").await,
        StatusCode::NOT_FOUND
    );
    let resp = call(&app, Method::DELETE, &recursive, b"").await;
    assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
    let resp = call(
        &app,
        Method::DELETE,
        &format!("{}?recursive=true", dir("")),
        b"",
    )
    .await;
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(names(&list(&app, v).await), ["top.txt"]);
}

#[rstest]
#[tokio::test]
async fn table_files_are_read_only(#[future(awt)] seeded: Router) {
    let app = seeded;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("_delta_log")).unwrap();
    std::fs::write(
        dir.path().join("_delta_log/00000000000000000000.json"),
        "{}",
    )
    .unwrap();
    std::fs::write(dir.path().join("part-0.parquet"), "data").unwrap();
    let table = json!({
        "name": "t", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("file://{}", dir.path().display()),
    });
    let resp = app
        .clone()
        .oneshot(req_json(Method::POST, "/tables", table))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "table").await;

    let t = "/Tables/main/default/t";
    assert_eq!(
        names(&list(&app, t).await),
        ["_delta_log/", "part-0.parquet"]
    );
    let get = call(&app, Method::GET, &format!("/files{t}/part-0.parquet"), b"").await;
    assert_eq!(
        bytes(assert_status(get, StatusCode::OK, "get").await).await,
        b"data"
    );
    for (method, path) in [
        (Method::PUT, format!("/files{t}/new.parquet")),
        (Method::DELETE, format!("/files{t}/part-0.parquet")),
        (Method::PUT, format!("/directories{t}/new")),
        (Method::DELETE, format!("/directories{t}/_delta_log")),
    ] {
        let resp = call(&app, method, &path, b"x").await;
        let body = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert!(
            body["message"].as_str().unwrap().contains("read-only"),
            "{body}"
        );
    }
    assert!(dir.path().join("part-0.parquet").exists());
}

#[rstest]
#[tokio::test]
async fn invalid_paths(#[future(awt)] seeded: Router) {
    let app = seeded;
    for path in [
        "/files/Other/main/default/v/x",
        "/files/Volumes/main/default",
        "/files/Volumes/main/default/v",
        "/files/Volumes/main/default/v/a/../b",
        "/directories/Volumes/main/default/v/./a",
    ] {
        let resp = call(&app, Method::GET, path, b"").await;
        assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    }
    let resp = call(
        &app,
        Method::GET,
        "/directories/Volumes/main/default/nope",
        b"",
    )
    .await;
    assert_error(resp, StatusCode::NOT_FOUND, "NOT_FOUND").await;
}

#[tokio::test]
async fn privileges() {
    let (app, metastore) = common::signed_in_app().await;
    let admin = common::active_user(&metastore, "admin@example.com").await;
    metastore
        .permissions()
        .set_metastore_owner("admin@example.com")
        .await
        .unwrap();
    let bob = common::active_user(&metastore, "bob@example.com").await;
    // A call of the admin that must succeed.
    let admin_call = |method: Method, path: &str, body: Value| {
        let request = common::signed(method, path, Some(body), &admin);
        let app = app.clone();
        async move {
            let resp = app.oneshot(request).await.unwrap();
            assert_status(resp, StatusCode::OK, "setup").await;
        }
    };
    let ok = |path: &str, body: Value| admin_call(Method::POST, path, body);
    // A local storage root, under an external location as sign-in needs.
    let root = common::temp_storage_root();
    let credential = json!({
        "name": "cred", "purpose": "STORAGE",
        "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" },
    });
    ok("/credentials", credential).await;
    let location = json!({ "name": "el", "url": root, "credential_name": "cred" });
    ok("/external-locations", location).await;
    let catalog = json!({ "name": "c", "storage_root": format!("{root}/c") });
    ok("/catalogs", catalog).await;
    ok("/schemas", json!({ "name": "s", "catalog_name": "c" })).await;
    let volume = json!({
        "name": "v", "catalog_name": "c", "schema_name": "s", "volume_type": "MANAGED",
    });
    ok("/volumes", volume).await;
    let grant = |securable: &str, name: &str, privilege: &str| {
        let body = json!({
            "changes": [{ "principal": "bob@example.com", "add": [privilege], "remove": [] }],
        });
        admin_call(
            Method::PATCH,
            &format!("/permissions/{securable}/{name}"),
            body,
        )
    };
    grant("catalog", "c", "USE CATALOG").await;
    grant("schema", "c.s", "USE SCHEMA").await;

    let as_user = |token: &str, method: Method, path: &str, body: &[u8]| {
        let request = fs(method, path, body, Some(token));
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap().status() }
    };
    let file = "/files/Volumes/c/s/v/f.txt";
    let dir = "/directories/Volumes/c/s/v";
    let anonymous = app
        .clone()
        .oneshot(fs(Method::GET, dir, b"", None))
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        as_user(&admin, Method::PUT, file, b"x").await,
        StatusCode::NO_CONTENT
    );
    // Reads need READ VOLUME; writes, ownership.
    assert_eq!(
        as_user(&bob, Method::GET, dir, b"").await,
        StatusCode::FORBIDDEN
    );
    grant("volume", "c.s.v", "READ VOLUME").await;
    assert_eq!(as_user(&bob, Method::GET, dir, b"").await, StatusCode::OK);
    assert_eq!(as_user(&bob, Method::GET, file, b"").await, StatusCode::OK);
    assert_eq!(
        as_user(&bob, Method::PUT, file, b"y").await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        as_user(&bob, Method::DELETE, file, b"").await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        as_user(&admin, Method::DELETE, file, b"").await,
        StatusCode::NO_CONTENT
    );
}
