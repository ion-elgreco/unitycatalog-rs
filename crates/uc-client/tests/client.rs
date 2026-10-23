//! `uc-client` against a uc-server in this process: each method, the
//! bearer token, names that need escaping in a URL, and errors, also from a
//! server that is not a UC server.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::{StatusCode, Uri};
use serde::Serialize;
use serde_json::{Value, json};
use tempfile::TempDir;

use uc_api::{
    AwsIamRoleRequest, ColumnInfo, ColumnTypeName, CreateCatalog, CreateCredentialRequest,
    CreateExternalLocation, CreateFunction, CreateFunctionRequest, CreateModelVersion, CreatePat,
    CreateRegisteredModel, CreateSchema, CreateStagingTable, CreateTable,
    CreateVolumeRequestContent, CredentialPurpose, DataSourceFormat, DeltaCommit, DeltaGetCommits,
    Dependency, DependencyList, FunctionParameterInfo, FunctionParameterInfos,
    GenerateTemporaryPathCredential, GenerateTemporaryTableCredential,
    GenerateTemporaryVolumeCredential, ModelVersionStatus, ParameterStyle, PathOperation,
    PermissionsChange, Privilege, RoutineBody, ScimEmail, ScimGroup, ScimUser, SecurityType,
    SqlDataAccess, TableDependency, TableOperation, TableType, UpdateCatalog,
    UpdateCredentialRequest, UpdateExternalLocation, UpdateModelVersion, UpdatePermissions,
    UpdateRegisteredModel, UpdateSchema, UpdateTable, UpdateVolumeRequestContent, VolumeOperation,
    VolumeType, schema_uri,
};
use uc_client::{
    API_BASE, AuditFilter, CONTROL_BASE, Client, Contents, Error, Method, Page, Result, ScimPage,
};
use uc_common::{ErrorCode, SecurableType};
use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::authn::security::SecurityContext;
use uc_server::{AppState, build_app};

/// Serve `app` on a free port of this machine; its address.
async fn serve(app: Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    addr
}

/// A uc-server in this process, with a new database and no sign-in; the
/// URL of its API.
async fn server() -> String {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database");
    let addr = serve(build_app(AppState::new(metastore))).await;
    format!("http://{addr}{API_BASE}")
}

/// A uc-server as [`server`], with signing keys, so that it takes the
/// tokens of signed-in users. The URL of its API, and a token of
/// `alice@example.com`.
async fn server_with_keys() -> (String, String) {
    server_with_keys_as(false).await
}

/// [`server_with_keys`], with alice an admin (`auth.admin_principals`)
/// when `admin`.
async fn server_with_keys_as(admin: bool) -> (String, String) {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database");
    let security = SecurityContext::generate().unwrap();
    let token = security
        .create_access_token("alice@example.com", Duration::from_secs(600))
        .unwrap();
    let mut state = AppState::new(metastore).with_security(Arc::new(security));
    if admin {
        state = state.with_admins(["alice@example.com"]);
    }
    let addr = serve(build_app(state)).await;
    (format!("http://{addr}{API_BASE}"), token)
}

/// A client of a new [`server`].
async fn client() -> Client {
    Client::new(server().await).unwrap()
}

/// A client of a server that is not a UC server, such as a proxy: it
/// answers each request with the status in the last segment of the path,
/// and a body that is not JSON.
async fn stub() -> Client {
    let app = Router::new().fallback(|uri: Uri| async move {
        let status = uri.path().rsplit('/').next().unwrap_or_default();
        (
            StatusCode::from_bytes(status.as_bytes()).unwrap(),
            "<html>proxy</html>",
        )
    });
    let addr = serve(app).await;
    Client::new(format!("http://{addr}{API_BASE}")).unwrap()
}

/// The catalog `c`, with its managed storage in the returned directory,
/// and its schema `c.s`.
async fn catalog_and_schema(uc: &Client) -> TempDir {
    let storage = tempfile::tempdir().unwrap();
    uc.create_catalog(&CreateCatalog {
        name: "c".into(),
        comment: None,
        properties: None,
        storage_root: Some(file_url(&storage)),
    })
    .await
    .unwrap();
    uc.create_schema(&CreateSchema {
        name: "s".into(),
        catalog_name: "c".into(),
        comment: None,
        properties: None,
        storage_root: None,
    })
    .await
    .unwrap();
    storage
}

/// The `file://` URL of `dir`.
fn file_url(dir: &TempDir) -> String {
    format!("file://{}", dir.path().display())
}

/// `value` as JSON: the values of the API are not `PartialEq`.
fn as_json(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}

/// The code and message of the error of a call that fails.
fn error<T: std::fmt::Debug>(result: Result<T>) -> (ErrorCode, String) {
    let error = result.expect_err("the call fails");
    (error.code(), error.message())
}

