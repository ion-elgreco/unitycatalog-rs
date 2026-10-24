//! The Iceberg REST catalog: a catalog is a warehouse, a schema a
//! namespace, and native Iceberg tables live in it.

use std::time::{SystemTime, UNIX_EPOCH};

use buffa_types::google::protobuf::{ListValue, Struct, Value};
use tempfile::TempDir;

use uc_client::Client;
use uc_client::models::iceberg::v1::{
    IcebergBlobMetadata, IcebergEncryptedKey, IcebergPartitionField, IcebergPartitionSpec,
    IcebergPartitionStatisticsFile, IcebergSchema, IcebergSnapshot, IcebergSortField,
    IcebergSortOrder, IcebergStatisticsFile, IcebergStructField, IcebergTableIdentifier,
    IcebergTableMetadata, IcebergTableRequirement, IcebergTableUpdate,
};
use uc_server::{AppState, build_app};

use crate::common::{client, file_url, metastore, rejected, serve};

/// The catalog `c`, with its managed storage in the returned directory,
/// and its namespace `s`.
async fn namespace(uc: &Client) -> TempDir {
    let storage = tempfile::tempdir().unwrap();
    uc.create_catalog("c")
        .with_storage_root(file_url(&storage))
        .await
        .unwrap();
    uc.create_iceberg_namespace("c", vec!["s".into()])
        .await
        .unwrap();
    storage
}

fn field(id: i32, name: &str, required: bool, type_name: &str) -> IcebergStructField {
    IcebergStructField {
        id,
        name: name.into(),
        required,
        field_type: Value::from(type_name).into(),
        ..Default::default()
    }
}

/// The schema `id long, data string`, with `category string` when `wide`.
fn schema(wide: bool) -> IcebergSchema {
    let mut fields = vec![
        field(1, "id", true, "long"),
        field(2, "data", false, "string"),
    ];
    if wide {
        fields.push(field(3, "category", false, "string"));
    }
    IcebergSchema {
        schema_type: "struct".into(),
        schema_id: i32::from(wide),
        fields,
        ..Default::default()
    }
}

fn identifier(namespace: &str, name: &str) -> IcebergTableIdentifier {
    IcebergTableIdentifier {
        namespace: vec![namespace.into()],
        name: name.into(),
        ..Default::default()
    }
}

/// A requirement `requirement_type`, with the fields that `set` sets.
fn requirement(
    requirement_type: &str,
    set: impl FnOnce(&mut IcebergTableRequirement),
) -> IcebergTableRequirement {
    let mut requirement = IcebergTableRequirement {
        requirement_type: requirement_type.into(),
        ..Default::default()
    };
    set(&mut requirement);
    requirement
}

/// An update `action`, with the fields that `set` sets.
fn update(action: &str, set: impl FnOnce(&mut IcebergTableUpdate)) -> IcebergTableUpdate {
    let mut update = IcebergTableUpdate {
        action: action.into(),
        ..Default::default()
    };
    set(&mut update);
    update
}

/// The metadata of the table `c.s.events` after a commit.
async fn commit(
    uc: &Client,
    requirements: Vec<IcebergTableRequirement>,
    updates: Vec<IcebergTableUpdate>,
) -> IcebergTableMetadata {
    let committed = uc
        .commit_iceberg_table("c", "s", "events")
        .with_requirements(requirements)
        .with_updates(updates)
        .await
        .unwrap();
    committed.metadata.into_option().unwrap()
}

/// The table `c.s.events` with [`schema`]; its metadata.
async fn events(uc: &Client) -> IcebergTableMetadata {
    let created = uc
        .create_iceberg_table("c", "s", "events", schema(false))
        .await
        .unwrap();
    created.metadata.into_option().unwrap()
}

#[tokio::test]
async fn config() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let config = uc.get_iceberg_config("c").await.unwrap();
    assert_eq!(config.overrides["prefix"], "catalogs/c");
    assert!(config.defaults.is_empty());
    assert_eq!(config.endpoints.len(), 16);
    let namespaces = "GET /v1/{prefix}/namespaces".to_string();
    assert!(config.endpoints.contains(&namespaces));
    // Errors have the body of the Iceberg REST API.
    let error = rejected(
        uc.get_iceberg_config("nope").await,
        404,
        "NoSuchNamespaceException",
    );
    assert_eq!(error.message, "Catalog not found: nope");
}

