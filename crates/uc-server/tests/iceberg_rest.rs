//! The Iceberg REST catalog, after upstream `IcebergRestCatalogServiceTest`.

mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{AppState, build_app};

use common::{
    BASE, assert_iceberg_error, assert_status, json_body, req, req_bearer, req_json,
    seed_main_default,
};

const READ_ENDPOINTS: [&str; 9] = [
    "GET /v1/{prefix}/namespaces",
    "GET /v1/{prefix}/namespaces/{namespace}",
    "HEAD /v1/{prefix}/namespaces/{namespace}",
    "HEAD /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "GET /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "GET /v1/{prefix}/namespaces/{namespace}/tables/{table}/credentials",
    "GET /v1/{prefix}/namespaces/{namespace}/views/{view}",
    "POST /v1/{prefix}/namespaces/{namespace}/tables/{table}/metrics",
    "GET /v1/{prefix}/namespaces/{namespace}/tables",
];

const WRITE_ENDPOINTS: [&str; 7] = [
    "POST /v1/{prefix}/namespaces",
    "DELETE /v1/{prefix}/namespaces/{namespace}",
    "POST /v1/{prefix}/namespaces/{namespace}/properties",
    "POST /v1/{prefix}/namespaces/{namespace}/tables",
    "POST /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "DELETE /v1/{prefix}/namespaces/{namespace}/tables/{table}",
    "POST /v1/{prefix}/tables/rename",
];

/// An app with `main.default`, and Iceberg writes on or off.
async fn app(writes: bool) -> Router {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let app = build_app(AppState::new(metastore).with_iceberg_writes_enabled(writes));
    seed_main_default(&app).await;
    app
}

async fn get(app: &Router, path: &str) -> axum::response::Response {
    send(app, Method::GET, path, None).await
}

/// A request to `path` under the Iceberg mount.
async fn send(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let path = format!("/iceberg{path}");
    let request = match body {
        Some(body) => req_json(method, &path, body),
        None => req(method, &path),
    };
    app.clone().oneshot(request).await.unwrap()
}

/// A request to the UC API, which must succeed.
async fn uc(app: &Router, method: Method, path: &str, body: Value) {
    let resp = app
        .clone()
        .oneshot(req_json(method, path, body))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, path).await;
}

async fn ok(resp: axum::response::Response) -> Value {
    json_body(assert_status(resp, StatusCode::OK, "iceberg").await).await
}

const NS: &str = "/v1/catalogs/main/namespaces";

#[tokio::test]
async fn config_gives_the_catalog_prefix_and_endpoints() {
    let app = app(true).await;
    let body = json_body(
        assert_status(
            get(&app, "/v1/config?warehouse=main").await,
            StatusCode::OK,
            "config",
        )
        .await,
    )
    .await;
    let endpoints: Vec<&str> = READ_ENDPOINTS
        .iter()
        .chain(WRITE_ENDPOINTS.iter())
        .copied()
        .collect();
    assert_eq!(
        body,
        json!({ "defaults": {}, "overrides": { "prefix": "catalogs/main" }, "endpoints": endpoints })
    );

    // Without writes, only the read endpoints are advertised.
    let app = self::app(false).await;
    let body = json_body(
        assert_status(
            get(&app, "/v1/config?warehouse=main").await,
            StatusCode::OK,
            "config",
        )
        .await,
    )
    .await;
    assert_eq!(body["endpoints"], json!(READ_ENDPOINTS));
}

#[tokio::test]
async fn config_needs_an_existing_catalog() {
    let app = app(true).await;
    let err = assert_iceberg_error(
        get(&app, "/v1/config").await,
        StatusCode::BAD_REQUEST,
        "BadRequestException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Must supply a proper catalog in warehouse property."
    );
    assert_iceberg_error(
        get(&app, "/v1/config?warehouse=nope").await,
        StatusCode::NOT_FOUND,
        "NoSuchNamespaceException",
    )
    .await;
}

#[tokio::test]
async fn errors_outside_the_handlers_have_the_iceberg_body() {
    let app = app(true).await;
    // Request parsing (a UC error body).
    let resp = get(&app, "/v1/config?warehouse=a&warehouse=b").await;
    assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    // Sign-in.
    let (signed_in, _) = common::signed_in_app().await;
    let resp = signed_in
        .clone()
        .oneshot(req_bearer(
            Method::GET,
            "/iceberg/v1/config?warehouse=main",
            "bogus",
        ))
        .await
        .unwrap();
    let err = assert_iceberg_error(resp, StatusCode::UNAUTHORIZED, "NotAuthorizedException").await;
    assert_eq!(err["message"], "Invalid access token.");
    // Unrouted paths and methods.
    let err = assert_iceberg_error(
        get(&app, "/v1/unknown").await,
        StatusCode::NOT_FOUND,
        "NotFoundException",
    )
    .await;
    assert_eq!(err["message"], "Not Found");
    let resp = app
        .clone()
        .oneshot(req(Method::PUT, "/iceberg/v1/config"))
        .await
        .unwrap();
    let err: Value =
        assert_iceberg_error(resp, StatusCode::METHOD_NOT_ALLOWED, "RESTException").await;
    assert_eq!(err["message"], "Method Not Allowed");
    // Other APIs keep the UC body.
    let resp = app
        .clone()
        .oneshot(req(Method::GET, "/catalogs/nope"))
        .await
        .unwrap();
    common::assert_error(resp, StatusCode::NOT_FOUND, "CATALOG_NOT_FOUND").await;
}

