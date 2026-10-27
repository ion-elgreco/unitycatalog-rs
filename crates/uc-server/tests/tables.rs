mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{app, assert_error, assert_status, json_body, req, req_json, seeded};

fn columns() -> Value {
    json!([
        { "name": "id", "type_name": "LONG", "type_text": "long", "type_json": "{\"name\":\"id\",\"type\":\"long\",\"nullable\":false,\"metadata\":{}}", "position": 0, "nullable": false },
        { "name": "email", "type_name": "STRING", "type_text": "string", "type_json": "{\"name\":\"email\",\"type\":\"string\",\"nullable\":true,\"metadata\":{}}", "position": 1, "nullable": true },
    ])
}

/// The client side of upstream's managed-table flow: reserve a staging
/// table, then create the MANAGED table at its location with its id.
pub async fn create_managed_table(app: &Router, name: &str) -> Value {
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/staging-tables",
            json!({ "name": name, "catalog_name": "main", "schema_name": "default" }),
        ))
        .await
        .unwrap();
    let staging = json_body(assert_status(resp, StatusCode::OK, "staging").await).await;
    let resp = app
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/tables",
            json!({
                "name": name, "catalog_name": "main", "schema_name": "default",
                "table_type": "MANAGED", "data_source_format": "DELTA",
                "storage_location": staging["staging_location"],
                "columns": columns(),
                "properties": { "io.unitycatalog.tableId": staging["id"] },
            }),
        ))
        .await
        .unwrap();
    json_body(assert_status(resp, StatusCode::OK, "create managed").await).await
}

#[rstest]
#[tokio::test]
async fn create_get_list_delete_table(#[future(awt)] seeded: Router) {
    let info = create_managed_table(&seeded, "users").await;
    assert_eq!(info["name"], "users");
    assert_eq!(info["table_type"], "MANAGED");
    assert_eq!(info["data_source_format"], "DELTA");
    assert_eq!(info["columns"].as_array().unwrap().len(), 2);
    assert!(info["table_id"].is_string());

    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/tables/main.default.users"))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "get").await;
    assert_eq!(json_body(resp).await["name"], "users");

    let resp = seeded
        .clone()
        .oneshot(req(
            Method::GET,
            "/tables?catalog_name=main&schema_name=default",
        ))
        .await
        .unwrap();
    let resp = assert_status(resp, StatusCode::OK, "list").await;
    assert_eq!(json_body(resp).await["tables"].as_array().unwrap().len(), 1);

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/tables/main.default.users",
            json!({
                "comment": "edited",
                "columns": [{ "name": "email", "comment": "The primary email" }],
            }),
        ))
        .await
        .unwrap();
    let updated = json_body(assert_status(resp, StatusCode::OK, "patch").await).await;
    assert_eq!(updated["comment"], "edited");
    assert_eq!(updated["columns"][0]["comment"], Value::Null);
    assert_eq!(updated["columns"][1]["comment"], "The primary email");

    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::PATCH,
            "/tables/main.default.users",
            json!({ "columns": [{ "name": "nope", "comment": "" }] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = seeded
        .oneshot(req(Method::DELETE, "/tables/main.default.users"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Upstream's models send `properties` as `{}` when a securable has none.
/// deltalake's Unity Catalog client needs the field of a table.
#[rstest]
#[tokio::test]
async fn securables_without_properties_answer_an_empty_map(#[future(awt)] seeded: Router) {
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/tables",
            json!({
                "name": "events", "catalog_name": "main", "schema_name": "default",
                "table_type": "EXTERNAL", "data_source_format": "DELTA",
                "columns": columns(), "storage_location": "s3://bucket/events",
            }),
        ))
        .await
        .unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(created["properties"], json!({}));
    for (path, list) in [
        ("/catalogs/main", None),
        ("/catalogs", Some("catalogs")),
        ("/schemas/main.default", None),
        ("/schemas?catalog_name=main", Some("schemas")),
        ("/tables/main.default.events", None),
        (
            "/tables?catalog_name=main&schema_name=default",
            Some("tables"),
        ),
    ] {
        let resp = seeded
            .clone()
            .oneshot(req(Method::GET, path))
            .await
            .unwrap();
        let body = json_body(assert_status(resp, StatusCode::OK, path).await).await;
        let info = match list {
            Some(list) => body[list][0].clone(),
            None => body,
        };
        assert_eq!(info["properties"], json!({}), "{path}");
    }
}

#[rstest]
#[tokio::test]
async fn external_table_rules(#[future(awt)] seeded: Router) {
    let external = |location: Option<&str>| {
        let mut body = json!({
            "name": "ext", "catalog_name": "main", "schema_name": "default",
            "table_type": "EXTERNAL", "data_source_format": "DELTA",
        });
        if let Some(l) = location {
            body["storage_location"] = json!(l);
        }
        body
    };
    for (body, message) in [
        (
            external(None),
            "Storage location is required for external table",
        ),
        (
            external(Some("s3://bucket")),
            "External table storage location must include a non-empty path prefix: s3://bucket",
        ),
    ] {
        let resp = seeded
            .clone()
            .oneshot(req_json(Method::POST, "/tables", body))
            .await
            .unwrap();
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], message);
    }

    // `columns` may be left out; the location is normalized.
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/tables",
            external(Some("s3://bucket/ext/")),
        ))
        .await
        .unwrap();
    let table = json_body(assert_status(resp, StatusCode::OK, "external").await).await;
    assert_eq!(table["storage_location"], "s3://bucket/ext");

    // A Lance table is external; the server does not manage Lance data.
    let mut lance = external(Some("s3://bucket/lance"));
    lance["name"] = json!("lance");
    lance["data_source_format"] = json!("LANCE");
    let resp = seeded
        .clone()
        .oneshot(req_json(Method::POST, "/tables", lance.clone()))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "lance").await;
    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/tables/main.default.lance"))
        .await
        .unwrap();
    let got = json_body(assert_status(resp, StatusCode::OK, "get lance").await).await;
    assert_eq!(got["data_source_format"], "LANCE");
    lance["name"] = json!("managed_lance");
    lance["table_type"] = json!("MANAGED");
    let resp = seeded
        .clone()
        .oneshot(req_json(Method::POST, "/tables", lance))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        "Managed table creation is only supported for Delta and Iceberg formats."
    );

    // Native Iceberg tables come from the Iceberg REST catalog only.
    let mut iceberg = external(Some("s3://bucket/ice"));
    iceberg["data_source_format"] = json!("ICEBERG");
    let resp = seeded
        .oneshot(req_json(Method::POST, "/tables", iceberg))
        .await
        .unwrap();
    assert_error(resp, StatusCode::BAD_REQUEST, "UNSUPPORTED_TABLE_FORMAT").await;
}