#[tokio::test]
async fn namespaces() {
    let uc = client().await;
    let storage = tempfile::tempdir().unwrap();
    uc.create_catalog("c")
        .with_storage_root(file_url(&storage))
        .await
        .unwrap();
    let created = uc
        .create_iceberg_namespace("c", vec!["s".into()])
        .with_properties([("owner", "data")])
        .await
        .unwrap();
    assert_eq!(created.namespace, ["s"]);
    assert_eq!(created.properties["owner"], "data");
    uc.create_iceberg_namespace("c", vec!["t".into()])
        .await
        .unwrap();
    let all = uc.list_iceberg_namespaces("c").await.unwrap();
    let [s, t] = ["s", "t"].map(|n| ListValue::from_values([n]));
    assert_eq!(all.namespaces, [s.clone(), t.clone()]);
    assert_eq!(all.next_page_token, None);
    // An empty token asks for the first page.
    let first = uc
        .list_iceberg_namespaces("c")
        .with_page_token(String::new())
        .with_page_size(1)
        .await
        .unwrap();
    assert_eq!(first.namespaces, [s]);
    let second = uc
        .list_iceberg_namespaces("c")
        .with_page_token(first.next_page_token)
        .with_page_size(1)
        .await
        .unwrap();
    assert_eq!(second.namespaces, [t]);
    // Namespaces are not nested.
    let children = uc
        .list_iceberg_namespaces("c")
        .with_parent("s".to_string())
        .await
        .unwrap();
    assert!(children.namespaces.is_empty());

    assert_eq!(uc.load_iceberg_namespace("c", "s").await.unwrap(), created);
    assert!(uc.iceberg_namespace_exists("c", "s").await.unwrap());
    assert!(!uc.iceberg_namespace_exists("c", "nope").await.unwrap());
    let changes = uc
        .update_iceberg_namespace_properties("c", "s")
        .with_updates([("added", "value")])
        .with_removals(["owner".to_string(), "absent".to_string()])
        .await
        .unwrap();
    assert_eq!(changes.updated, ["added"]);
    assert_eq!(changes.removed, ["owner"]);
    assert_eq!(changes.missing, ["absent"]);
    let loaded = uc.load_iceberg_namespace("c", "s").await.unwrap();
    assert_eq!(loaded.properties.len(), 1);
    assert_eq!(loaded.properties["added"], "value");

    uc.drop_iceberg_namespace("c", "t").await.unwrap();
    assert!(!uc.iceberg_namespace_exists("c", "t").await.unwrap());
    rejected(
        uc.load_iceberg_namespace("c", "t").await,
        404,
        "NoSuchNamespaceException",
    );
}

/// A native table: create, exists, list, load, credentials, rename and
/// drop. Views are never found.
#[tokio::test]
async fn tables() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let created = uc
        .create_iceberg_table("c", "s", "events", schema(false))
        .with_properties([("created-by", "uc-client")])
        .await
        .unwrap();
    let metadata = created.metadata.as_option().unwrap();
    assert_eq!(metadata.properties["created-by"], "uc-client");
    assert_eq!(metadata.schemas[0].fields, schema(false).fields);
    assert!(uc.iceberg_table_exists("c", "s", "events").await.unwrap());
    assert!(!uc.iceberg_table_exists("c", "s", "nope").await.unwrap());
    let listed = uc.list_iceberg_tables("c", "s").await.unwrap();
    assert_eq!(listed.identifiers, [identifier("s", "events")]);
    let page = uc
        .list_iceberg_tables("c", "s")
        .with_page_token(String::new())
        .with_page_size(1)
        .await
        .unwrap();
    assert_eq!(page.identifiers, [identifier("s", "events")]);

    let loaded = uc
        .load_iceberg_table("c", "s", "events")
        .with_snapshots("all".to_string())
        .await
        .unwrap();
    assert_eq!(loaded.metadata_location, created.metadata_location);
    let etag = loaded.etag.clone().unwrap();
    assert!(etag.starts_with("W/\""), "{etag}");
    assert!(loaded.config.is_empty());
    // The snapshots mode has a tag of its own.
    let refs = uc
        .load_iceberg_table("c", "s", "events")
        .with_snapshots("refs".to_string())
        .await
        .unwrap();
    assert_ne!(refs.etag, loaded.etag);
    let error = rejected(
        uc.load_iceberg_table("c", "s", "events")
            .with_snapshots("some".to_string())
            .await,
        400,
        "BadRequestException",
    );
    let message = "Invalid snapshots parameter: some. Valid values are all and refs.";
    assert_eq!(error.message, message);
    let credentials = uc
        .load_iceberg_table_credentials("c", "s", "events")
        .await
        .unwrap();
    assert!(credentials.storage_credentials.is_empty());
    rejected(
        uc.load_iceberg_view("c", "s", "v").await,
        404,
        "NoSuchViewException",
    );

    let renamed = identifier("s", "renamed");
    uc.rename_iceberg_table("c", identifier("s", "events"), renamed)
        .await
        .unwrap();
    assert!(uc.iceberg_table_exists("c", "s", "renamed").await.unwrap());
    assert!(!uc.iceberg_table_exists("c", "s", "events").await.unwrap());
    uc.drop_iceberg_table("c", "s", "renamed")
        .with_purge_requested(true)
        .await
        .unwrap();
    rejected(
        uc.load_iceberg_table("c", "s", "renamed").await,
        404,
        "NoSuchTableException",
    );
}

