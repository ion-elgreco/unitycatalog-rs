mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::{fixture, rstest};
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req, req_json};

async fn make_cred(app: &Router, name: &str) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({
                "name": name, "purpose": "STORAGE",
                "aws_iam_role": { "role_arn": format!("arn:aws:iam::1:role/{name}") },
            }),
        ))
        .await
        .unwrap();
}

/// An app with the credential `c1`, which an external location needs.
#[fixture]
async fn with_cred(#[future(awt)] app: Router) -> Router {
    make_cred(&app, "c1").await;
    app
}

#[rstest]
#[tokio::test]
async fn create_get_list_update_delete_external_location(#[future(awt)] with_cred: Router) {
    let resp = with_cred
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "loc1", "url": "s3://bucket/prefix",
                "credential_name": "c1", "comment": "data lake root",
            }),
        ))
        .await
        .unwrap();
    let info = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(info["name"], "loc1");
    assert_eq!(info["url"], "s3://bucket/prefix");
    assert_eq!(info["credential_name"], "c1");

    let resp = with_cred
        .clone()
        .oneshot(req(Method::GET, "/external-locations/loc1"))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "get").await;

    let resp = with_cred
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/external-locations/loc1",
            json!({ "comment": "edited" }),
        ))
        .await
        .unwrap();
    let updated = json_body(assert_status(resp, StatusCode::OK, "patch").await).await;
    assert_eq!(updated["comment"], "edited");

    let resp = with_cred
        .clone()
        .oneshot(req(Method::GET, "/external-locations"))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    assert_eq!(list["external_locations"].as_array().unwrap().len(), 1);

    let resp = with_cred
        .oneshot(req(Method::DELETE, "/external-locations/loc1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn create_with_missing_credential_returns_404(#[future(awt)] app: Router) {
    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "loc1", "url": "s3://b/p", "credential_name": "ghost",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn create_with_overlapping_url_is_rejected(#[future(awt)] app: Router) {
    let _ = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/credentials",
            json!({ "name": "c1", "purpose": "STORAGE",
            "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/c1" } }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({ "name": "outer", "url": "s3://bucket/data", "credential_name": "c1" }),
        ))
        .await
        .unwrap();

    // A URL below `outer` overlaps it.
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "inner",
                "url": "s3://bucket/data/sub",
                "credential_name": "c1",
            }),
        ))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;

    // A URL above `outer` overlaps it too.
    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({
                "name": "parent",
                "url": "s3://bucket",
                "credential_name": "c1",
            }),
        ))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
}

async fn post_location(app: &Router, name: &str, url: &str) -> axum::response::Response {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({ "name": name, "url": url, "credential_name": "c1" }),
        ))
        .await
        .unwrap()
}

/// After upstream `testExternalLocationUrlOverlapPrevention`.
#[rstest]
#[tokio::test]
async fn urls_are_normalized_and_must_not_overlap(#[future(awt)] with_cred: Router) {
    let overlap = "Cannot accept an external location that duplicates or overlaps with existing external location";
    let resp = post_location(&with_cred, "loc", "s3://bucket//root/").await;
    let created = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(created["url"], "s3://bucket/root");
    assert!(created["credential_id"].is_string());

    for (name, url) in [
        ("dup", "s3://bucket/root"),
        ("child", "s3://bucket/root/sub"),
        ("slash", "s3://bucket/root/"),
    ] {
        let err = assert_error(
            post_location(&with_cred, name, url).await,
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
        )
        .await;
        assert_eq!(err["message"], overlap, "{url}");
    }
    // A sibling with a shared name prefix does not overlap.
    assert_status(
        post_location(&with_cred, "sibling", "s3://bucket/root2").await,
        StatusCode::OK,
        "sibling",
    )
    .await;

    // Moving the location below itself is allowed; a parent then overlaps.
    let resp = with_cred
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/external-locations/loc",
            json!({ "url": "s3://bucket/root/deeper/" }),
        ))
        .await
        .unwrap();
    let moved = json_body(assert_status(resp, StatusCode::OK, "move").await).await;
    assert_eq!(moved["url"], "s3://bucket/root/deeper");
    let err = assert_error(
        post_location(&with_cred, "parent", "s3://bucket/root").await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(err["message"], overlap);
    // An update onto another location's path overlaps too.
    let resp = with_cred
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/external-locations/sibling",
            json!({ "url": "s3://bucket/" }),
        ))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;

    let err = assert_error(
        post_location(&with_cred, "bad", "ftp://host/x").await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(err["message"], "Unsupported URI scheme: ftp");
    let err = assert_error(
        post_location(&with_cred, "managed", "s3://other/__unitystorage/x").await,
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(
        err["message"],
        "Input path 's3://other/__unitystorage/x' contains managed storage prefix __unitystorage."
    );
}

#[rstest]
#[tokio::test]
async fn only_storage_credentials_back_locations(#[future(awt)] with_cred: Router) {
    with_cred.clone().oneshot(req_json(Method::POST, "/credentials", json!({
        "name": "sp", "purpose": "SERVICE_PRINCIPAL", "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/sp" },
    }))).await.unwrap();
    let resp = with_cred
        .oneshot(req_json(
            Method::POST,
            "/external-locations",
            json!({ "name": "l", "url": "s3://b/p", "credential_name": "sp" }),
        ))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Credential not of type storage: sp");
}