#[rstest]
#[tokio::test]
async fn create_table_under_missing_schema_returns_404(#[future(awt)] app: Router) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "main" }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(req_json(
            Method::POST,
            "/staging-tables",
            json!({ "name": "users", "catalog_name": "main", "schema_name": "default" }),
        ))
        .await
        .unwrap();
    assert_error(resp, StatusCode::NOT_FOUND, "SCHEMA_NOT_FOUND").await;
}

#[rstest]
#[tokio::test]
async fn cascade_delete_schema_requires_force_when_tables_exist(#[future(awt)] seeded: Router) {
    create_managed_table(&seeded, "t1").await;

    let resp = seeded
        .clone()
        .oneshot(req(Method::DELETE, "/schemas/main.default"))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
    assert_eq!(err["message"], "Cannot delete schema with tables");

    let resp = seeded
        .clone()
        .oneshot(req(Method::DELETE, "/schemas/main.default?force=true"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = seeded
        .oneshot(req(Method::GET, "/tables/main.default.t1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn staging_tables_reserve_a_managed_location(#[future(awt)] seeded: Router) {
    let catalog = json_body(
        seeded
            .clone()
            .oneshot(req(Method::GET, "/catalogs/main"))
            .await
            .unwrap(),
    )
    .await;
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/staging-tables",
            json!({ "name": "incoming", "catalog_name": "main", "schema_name": "default" }),
        ))
        .await
        .unwrap();
    let info = json_body(assert_status(resp, StatusCode::OK, "create staging").await).await;
    assert_eq!(info["name"], "incoming");
    assert_eq!(
        info["staging_location"],
        format!(
            "{}/tables/{}",
            catalog["storage_location"].as_str().unwrap(),
            info["id"].as_str().unwrap()
        )
    );

    // The staging id is a table id for temporary credentials.
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/temporary-table-credentials",
            json!({ "table_id": info["id"], "operation": "READ_WRITE" }),
        ))
        .await
        .unwrap();
    let body = json_body(assert_status(resp, StatusCode::OK, "staging creds").await).await;
    assert_eq!(body["url"], info["staging_location"]);

    // A name used by a table cannot be staged.
    create_managed_table(&seeded, "taken").await;
    let resp = seeded
        .oneshot(req_json(
            Method::POST,
            "/staging-tables",
            json!({ "name": "taken", "catalog_name": "main", "schema_name": "default" }),
        ))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "TABLE_ALREADY_EXISTS").await;
    assert_eq!(err["message"], "Table already exists: taken");
}