#[tokio::test]
async fn catalogs() {
    let uc = client().await;
    let storage = tempfile::tempdir().unwrap();
    let properties = BTreeMap::from([("team".to_string(), "data".to_string())]);
    let created = uc
        .create_catalog(&CreateCatalog {
            name: "c".into(),
            comment: Some("primary".into()),
            properties: Some(properties.clone()),
            storage_root: Some(file_url(&storage)),
        })
        .await
        .unwrap();
    assert_eq!(created.name.as_deref(), Some("c"));
    assert_eq!(created.comment.as_deref(), Some("primary"));
    assert_eq!(created.properties, Some(properties));
    assert!(created.id.is_some() && created.created_at.is_some());
    assert!(created.storage_location.is_some());
    let fetched = uc.get_catalog("c").await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&created));
    let listed = uc.list_catalogs(&Page::default()).await.unwrap().catalogs;
    let names: Vec<_> = listed.iter().map(|c| c.name.as_deref()).collect();
    assert_eq!(names, [Some("c")]);

    let updated = uc
        .update_catalog(
            "c",
            &UpdateCatalog {
                comment: Some("edited".into()),
                properties: None,
                new_name: Some("c2".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name.as_deref(), Some("c2"));
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    assert_eq!(updated.id, created.id);

    // A catalog with a schema goes only with `force`.
    uc.create_schema(&CreateSchema {
        name: "s".into(),
        catalog_name: "c2".into(),
        comment: None,
        properties: None,
        storage_root: None,
    })
    .await
    .unwrap();
    let (code, _) = error(uc.delete_catalog("c2", false).await);
    assert_eq!(code, ErrorCode::FailedPrecondition);
    uc.delete_catalog("c2", true).await.unwrap();
    let (code, _) = error(uc.get_catalog("c2").await);
    assert_eq!(code, ErrorCode::CatalogNotFound);
}

#[tokio::test]
async fn schemas() {
    let uc = client().await;
    let storage = tempfile::tempdir().unwrap();
    uc.create_catalog(&CreateCatalog {
        name: "c".into(),
        comment: None,
        properties: None,
        storage_root: Some(file_url(&storage)),
    })
    .await
    .unwrap();
    let properties = BTreeMap::from([("layer".to_string(), "bronze".to_string())]);
    let created = uc
        .create_schema(&CreateSchema {
            name: "s".into(),
            catalog_name: "c".into(),
            comment: Some("raw data".into()),
            properties: Some(properties.clone()),
            storage_root: None,
        })
        .await
        .unwrap();
    assert_eq!(created.full_name.as_deref(), Some("c.s"));
    assert_eq!(created.comment.as_deref(), Some("raw data"));
    assert_eq!(created.properties, Some(properties));
    let fetched = uc.get_schema("c.s").await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&created));
    let listed = uc
        .list_schemas("c", &Page::default())
        .await
        .unwrap()
        .schemas;
    let names: Vec<_> = listed.iter().map(|s| s.name.as_deref()).collect();
    assert_eq!(names, [Some("s")]);

    let updated = uc
        .update_schema(
            "c.s",
            &UpdateSchema {
                comment: Some("edited".into()),
                properties: None,
                new_name: Some("s2".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.full_name.as_deref(), Some("c.s2"));
    assert_eq!(updated.comment.as_deref(), Some("edited"));

    // A schema with a model goes only with `force`.
    uc.create_registered_model(&CreateRegisteredModel {
        name: "m".into(),
        catalog_name: "c".into(),
        schema_name: "s2".into(),
        comment: None,
    })
    .await
    .unwrap();
    let (code, _) = error(uc.delete_schema("c.s2", false).await);
    assert_eq!(code, ErrorCode::FailedPrecondition);
    uc.delete_schema("c.s2", true).await.unwrap();
    assert!(
        uc.list_schemas("c", &Page::default())
            .await
            .unwrap()
            .schemas
            .is_empty()
    );
}

#[tokio::test]
async fn tables_and_views() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let columns = vec![
        ColumnInfo {
            name: Some("id".into()),
            type_text: Some("bigint".into()),
            type_json: Some(r#"{"name":"id","type":"long","nullable":false,"metadata":{}}"#.into()),
            type_name: Some(ColumnTypeName::Long),
            position: Some(0),
            nullable: Some(false),
            ..Default::default()
        },
        ColumnInfo {
            name: Some("email".into()),
            type_text: Some("string".into()),
            type_json: Some(
                r#"{"name":"email","type":"string","nullable":true,"metadata":{}}"#.into(),
            ),
            type_name: Some(ColumnTypeName::String),
            position: Some(1),
            nullable: Some(true),
            ..Default::default()
        },
    ];
    let staging = uc
        .create_staging_table(&CreateStagingTable {
            name: "t".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
        })
        .await
        .unwrap();
    let table_id = staging.id.unwrap();
    let table = uc
        .create_table(&CreateTable {
            name: "t".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            table_type: TableType::Managed,
            columns: columns.clone(),
            data_source_format: Some(DataSourceFormat::Delta),
            storage_location: staging.staging_location,
            comment: Some("users".into()),
            properties: Some(BTreeMap::from([(
                "io.unitycatalog.tableId".to_string(),
                table_id.clone(),
            )])),
            view_definition: None,
            view_dependencies: None,
        })
        .await
        .unwrap();
    assert_eq!(table.table_type, Some(TableType::Managed));
    assert_eq!(table.data_source_format, Some(DataSourceFormat::Delta));
    assert_eq!(as_json(&table.columns), as_json(&columns));
    assert_eq!(table.table_id, Some(table_id));
    let fetched = uc.get_table("c.s.t", false, false).await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&table));
    let edited = uc
        .update_table(
            "c.s.t",
            &UpdateTable {
                comment: Some("people".into()),
                columns: Vec::new(),
            },
        )
        .await
        .unwrap();
    assert_eq!(edited.comment.as_deref(), Some("people"));

    let dependencies = DependencyList {
        dependencies: Some(vec![Dependency {
            table: Some(TableDependency {
                table_full_name: "c.s.t".into(),
            }),
            function: None,
        }]),
    };
    let view = uc
        .create_table(&CreateTable {
            name: "v".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            table_type: TableType::View,
            columns,
            data_source_format: None,
            storage_location: None,
            comment: None,
            properties: None,
            view_definition: Some("select * from t".into()),
            view_dependencies: Some(dependencies.clone()),
        })
        .await
        .unwrap();
    assert_eq!(view.view_definition.as_deref(), Some("select * from t"));
    assert_eq!(as_json(&view.view_dependencies), as_json(&dependencies));
    let listed = uc
        .list_tables("c", "s", &Page::default())
        .await
        .unwrap()
        .tables;
    let names: Vec<_> = listed.iter().map(|t| t.name.as_deref()).collect();
    assert_eq!(names, [Some("t"), Some("v")]);

    uc.delete_table("c.s.v").await.unwrap();
    uc.delete_table("c.s.t").await.unwrap();
    assert!(
        uc.list_tables("c", "s", &Page::default())
            .await
            .unwrap()
            .tables
            .is_empty()
    );
}