#[tokio::test]
async fn namespace_exists_answers_204() {
    let app = app(true).await;
    let resp = send(&app, Method::HEAD, &format!("{NS}/default"), None).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(json_body_bytes(resp).await.is_empty());
    let resp = send(&app, Method::HEAD, &format!("{NS}/nope"), None).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

async fn json_body_bytes(resp: axum::response::Response) -> Vec<u8> {
    use http_body_util::BodyExt;
    resp.into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

#[tokio::test]
async fn namespaces_are_the_catalog_schemas() {
    let app = app(true).await;
    uc(
        &app,
        Method::POST,
        "/schemas",
        json!({ "name": "props", "catalog_name": "main",
        "properties": { "a": "1" } }),
    )
    .await;
    let body = ok(get(&app, &format!("{NS}/props")).await).await;
    assert_eq!(
        body,
        json!({ "namespace": ["props"], "properties": { "a": "1" } })
    );
    assert_iceberg_error(
        get(&app, &format!("{NS}/nope")).await,
        StatusCode::NOT_FOUND,
        "NoSuchNamespaceException",
    )
    .await;
    // A multi-level name is a bad request rather than a missing namespace.
    assert_iceberg_error(
        get(&app, &format!("{NS}/nested.namespace")).await,
        StatusCode::BAD_REQUEST,
        "BadRequestException",
    )
    .await;

    let body = ok(get(&app, NS).await).await;
    assert_eq!(body, json!({ "namespaces": [["default"], ["props"]] }));
    assert_iceberg_error(
        get(&app, "/v1/catalogs/nope/namespaces").await,
        StatusCode::NOT_FOUND,
        "NoSuchNamespaceException",
    )
    .await;
}

#[tokio::test]
async fn namespaces_under_a_parent() {
    let app = app(true).await;
    assert_eq!(
        ok(get(&app, &format!("{NS}?parent=default")).await).await,
        json!({ "namespaces": [] })
    );
    let resp = get(&app, &format!("{NS}?parent=nope")).await;
    assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchNamespaceException").await;
    // An empty parent is no parent.
    assert_eq!(
        ok(get(&app, &format!("{NS}?parent=")).await).await,
        json!({ "namespaces": [["default"]] })
    );
    let resp = get(&app, "/v1/catalogs/nope/namespaces?parent=default").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn namespace_listing_pages() {
    let app = app(true).await;
    uc(&app, Method::POST, "/catalogs", json!({ "name": "many" })).await;
    let created: Vec<String> = (0..=100).map(|i| format!("schema_{i:03}")).collect();
    for name in &created {
        uc(
            &app,
            Method::POST,
            "/schemas",
            json!({ "name": name, "catalog_name": "many" }),
        )
        .await;
    }
    let path = "/v1/catalogs/many/namespaces";
    let names = |body: &Value| -> Vec<String> {
        body["namespaces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n[0].as_str().unwrap().to_string())
            .collect()
    };
    // Without pageToken the answer is the whole listing, without a token.
    let body = ok(get(&app, path).await).await;
    assert_eq!(names(&body), created);
    assert!(body.get("next-page-token").is_none());
    assert_eq!(
        names(&ok(get(&app, &format!("{path}?pageSize=10")).await).await).len(),
        created.len()
    );

    // An empty token opens a paged listing.
    let (mut paged, mut token, mut pages) = (Vec::new(), String::new(), 0);
    loop {
        let body = ok(get(&app, &format!("{path}?pageToken={token}")).await).await;
        paged.extend(names(&body));
        pages += 1;
        match body["next-page-token"].as_str() {
            Some(next) => token = next.to_string(),
            None => break,
        }
    }
    assert_eq!(pages, 2);
    assert_eq!(paged, created);
    assert_eq!(
        names(&ok(get(&app, &format!("{path}?pageToken=&pageSize=10")).await).await).len(),
        10
    );
    for size in ["0", "-1", "ten"] {
        let resp = get(&app, &format!("{path}?pageToken=&pageSize={size}")).await;
        let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
        assert_eq!(
            err["message"],
            format!("Invalid pageSize: {size}. It must be a whole number of at least 1.")
        );
    }
}

#[tokio::test]
async fn create_namespace() {
    let app = app(true).await;
    let resp = send(
        &app,
        Method::POST,
        NS,
        Some(json!({ "namespace": ["ns"], "properties": { "k": "v" } })),
    )
    .await;
    assert_eq!(
        ok(resp).await,
        json!({ "namespace": ["ns"], "properties": { "k": "v" } })
    );
    assert_eq!(
        ok(get(&app, &format!("{NS}/ns")).await).await["properties"],
        json!({ "k": "v" })
    );
    let resp = send(
        &app,
        Method::POST,
        NS,
        Some(json!({ "namespace": ["bare"] })),
    )
    .await;
    assert_eq!(
        ok(resp).await,
        json!({ "namespace": ["bare"], "properties": {} })
    );

    let resp = send(&app, Method::POST, NS, Some(json!({ "namespace": ["ns"] }))).await;
    assert_iceberg_error(resp, StatusCode::CONFLICT, "AlreadyExistsException").await;
    let resp = send(
        &app,
        Method::POST,
        NS,
        Some(json!({ "namespace": ["a", "b"] })),
    )
    .await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert_eq!(err["message"], "Nested namespaces are not supported: a.b");
    let resp = send(&app, Method::POST, NS, Some(json!({}))).await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert_eq!(err["message"], "Invalid namespace: null");
    let resp = send(
        &app,
        Method::POST,
        "/v1/catalogs/nope/namespaces",
        Some(json!({ "namespace": ["x"] })),
    )
    .await;
    assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchNamespaceException").await;
    // An unreadable body does not quote the parser.
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(Method::POST)
                .uri(format!("{BASE}/iceberg{NS}"))
                .header("content-type", "application/json")
                .body(axum::body::Body::from("{not json"))
                .unwrap(),
        )
        .await
        .unwrap();
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert_eq!(err["message"], "Malformed request body: not valid JSON");
}

#[tokio::test]
async fn drop_namespace() {
    let app = app(true).await;
    uc(&app, Method::POST, "/tables", json!({ "name": "t", "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA", "storage_location": "s3://lake/t" })).await;
    let path = format!("{NS}/default");
    let err = assert_iceberg_error(
        send(&app, Method::DELETE, &path, None).await,
        StatusCode::CONFLICT,
        "NamespaceNotEmptyException",
    )
    .await;
    assert_eq!(err["message"], "Namespace is not empty: default");

    let resp = app
        .clone()
        .oneshot(req(Method::DELETE, "/tables/main.default.t"))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "delete table").await;
    let resp = send(&app, Method::DELETE, &path, None).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(json_body_bytes(resp).await.is_empty());
    assert_eq!(get(&app, &path).await.status(), StatusCode::NOT_FOUND);
    assert_iceberg_error(
        send(&app, Method::DELETE, &path, None).await,
        StatusCode::NOT_FOUND,
        "NoSuchNamespaceException",
    )
    .await;
}

#[tokio::test]
async fn update_namespace_properties() {
    let app = app(true).await;
    uc(
        &app,
        Method::POST,
        "/schemas",
        json!({ "name": "ns", "catalog_name": "main",
        "properties": { "keep": "me", "drop": "this" } }),
    )
    .await;
    let path = format!("{NS}/ns/properties");
    let resp = send(
        &app,
        Method::POST,
        &path,
        Some(json!({ "removals": ["drop", "absent"], "updates": { "added": "value" } })),
    )
    .await;
    assert_eq!(
        ok(resp).await,
        json!({ "updated": ["added"], "removed": ["drop"], "missing": ["absent"] })
    );
    assert_eq!(
        ok(get(&app, &format!("{NS}/ns")).await).await["properties"],
        json!({ "keep": "me", "added": "value" })
    );

    // Removing every key leaves none.
    let resp = send(
        &app,
        Method::POST,
        &path,
        Some(json!({ "removals": ["keep", "added"], "updates": {} })),
    )
    .await;
    let body = ok(resp).await;
    assert_eq!(body["removed"], json!(["keep", "added"]));
    assert_eq!(body["updated"], json!([]));
    assert_eq!(
        ok(get(&app, &format!("{NS}/ns")).await).await["properties"],
        json!({})
    );

    let resp = send(
        &app,
        Method::POST,
        &path,
        Some(json!({ "removals": ["both"], "updates": { "both": "value" } })),
    )
    .await;
    let err = assert_iceberg_error(
        resp,
        StatusCode::UNPROCESSABLE_ENTITY,
        "UnprocessableEntityException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Invalid namespace properties update: cannot simultaneously set and remove keys: [both]"
    );
    let resp = send(
        &app,
        Method::POST,
        &format!("{NS}/nope/properties"),
        Some(json!({ "removals": [], "updates": { "a": "b" } })),
    )
    .await;
    assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchNamespaceException").await;
}

#[tokio::test]
async fn namespace_writes_can_be_disabled() {
    let app = app(false).await;
    let disabled = "Iceberg table writes are currently disabled. To enable them, set tables.iceberg_writes = true (UC_TABLES_ICEBERG_WRITES)";
    for (method, path, body) in [
        (
            Method::POST,
            NS.to_string(),
            Some(json!({ "namespace": ["ns"] })),
        ),
        (Method::DELETE, format!("{NS}/default"), None),
        (
            Method::POST,
            format!("{NS}/default/properties"),
            Some(json!({ "updates": { "a": "b" } })),
        ),
    ] {
        let resp = send(&app, method, &path, body).await;
        let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
        assert_eq!(err["message"], disabled, "{path}");
    }
    // Reads still work.
    ok(get(&app, &format!("{NS}/default")).await).await;
}

#[tokio::test]
async fn only_the_upstream_routes_are_served() {
    let app = app(true).await;
    // Non-prefixed routes, register and view writes are not part of the catalog.
    for (method, path) in [
        (Method::GET, "/v1/namespaces"),
        (Method::GET, "/v1/namespaces/main.default"),
        (Method::GET, "/v1/namespaces/main.default/tables"),
        (Method::POST, "/v1/namespaces/main.default/register"),
        (Method::POST, "/v1/tables/rename"),
        (Method::GET, "/v1/namespaces/main.default/views"),
        (Method::POST, "/v1/views/rename"),
        (
            Method::POST,
            "/v1/catalogs/main/namespaces/default/register",
        ),
        (Method::GET, "/v1/catalogs/main/namespaces/default/views"),
    ] {
        let resp = send(&app, method.clone(), path, Some(json!({}))).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{method} {path}");
        assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NotFoundException").await;
    }
    // A view is never found.
    let resp = get(&app, &format!("{NS}/default/views/v")).await;
    let err = assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchViewException").await;
    assert_eq!(err["message"], "View does not exist: default.v");
}

/// The Delta API request that creates the MANAGED table of `staging` with
/// UniForm Iceberg metadata at `metadata_location`.
fn uniform_request(name: &str, staging: &Value, metadata_location: &str) -> Value {
    json!({
        "name": name, "location": staging["location"], "table-type": "MANAGED",
        "columns": { "type": "struct", "fields": [
            { "name": "x", "type": "integer", "nullable": true, "metadata": {} },
            { "name": "y", "type": "string", "nullable": true, "metadata": {} },
        ] },
        "protocol": {
            "min-reader-version": 3, "min-writer-version": 7,
            "reader-features": ["catalogManaged", "deletionVectors", "v2Checkpoint", "vacuumProtocolCheck"],
            "writer-features": ["catalogManaged", "deletionVectors", "v2Checkpoint", "vacuumProtocolCheck",
                "inCommitTimestamp"],
        },
        "properties": {
            "delta.enableDeletionVectors": "true", "delta.checkpointPolicy": "v2",
            "delta.enableInCommitTimestamps": "true", "delta.checkpoint.writeStatsAsStruct": "true",
            "delta.checkpoint.writeStatsAsJson": "true", "io.unitycatalog.tableId": staging["table-id"],
            "delta.universalFormat.enabledFormats": "iceberg",
        },
        "last-commit-timestamp-ms": 1_717_798_710_940i64,
        "uniform": { "iceberg": { "metadata-location": metadata_location, "converted-delta-version": 1,
            "converted-delta-timestamp": 1_717_798_710_940i64 } },
    })
}

/// A UniForm Delta table `name` in `main.default` whose Iceberg metadata is
/// the upstream `fixture`, its table root moved to the table's location.
/// Returns the table location and the metadata file.
async fn uniform_table(app: &Router, name: &str, fixture: &str) -> (String, std::path::PathBuf) {
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/delta/v1/catalogs/main/schemas/default/staging-tables",
            json!({ "name": name }),
        ))
        .await
        .unwrap();
    let staging = ok(resp).await;
    let location = staging["location"].as_str().unwrap().to_string();
    let root = std::path::PathBuf::from(location.strip_prefix("file://").unwrap());
    let file = root.join("metadata/v1.metadata.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let fixture = std::fs::read_to_string(format!(
        "{}/tests/fixtures/{fixture}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    std::fs::write(
        &file,
        fixture.replace("file:/tmp/uniform_iceberg_table", &location),
    )
    .unwrap();
    let request = uniform_request(
        name,
        &staging,
        &format!("{location}/metadata/v1.metadata.json"),
    );
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/delta/v1/catalogs/main/schemas/default/tables",
            request,
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "uniform table").await;
    (location, file)
}