#[rstest]
#[tokio::test]
async fn managed_tables_take_over_their_staging_table(#[future(awt)] seeded: Router) {
    let stage = |name: &str| {
        req_json(
            Method::POST,
            "/staging-tables",
            json!({ "name": name, "catalog_name": "main", "schema_name": "default" }),
        )
    };
    let staging = json_body(seeded.clone().oneshot(stage("t")).await.unwrap()).await;
    let managed = |location: &Value, props: Value, format: &str| {
        json!({
            "name": "t", "catalog_name": "main", "schema_name": "default",
            "table_type": "MANAGED", "data_source_format": format,
            "storage_location": location, "columns": [], "properties": props,
        })
    };
    let good_props = json!({ "io.unitycatalog.tableId": staging["id"] });

    let cases = [
        (
            managed(&staging["staging_location"], good_props.clone(), "PARQUET"),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "Managed table creation is only supported for Delta and Iceberg formats.".to_string(),
        ),
        (
            managed(
                &json!("s3://bucket/not/staged"),
                good_props.clone(),
                "DELTA",
            ),
            StatusCode::NOT_FOUND,
            "TABLE_NOT_FOUND",
            "Staging table not found: s3://bucket/not/staged".to_string(),
        ),
        (
            managed(&staging["staging_location"], json!({}), "DELTA"),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "Properties does not contain io.unitycatalog.tableId.".to_string(),
        ),
        (
            managed(
                &staging["staging_location"],
                json!({ "io.unitycatalog.tableId": "other" }),
                "DELTA",
            ),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            format!(
                "the table id ({}) does not match the properties io.unitycatalog.tableId(other).",
                staging["id"].as_str().unwrap()
            ),
        ),
    ];
    for (body, status, code, message) in cases {
        let resp = seeded
            .clone()
            .oneshot(req_json(Method::POST, "/tables", body))
            .await
            .unwrap();
        let err = assert_error(resp, status, code).await;
        assert_eq!(err["message"], message.as_str());
    }

    // The table keeps the staging id; the staging table can be used once.
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/tables",
            managed(&staging["staging_location"], good_props.clone(), "DELTA"),
        ))
        .await
        .unwrap();
    let table = json_body(assert_status(resp, StatusCode::OK, "create").await).await;
    assert_eq!(table["table_id"], staging["id"]);
    assert_eq!(table["storage_location"], staging["staging_location"]);

    let resp = seeded
        .clone()
        .oneshot(req(Method::DELETE, "/tables/main.default.t"))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "delete").await;
    let resp = seeded
        .oneshot(req_json(
            Method::POST,
            "/tables",
            managed(&staging["staging_location"], good_props, "DELTA"),
        ))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "FAILED_PRECONDITION").await;
    assert_eq!(
        err["message"],
        format!(
            "Staging table already committed: {}",
            staging["staging_location"].as_str().unwrap()
        )
    );
}

#[rstest]
#[tokio::test]
async fn view_like_tables_follow_upstream_rules(#[future(awt)] seeded: Router) {
    let base = create_managed_table(&seeded, "base").await;
    assert_eq!(base["name"], "base");
    let view = |kind: &str, definition: Option<&str>, cols: Value, deps: Option<Value>| {
        let mut body = json!({
            "name": "v", "catalog_name": "main", "schema_name": "default",
            "table_type": kind, "columns": cols,
        });
        if let Some(d) = definition {
            body["view_definition"] = json!(d);
        }
        if let Some(d) = deps {
            body["view_dependencies"] = json!({ "dependencies": d });
        }
        body
    };
    let table_dep = |name: &str| json!([{ "table": { "table_full_name": name } }]);
    let cases = [
        (
            view("VIEW", None, columns(), None),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "view_definition is required for view",
        ),
        (
            view("VIEW", Some("select 1"), json!([]), None),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "columns must contain at least one entry for view",
        ),
        (
            view("VIEW", Some("select 1"), columns(), Some(table_dep("x"))),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "Dependency table full name must be a three-part name (catalog.schema.table); got: x",
        ),
        (
            view(
                "VIEW",
                Some("select 1"),
                columns(),
                Some(table_dep("main.default.nope")),
            ),
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "View dependency table does not exist: main.default.nope",
        ),
        (
            view("METRIC_VIEW", Some("version: 0.1"), columns(), None),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "view_dependencies must contain at least one entry for metric view",
        ),
        (
            view("STREAMING_TABLE", None, columns(), None),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "STREAMING TABLE creation is not supported yet.",
        ),
        (
            view("MATERIALIZED_VIEW", None, columns(), None),
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "MATERIALIZED VIEW creation is not supported yet.",
        ),
    ];
    for (body, status, code, message) in cases {
        let resp = seeded
            .clone()
            .oneshot(req_json(Method::POST, "/tables", body.clone()))
            .await
            .unwrap();
        let err = assert_error(resp, status, code).await;
        assert_eq!(err["message"], message, "{body}");
    }

    // A valid view: no storage location, dependencies kept.
    let resp = seeded
        .clone()
        .oneshot(req_json(
            Method::POST,
            "/tables",
            view(
                "VIEW",
                Some("select * from base"),
                columns(),
                Some(table_dep("main.default.base")),
            ),
        ))
        .await
        .unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "view").await).await;
    assert!(created.get("storage_location").is_none_or(Value::is_null));
    let resp = seeded
        .oneshot(req(Method::GET, "/tables/main.default.v"))
        .await
        .unwrap();
    let got = json_body(assert_status(resp, StatusCode::OK, "get view").await).await;
    assert_eq!(got["table_type"], "VIEW");
    assert_eq!(
        got["view_dependencies"]["dependencies"][0]["table"]["table_full_name"],
        "main.default.base"
    );
}