/// A commit writes the next metadata file; a requirement that no longer
/// holds fails the commit.
#[tokio::test]
async fn commits() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let first = uc
        .create_iceberg_table("c", "s", "events", schema(false))
        .await
        .unwrap();
    let uuid = first.metadata.table_uuid.clone();
    let committed = uc
        .commit_iceberg_table("c", "s", "events")
        .with_identifier(identifier("s", "events"))
        .with_requirements([requirement("assert-table-uuid", |r| {
            r.uuid = Some(uuid.clone())
        })])
        .with_updates([
            update("add-schema", |u| u.schema = schema(true).into()),
            update("set-current-schema", |u| u.schema_id = Some(-1)),
            update("set-properties", |u| {
                u.updates = [("foo".into(), "bar".into()), ("gone".into(), "x".into())].into()
            }),
        ])
        .await
        .unwrap();
    assert_ne!(committed.metadata_location, first.metadata_location);
    let metadata = committed.metadata.as_option().unwrap();
    assert_eq!(metadata.current_schema_id, 1);
    assert_eq!(metadata.properties["foo"], "bar");
    let log = &metadata.metadata_log[0].metadata_file;
    assert_eq!(Some(log), first.metadata_location.as_ref());

    let removed = commit(
        &uc,
        vec![],
        vec![update("remove-properties", |u| {
            u.removals = vec!["gone".into()]
        })],
    )
    .await;
    assert!(!removed.properties.contains_key("gone"));
    let other = "00000000-0000-0000-0000-000000000000".to_string();
    let conflict = uc
        .commit_iceberg_table("c", "s", "events")
        .with_requirements([requirement("assert-table-uuid", |r| r.uuid = Some(other))])
        .await;
    rejected(conflict, 409, "CommitFailedException");
}

/// A scan report and a commit report.
#[tokio::test]
async fn metrics() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    events(&uc).await;
    let scan = |table: &str| {
        let counter =
            Struct::from_fields([("unit", Value::from("count")), ("value", Value::from(1))]);
        uc.report_iceberg_metrics("c", "s", table, "scan-report")
            .with_table_name(format!("c.s.{table}"))
            .with_snapshot_id(1)
            .with_filter(Value::from(true))
            .with_schema_id(0)
            .with_projected_field_ids([1])
            .with_projected_field_names(["id".to_string()])
            .with_metrics(Struct::from_fields([("total-planning-duration", counter)]))
            .with_metadata([("engine", "uc-client")])
    };
    scan("events").await.unwrap();
    uc.report_iceberg_metrics("c", "s", "events", "commit-report")
        .with_table_name("c.s.events".to_string())
        .with_snapshot_id(1)
        .with_sequence_number(1)
        .with_operation("append".to_string())
        .with_metrics(Struct::new())
        .await
        .unwrap();
    rejected(scan("nope").await, 404, "NoSuchTableException");
    let error = rejected(
        uc.report_iceberg_metrics("c", "s", "events", "scan-report")
            .await,
        400,
        "BadRequestException",
    );
    assert_eq!(error.message, "Cannot parse missing string: table-name");
}