/// A Delta table without Iceberg metadata.
async fn plain_table(app: &Router, name: &str) {
    uc(
        app,
        Method::POST,
        "/tables",
        json!({ "name": name, "catalog_name": "main", "schema_name": "default",
        "table_type": "EXTERNAL", "data_source_format": "DELTA",
        "storage_location": format!("s3://lake/{name}") }),
    )
    .await;
}

const TABLES: &str = "/v1/catalogs/main/namespaces/default/tables";

#[tokio::test]
async fn tables_with_iceberg_metadata_are_served() {
    let app = app(true).await;
    plain_table(&app, "plain").await;
    assert_eq!(
        send(&app, Method::HEAD, &format!("{TABLES}/plain"), None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let err = assert_iceberg_error(
        get(&app, &format!("{TABLES}/plain")).await,
        StatusCode::NOT_FOUND,
        "NoSuchTableException",
    )
    .await;
    assert_eq!(err["message"], "Table does not exist: default.plain");

    let (location, file) = uniform_table(&app, "uni", "iceberg.metadata.json").await;
    let resp = send(&app, Method::HEAD, &format!("{TABLES}/uni"), None).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(json_body_bytes(resp).await.is_empty());

    let body = ok(get(&app, &format!("{TABLES}/uni")).await).await;
    // A local file is named by its path, as Iceberg's local file IO does.
    assert_eq!(body["metadata-location"], file.to_str().unwrap());
    assert_eq!(body["metadata"]["location"], location);
    assert_eq!(
        body["metadata"]["table-uuid"],
        "55d4dc69-5b14-4483-bfc8-f33b80f99f99"
    );
    assert_eq!(
        body["metadata"]["current-snapshot-id"],
        6116814959227549927i64
    );
    assert_eq!(body["config"], json!({}));

    let body = ok(get(&app, TABLES).await).await;
    assert_eq!(
        body,
        json!({ "identifiers": [{ "namespace": ["default"], "name": "uni" }] })
    );

    let resp = get(&app, &format!("{TABLES}/nope")).await;
    assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
    let resp = get(&app, "/v1/catalogs/main/namespaces/nope/tables/uni").await;
    assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchNamespaceException").await;
    let resp = get(&app, "/v1/catalogs/main/namespaces/nope/tables").await;
    assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchNamespaceException").await;
}

#[tokio::test]
async fn metadata_must_be_readable_and_inside_the_table() {
    let app = app(true).await;
    // Metadata that records another table root is refused rather than served.
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/delta/v1/catalogs/main/schemas/default/staging-tables",
            json!({ "name": "moved" }),
        ))
        .await
        .unwrap();
    let staging = ok(resp).await;
    let location = staging["location"].as_str().unwrap().to_string();
    let file = std::path::PathBuf::from(location.strip_prefix("file://").unwrap())
        .join("metadata/v1.metadata.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::copy(
        format!(
            "{}/tests/fixtures/iceberg.metadata.json",
            env!("CARGO_MANIFEST_DIR")
        ),
        &file,
    )
    .unwrap();
    let request = uniform_request(
        "moved",
        &staging,
        &format!("{location}/metadata/v1.metadata.json"),
    );
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/delta/v1/catalogs/main/schemas/default/tables",
            request,
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "uniform table").await;
    let err = assert_iceberg_error(
        get(&app, &format!("{TABLES}/moved")).await,
        StatusCode::BAD_REQUEST,
        "BadRequestException",
    )
    .await;
    assert_eq!(
        err["message"],
        format!(
            "Iceberg table location ('file:///tmp/uniform_iceberg_table') must match the persisted table location ('{location}')."
        )
    );

    // A metadata file that is gone is a server failure that does not say where.
    let (_, file) = uniform_table(&app, "gone", "iceberg.metadata.json").await;
    std::fs::remove_file(&file).unwrap();
    let err = assert_iceberg_error(
        get(&app, &format!("{TABLES}/gone")).await,
        StatusCode::INTERNAL_SERVER_ERROR,
        "ServiceFailureException",
    )
    .await;
    assert_eq!(err["message"], "Could not read this table");
}