/// After upstream `testExternalLocationDeletion`.
#[rstest]
#[tokio::test]
async fn locations_in_use_need_force_to_delete(#[future(awt)] with_cred: Router) {
    common::seed_main_default(&with_cred).await;
    let delete = |name: &str, force: bool| {
        let path = format!("/external-locations/{name}?force={force}");
        with_cred.clone().oneshot(req(Method::DELETE, &path))
    };

    assert_status(
        post_location(&with_cred, "vols", "s3://lake/volumes").await,
        StatusCode::OK,
        "vols",
    )
    .await;
    let resp = with_cred
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/volumes",
            json!({
                "name": "v", "catalog_name": "main", "schema_name": "default",
                "volume_type": "EXTERNAL", "storage_location": "s3://lake/volumes/v",
            }),
        ))
        .await
        .unwrap();
    let volume = json_body(assert_status(resp, StatusCode::OK, "volume").await).await;
    let err = assert_error(
        delete("vols", false).await.unwrap(),
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert_eq!(
        err["message"],
        format!(
            "External location still used by volume '{}'",
            volume["volume_id"].as_str().unwrap()
        )
    );
    assert_status(delete("vols", true).await.unwrap(), StatusCode::OK, "force").await;

    assert_status(
        post_location(&with_cred, "tables", "s3://lake/tables").await,
        StatusCode::OK,
        "tables",
    )
    .await;
    let resp = with_cred.clone().oneshot(req_json(Method::POST, "/tables", json!({
        "name": "t", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA", "storage_location": "s3://lake/tables/t",
    }))).await.unwrap();
    assert_status(resp, StatusCode::OK, "table").await;
    let err = assert_error(
        delete("tables", false).await.unwrap(),
        StatusCode::BAD_REQUEST,
        "INVALID_ARGUMENT",
    )
    .await;
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .starts_with("External location still used by table '")
    );

    assert_status(
        post_location(&with_cred, "empty", "s3://lake/empty").await,
        StatusCode::OK,
        "empty",
    )
    .await;
    assert_status(
        delete("empty", false).await.unwrap(),
        StatusCode::OK,
        "empty delete",
    )
    .await;

    // Files of a dropped managed volume wait for cleanup under the location.
    assert_status(
        post_location(&with_cred, "managed", "s3://lake/managed").await,
        StatusCode::OK,
        "managed",
    )
    .await;
    for (path, body) in [
        (
            "/catalogs",
            json!({ "name": "mc", "storage_root": "s3://lake/managed" }),
        ),
        ("/schemas", json!({ "name": "s", "catalog_name": "mc" })),
        (
            "/volumes",
            json!({ "name": "mv", "catalog_name": "mc", "schema_name": "s", "volume_type": "MANAGED" }),
        ),
    ] {
        let resp = with_cred
            .clone()
            .oneshot(req_json(Method::POST, path, body))
            .await
            .unwrap();
        assert_status(resp, StatusCode::OK, path).await;
    }
    let resp = with_cred
        .clone()
        .oneshot(req(Method::DELETE, "/volumes/mc.s.mv"))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "drop volume").await;
    let err = assert_error(
        delete("managed", false).await.unwrap(),
        StatusCode::FORBIDDEN,
        "PERMISSION_DENIED",
    )
    .await;
    assert_eq!(
        err["message"],
        "Input path overlaps pending storage cleanup."
    );
}