/// A staged create holds the metadata of the table without creating it; a
/// commit with `assert-create` creates it.
#[tokio::test]
async fn staged_creates() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let staged = uc
        .create_iceberg_table("c", "s", "events", schema(false))
        .with_stage_create(true)
        .await
        .unwrap();
    assert_eq!(staged.metadata_location, None);
    assert!(!uc.iceberg_table_exists("c", "s", "events").await.unwrap());
    let staged = staged.metadata.into_option().unwrap();
    let uuid = staged.table_uuid.clone();
    let created = commit(
        &uc,
        vec![requirement("assert-create", |_| {})],
        vec![
            update("assign-uuid", |u| u.uuid = Some(uuid)),
            update("upgrade-format-version", |u| u.format_version = Some(2)),
            update("add-schema", |u| u.schema = schema(false).into()),
            update("set-current-schema", |u| u.schema_id = Some(-1)),
            update("add-spec", |u| {
                u.spec = IcebergPartitionSpec::default().into()
            }),
            update("set-default-spec", |u| u.spec_id = Some(-1)),
            update("add-sort-order", |u| {
                u.sort_order = IcebergSortOrder::default().into()
            }),
            update("set-default-sort-order", |u| u.sort_order_id = Some(-1)),
            update("set-location", |u| {
                u.location = Some(staged.location.clone())
            }),
            update("set-properties", |u| {
                u.updates = [("staged".into(), "true".into())].into()
            }),
        ],
    )
    .await;
    assert_eq!(created.table_uuid, staged.table_uuid);
    assert_eq!(created.properties["staged"], "true");
    assert!(uc.iceberg_table_exists("c", "s", "events").await.unwrap());
}

/// A table at a location of its own: EXTERNAL, below a local root that the
/// server allows.
#[tokio::test]
async fn external_tables() {
    let root = tempfile::tempdir().unwrap();
    let state =
        AppState::new(metastore().await).with_external_local_roots(vec![root.path().to_path_buf()]);
    let uc = Client::new_unauthenticated(serve(build_app(state)).await);
    let _storage = namespace(&uc).await;
    let location = format!("{}/events", file_url(&root));
    let created = uc
        .create_iceberg_table("c", "s", "events", schema(false))
        .with_location(location.clone())
        .await
        .unwrap();
    assert_eq!(created.metadata.location, location);
    let table = uc.table("c.s.events").get().await.unwrap();
    assert_eq!(
        table.table_type,
        uc_client::models::tables::v1::TableType::External
    );
}