#[tokio::test]
async fn table_listing_pages() {
    let app = app(true).await;
    plain_table(&app, "a_plain").await;
    for name in ["b_uni", "c_uni", "d_uni"] {
        uniform_table(&app, name, "iceberg.metadata.json").await;
    }
    plain_table(&app, "e_plain").await;
    let names = |body: &Value| -> Vec<String> {
        body["identifiers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        names(&ok(get(&app, TABLES).await).await),
        ["b_uni", "c_uni", "d_uni"]
    );

    // A page of one at a time; a full page always has a token, so the last page is empty.
    let (mut paged, mut token, mut pages) = (Vec::new(), String::new(), 0);
    loop {
        let body = ok(get(&app, &format!("{TABLES}?pageToken={token}&pageSize=1")).await).await;
        paged.extend(names(&body));
        pages += 1;
        match body["next-page-token"].as_str() {
            Some(next) => token = next.to_string(),
            None => break,
        }
    }
    assert_eq!(paged, ["b_uni", "c_uni", "d_uni"]);
    assert_eq!(pages, 4);
}

#[tokio::test]
async fn metrics_reports() {
    let app = app(true).await;
    uniform_table(&app, "uni", "iceberg.metadata.json").await;
    plain_table(&app, "plain").await;
    let scan = json!({ "report-type": "scan-report", "table-name": "uni", "snapshot-id": 23, "filter": true,
        "schema-id": 0, "projected-field-ids": [1], "projected-field-names": ["x"], "metrics": {} });
    let commit = json!({ "report-type": "commit-report", "table-name": "uni", "snapshot-id": 23,
        "sequence-number": 4, "operation": "append", "metrics": {} });
    for report in [scan.clone(), commit] {
        let resp = send(
            &app,
            Method::POST,
            &format!("{TABLES}/uni/metrics"),
            Some(report),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
    let resp = send(
        &app,
        Method::POST,
        &format!("{TABLES}/uni/metrics"),
        Some(json!({ "foo": "bar" })),
    )
    .await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert_eq!(err["message"], "Cannot parse missing string: report-type");
    for table in ["plain", "missing"] {
        let resp = send(
            &app,
            Method::POST,
            &format!("{TABLES}/{table}/metrics"),
            Some(scan.clone()),
        )
        .await;
        assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
    }

    // Unreadable bodies do not quote the parser.
    let raw = |path: String, body: &'static str| {
        let app = app.clone();
        async move {
            app.oneshot(
                axum::http::Request::builder()
                    .method(Method::POST)
                    .uri(format!("{BASE}/iceberg{path}"))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
        }
    };
    for (path, body, message) in [
        (
            format!("{TABLES}/uni/metrics"),
            "",
            "Malformed request body: no content",
        ),
        (
            format!("{TABLES}/uni/metrics"),
            "{",
            "Malformed request body: not valid JSON",
        ),
        (
            NS.to_string(),
            r#"{"namespace": ["x"], "properties": 5}"#,
            "Malformed request body: not the structure this endpoint accepts",
        ),
    ] {
        let err = assert_iceberg_error(
            raw(path, body).await,
            StatusCode::BAD_REQUEST,
            "BadRequestException",
        )
        .await;
        assert_eq!(err["message"], message);
    }
}

#[tokio::test]
async fn the_snapshots_parameter() {
    let app = app(true).await;
    uniform_table(&app, "uni", "iceberg.metadata.two-snapshots.json").await;
    let snapshot_ids = |body: &Value| -> Vec<i64> {
        body["metadata"]["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["snapshot-id"].as_i64().unwrap())
            .collect()
    };
    for query in ["", "?snapshots=all", "?snapshots=ALL"] {
        assert_eq!(
            snapshot_ids(&ok(get(&app, &format!("{TABLES}/uni{query}")).await).await).len(),
            2,
            "{query}"
        );
    }
    // Only what the refs point at; the rest of the metadata is the same table.
    let all = ok(get(&app, &format!("{TABLES}/uni")).await).await;
    let refs = ok(get(&app, &format!("{TABLES}/uni?snapshots=refs")).await).await;
    assert_eq!(snapshot_ids(&refs), [6116814959227549927]);
    assert_eq!(refs["metadata-location"], all["metadata-location"]);
    assert_eq!(
        refs["metadata"]["snapshot-log"],
        all["metadata"]["snapshot-log"]
    );
    let err = assert_iceberg_error(
        get(&app, &format!("{TABLES}/uni?snapshots=some")).await,
        StatusCode::BAD_REQUEST,
        "BadRequestException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Invalid snapshots parameter: some. Valid values are all and refs."
    );
}

#[tokio::test]
async fn conditional_loads() {
    let app = app(true).await;
    uniform_table(&app, "uni", "iceberg.metadata.two-snapshots.json").await;
    uniform_table(&app, "other", "iceberg.metadata.json").await;
    let path = format!("/iceberg{TABLES}/uni");
    let load = |path: String, tag: Option<&'static str>| {
        let app = app.clone();
        let tag = tag.map(str::to_string);
        async move {
            let mut request = axum::http::Request::builder()
                .method(Method::GET)
                .uri(format!("{BASE}{path}"));
            if let Some(tag) = tag {
                request = request.header("if-none-match", tag);
            }
            app.oneshot(request.body(axum::body::Body::empty()).unwrap())
                .await
                .unwrap()
        }
    };
    let etag_of =
        |resp: &axum::response::Response| resp.headers()["etag"].to_str().unwrap().to_string();

    let resp = load(path.clone(), None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag = etag_of(&resp);
    // Weak, and it does not hand out the metadata location.
    assert!(etag.starts_with("W/\"") && etag.ends_with('"'), "{etag}");
    assert!(!etag.contains("metadata"), "{etag}");
    let etag: &'static str = Box::leak(etag.into_boxed_str());

    let resp = load(path.clone(), Some(etag)).await;
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_of(&resp), etag);
    assert!(json_body_bytes(resp).await.is_empty());
    let listed: &'static str = Box::leak(format!("W/\"0\", {etag}").into_boxed_str());
    assert_eq!(
        load(path.clone(), Some(listed)).await.status(),
        StatusCode::NOT_MODIFIED
    );
    for other in ["W/\"0\"", "*"] {
        assert_eq!(
            load(path.clone(), Some(other)).await.status(),
            StatusCode::OK,
            "{other}"
        );
    }

    // `refs` has its own tag.
    let refs_path = format!("{path}?snapshots=refs");
    let refs_etag = etag_of(&load(refs_path.clone(), None).await);
    assert_ne!(refs_etag, etag);
    assert_eq!(
        load(refs_path.clone(), Some(etag)).await.status(),
        StatusCode::OK
    );
    let refs_etag: &'static str = Box::leak(refs_etag.into_boxed_str());
    assert_eq!(
        load(refs_path, Some(refs_etag)).await.status(),
        StatusCode::NOT_MODIFIED
    );
    // A bad mode is refused, not answered from the caller's copy.
    assert_eq!(
        load(format!("{path}?snapshots=some"), Some(etag))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    // Another metadata version has another tag.
    let other = etag_of(&load(format!("/iceberg{TABLES}/other"), None).await);
    assert_ne!(other, etag);
}

#[tokio::test]
async fn the_etag_header_is_spelled_etag_on_the_wire() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let app = app(true).await;
    uniform_table(&app, "uni", "iceberg.metadata.json").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap()
    });
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    let request = format!(
        "GET {BASE}/iceberg{TABLES}/uni HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    );
    socket.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    let head = &response[..response.find("\r\n\r\n").unwrap()];
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(head.contains("\r\nETag: W/\""), "{head}");
}

/// A vendor whose session token names the operation it vended for.
struct OpVendor;

#[async_trait::async_trait]
impl uc_server::storage::vendor::CredentialVendor for OpVendor {
    async fn vend(
        &self,
        ctx: &uc_server::storage::vendor::CredentialContext,
    ) -> uc_common::Result<uc_api::TemporaryCredentials> {
        let op = ctx.op();
        Ok(uc_api::TemporaryCredentials {
            aws_temp_credentials: Some(uc_api::AwsCredentials {
                access_key_id: Some("ak".into()),
                secret_access_key: Some("sk".into()),
                session_token: Some(format!("{op:?}")),
            }),
            expiration_time: Some(1_900_000_000_000),
            ..Default::default()
        })
    }
}

#[tokio::test]
async fn vended_credentials() {
    let app = app(true).await;
    uniform_table(&app, "uni", "iceberg.metadata.json").await;
    plain_table(&app, "plain").await;
    // A local table vends nothing.
    assert_eq!(
        ok(get(&app, &format!("{TABLES}/uni")).await).await["config"],
        json!({})
    );
    assert_eq!(
        ok(get(&app, &format!("{TABLES}/uni/credentials")).await).await,
        json!({ "storage-credentials": [] })
    );
    for table in ["plain", "nope"] {
        let resp = get(&app, &format!("{TABLES}/{table}/credentials")).await;
        assert_iceberg_error(resp, StatusCode::NOT_FOUND, "NoSuchTableException").await;
    }

    // A cloud table vends for its persisted location; UniForm tables are read-only.
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let bucket = uc_common::NormalizedUrl::parse("s3://bucket").unwrap();
    let state = AppState::new(metastore)
        .with_vendor(OpVendor)
        .with_s3_region(&bucket, "us-west-2");
    let app = build_app(state);
    for (path, body) in [
        (
            "/credentials",
            json!({ "name": "c", "purpose": "STORAGE", "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } }),
        ),
        (
            "/external-locations",
            json!({ "name": "loc", "url": "s3://bucket/prefix", "credential_name": "c" }),
        ),
        (
            "/catalogs",
            json!({ "name": "cloud", "storage_root": "s3://bucket/prefix/root" }),
        ),
        ("/schemas", json!({ "name": "s", "catalog_name": "cloud" })),
    ] {
        uc(&app, Method::POST, path, body).await;
    }
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/delta/v1/catalogs/cloud/schemas/s/staging-tables",
            json!({ "name": "uni" }),
        ))
        .await
        .unwrap();
    let staging = ok(resp).await;
    let location = staging["location"].as_str().unwrap().to_string();
    let request = uniform_request(
        "uni",
        &staging,
        &format!("{location}/metadata/v1.metadata.json"),
    );
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/delta/v1/catalogs/cloud/schemas/s/tables",
            request,
        ))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "uniform table").await;
    let body = ok(get(
        &app,
        "/v1/catalogs/cloud/namespaces/s/tables/uni/credentials",
    )
    .await)
    .await;
    assert_eq!(
        body,
        json!({ "storage-credentials": [{ "prefix": location, "config": {
        "s3.access-key-id": "ak", "s3.secret-access-key": "sk", "s3.session-token": "Read",
        "client.region": "us-west-2", "s3.session-token-expires-at-ms": "1900000000000",
        "client.refresh-credentials-endpoint": "v1/catalogs/cloud/namespaces/s/tables/uni/credentials",
    } }] })
    );
}