/// Upstream `ColumnUtils.validateTypeJson` on create.
#[rstest]
#[tokio::test]
async fn column_type_json_is_validated(#[future(awt)] seeded: Router) {
    let table = |format: &str, column: Value| {
        json!({
            "name": "t", "catalog_name": "main", "schema_name": "default",
            "table_type": "EXTERNAL", "data_source_format": format,
            "storage_location": "s3://lake/t", "columns": [column],
        })
    };
    let column = |type_json: Option<&str>| {
        let mut c = json!({ "name": "c", "type_name": "INT", "type_text": "int", "position": 0 });
        if let Some(t) = type_json {
            c["type_json"] = json!(t);
        }
        c
    };
    let prefix = "Invalid type_json for column c: ";
    let cases = [
        (
            "DELTA",
            column(None),
            format!("{prefix}Column c has null/empty typeJson"),
        ),
        (
            "DELTA",
            column(Some("nope")),
            format!("{prefix}Failed to parse typeJson for column c: nope"),
        ),
        (
            "DELTA",
            column(Some(r#""timestamp""#)),
            format!(r#"{prefix}Failed to parse typeJson for column c: "timestamp""#),
        ),
        (
            "DELTA",
            column(Some(
                r#"{"name":"c","type":"int","nullable":true,"metadata":{}}"#,
            )),
            format!("{prefix}type_json.type: Unsupported Delta primitive type: int"),
        ),
        (
            "DELTA",
            column(Some(r#"{"name":"c","type":"integer","nullable":true}"#)),
            format!("{prefix}type_json.metadata is required."),
        ),
        (
            "DELTA",
            column(Some(
                r#"{"name":"c","type":"decimal(40,2)","nullable":true,"metadata":{}}"#,
            )),
            format!("{prefix}type_json.type.precision must be in [0, 38], got 40."),
        ),
        (
            "DELTA",
            column(Some(
                r#"{"name":"d","type":"integer","nullable":true,"metadata":{}}"#,
            )),
            format!("{prefix}field name d does not match column name c"),
        ),
        (
            "PARQUET",
            column(Some(r#"{"name":"c","type":"int","nullable":true}"#)),
            "type_json.metadata is required.".to_string(),
        ),
    ];
    for (format, column, message) in cases {
        let resp = seeded
            .clone()
            .oneshot(req_json(
                Method::POST,
                "/tables",
                table(format, column.clone()),
            ))
            .await
            .unwrap();
        let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
        assert_eq!(err["message"], message, "{column}");
    }

    let mut mismatch = column(Some(
        r#"{"name":"c","type":"integer","nullable":true,"metadata":{}}"#,
    ));
    mismatch["nullable"] = json!(false);
    let resp = seeded
        .clone()
        .oneshot(req_json(Method::POST, "/tables", table("DELTA", mismatch)))
        .await
        .unwrap();
    let err = assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(
        err["message"],
        format!("{prefix}field nullable true does not match column nullable false")
    );

    // Non-Delta tables accept Spark-only types; type_text is lowercased and
    // nullable defaults to false.
    let mut ok = column(Some(
        r#"{"name":"c","type":"varchar(10)","nullable":true,"metadata":{}}"#,
    ));
    ok["type_text"] = json!("VARCHAR(10)");
    let resp = seeded
        .clone()
        .oneshot(req_json(Method::POST, "/tables", table("PARQUET", ok)))
        .await
        .unwrap();
    let created = json_body(assert_status(resp, StatusCode::OK, "parquet").await).await;
    assert_eq!(created["columns"][0]["type_text"], "varchar(10)");
    assert_eq!(created["columns"][0]["nullable"], false);
    let resp = seeded
        .oneshot(req(Method::GET, "/tables/main.default.t"))
        .await
        .unwrap();
    let got = json_body(assert_status(resp, StatusCode::OK, "get").await).await;
    assert_eq!(got["columns"][0]["nullable"], false);
}