/// A table created with a partition spec and a sort order; a commit that
/// requires each id the table assigned and changes the spec, the sort order
/// and the schema; a commit that removes the old ones.
#[tokio::test]
async fn specs_sort_orders_and_schemas() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let bucket = IcebergPartitionField {
        source_id: 1,
        name: "id_bucket".into(),
        transform: "bucket[4]".into(),
        ..Default::default()
    };
    let by_id = IcebergSortField {
        source_id: 1,
        transform: "identity".into(),
        direction: "asc".into(),
        null_order: "nulls-first".into(),
        ..Default::default()
    };
    let created = uc
        .create_iceberg_table("c", "s", "events", schema(false))
        .with_partition_spec(IcebergPartitionSpec {
            fields: vec![bucket],
            ..Default::default()
        })
        .with_write_order(IcebergSortOrder {
            order_id: 1,
            fields: vec![by_id.clone()],
            ..Default::default()
        })
        .await
        .unwrap();
    let metadata = created.metadata.into_option().unwrap();
    let spec = &metadata.partition_specs[0].fields[0];
    assert_eq!(
        (spec.name.as_str(), spec.transform.as_str()),
        ("id_bucket", "bucket[4]")
    );
    let default_order = metadata.default_sort_order_id;
    let order = metadata
        .sort_orders
        .iter()
        .find(|o| o.order_id == default_order);
    assert_eq!(order.unwrap().fields, [by_id]);

    let requirements = vec![
        requirement("assert-table-uuid", |r| {
            r.uuid = Some(metadata.table_uuid.clone())
        }),
        requirement("assert-current-schema-id", |r| {
            r.current_schema_id = Some(metadata.current_schema_id)
        }),
        requirement("assert-last-assigned-field-id", |r| {
            r.last_assigned_field_id = Some(metadata.last_column_id)
        }),
        requirement("assert-last-assigned-partition-id", |r| {
            r.last_assigned_partition_id = Some(metadata.last_partition_id)
        }),
        requirement("assert-default-spec-id", |r| {
            r.default_spec_id = Some(metadata.default_spec_id)
        }),
        requirement("assert-default-sort-order-id", |r| {
            r.default_sort_order_id = Some(default_order)
        }),
    ];
    let updated = commit(
        &uc,
        requirements,
        vec![
            update("add-schema", |u| u.schema = schema(true).into()),
            update("set-current-schema", |u| u.schema_id = Some(-1)),
            update("add-spec", |u| {
                u.spec = IcebergPartitionSpec::default().into()
            }),
            update("set-default-spec", |u| u.spec_id = Some(-1)),
            update("add-sort-order", |u| {
                u.sort_order = IcebergSortOrder::default().into()
            }),
            update("set-default-sort-order", |u| u.sort_order_id = Some(-1)),
        ],
    )
    .await;
    assert_eq!(updated.current_schema_id, 1);
    assert_ne!(updated.default_spec_id, metadata.default_spec_id);
    assert_eq!(updated.default_sort_order_id, 0);

    let removed = commit(
        &uc,
        vec![],
        vec![
            update("remove-partition-specs", |u| {
                u.spec_ids = vec![metadata.default_spec_id]
            }),
            update("remove-schemas", |u| u.schema_ids = vec![0]),
        ],
    )
    .await;
    assert_eq!(removed.partition_specs.len(), 1);
    assert_eq!(removed.schemas.len(), 1);
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// The snapshot `id` of the table at `location`, an append.
fn snapshot(location: &str, id: i64) -> IcebergSnapshot {
    IcebergSnapshot {
        snapshot_id: id,
        sequence_number: id,
        timestamp_ms: now_ms(),
        manifest_list: format!("{location}/metadata/snap-{id}.avro"),
        summary: [("operation".into(), "append".into())].into(),
        schema_id: Some(0),
        ..Default::default()
    }
}

/// A reference `name` of `ref_type` (`branch`, `tag`) to the snapshot `id`.
fn reference(name: &str, ref_type: &str, id: i64) -> impl FnOnce(&mut IcebergTableUpdate) {
    let (name, ref_type) = (name.to_string(), ref_type.to_string());
    move |u| {
        u.ref_name = Some(name);
        u.ref_type = Some(ref_type);
        u.snapshot_id = Some(id);
    }
}

