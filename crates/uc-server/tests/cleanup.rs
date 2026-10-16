//! Upstream storage cleanup: dropping managed data queues a task in the
//! drop transaction, and the worker deletes the files after the initial
//! delay.

mod common;

use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    Router,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use uc_common::NormalizedUrl;
use uc_metastore::DynMetastore;
use uc_server::{
    storage::cleanup::{
        CleanupConfig, CleanupError, CleanupStorage, DefaultCleanupStorage, StorageCleanupWorker,
    },
    storage::vendor::PlaceholderVendor,
};

use common::{assert_status, json_body, req, req_json};

const HOUR: Duration = Duration::from_secs(3_600);

async fn ok(app: &Router, method: Method, path: &str, body: Option<Value>) -> Value {
    let request = match body {
        Some(b) => req_json(method.clone(), path, b),
        None => req(method.clone(), path),
    };
    let resp = app.clone().oneshot(request).await.unwrap();
    let resp = assert_status(resp, StatusCode::OK, &format!("{method} {path}")).await;
    if method == Method::DELETE {
        Value::Null
    } else {
        json_body(resp).await
    }
}

fn worker_with(
    metastore: &DynMetastore,
    storage: Arc<dyn CleanupStorage>,
    initial_delay: Duration,
    retry_backoff: Duration,
) -> StorageCleanupWorker {
    let config = CleanupConfig {
        initial_delay,
        retry_backoff,
        ..Default::default()
    };
    StorageCleanupWorker::new(metastore.clone(), storage, config)
}

fn worker(metastore: &DynMetastore, initial_delay: Duration) -> StorageCleanupWorker {
    let storage = Arc::new(DefaultCleanupStorage::new(
        metastore.clone(),
        Arc::new(PlaceholderVendor),
    ));
    worker_with(metastore, storage, initial_delay, HOUR)
}

fn local(url: &Value) -> PathBuf {
    PathBuf::from(url.as_str().unwrap().strip_prefix("file://").unwrap())
}

/// A managed catalog `c.s` rooted in a fresh temp directory.
async fn managed_schema(app: &Router) {
    let root = common::temp_storage_root();
    ok(
        app,
        Method::POST,
        "/catalogs",
        Some(json!({ "name": "c", "storage_root": root })),
    )
    .await;
    ok(
        app,
        Method::POST,
        "/schemas",
        Some(json!({ "name": "s", "catalog_name": "c" })),
    )
    .await;
}