/// An app with `main.default` whose local external tables may live under a
/// fresh root, which it returns as a `file://` URL.
async fn writable_app() -> (Router, String) {
    let root = tempfile::Builder::new()
        .prefix("uc-iceberg-")
        .tempdir()
        .unwrap()
        .keep();
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let state = AppState::new(metastore).with_external_local_roots(vec![root.clone()]);
    let app = build_app(state);
    seed_main_default(&app).await;
    (app, format!("file://{}", root.display()))
}

fn id_data_schema() -> Value {
    json!({ "type": "struct", "schema-id": 0, "fields": [
        { "id": 1, "name": "id", "required": true, "type": "long" },
        { "id": 2, "name": "data", "required": false, "type": "string" },
    ] })
}

async fn create(app: &Router, body: Value) -> axum::response::Response {
    send(app, Method::POST, TABLES, Some(body)).await
}

async fn commit(
    app: &Router,
    table: &str,
    requirements: Value,
    updates: Value,
) -> axum::response::Response {
    send(
        app,
        Method::POST,
        &format!("{TABLES}/{table}"),
        Some(json!({ "requirements": requirements, "updates": updates })),
    )
    .await
}

async fn uc_table(app: &Router, name: &str) -> Value {
    let resp = app
        .clone()
        .oneshot(req(Method::GET, &format!("/tables/main.default.{name}")))
        .await
        .unwrap();
    ok(resp).await
}