#[tokio::test]
async fn volumes() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let created = uc
        .create_volume(&CreateVolumeRequestContent {
            catalog_name: "c".into(),
            schema_name: "s".into(),
            name: "v".into(),
            volume_type: VolumeType::Managed,
            comment: Some("scratch".into()),
            storage_location: None,
        })
        .await
        .unwrap();
    assert_eq!(created.full_name.as_deref(), Some("c.s.v"));
    assert_eq!(created.volume_type, Some(VolumeType::Managed));
    assert_eq!(created.comment.as_deref(), Some("scratch"));
    assert!(created.storage_location.is_some());
    let fetched = uc.get_volume("c.s.v").await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&created));
    let listed = uc
        .list_volumes("c", "s", &Page::default())
        .await
        .unwrap()
        .volumes;
    let names: Vec<_> = listed.iter().map(|v| v.name.as_deref()).collect();
    assert_eq!(names, [Some("v")]);

    let updated = uc
        .update_volume(
            "c.s.v",
            &UpdateVolumeRequestContent {
                comment: Some("edited".into()),
                new_name: Some("v2".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name.as_deref(), Some("v2"));
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    uc.delete_volume("c.s.v2").await.unwrap();
    assert!(
        uc.list_volumes("c", "s", &Page::default())
            .await
            .unwrap()
            .volumes
            .is_empty()
    );
}

#[tokio::test]
async fn functions() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let parameters = FunctionParameterInfos {
        parameters: Some(vec![FunctionParameterInfo {
            name: "x".into(),
            type_text: "long".into(),
            type_json: Some(r#""long""#.into()),
            type_name: ColumnTypeName::Long,
            type_precision: None,
            type_scale: None,
            type_interval_type: None,
            position: Some(0),
            parameter_mode: None,
            parameter_type: None,
            parameter_default: None,
            comment: None,
        }]),
    };
    let created = uc
        .create_function(&CreateFunctionRequest {
            function_info: CreateFunction {
                name: Some("inc".into()),
                catalog_name: Some("c".into()),
                schema_name: Some("s".into()),
                input_params: Some(parameters.clone()),
                data_type: Some(ColumnTypeName::Long),
                full_data_type: Some("long".into()),
                return_params: None,
                routine_body: RoutineBody::Sql,
                routine_definition: Some("x + 1".into()),
                routine_dependencies: None,
                parameter_style: ParameterStyle::S,
                is_deterministic: Some(true),
                sql_data_access: Some(SqlDataAccess::NoSql),
                is_null_call: Some(false),
                security_type: SecurityType::Definer,
                specific_name: Some("inc".into()),
                comment: Some("adds one".into()),
                properties: None,
                external_language: None,
            },
        })
        .await
        .unwrap();
    assert_eq!(created.full_name.as_deref(), Some("c.s.inc"));
    assert_eq!(as_json(&created.input_params), as_json(&parameters));
    assert_eq!(created.routine_body, Some(RoutineBody::Sql));
    assert_eq!(created.parameter_style, Some(ParameterStyle::S));
    assert_eq!(created.security_type, Some(SecurityType::Definer));
    assert_eq!(created.data_type, Some(ColumnTypeName::Long));
    assert_eq!(created.sql_data_access, Some(SqlDataAccess::NoSql));
    assert_eq!(created.routine_definition.as_deref(), Some("x + 1"));
    assert_eq!(created.comment.as_deref(), Some("adds one"));
    let fetched = uc.get_function("c.s.inc").await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&created));
    let listed = uc
        .list_functions("c", "s", &Page::default())
        .await
        .unwrap()
        .functions;
    let names: Vec<_> = listed.iter().map(|f| f.name.as_deref()).collect();
    assert_eq!(names, [Some("inc")]);

    uc.delete_function("c.s.inc").await.unwrap();
    let listed = uc
        .list_functions("c", "s", &Page::default())
        .await
        .unwrap()
        .functions;
    assert!(listed.is_empty());
}