/// A managed Delta table `c.s.<name>` through the staging flow, with a file
/// in its directory.
async fn managed_table(app: &Router, name: &str) -> Value {
    let staging = ok(
        app,
        Method::POST,
        "/staging-tables",
        Some(json!({ "name": name, "catalog_name": "c", "schema_name": "s" })),
    )
    .await;
    let table = ok(
        app,
        Method::POST,
        "/tables",
        Some(json!({
            "name": name, "catalog_name": "c", "schema_name": "s",
            "table_type": "MANAGED", "data_source_format": "DELTA",
            "storage_location": staging["staging_location"],
            "columns": [{ "name": "id", "type_name": "LONG", "type_text": "long", "position": 0,
                "type_json": r#"{"name":"id","type":"long","nullable":true,"metadata":{}}"# }],
            "properties": { "io.unitycatalog.tableId": staging["id"] },
        })),
    )
    .await;
    let dir = local(&table["storage_location"]);
    std::fs::create_dir_all(dir.join("_delta_log")).unwrap();
    std::fs::write(dir.join("_delta_log/0.json"), "{}").unwrap();
    table
}

async fn task_kinds(metastore: &DynMetastore) -> Vec<String> {
    let mut kinds: Vec<String> = metastore
        .cleanup()
        .list_tasks()
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.resource_type)
        .collect();
    kinds.sort();
    kinds
}

#[tokio::test]
async fn dropped_managed_data_is_cleaned_up_after_the_delay() {
    let (app, metastore) = common::app_with_metastore().await;
    managed_schema(&app).await;

    let table = managed_table(&app, "t").await;
    let table_id = table["table_id"].as_str().unwrap().to_string();
    let commit = uc_api::DeltaCommit {
        table_id: Some(table_id.clone()),
        table_uri: table["storage_location"].as_str().map(String::from),
        commit_info: Some(uc_api::DeltaCommitInfo {
            version: Some(1),
            timestamp: Some(1),
            file_name: Some("c1.json".into()),
            file_size: Some(1),
            file_modification_timestamp: Some(1),
        }),
        ..Default::default()
    };
    metastore.delta_tables().commit(commit, None).await.unwrap();

    let volume = ok(
        &app,
        Method::POST,
        "/volumes",
        Some(json!({
            "name": "v", "catalog_name": "c", "schema_name": "s", "volume_type": "MANAGED",
        })),
    )
    .await;
    let volume_dir = local(&volume["storage_location"]);
    std::fs::create_dir_all(&volume_dir).unwrap();
    std::fs::write(volume_dir.join("data.bin"), "x").unwrap();

    let model = ok(
        &app,
        Method::POST,
        "/models",
        Some(json!({ "name": "m", "catalog_name": "c", "schema_name": "s" })),
    )
    .await;
    let version = ok(
        &app,
        Method::POST,
        "/models/versions",
        Some(json!({
            "model_name": "m", "catalog_name": "c", "schema_name": "s", "source": "s3://run",
        })),
    )
    .await;
    std::fs::write(local(&version["storage_location"]).join("model.pkl"), "x").unwrap();

    // External data is never cleaned up.
    ok(
        &app,
        Method::POST,
        "/volumes",
        Some(json!({
            "name": "ext", "catalog_name": "c", "schema_name": "s", "volume_type": "EXTERNAL",
            "storage_location": "s3://elsewhere/ext",
        })),
    )
    .await;

    for path in [
        "/tables/c.s.t",
        "/volumes/c.s.v",
        "/volumes/c.s.ext",
        "/models/c.s.m/versions/1",
        "/models/c.s.m",
    ] {
        ok(&app, Method::DELETE, path, None).await;
    }
    assert_eq!(
        task_kinds(&metastore).await,
        ["MODEL_VERSION", "REGISTERED_MODEL", "TABLE", "VOLUME"]
    );
    // The dropped table's Delta commits go with it.
    let backup = app
        .clone()
        .oneshot(common::as_admin(req(Method::GET, "/admin/backup")))
        .await
        .unwrap();
    let snapshot = json_body(assert_status(backup, StatusCode::OK, "backup").await).await;
    assert_eq!(snapshot["tables"]["delta_commit"], json!([]), "{table_id}");

    // Files stay for the initial delay.
    assert!(!worker(&metastore, HOUR).run_once().await.unwrap());
    assert!(volume_dir.exists());

    // Then one task per run.
    let worker = worker(&metastore, Duration::ZERO);
    for _ in 0..4 {
        assert!(worker.run_once().await.unwrap());
    }
    assert!(!worker.run_once().await.unwrap());
    assert!(task_kinds(&metastore).await.is_empty());
    for dir in [
        local(&table["storage_location"]),
        volume_dir,
        local(&model["storage_location"]),
    ] {
        assert!(!dir.exists(), "{}", dir.display());
    }
}

#[tokio::test]
async fn cascades_queue_cleanup_for_every_managed_child() {
    let (app, metastore) = common::app_with_metastore().await;
    managed_schema(&app).await;
    managed_table(&app, "t").await;
    ok(&app, Method::POST, "/tables", Some(json!({
        "name": "ext", "catalog_name": "c", "schema_name": "s",
        "table_type": "EXTERNAL", "data_source_format": "DELTA", "storage_location": "s3://lake/ext",
    }))).await;
    ok(
        &app,
        Method::POST,
        "/volumes",
        Some(json!({
            "name": "v", "catalog_name": "c", "schema_name": "s", "volume_type": "MANAGED",
        })),
    )
    .await;
    ok(
        &app,
        Method::POST,
        "/models",
        Some(json!({ "name": "m", "catalog_name": "c", "schema_name": "s" })),
    )
    .await;
    ok(
        &app,
        Method::POST,
        "/models/versions",
        Some(json!({
            "model_name": "m", "catalog_name": "c", "schema_name": "s", "source": "s3://run",
        })),
    )
    .await;

    ok(&app, Method::DELETE, "/catalogs/c?force=true", None).await;
    // One model task covers its versions.
    assert_eq!(
        task_kinds(&metastore).await,
        ["REGISTERED_MODEL", "TABLE", "VOLUME"]
    );
}

/// Storage that always fails.
struct Broken;

#[async_trait]
impl CleanupStorage for Broken {
    async fn delete_prefix(&self, _: &NormalizedUrl) -> Result<(), CleanupError> {
        Err(CleanupError::Storage(
            "access denied for s3://secret/path".into(),
        ))
    }
}

#[tokio::test]
async fn failures_are_recorded_and_retried_after_the_backoff() {
    let (app, metastore) = common::app_with_metastore().await;
    managed_schema(&app).await;
    ok(
        &app,
        Method::POST,
        "/volumes",
        Some(json!({
            "name": "v", "catalog_name": "c", "schema_name": "s", "volume_type": "MANAGED",
        })),
    )
    .await;
    ok(&app, Method::DELETE, "/volumes/c.s.v", None).await;

    // Without a backoff the task is ready again at once.
    let eager = worker_with(&metastore, Arc::new(Broken), Duration::ZERO, Duration::ZERO);
    assert!(eager.run_once().await.unwrap());
    assert!(eager.run_once().await.unwrap());
    let task = &metastore.cleanup().list_tasks().await.unwrap()[0];
    assert_eq!(task.failure_count, 2);
    // Only the sanitized kind is kept, never the provider message.
    assert_eq!(
        task.last_error.as_deref(),
        Some("Storage cleanup failed: StorageError")
    );

    // With a backoff it waits.
    let patient = worker_with(&metastore, Arc::new(Broken), Duration::ZERO, HOUR);
    assert!(patient.run_once().await.unwrap());
    assert!(!patient.run_once().await.unwrap());
    assert_eq!(
        metastore.cleanup().list_tasks().await.unwrap()[0].failure_count,
        3
    );
}

#[tokio::test]
async fn leases_guard_claimed_tasks() {
    let (app, metastore) = common::app_with_metastore().await;
    managed_schema(&app).await;
    ok(
        &app,
        Method::POST,
        "/volumes",
        Some(json!({
            "name": "v", "catalog_name": "c", "schema_name": "s", "volume_type": "MANAGED",
        })),
    )
    .await;
    ok(&app, Method::DELETE, "/volumes/c.s.v", None).await;

    let tasks = metastore.cleanup();
    let claim = tasks
        .claim(HOUR, Duration::ZERO)
        .await
        .unwrap()
        .expect("ready task");
    assert_eq!(claim.resource_type, "VOLUME");
    // A live lease hides the task from other workers.
    assert_eq!(tasks.claim(HOUR, Duration::ZERO).await.unwrap(), None);
    // Only the lease holder may finish or fail it.
    assert!(!tasks.finish(&claim.resource_id, "other").await.unwrap());
    assert!(
        !tasks
            .report_failure(&claim.resource_id, "other", "x", HOUR)
            .await
            .unwrap()
    );
    assert!(
        tasks
            .finish(&claim.resource_id, &claim.lease_token)
            .await
            .unwrap()
    );
    assert!(tasks.list_tasks().await.unwrap().is_empty());
}