#[tokio::test]
async fn iceberg_table_write_lifecycle() {
    let (app, root) = writable_app().await;
    let location = format!("{root}/events");

    // A staged create registers nothing and writes no metadata.
    let staged = ok(create(
        &app,
        json!({ "name": "events", "schema": id_data_schema(),
        "location": location, "stage-create": true }),
    )
    .await)
    .await;
    assert!(staged.get("metadata-location").is_none(), "{staged}");
    assert_eq!(staged["metadata"]["location"], location);
    assert_eq!(
        get(&app, &format!("{TABLES}/events")).await.status(),
        StatusCode::NOT_FOUND
    );

    let created = ok(create(
        &app,
        json!({ "name": "events", "schema": id_data_schema(),
        "location": location, "properties": { "created-by": "iceberg-rest-test" } }),
    )
    .await)
    .await;
    let first = created["metadata-location"].as_str().unwrap().to_string();
    assert!(
        first.contains("/metadata/00000-") && std::path::Path::new(&first).is_file(),
        "{first}"
    );
    assert_eq!(
        created["metadata"]["properties"]["created-by"],
        "iceberg-rest-test"
    );
    assert_eq!(
        created["metadata"]["properties"]["write.parquet.compression-codec"],
        "zstd"
    );
    let err = assert_iceberg_error(
        create(
            &app,
            json!({ "name": "events", "schema": id_data_schema(),
        "location": location }),
        )
        .await,
        StatusCode::CONFLICT,
        "AlreadyExistsException",
    )
    .await;
    assert_eq!(err["message"], "Table already exists: main.default.events");

    // A native Iceberg table in UC, with converted columns.
    let info = uc_table(&app, "events").await;
    assert_eq!(
        (
            info["data_source_format"].as_str(),
            info["table_type"].as_str()
        ),
        (Some("ICEBERG"), Some("EXTERNAL"))
    );
    let names: Vec<&str> = info["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["id", "data"]);
    assert_eq!(info["properties"]["created-by"], "iceberg-rest-test");

    // Loadable and listed.
    assert_eq!(
        send(&app, Method::HEAD, &format!("{TABLES}/events"), None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    let loaded = ok(get(&app, &format!("{TABLES}/events")).await).await;
    assert_eq!(loaded["metadata-location"], first.as_str());
    let uuid = loaded["metadata"]["table-uuid"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        ok(get(&app, TABLES).await).await["identifiers"],
        json!([{ "namespace": ["default"], "name": "events" }])
    );

    // A commit writes the next version and moves the pointer.
    let wider = json!({ "type": "struct", "schema-id": 1, "fields": [
        { "id": 1, "name": "id", "required": true, "type": "long" },
        { "id": 2, "name": "data", "required": false, "type": "string" },
        { "id": 3, "name": "category", "required": false, "type": "string" },
    ] });
    let committed = ok(commit(
        &app,
        "events",
        json!([{ "type": "assert-table-uuid", "uuid": uuid }]),
        json!([
            { "action": "add-schema", "schema": wider },
            { "action": "set-current-schema", "schema-id": -1 },
            { "action": "set-properties", "updates": { "foo": "bar" } },
        ]),
    )
    .await)
    .await;
    let second = committed["metadata-location"].as_str().unwrap().to_string();
    assert!(second.contains("/metadata/00001-"), "{second}");
    assert_eq!(committed["metadata"]["properties"]["foo"], "bar");
    assert_eq!(
        committed["metadata"]["metadata-log"][0]["metadata-file"],
        first.as_str()
    );
    let info = uc_table(&app, "events").await;
    let names: Vec<&str> = info["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["id", "data", "category"]);
    assert_eq!(info["properties"]["foo"], "bar");
    assert_eq!(
        ok(get(&app, &format!("{TABLES}/events")).await).await["metadata-location"],
        second.as_str()
    );

    // A requirement that no longer holds; a commit that changes nothing.
    let resp = commit(
        &app,
        "events",
        json!([{ "type": "assert-table-uuid", "uuid": "00000000-0000-0000-0000-000000000000" }]),
        json!([{ "action": "set-properties", "updates": { "should": "fail" } }]),
    )
    .await;
    assert_iceberg_error(resp, StatusCode::CONFLICT, "CommitFailedException").await;
    let unchanged = ok(commit(&app, "events", json!([]), json!([])).await).await;
    assert_eq!(unchanged["metadata-location"], second.as_str());
    let resp = commit(&app, "events", json!([]), json!([
        { "action": "set-location", "location": location }, { "action": "set-location", "location": location }])).await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert_eq!(
        err["message"],
        "A commit may set the table location at most once, but received more than one set-location update."
    );

    // Rename within the namespace, and back.
    let rename = |source: &str, ns: &str, destination: &str| {
        json!({
        "source": { "namespace": ["default"], "name": source },
        "destination": { "namespace": [ns], "name": destination } })
    };
    let path = "/v1/catalogs/main/tables/rename";
    assert_eq!(
        send(
            &app,
            Method::POST,
            path,
            Some(rename("events", "default", "renamed"))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&app, Method::HEAD, &format!("{TABLES}/renamed"), None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&app, Method::HEAD, &format!("{TABLES}/events"), None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_iceberg_error(
        send(
            &app,
            Method::POST,
            path,
            Some(rename("events", "default", "other")),
        )
        .await,
        StatusCode::NOT_FOUND,
        "NoSuchTableException",
    )
    .await;
    plain_table(&app, "taken").await;
    assert_iceberg_error(
        send(
            &app,
            Method::POST,
            path,
            Some(rename("renamed", "default", "taken")),
        )
        .await,
        StatusCode::CONFLICT,
        "AlreadyExistsException",
    )
    .await;
    let err = assert_iceberg_error(
        send(
            &app,
            Method::POST,
            path,
            Some(rename("renamed", "other_ns", "moved")),
        )
        .await,
        StatusCode::NOT_IMPLEMENTED,
        "UnsupportedOperationException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Renaming a table into another namespace is not supported: default to other_ns"
    );
    let err = assert_iceberg_error(
        send(&app, Method::POST, path, Some(json!({}))).await,
        StatusCode::BAD_REQUEST,
        "BadRequestException",
    )
    .await;
    assert_eq!(err["message"], "Invalid source table: null");
    assert_eq!(
        send(
            &app,
            Method::POST,
            path,
            Some(rename("renamed", "default", "events"))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );

    // Drop.
    assert_eq!(
        send(&app, Method::DELETE, &format!("{TABLES}/events"), None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&app, Method::HEAD, &format!("{TABLES}/events"), None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_iceberg_error(
        get(&app, &format!("{TABLES}/events")).await,
        StatusCode::NOT_FOUND,
        "NoSuchTableException",
    )
    .await;

    // Without a location the table is MANAGED, at a server-assigned location.
    let managed = ok(create(
        &app,
        json!({ "name": "managed", "schema": id_data_schema() }),
    )
    .await)
    .await;
    let managed_location = managed["metadata"]["location"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(managed_location.contains("/tables/"), "{managed_location}");
    assert!(
        managed["metadata-location"]
            .as_str()
            .unwrap()
            .contains("/metadata/00000-")
    );
    let info = uc_table(&app, "managed").await;
    assert_eq!(
        (
            info["data_source_format"].as_str(),
            info["table_type"].as_str()
        ),
        (Some("ICEBERG"), Some("MANAGED"))
    );
    assert_eq!(info["storage_location"], managed_location.as_str());
    assert_eq!(
        send(&app, Method::DELETE, &format!("{TABLES}/managed"), None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn staged_create_and_commit() {
    let (app, _) = writable_app().await;
    let staged = ok(create(
        &app,
        json!({ "name": "staged", "schema": id_data_schema(), "stage-create": true }),
    )
    .await)
    .await;
    assert!(staged.get("metadata-location").is_none());
    let metadata = &staged["metadata"];
    let location = metadata["location"].as_str().unwrap().to_string();
    assert!(location.contains("/tables/"), "{location}");
    assert_eq!(
        get(&app, &format!("{TABLES}/staged")).await.status(),
        StatusCode::NOT_FOUND
    );

    let updates = json!([
        { "action": "assign-uuid", "uuid": metadata["table-uuid"] },
        { "action": "upgrade-format-version", "format-version": metadata["format-version"] },
        { "action": "add-schema", "schema": metadata["schemas"][0] },
        { "action": "set-current-schema", "schema-id": -1 },
        { "action": "add-spec", "spec": metadata["partition-specs"][0] },
        { "action": "set-default-spec", "spec-id": -1 },
        { "action": "add-sort-order", "sort-order": metadata["sort-orders"][0] },
        { "action": "set-default-sort-order", "sort-order-id": -1 },
        { "action": "set-location", "location": location },
        { "action": "set-properties", "updates": { "staged": "true" } },
    ]);
    let requirements = json!([{ "type": "assert-create" }]);
    let committed = ok(commit(&app, "staged", requirements.clone(), updates.clone()).await).await;
    assert!(
        committed["metadata-location"]
            .as_str()
            .unwrap()
            .contains("/metadata/00000-")
    );
    assert_eq!(committed["metadata"]["table-uuid"], metadata["table-uuid"]);
    assert_eq!(committed["metadata"]["properties"]["staged"], "true");
    let info = uc_table(&app, "staged").await;
    assert_eq!(
        (
            info["data_source_format"].as_str(),
            info["table_type"].as_str()
        ),
        (Some("ICEBERG"), Some("MANAGED"))
    );
    // The table took over the staging table: its id ends the location.
    assert!(
        location.ends_with(info["table_id"].as_str().unwrap()),
        "{location} {info}"
    );
    assert_eq!(
        ok(get(&app, &format!("{TABLES}/staged")).await).await["metadata"]["table-uuid"],
        metadata["table-uuid"]
    );

    // A replay loses: the table exists.
    let err = assert_iceberg_error(
        commit(&app, "staged", requirements, updates).await,
        StatusCode::CONFLICT,
        "CommitFailedException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Requirement failed: table already exists: default.staged"
    );
    // So does staging it again.
    assert_iceberg_error(
        create(
            &app,
            json!({ "name": "staged", "schema": id_data_schema(), "stage-create": true }),
        )
        .await,
        StatusCode::CONFLICT,
        "AlreadyExistsException",
    )
    .await;
    // A create commit takes only assert-create.
    let resp = commit(&app, "other", json!([{ "type": "assert-create" }, { "type": "assert-current-schema-id", "current-schema-id": 0 }]), json!([])).await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert_eq!(
        err["message"],
        "Invalid requirement for a create commit: AssertCurrentSchemaID"
    );
}

/// A staged-create commit for an external table at `location`.
fn staged_create_commit(location: &str) -> Value {
    json!({ "requirements": [{ "type": "assert-create" }], "updates": [
        { "action": "assign-uuid", "uuid": "9c12d441-03fe-4693-9a96-a0705ddf69c1" },
        { "action": "upgrade-format-version", "format-version": 2 },
        { "action": "add-schema", "schema": id_data_schema() },
        { "action": "set-current-schema", "schema-id": -1 },
        { "action": "add-spec", "spec": { "spec-id": 0, "fields": [] } },
        { "action": "set-default-spec", "spec-id": -1 },
        { "action": "add-sort-order", "sort-order": { "order-id": 0, "fields": [] } },
        { "action": "set-default-sort-order", "sort-order-id": -1 },
        { "action": "set-location", "location": location },
    ] })
}

#[tokio::test]
async fn local_external_locations() {
    let (app, root) = writable_app().await;
    let root_path = std::path::PathBuf::from(root.strip_prefix("file://").unwrap());
    let attempts = |location: String| {
        let app = app.clone();
        async move {
            let mut answers = Vec::new();
            for staged in [true, false] {
                answers.push(
                    create(
                        &app,
                        json!({ "name": "t", "schema": id_data_schema(),
                    "location": location, "stage-create": staged }),
                    )
                    .await,
                );
            }
            answers.push(
                send(
                    &app,
                    Method::POST,
                    &format!("{TABLES}/t"),
                    Some(staged_create_commit(&location)),
                )
                .await,
            );
            answers
        }
    };
    // Not one plain path.
    for location in [
        format!("{root}/a%00b"),
        format!("{root}/a%2Fb"),
        format!("{root}/a/%2e%2e/b"),
        format!("{root}/t?x=1"),
    ] {
        for resp in attempts(location.clone()).await {
            let err =
                assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
            assert!(
                err["message"]
                    .as_str()
                    .unwrap()
                    .contains("Unsupported local path"),
                "{location}: {err}"
            );
        }
    }
    // Not strictly under a root.
    let outside = tempfile::Builder::new()
        .prefix("uc-outside-")
        .tempdir()
        .unwrap()
        .keep();
    for location in [
        format!("file://{}/t", outside.display()),
        root.clone(),
        format!("{root}-sibling/t"),
    ] {
        for resp in attempts(location.clone()).await {
            let err = assert_iceberg_error(resp, StatusCode::FORBIDDEN, "ForbiddenException").await;
            assert!(
                err["message"]
                    .as_str()
                    .unwrap()
                    .contains("storage.external_local_roots"),
                "{location}: {err}"
            );
        }
    }
    assert!(!outside.join("t").exists() && !root_path.join("metadata").exists());

    // An external location allows a place outside the roots.
    let registered = format!("file://{}/registered", outside.display());
    uc(
        &app,
        Method::POST,
        "/credentials",
        json!({ "name": "local_cred", "purpose": "STORAGE",
        "aws_iam_role": { "role_arn": "arn:aws:iam::1:role/x" } }),
    )
    .await;
    uc(
        &app,
        Method::POST,
        "/external-locations",
        json!({ "name": "local_el", "url": registered,
        "credential_name": "local_cred" }),
    )
    .await;
    ok(create(&app, json!({ "name": "registered", "schema": id_data_schema(), "location": format!("{registered}/t") })).await).await;
    assert!(outside.join("registered/t/metadata").is_dir());
    let resp = create(
        &app,
        json!({ "name": "unregistered", "schema": id_data_schema(),
        "location": format!("file://{}/unregistered", outside.display()) }),
    )
    .await;
    assert_iceberg_error(resp, StatusCode::FORBIDDEN, "ForbiddenException").await;

    // An external location spelled with an unneeded escape covers nothing.
    uc(
        &app,
        Method::POST,
        "/external-locations",
        json!({ "name": "escaped_el", "url": format!("{root}/el%2Dx"),
        "credential_name": "local_cred" }),
    )
    .await;
    let resp = create(&app, json!({ "name": "escaped", "schema": id_data_schema(), "location": format!("{root}/el%2Dx/t") })).await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .contains("unneeded escapes"),
        "{err}"
    );
    let resp = create(&app, json!({ "name": "unescaped", "schema": id_data_schema(), "location": format!("{root}/el-x/u") })).await;
    let err = assert_iceberg_error(resp, StatusCode::FORBIDDEN, "ForbiddenException").await;
    assert!(err["message"].as_str().unwrap().contains("el%2Dx"), "{err}");
    assert!(!root_path.join("el-x").exists());

    // A staged-create commit at an escaped managed location is external, and refused.
    let staged = ok(create(
        &app,
        json!({ "name": "m", "schema": id_data_schema(), "stage-create": true }),
    )
    .await)
    .await;
    let escaped = staged["metadata"]["location"]
        .as_str()
        .unwrap()
        .replace("__unitystorage", "%5F%5Funitystorage");
    let resp = send(
        &app,
        Method::POST,
        &format!("{TABLES}/m"),
        Some(staged_create_commit(&escaped)),
    )
    .await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .contains("unneeded escapes"),
        "{err}"
    );
}

#[tokio::test]
async fn commits_do_not_follow_symbolic_links() {
    let (app, root) = writable_app().await;
    let root_path = std::path::PathBuf::from(root.strip_prefix("file://").unwrap());
    ok(create(&app, json!({ "name": "linked", "schema": id_data_schema(), "location": format!("{root}/linked") })).await).await;
    // Another client swaps the metadata directory for a link to a copy.
    let metadata = root_path.join("linked/metadata");
    let elsewhere = root_path.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    for file in std::fs::read_dir(&metadata).unwrap() {
        let file = file.unwrap().path();
        std::fs::copy(&file, elsewhere.join(file.file_name().unwrap())).unwrap();
    }
    std::fs::rename(&metadata, root_path.join("linked/metadata.moved")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &metadata).unwrap();
    let before = std::fs::read_dir(&elsewhere).unwrap().count();
    let resp = commit(
        &app,
        "linked",
        json!([]),
        json!([{ "action": "set-properties", "updates": { "k": "v" } }]),
    )
    .await;
    let err = assert_iceberg_error(resp, StatusCode::FORBIDDEN, "ForbiddenException").await;
    assert!(
        err["message"]
            .as_str()
            .unwrap()
            .contains("goes through a symbolic link"),
        "{err}"
    );
    assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), before);
}

#[tokio::test]
async fn concurrent_commits_serialize() {
    let (app, root) = writable_app().await;
    ok(create(
        &app,
        json!({ "name": "busy", "schema": id_data_schema(), "location": format!("{root}/busy") }),
    )
    .await)
    .await;
    let bumped = json!({ "type": "struct", "schema-id": 1, "fields": [
        { "id": 1, "name": "id", "required": true, "type": "long" },
        { "id": 2, "name": "data", "required": false, "type": "string" },
        { "id": 3, "name": "added", "required": false, "type": "string" },
    ] });
    let tasks: Vec<_> = (0..8).map(|_| {
        let (app, bumped) = (app.clone(), bumped.clone());
        tokio::spawn(async move {
            commit(&app, "busy", json!([{ "type": "assert-current-schema-id", "current-schema-id": 0 }]), json!([
                { "action": "add-schema", "schema": bumped }, { "action": "set-current-schema", "schema-id": -1 },
            ])).await.status()
        })
    }).collect();
    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(task.await.unwrap());
    }
    assert_eq!(
        statuses.iter().filter(|s| **s == StatusCode::OK).count(),
        1,
        "{statuses:?}"
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|s| **s == StatusCode::CONFLICT)
            .count(),
        7,
        "{statuses:?}"
    );
    let loaded = ok(get(&app, &format!("{TABLES}/busy")).await).await;
    assert!(
        loaded["metadata-location"]
            .as_str()
            .unwrap()
            .contains("/metadata/00001-")
    );
    // The losers' files are gone.
    let files = std::fs::read_dir(format!(
        "{}/busy/metadata",
        root.strip_prefix("file://").unwrap()
    ))
    .unwrap()
    .count();
    assert_eq!(files, 2);
}

#[tokio::test]
async fn drop_accepts_boolean_spellings_of_purge_requested() {
    let (app, _) = writable_app().await;
    for (i, spelling) in [
        "true", "false", "True", "False", "TRUE", "FALSE", "tRuE", "1", "0", "",
    ]
    .iter()
    .enumerate()
    {
        let name = format!("purge_{i}");
        ok(create(&app, json!({ "name": name, "schema": id_data_schema() })).await).await;
        let resp = send(
            &app,
            Method::DELETE,
            &format!("{TABLES}/{name}?purgeRequested={spelling}"),
            None,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT, "{spelling}");
        assert_eq!(
            get(&app, &format!("{TABLES}/{name}")).await.status(),
            StatusCode::NOT_FOUND
        );
    }
    for (i, rejected) in ["yes", "purge", "2"].iter().enumerate() {
        let name = format!("purge_bad_{i}");
        ok(create(&app, json!({ "name": name, "schema": id_data_schema() })).await).await;
        let resp = send(
            &app,
            Method::DELETE,
            &format!("{TABLES}/{name}?purgeRequested={rejected}"),
            None,
        )
        .await;
        let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
        assert_eq!(
            err["message"],
            format!("Invalid purgeRequested: {rejected}. It must be true or false.")
        );
        assert_eq!(
            get(&app, &format!("{TABLES}/{name}")).await.status(),
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn uniform_tables_are_read_only() {
    let app = app(true).await;
    uniform_table(&app, "uni", "iceberg.metadata.json").await;
    let resp = commit(
        &app,
        "uni",
        json!([]),
        json!([{ "action": "set-properties", "updates": { "foo": "bar" } }]),
    )
    .await;
    let err = assert_iceberg_error(resp, StatusCode::BAD_REQUEST, "BadRequestException").await;
    assert_eq!(
        err["message"],
        "Table is not a native Iceberg table and cannot be committed to via the Iceberg REST catalog: main.default.uni"
    );
    let err = assert_iceberg_error(
        send(&app, Method::DELETE, &format!("{TABLES}/uni"), None).await,
        StatusCode::BAD_REQUEST,
        "BadRequestException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Table main.default.uni was not created through the Iceberg REST catalog; drop it through the Unity Catalog API instead."
    );
    let rename = json!({ "source": { "namespace": ["default"], "name": "uni" }, "destination": { "namespace": ["default"], "name": "renamed" } });
    let err = assert_iceberg_error(
        send(
            &app,
            Method::POST,
            "/v1/catalogs/main/tables/rename",
            Some(rename),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "BadRequestException",
    )
    .await;
    assert_eq!(
        err["message"],
        "Table main.default.uni was not created through the Iceberg REST catalog; rename it through the Unity Catalog API instead."
    );
}

#[tokio::test]
async fn table_writes_can_be_disabled() {
    let app = app(false).await;
    let disabled = "Iceberg table writes are currently disabled. To enable them, set tables.iceberg_writes = true (UC_TABLES_ICEBERG_WRITES)";
    for (method, path, body) in [
        (
            Method::POST,
            TABLES.to_string(),
            Some(json!({ "name": "t", "schema": id_data_schema() })),
        ),
        (
            Method::POST,
            format!("{TABLES}/t"),
            Some(json!({ "updates": [] })),
        ),
        (Method::DELETE, format!("{TABLES}/t"), None),
        (
            Method::POST,
            "/v1/catalogs/main/tables/rename".to_string(),
            Some(json!({})),
        ),
    ] {
        let err = assert_iceberg_error(
            send(&app, method, &path, body).await,
            StatusCode::BAD_REQUEST,
            "BadRequestException",
        )
        .await;
        assert_eq!(err["message"], disabled, "{path}");
    }
}