/// A snapshot, and the branch `main` and the tag `v1` that point at it;
/// then their removal.
#[tokio::test]
async fn snapshots_and_refs() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let first = snapshot(&events(&uc).await.location, 1);
    // `main` does not exist yet.
    let main_is_new = requirement("assert-ref-snapshot-id", |r| {
        r.ref_name = Some("main".into())
    });
    let added = commit(
        &uc,
        vec![main_is_new],
        vec![
            update("add-snapshot", |u| u.snapshot = first.clone().into()),
            update("set-snapshot-ref", |u| {
                reference("main", "branch", 1)(u);
                u.min_snapshots_to_keep = Some(1);
                u.max_snapshot_age_ms = Some(86_400_000);
            }),
        ],
    )
    .await;
    assert_eq!(added.current_snapshot_id, Some(1));
    assert_eq!(added.snapshots, [first]);
    let main = &added.refs["main"];
    assert_eq!((main.snapshot_id, main.r#type.as_str()), (1, "branch"));
    assert_eq!(main.min_snapshots_to_keep, Some(1));

    let main_at_1 = requirement("assert-ref-snapshot-id", |r| {
        r.ref_name = Some("main".into());
        r.snapshot_id = Some(1);
    });
    let tagged = commit(
        &uc,
        vec![main_at_1],
        vec![update("set-snapshot-ref", |u| {
            reference("v1", "tag", 1)(u);
            u.max_ref_age_ms = Some(86_400_000);
        })],
    )
    .await;
    let tag = &tagged.refs["v1"];
    assert_eq!(
        (tag.r#type.as_str(), tag.max_ref_age_ms),
        ("tag", Some(86_400_000))
    );

    let removed = commit(
        &uc,
        vec![],
        vec![
            update("remove-snapshot-ref", |u| u.ref_name = Some("v1".into())),
            update("remove-snapshot-ref", |u| u.ref_name = Some("main".into())),
            update("remove-snapshots", |u| u.snapshot_ids = vec![1]),
        ],
    )
    .await;
    assert_eq!(removed.current_snapshot_id, None);
    assert!(removed.refs.is_empty() && removed.snapshots.is_empty());
}

/// The statistics and partition statistics of a snapshot; then their
/// removal.
#[tokio::test]
async fn statistics() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let location = events(&uc).await.location;
    let statistics = IcebergStatisticsFile {
        snapshot_id: 1,
        statistics_path: format!("{location}/metadata/1.stats"),
        file_size_in_bytes: 100,
        file_footer_size_in_bytes: 10,
        blob_metadata: vec![IcebergBlobMetadata {
            blob_type: "apache-datasketches-theta-v1".into(),
            snapshot_id: 1,
            sequence_number: 1,
            fields: vec![1],
            ..Default::default()
        }],
        ..Default::default()
    };
    let partition_statistics = IcebergPartitionStatisticsFile {
        snapshot_id: 1,
        statistics_path: format!("{location}/metadata/1.partition-stats"),
        file_size_in_bytes: 50,
        ..Default::default()
    };
    let set = commit(
        &uc,
        vec![],
        vec![
            update("add-snapshot", |u| {
                u.snapshot = snapshot(&location, 1).into()
            }),
            update("set-statistics", |u| {
                u.snapshot_id = Some(1);
                u.statistics = statistics.clone().into();
            }),
            update("set-partition-statistics", |u| {
                u.partition_statistics = partition_statistics.clone().into()
            }),
        ],
    )
    .await;
    assert_eq!(set.statistics, [statistics]);
    assert_eq!(set.partition_statistics, [partition_statistics]);

    let removed = commit(
        &uc,
        vec![],
        vec![
            update("remove-statistics", |u| u.snapshot_id = Some(1)),
            update("remove-partition-statistics", |u| u.snapshot_id = Some(1)),
        ],
    )
    .await;
    assert!(removed.statistics.is_empty() && removed.partition_statistics.is_empty());
}

/// Format version 3, and its encryption keys.
#[tokio::test]
async fn format_versions_and_encryption_keys() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    assert_eq!(events(&uc).await.format_version, 2);
    let key = IcebergEncryptedKey {
        key_id: "k1".into(),
        encrypted_key_metadata: "AAEC".into(),
        encrypted_by_id: Some("kms".into()),
        properties: [("source".into(), "test".into())].into(),
        ..Default::default()
    };
    let upgraded = commit(
        &uc,
        vec![],
        vec![
            update("upgrade-format-version", |u| u.format_version = Some(3)),
            update("add-encryption-key", |u| {
                u.encryption_key = key.clone().into()
            }),
        ],
    )
    .await;
    assert_eq!(upgraded.format_version, 3);
    assert_eq!(upgraded.encryption_keys, [key]);
    let removed = commit(
        &uc,
        vec![],
        vec![update("remove-encryption-key", |u| {
            u.key_id = Some("k1".into())
        })],
    )
    .await;
    assert!(removed.encryption_keys.is_empty());
}

/// A snapshot id is any long, 0 too. The client leaves a zero
/// `snapshot-id` out of the snapshot, and the server answers 400
/// `Malformed request body`.
#[tokio::test]
async fn a_snapshot_id_may_be_zero() {
    let uc = client().await;
    let _storage = namespace(&uc).await;
    let location = events(&uc).await.location;
    let snapshot = IcebergSnapshot {
        snapshot_id: 0,
        sequence_number: 1,
        timestamp_ms: now_ms(),
        manifest_list: format!("{location}/metadata/snap-0.avro"),
        summary: [("operation".into(), "append".into())].into(),
        ..Default::default()
    };
    let added = commit(
        &uc,
        vec![],
        vec![update("add-snapshot", |u| u.snapshot = snapshot.into())],
    )
    .await;
    assert_eq!(added.snapshots[0].snapshot_id, 0);
}