#[tokio::test]
async fn what_catalogs_schemas_and_models_hold() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    for name in ["v1", "v2"] {
        uc.create_volume(&CreateVolumeRequestContent {
            catalog_name: "c".into(),
            schema_name: "s".into(),
            name: name.into(),
            volume_type: VolumeType::Managed,
            comment: None,
            storage_location: None,
        })
        .await
        .unwrap();
    }
    uc.create_registered_model(&CreateRegisteredModel {
        name: "m".into(),
        catalog_name: "c".into(),
        schema_name: "s".into(),
        comment: None,
    })
    .await
    .unwrap();
    for run in ["1", "2"] {
        uc.create_model_version(&CreateModelVersion {
            model_name: "m".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            source: format!("s3://bucket/m/v{run}"),
            run_id: None,
            comment: None,
        })
        .await
        .unwrap();
    }
    uc.create_schema(&CreateSchema {
        name: "empty".into(),
        catalog_name: "c".into(),
        comment: None,
        properties: None,
        storage_root: None,
    })
    .await
    .unwrap();

    let schema = Contents {
        volumes: 2,
        models: 1,
        versions: 2,
        ..Contents::default()
    };
    assert_eq!(uc.schema_contents("c", "s").await.unwrap(), schema);
    assert_eq!(
        uc.schema_contents("c", "empty").await.unwrap(),
        Contents::default()
    );
    assert_eq!(
        uc.catalog_contents("c").await.unwrap(),
        Contents {
            schemas: 2,
            ..schema
        }
    );
    assert_eq!(
        uc.model_contents("c.s.m").await.unwrap(),
        Contents {
            versions: 2,
            ..Contents::default()
        }
    );
}

