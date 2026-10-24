//! A uc-server in this process, and what the tests of the areas share.

use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::Router;
use tempfile::TempDir;
use url::Url;

use uc_client::Client;
use uc_client::error::{ApiError, Error, Result};
use uc_client::models::scim::v1::ScimEmail;
use uc_client::models::tables::v1::{
    ColumnInfo, ColumnTypeName, DataSourceFormat, TableInfo, TableType,
};
use uc_metastore::{DynMetastore, MetastoreConfig, SqlMetastore};
use uc_server::authn::security::SecurityContext;
use uc_server::{AppState, build_app};

/// Serve `app` on a free port of this machine; its root URL.
pub async fn serve(app: Router) -> Url {
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
    format!("http://{addr}/").parse().unwrap()
}

/// A new test database.
pub async fn metastore() -> DynMetastore {
    SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database")
}

/// The signing keys of the servers of the tests.
pub fn security() -> Arc<SecurityContext> {
    static SECURITY: OnceLock<Arc<SecurityContext>> = OnceLock::new();
    SECURITY
        .get_or_init(|| Arc::new(SecurityContext::generate().unwrap()))
        .clone()
}

/// A UC access token of `user`, which the servers with [`security`] take.
pub fn token(user: &str) -> String {
    security()
        .create_access_token(user, Duration::from_secs(600))
        .unwrap()
}

/// A uc-server with a new database and no sign-in; its root URL.
pub async fn server() -> Url {
    serve(build_app(AppState::new(metastore().await))).await
}

/// A client of a new [`server`].
pub async fn client() -> Client {
    Client::new_unauthenticated(server().await)
}

/// A uc-server as [`server`] that also takes the tokens of [`token`]: a
/// token names its caller. Its root URL.
pub async fn server_with_keys() -> Url {
    let state = AppState::new(metastore().await).with_security(security());
    serve(build_app(state)).await
}

/// The catalog `c`, with its managed storage in the returned directory,
/// and its schema `c.s`.
pub async fn catalog_and_schema(uc: &Client) -> TempDir {
    let storage = tempfile::tempdir().unwrap();
    uc.create_catalog("c")
        .with_storage_root(file_url(&storage))
        .await
        .unwrap();
    uc.create_schema("s", "c").await.unwrap();
    storage
}

/// The `file://` URL of `dir`.
pub fn file_url(dir: &TempDir) -> String {
    format!("file://{}", dir.path().display())
}

/// A nullable column `name` at `position`: of the Spark type `spark`, such
/// as `long`, which is `sql` in SQL, such as `bigint`.
pub fn column(
    name: &str,
    sql: &str,
    spark: &str,
    type_name: ColumnTypeName,
    position: i32,
) -> ColumnInfo {
    ColumnInfo {
        name: name.into(),
        type_text: Some(sql.into()),
        type_json: format!(
            r#"{{"name":"{name}","type":"{spark}","nullable":true,"metadata":{{}}}}"#
        ),
        type_name: Some(type_name.into()),
        position: Some(position),
        nullable: Some(true),
        ..Default::default()
    }
}

/// The EXTERNAL Delta table `c.s.<name>`, with the column `id`, in the
/// folder `name` of `storage`.
pub async fn external_table(uc: &Client, storage: &TempDir, name: &str) -> TableInfo {
    uc.create_table(name, "c", "s", TableType::External)
        .with_columns([column("id", "bigint", "long", ColumnTypeName::Long, 0)])
        .with_data_source_format(DataSourceFormat::Delta)
        .with_storage_location(format!("{}/{name}", file_url(storage)))
        .await
        .unwrap()
}

/// The primary email `address` of a SCIM user.
pub fn email(address: &str) -> ScimEmail {
    ScimEmail {
        value: address.into(),
        primary: Some(true),
        ..Default::default()
    }
}

/// The error answer of a call that fails.
#[track_caller]
pub fn api_error<T: Debug>(result: Result<T>) -> ApiError {
    match result {
        Err(Error::Api(error)) => error,
        other => panic!("not an error answer: {other:?}"),
    }
}

/// The error answer of a call that fails with HTTP `status` and the error
/// code `code`.
#[track_caller]
pub fn rejected<T: Debug>(result: Result<T>, status: u16, code: &str) -> ApiError {
    let error = api_error(result);
    assert_eq!(
        (error.status, error.code.as_deref()),
        (status, Some(code)),
        "{error:?}"
    );
    error
}