#[tokio::test]
async fn registered_models_and_versions() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let created = uc
        .create_registered_model(&CreateRegisteredModel {
            name: "m".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            comment: Some("first".into()),
        })
        .await
        .unwrap();
    assert_eq!(created.full_name.as_deref(), Some("c.s.m"));
    assert_eq!(created.comment.as_deref(), Some("first"));
    assert!(created.storage_location.is_some());
    let fetched = uc.get_registered_model("c.s.m").await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&created));
    let listed = uc
        .list_registered_models(None, None, &Page::default())
        .await
        .unwrap()
        .registered_models;
    let names: Vec<_> = listed.iter().map(|m| m.full_name.as_deref()).collect();
    assert_eq!(names, [Some("c.s.m")]);

    let version = uc
        .create_model_version(&CreateModelVersion {
            model_name: "m".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            source: "s3://bucket/m/v1".into(),
            run_id: Some("run-1".into()),
            comment: Some("v1".into()),
        })
        .await
        .unwrap();
    assert_eq!(version.version, Some(1));
    assert_eq!(version.run_id.as_deref(), Some("run-1"));
    assert_eq!(version.comment.as_deref(), Some("v1"));
    assert!(version.status.is_some());
    let versions = uc
        .list_model_versions("c.s.m", &Page::default())
        .await
        .unwrap();
    let ids: Vec<_> = versions
        .model_versions
        .iter()
        .map(|v| v.id.as_deref())
        .collect();
    assert_eq!(ids, [version.id.as_deref()]);
    let fetched = uc.get_model_version("c.s.m", 1).await.unwrap();
    assert_eq!(fetched.id, version.id);
    let edited = uc
        .update_model_version(
            "c.s.m",
            1,
            &UpdateModelVersion {
                comment: Some("v1, edited".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(edited.comment.as_deref(), Some("v1, edited"));
    let ready = uc.finalize_model_version("c.s.m", 1).await.unwrap();
    assert_eq!(ready.status, Some(ModelVersionStatus::Ready));
    let second = uc
        .create_model_version(&CreateModelVersion {
            model_name: "m".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            source: "s3://bucket/m/v2".into(),
            run_id: None,
            comment: None,
        })
        .await
        .unwrap();
    assert_eq!(second.version, Some(2));
    uc.delete_model_version("c.s.m", 2).await.unwrap();
    let (code, _) = error(uc.get_model_version("c.s.m", 2).await);
    assert_eq!(code, ErrorCode::NotFound);

    let updated = uc
        .update_registered_model(
            "c.s.m",
            &UpdateRegisteredModel {
                comment: Some("edited".into()),
                new_name: Some("m2".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name.as_deref(), Some("m2"));
    assert_eq!(updated.comment.as_deref(), Some("edited"));

    // A model with versions goes only with `force`.
    let (code, _) = error(uc.delete_registered_model("c.s.m2", false).await);
    assert_eq!(code, ErrorCode::Aborted);
    uc.delete_registered_model("c.s.m2", true).await.unwrap();
    let listed = uc
        .list_registered_models(None, None, &Page::default())
        .await
        .unwrap()
        .registered_models;
    assert!(listed.is_empty());
}

#[tokio::test]
async fn credentials_and_external_locations() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let role = "arn:aws:iam::123456789012:role/uc";
    let credential = uc
        .create_credential(&CreateCredentialRequest {
            name: "cred".into(),
            comment: Some("prod".into()),
            aws_iam_role: Some(AwsIamRoleRequest {
                role_arn: role.into(),
            }),
            purpose: Some(CredentialPurpose::Storage),
        })
        .await
        .unwrap();
    assert_eq!(credential.purpose, Some(CredentialPurpose::Storage));
    assert_eq!(credential.comment.as_deref(), Some("prod"));
    let aws = credential.aws_iam_role.as_ref().unwrap();
    assert_eq!(aws.role_arn.as_deref(), Some(role));
    assert!(aws.external_id.is_some());
    let fetched = uc.get_credential("cred").await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&credential));
    let listed = uc
        .list_credentials(&Page::default())
        .await
        .unwrap()
        .credentials;
    let names: Vec<_> = listed.iter().map(|c| c.name.as_deref()).collect();
    assert_eq!(names, [Some("cred")]);

    let new_role = format!("{role}2");
    let updated = uc
        .update_credential(
            "cred",
            &UpdateCredentialRequest {
                comment: Some("edited".into()),
                aws_iam_role: Some(AwsIamRoleRequest {
                    role_arn: new_role.clone(),
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    assert_eq!(updated.aws_iam_role.unwrap().role_arn, Some(new_role));

    let location = uc
        .create_external_location(&CreateExternalLocation {
            name: "loc".into(),
            url: "s3://bucket/loc".into(),
            credential_name: "cred".into(),
            comment: Some("lake".into()),
        })
        .await
        .unwrap();
    assert_eq!(location.url.as_deref(), Some("s3://bucket/loc"));
    assert_eq!(location.credential_name.as_deref(), Some("cred"));
    assert_eq!(location.credential_id, credential.id);
    let fetched = uc.get_external_location("loc").await.unwrap();
    assert_eq!(as_json(&fetched), as_json(&location));
    let listed = uc.list_external_locations(&Page::default()).await.unwrap();
    let names: Vec<_> = listed
        .external_locations
        .iter()
        .map(|l| l.name.as_deref())
        .collect();
    assert_eq!(names, [Some("loc")]);
    let moved = uc
        .update_external_location(
            "loc",
            &UpdateExternalLocation {
                comment: Some("edited".into()),
                new_name: Some("loc2".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(moved.name.as_deref(), Some("loc2"));
    assert_eq!(moved.comment.as_deref(), Some("edited"));

    // A credential that a location uses, and a location that a volume
    // uses, go only with `force`.
    uc.create_volume(&CreateVolumeRequestContent {
        catalog_name: "c".into(),
        schema_name: "s".into(),
        name: "v".into(),
        volume_type: VolumeType::External,
        comment: None,
        storage_location: Some("s3://bucket/loc/v".into()),
    })
    .await
    .unwrap();
    let (code, _) = error(uc.delete_credential("cred", false).await);
    assert_eq!(code, ErrorCode::InvalidArgument);
    uc.delete_credential("cred", true).await.unwrap();
    assert!(
        uc.list_credentials(&Page::default())
            .await
            .unwrap()
            .credentials
            .is_empty()
    );
    let (code, _) = error(uc.delete_external_location("loc2", false).await);
    assert_eq!(code, ErrorCode::InvalidArgument);
    uc.delete_external_location("loc2", true).await.unwrap();
    let listed = uc.list_external_locations(&Page::default()).await.unwrap();
    assert!(listed.external_locations.is_empty());
}

#[tokio::test]
async fn permissions() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    // Grants need a user.
    let user = "ada@example.com";
    uc.raw_json(
        Method::POST,
        &format!("{CONTROL_BASE}/scim2/Users"),
        &[],
        Some(&json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": user,
            "emails": [{ "value": user, "primary": true }],
            "active": true,
        })),
    )
    .await
    .unwrap();
    let none = uc.get_permissions(SecurableType::Catalog, "c", None).await;
    assert!(none.unwrap().privilege_assignments.is_empty());

    let grant = UpdatePermissions {
        changes: vec![PermissionsChange {
            principal: user.into(),
            add: vec![Privilege::UseCatalog, Privilege::CreateSchema],
            remove: vec![],
        }],
    };
    let granted = uc
        .update_permissions(SecurableType::Catalog, "c", &grant)
        .await
        .unwrap();
    let [assignment] = granted.privilege_assignments.as_slice() else {
        panic!("one assignment: {granted:?}");
    };
    assert_eq!(assignment.principal, user);
    let mut privileges = assignment.privileges.clone();
    privileges.sort();
    assert_eq!(privileges, [Privilege::UseCatalog, Privilege::CreateSchema]);
    let fetched = uc.get_permissions(SecurableType::Catalog, "c", None).await;
    assert_eq!(as_json(&fetched.unwrap()), as_json(&granted));

    let revoke = UpdatePermissions {
        changes: vec![PermissionsChange {
            principal: user.into(),
            add: vec![],
            remove: vec![Privilege::UseCatalog, Privilege::CreateSchema],
        }],
    };
    let revoked = uc
        .update_permissions(SecurableType::Catalog, "c", &revoke)
        .await
        .unwrap();
    assert!(revoked.privilege_assignments.is_empty());
}

#[tokio::test]
async fn metastore_summary() {
    let uc = client().await;
    let summary = uc.metastore_summary().await.unwrap();
    assert!(summary.metastore_id.is_some());
    let again = uc.metastore_summary().await.unwrap();
    assert_eq!(again.metastore_id, summary.metastore_id);
}

/// Personal access tokens belong to a signed-in user. Each kind of request
/// (with a result, without one, and `raw_json`) carries the bearer token.
#[tokio::test]
async fn personal_access_tokens_and_the_bearer_token() {
    let (url, session) = server_with_keys().await;
    let anonymous = Client::new(&url).unwrap();
    let alice = Client::new(&url).unwrap().with_bearer(&session);

    let request = CreatePat {
        name: "ci".into(),
        expires_at: None,
    };
    let (code, _) = error(anonymous.create_pat(&request).await);
    assert_eq!(code, ErrorCode::Unauthenticated);
    let created = alice.create_pat(&request).await.unwrap();
    assert_eq!(created.name.as_deref(), Some("ci"));
    assert_eq!(created.owner.as_deref(), Some("alice@example.com"));
    let id = created.id.unwrap();

    // The new token signs in as well; lists leave its value out.
    let with_pat = Client::new(&url)
        .unwrap()
        .with_bearer(created.token_value.expect("the token, once"));
    let tokens = with_pat.list_pats(&Page::default()).await.unwrap().tokens;
    let ids: Vec<_> = tokens.iter().map(|t| t.id.as_deref()).collect();
    assert_eq!(ids, [Some(id.as_str())]);
    assert!(tokens[0].token_value.is_none());

    let raw = anonymous.raw_json(Method::GET, "tokens", &[], None).await;
    let (code, _) = error(raw);
    assert_eq!(code, ErrorCode::Unauthenticated);
    let listed = alice
        .raw_json(Method::GET, "tokens", &[], None)
        .await
        .unwrap();
    assert_eq!(listed["tokens"][0]["id"], id.as_str());

    let (code, _) = error(anonymous.revoke_pat(&id).await);
    assert_eq!(code, ErrorCode::Unauthenticated);
    alice.revoke_pat(&id).await.unwrap();
    assert!(
        alice
            .list_pats(&Page::default())
            .await
            .unwrap()
            .tokens
            .is_empty()
    );
}

/// `raw_json` takes a path below the base URL, or one from the server's
/// root, and a query; a success without a body is `null`. A refused call
/// keeps its status and body.
#[tokio::test]
async fn raw_json() {
    let uc = client().await;
    for name in ["c", "d"] {
        let body = json!({ "name": name, "comment": "raw" });
        let created = uc
            .raw_json(Method::POST, "catalogs", &[], Some(&body))
            .await
            .unwrap();
        assert_eq!(created["comment"], "raw");
    }
    let page = [("max_results", "1".to_string())];
    let listed = uc
        .raw_json(Method::GET, "catalogs", &page, None)
        .await
        .unwrap();
    assert_eq!(listed["catalogs"].as_array().unwrap().len(), 1, "{listed}");
    assert!(listed["next_page_token"].is_string(), "{listed}");

    let missing = uc
        .raw_json(Method::GET, "catalogs/missing", &[], None)
        .await;
    let Err(Error::Status { status, body }) = missing else {
        panic!("{missing:?}");
    };
    assert_eq!(status, 404);
    assert!(body.contains("CATALOG_NOT_FOUND"), "{body}");
    let missing = uc
        .raw_json(Method::GET, "catalogs/missing", &[], None)
        .await;
    let (code, _) = error(missing);
    assert_eq!(code, ErrorCode::CatalogNotFound);
    let deleted = uc.raw_json(Method::DELETE, "catalogs/c", &[], None).await;
    assert_eq!(deleted.unwrap(), Value::Null);

    let users = format!("{CONTROL_BASE}/scim2/Users");
    let listed = uc.raw_json(Method::GET, &users, &[], None).await.unwrap();
    assert!(listed["Resources"].is_array(), "{listed}");
}

/// `post_form` posts a form, as the token exchange takes it.
#[tokio::test]
async fn post_form() {
    let uc = client().await;
    let form = [
        (
            "grant_type",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ),
        ("subject_token", "not-a-token"),
    ];
    let refused = uc
        .post_form(&format!("{CONTROL_BASE}/auth/tokens"), &form)
        .await;
    let Err(Error::Status { status, body }) = refused else {
        panic!("{refused:?}");
    };
    assert!((400..500).contains(&status), "{status} {body}");
}

/// Pages: `max_results` and the `next_page_token` of the page before.
#[tokio::test]
async fn lists_page() {
    let uc = client().await;
    for name in ["a", "b", "c"] {
        let catalog = CreateCatalog {
            name: name.into(),
            comment: None,
            properties: None,
            storage_root: None,
        };
        uc.create_catalog(&catalog).await.unwrap();
    }
    let mut page = Page {
        max_results: Some(2),
        page_token: None,
    };
    let first = uc.list_catalogs(&page).await.unwrap();
    assert_eq!(first.catalogs.len(), 2);
    page.page_token = first.next_page_token.map(|t| t.0);
    assert!(page.page_token.is_some());
    let second = uc.list_catalogs(&page).await.unwrap();
    let names: Vec<_> = second.catalogs.iter().map(|c| c.name.as_deref()).collect();
    assert_eq!(names, [Some("c")]);
}

/// Credentials the server vends for a table, a volume and a path. Local
/// storage needs none, so they come back empty.
#[tokio::test]
async fn temporary_credentials() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    let table = uc
        .create_table(&CreateTable {
            name: "t".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            table_type: TableType::External,
            columns: Vec::new(),
            data_source_format: Some(DataSourceFormat::Delta),
            storage_location: Some(format!("{}/t", file_url(&storage))),
            comment: None,
            properties: None,
            view_definition: None,
            view_dependencies: None,
        })
        .await
        .unwrap();
    let request = GenerateTemporaryTableCredential {
        table_id: table.table_id.unwrap(),
        operation: TableOperation::Read,
    };
    let credentials = uc
        .generate_temporary_table_credentials(&request)
        .await
        .unwrap();
    assert!(credentials.aws_temp_credentials.is_none());

    let volume = uc
        .create_volume(&CreateVolumeRequestContent {
            catalog_name: "c".into(),
            schema_name: "s".into(),
            name: "v".into(),
            volume_type: VolumeType::Managed,
            comment: None,
            storage_location: None,
        })
        .await
        .unwrap();
    let request = GenerateTemporaryVolumeCredential {
        volume_id: volume.volume_id.unwrap(),
        operation: VolumeOperation::ReadVolume,
    };
    uc.generate_temporary_volume_credentials(&request)
        .await
        .unwrap();

    let request = GenerateTemporaryPathCredential {
        url: format!("{}/t", file_url(&storage)),
        operation: PathOperation::PathRead,
    };
    uc.generate_temporary_path_credentials(&request)
        .await
        .unwrap();
}

/// SCIM users and groups of the control API.
#[tokio::test]
async fn scim_users_and_groups() {
    let uc = client().await;
    let user = ScimUser {
        schemas: vec![schema_uri::USER.into()],
        display_name: Some("Ada".into()),
        emails: Some(vec![ScimEmail {
            value: "ada@example.com".into(),
            primary: Some(true),
            r#type: None,
        }]),
        ..Default::default()
    };
    let created = uc.create_user(&user).await.unwrap();
    let id = created.id.clone().unwrap();
    assert_eq!(created.user_name, "ada@example.com");
    assert_eq!(uc.get_user(&id).await.unwrap().id, created.id);
    let page = ScimPage {
        filter: None,
        start_index: Some(1),
        count: Some(10),
    };
    let users = uc.list_users(&page).await.unwrap();
    assert!(users.resources.iter().any(|u| u.id == created.id));
    let renamed = ScimUser {
        id: Some(id.clone()),
        display_name: Some("Ada L".into()),
        ..user
    };
    let updated = uc.update_user(&id, &renamed).await.unwrap();
    assert_eq!(updated.display_name.as_deref(), Some("Ada L"));
    // Deleting a user disables it, as upstream.
    uc.delete_user(&id).await.unwrap();
    assert_eq!(uc.get_user(&id).await.unwrap().active, Some(false));

    let group = ScimGroup {
        schemas: vec![schema_uri::GROUP.into()],
        display_name: "admins".into(),
        ..Default::default()
    };
    let created = uc.create_group(&group).await.unwrap();
    let id = created.id.clone().unwrap();
    assert_eq!(uc.get_group(&id).await.unwrap().display_name, "admins");
    let renamed = ScimGroup {
        display_name: "owners".into(),
        ..created.clone()
    };
    let updated = uc.update_group(&id, &renamed).await.unwrap();
    assert_eq!(updated.display_name, "owners");
    let groups = uc.list_groups(&ScimPage::default()).await.unwrap();
    assert_eq!(groups.resources.len(), 1);
    uc.delete_group(&id).await.unwrap();
    assert!(
        uc.list_groups(&ScimPage::default())
            .await
            .unwrap()
            .resources
            .is_empty()
    );
}

/// `me` is the user of the client's token. Without a token, the server
/// knows no caller: as upstream, a SCIM 400 error in an HTTP 500.
#[tokio::test]
async fn me() {
    let (url, session) = server_with_keys().await;
    let alice = Client::new(&url).unwrap().with_bearer(&session);
    let user = ScimUser {
        schemas: vec![schema_uri::USER.into()],
        display_name: Some("Alice".into()),
        emails: Some(vec![ScimEmail {
            value: "alice@example.com".into(),
            primary: Some(true),
            r#type: None,
        }]),
        ..Default::default()
    };
    alice.create_user(&user).await.unwrap();
    let me = alice.me().await.unwrap();
    assert_eq!(me.user_name, "alice@example.com");
    assert_eq!(me.display_name.as_deref(), Some("Alice"));
    let anonymous = Client::new(&url).unwrap();
    assert_eq!(anonymous.me().await.unwrap_err().status(), Some(500));
}

/// Backup, restore and audit take an admin.
#[tokio::test]
async fn admin_backup_restore_and_audit() {
    let (url, token) = server_with_keys_as(true).await;
    let uc = Client::new(&url).unwrap().with_bearer(&token);
    let catalog = CreateCatalog {
        name: "c".into(),
        comment: None,
        properties: None,
        storage_root: None,
    };
    uc.create_catalog(&catalog).await.unwrap();
    let snapshot = uc.backup().await.unwrap();
    assert!(!snapshot.tables.is_empty());
    let report = uc.restore(&snapshot).await.unwrap();
    assert!(!report.imported.is_empty());
    let filter = AuditFilter {
        method: Some("POST".into()),
        ..Default::default()
    };
    let events = uc.list_audit_events(&filter).await.unwrap().events;
    assert!(
        events.iter().any(|e| e.path.ends_with("/catalogs")),
        "{events:?}"
    );

    let (code, _) = error(Client::new(&url).unwrap().backup().await);
    assert_eq!(code, ErrorCode::Unauthenticated);
}

/// The Iceberg REST catalog: a catalog is a warehouse, a schema a
/// namespace.
#[tokio::test]
async fn iceberg_rest_catalog() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let config = uc.iceberg_config("c").await.unwrap();
    assert_eq!(config.overrides["prefix"], "catalogs/c");
    let namespaces = uc
        .list_iceberg_namespaces("c", &Page::default())
        .await
        .unwrap();
    assert_eq!(namespaces.namespaces, [vec!["s".to_string()]]);
    let tables = uc
        .list_iceberg_tables("c", "s", &Page::default())
        .await
        .unwrap();
    assert!(tables.identifiers.is_empty());
    let missing = uc.load_iceberg_table("c", "s", "t").await.unwrap_err();
    assert_eq!(missing.status(), Some(404));
}

/// The commits of a catalog-managed table go through the server; a table
/// it does not know is an error.
#[tokio::test]
async fn delta_commits() {
    let uc = client().await;
    let request = DeltaGetCommits {
        table_id: Some("00000000-0000-0000-0000-000000000000".into()),
        table_uri: Some("file:///tmp/t".into()),
        start_version: Some(0),
        end_version: None,
    };
    let missing = uc.get_delta_commits(&request).await.unwrap_err();
    assert_eq!(missing.status(), Some(404));
    let commit = DeltaCommit {
        table_id: request.table_id.clone(),
        table_uri: request.table_uri.clone(),
        latest_backfilled_version: Some(0),
        ..Default::default()
    };
    let missing = uc.commit_delta(&commit).await.unwrap_err();
    assert_eq!(missing.status(), Some(404));
}

/// The server's errors keep their upstream code and message.
#[tokio::test]
async fn errors_of_the_server_keep_their_code_and_message() {
    let uc = client().await;
    let not_found = (
        ErrorCode::CatalogNotFound,
        "Catalog not found: missing".to_string(),
    );
    assert_eq!(error(uc.get_catalog("missing").await), not_found);
    assert_eq!(error(uc.delete_catalog("missing", false).await), not_found);
}

/// Names reach the server as they are: in a path segment and in a query
/// value, the characters that would end one or start an escape are
/// escaped.
#[tokio::test]
async fn names_reach_the_server_as_they_are() {
    let uc = client().await;
    for name in ["a/b", "a?b", "a#b", "a%20b", "a b", "a&b=c+d", "été"] {
        let message = format!("Catalog not found: {name}");
        let not_found = (ErrorCode::CatalogNotFound, message);
        assert_eq!(error(uc.get_catalog(name).await), not_found);
        assert_eq!(
            error(uc.list_schemas(name, &Page::default()).await),
            not_found
        );
    }
}

#[tokio::test]
async fn the_base_url_may_end_with_a_slash_or_not() {
    let url = server().await;
    for base in [url.clone(), format!("{url}/")] {
        let uc = Client::new(&base).unwrap();
        let catalogs = uc.list_catalogs(&Page::default()).await.unwrap().catalogs;
        assert!(catalogs.is_empty(), "{base}");
    }
}

#[test]
fn a_base_url_that_does_not_parse() {
    let error = Client::new("not a url").err().expect("an error");
    assert_eq!(error.code(), ErrorCode::InvalidArgument);
    assert!(
        error.message().starts_with("invalid base URL: "),
        "{}",
        error.message()
    );
}

#[tokio::test]
async fn a_server_that_does_not_answer() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let uc = Client::new(format!("http://127.0.0.1:{port}{API_BASE}")).unwrap();
    let (code, message) = error(uc.list_catalogs(&Page::default()).await);
    assert_eq!(code, ErrorCode::Internal);
    assert!(message.starts_with("http: "), "{message}");
    // The message says why, not only that the request failed.
    #[cfg(unix)]
    assert!(message.contains(": Connection refused"), "{message}");
}

/// Errors without the JSON body of a UC server, from a proxy say, take
/// their code from the HTTP status.
#[tokio::test]
async fn errors_without_a_uc_body_take_their_code_from_the_status() {
    let uc = stub().await;
    for (status, code, message) in [
        ("400", ErrorCode::InvalidArgument, "http 400 Bad Request"),
        ("401", ErrorCode::Unauthenticated, "http 401 Unauthorized"),
        ("403", ErrorCode::PermissionDenied, "http 403 Forbidden"),
        ("404", ErrorCode::NotFound, "http 404 Not Found"),
        ("409", ErrorCode::AlreadyExists, "http 409 Conflict"),
        ("502", ErrorCode::Internal, "http 502 Bad Gateway"),
    ] {
        let expected = (code, message.to_string());
        assert_eq!(error(uc.get_catalog(status).await), expected);
        assert_eq!(error(uc.delete_table(status).await), expected);
        let raw = uc.raw_json(Method::GET, status, &[], None).await;
        assert_eq!(error(raw), expected);
    }
}

/// A success whose body is not the result of the method is an error;
/// methods without a result take any body.
#[tokio::test]
async fn a_success_with_a_body_of_another_shape() {
    let uc = stub().await;
    let (code, message) = error(uc.get_catalog("200").await);
    assert_eq!(code, ErrorCode::Internal);
    assert!(message.starts_with("decode: "), "{message}");
    let raw = uc.raw_json(Method::GET, "200", &[], None).await;
    let (code, message) = error(raw);
    assert_eq!(code, ErrorCode::Internal);
    assert!(message.starts_with("decode: "), "{message}");
    uc.delete_table("200").await.unwrap();
}

/// Each method of the client has a call in this file.
#[test]
fn every_method_has_a_test() {
    let tests = include_str!("client.rs");
    let methods: Vec<_> = include_str!("../src/lib.rs")
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub async fn "))
        .map(|rest| rest.split(['(', '<']).next().unwrap())
        .collect();
    assert!(!methods.is_empty());
    for method in methods {
        let call = format!(".{method}(");
        assert!(tests.contains(&call), "no test calls `{method}`");
    }
}
