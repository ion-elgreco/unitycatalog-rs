//! The Delta REST API (`/delta/v1`) and the commit coordinator of MANAGED
//! tables (`/delta/preview/commits`).

use buffa_types::google::protobuf::{Struct, Value};
use tempfile::TempDir;

use uc_client::Client;
use uc_client::models::delta::v1::{
    DeltaApiCommit, DeltaCommitInfo, DeltaCommitReport, DeltaCredentialOperation,
    DeltaDomainMetadataUpdates, DeltaFileSizeHistogram, DeltaLoadTableResponse,
    DeltaMaintenanceOperation, DeltaMetadata, DeltaMetricsReport, DeltaProtocol,
    DeltaRowTrackingDomainMetadata, DeltaStructField, DeltaStructType, DeltaTableRequirement,
    DeltaTableType, DeltaTableUpdate, DeltaUniform, DeltaUniformIceberg, DeltaUniformMetadata,
    DeltaUniformMetadataIceberg,
};

use crate::common::{catalog_and_schema, client, file_url, rejected};

const MISSING: &str = "00000000-0000-0000-0000-000000000000";

/// A field `name` of the primitive type `type_name`.
fn field(name: &str, type_name: &str, nullable: bool) -> DeltaStructField {
    DeltaStructField {
        name: name.into(),
        data_type: Value::from(type_name).into(),
        nullable,
        metadata: Struct::new().into(),
        ..Default::default()
    }
}

/// The columns `id long, amount double`, with `flag boolean` when `wide`.
fn columns(wide: bool) -> DeltaStructType {
    let mut fields = vec![field("id", "long", false), field("amount", "double", true)];
    if wide {
        fields.push(field("flag", "boolean", true));
    }
    DeltaStructType {
        type_name: "struct".into(),
        fields,
        ..Default::default()
    }
}

/// The requirements of an update of `table`: its id and its etag.
fn requirements(table: &DeltaLoadTableResponse) -> Vec<DeltaTableRequirement> {
    vec![
        DeltaTableRequirement {
            requirement_type: "assert-table-uuid".into(),
            uuid: Some(table.metadata.table_uuid.clone()),
            ..Default::default()
        },
        DeltaTableRequirement {
            requirement_type: "assert-etag".into(),
            etag: Some(table.metadata.etag.clone()),
            ..Default::default()
        },
    ]
}

/// An update `action` with the fields that `set` sets.
fn update(action: &str, set: impl FnOnce(&mut DeltaTableUpdate)) -> DeltaTableUpdate {
    let mut update = DeltaTableUpdate {
        action: action.into(),
        ..Default::default()
    };
    set(&mut update);
    update
}

/// The EXTERNAL table `c.s.<name>`, in the folder `name` of `storage`.
async fn external(uc: &Client, storage: &TempDir, name: &str) -> DeltaLoadTableResponse {
    let protocol = DeltaProtocol {
        min_reader_version: Some(1),
        min_writer_version: Some(2),
        ..Default::default()
    };
    let location = format!("{}/{name}", file_url(storage));
    uc.create_delta_table(
        "c",
        "s",
        name,
        location,
        DeltaTableType::External,
        columns(false),
        protocol,
        1,
    )
    .await
    .unwrap()
}

/// The MANAGED table `c.s.<name>`, with the protocol and properties that
/// its staging table requires, and the `rowTracking` writer feature; a
/// UniForm table with `uniform`, whose metadata location is relative to
/// the table.
async fn managed(uc: &Client, name: &str, uniform: Option<(&str, i64)>) -> DeltaLoadTableResponse {
    let staging = uc.create_delta_staging_table("c", "s", name).await.unwrap();
    let mut properties: Vec<(String, String)> = staging
        .required_properties
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
        .collect();
    let mut protocol = staging.required_protocol.as_option().unwrap().clone();
    protocol.writer_features.push("rowTracking".into());
    let mut create = uc.create_delta_table(
        "c",
        "s",
        name,
        &staging.location,
        DeltaTableType::Managed,
        columns(false),
        protocol,
        1_700_000_000_000,
    );
    if let Some((file, version)) = uniform {
        properties.push((
            "delta.universalFormat.enabledFormats".into(),
            "iceberg".into(),
        ));
        create = create.with_uniform(uniform_metadata(&staging.location, file, version));
    }
    create.with_properties(properties).await.unwrap()
}

/// UniForm metadata: Iceberg metadata in the file `file` of the table at
/// `location`, converted from the Delta version `version`.
fn uniform_metadata(location: &str, file: &str, version: i64) -> DeltaUniformMetadata {
    DeltaUniformMetadata {
        iceberg: DeltaUniformMetadataIceberg {
            metadata_location: Some(format!("{location}/{file}")),
            converted_delta_version: Some(version),
            converted_delta_timestamp: Some(1_700_000_000_000),
            ..Default::default()
        }
        .into(),
        ..Default::default()
    }
}

fn api_commit(version: i64) -> DeltaApiCommit {
    DeltaApiCommit {
        version,
        timestamp: 1_700_000_000_000 + version,
        file_name: format!("{version}.json"),
        file_size: 7,
        file_modification_timestamp: 1_700_000_000_000 + version,
        ..Default::default()
    }
}

#[tokio::test]
async fn config() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let config = uc.get_delta_config("c", "1.0").await.unwrap();
    assert_eq!(config.protocol_version, "1.0");
    assert_eq!(config.endpoints.len(), 12);
    let load = "GET /v1/catalogs/c/schemas/{schema}/tables/{table}";
    assert!(config.endpoints.contains(&load.to_string()));
    // Errors have the body of the Delta API: `{"error": {"type", ...}}`.
    let error = rejected(
        uc.get_delta_config("nope", "1.0").await,
        404,
        "NoSuchCatalogException",
    );
    assert_eq!(error.message, "Catalog not found: nope");
}

/// A MANAGED table: its staging table and its credentials, and the table,
/// with partition columns and domain metadata.
#[tokio::test]
async fn managed_tables() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let staging = uc.create_delta_staging_table("c", "s", "m").await.unwrap();
    assert_eq!(staging.table_type, DeltaTableType::Managed);
    let suffix = format!("/tables/{}", staging.table_id);
    assert!(staging.location.ends_with(&suffix));
    let id = staging.required_properties["io.unitycatalog.tableId"].as_str();
    assert_eq!(id, Some(staging.table_id.as_str()));
    assert!(staging.suggested_properties["delta.columnMapping.maxColumnId"].is_null());
    assert_eq!(staging.storage_credentials[0].prefix, staging.location);
    let credentials = uc
        .get_delta_staging_table_credentials(&staging.table_id)
        .await
        .unwrap();
    let [credential] = credentials.storage_credentials.as_slice() else {
        panic!("one credential: {credentials:?}");
    };
    assert_eq!(credential.prefix, staging.location);
    assert_eq!(credential.operation, DeltaCredentialOperation::ReadWrite);
    rejected(
        uc.get_delta_staging_table_credentials(MISSING).await,
        404,
        "NoSuchTableException",
    );

    let properties: Vec<(String, String)> = staging
        .required_properties
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
        .collect();
    let mut protocol = staging.required_protocol.as_option().unwrap().clone();
    protocol.writer_features.push("rowTracking".into());
    let tracking = DeltaRowTrackingDomainMetadata {
        row_id_high_water_mark: Some(10),
        ..Default::default()
    };
    let created = uc
        .create_delta_table(
            "c",
            "s",
            "m",
            &staging.location,
            DeltaTableType::Managed,
            columns(false),
            protocol,
            1_700_000_000_000,
        )
        .with_comment("sales".to_string())
        .with_properties(properties)
        .with_partition_columns(["id".to_string()])
        .with_domain_metadata(DeltaDomainMetadataUpdates {
            delta_row_tracking: tracking.into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let metadata = created.metadata.as_option().unwrap();
    assert_eq!(metadata.table_type, DeltaTableType::Managed);
    assert_eq!(metadata.table_uuid, staging.table_id);
    assert_eq!(metadata.location, staging.location);
    assert_eq!(metadata.partition_columns, ["id"]);
    let high_water_mark = &metadata.properties["delta.rowTracking.rowIdHighWaterMark"];
    assert_eq!(high_water_mark, "10");
    assert_eq!(metadata.last_commit_timestamp_ms, Some(1_700_000_000_000));
    assert!(created.commits.is_empty());
    assert_eq!(created.latest_table_version, Some(0));
    let table = uc.table("c.s.m").get().await.unwrap();
    assert_eq!(table.comment.as_deref(), Some("sales"));
}

/// The commit coordinator: a commit with a metadata change, the commits a
/// client reads back, and a backfill.
#[tokio::test]
async fn commits() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let id = managed(&uc, "m", None).await.metadata.table_uuid.clone();
    let uri = uc
        .table("c.s.m")
        .get()
        .await
        .unwrap()
        .storage_location
        .unwrap();
    let commit = DeltaCommitInfo {
        version: 1,
        timestamp: 11,
        file_name: "c1.json".into(),
        file_size: 7,
        file_modification_timestamp: 12,
        ..Default::default()
    };
    let description = DeltaMetadata {
        description: Some("sales, v1".into()),
        ..Default::default()
    };
    uc.commit_delta(&id, &uri)
        .with_commit_info(commit.clone())
        .with_metadata(description)
        .await
        .unwrap();
    let commits = uc
        .get_delta_commits(&id, 0)
        .with_table_uri(uri.clone())
        .with_end_version(1)
        .await
        .unwrap();
    assert_eq!(commits.commits, [commit]);
    assert_eq!(commits.latest_table_version, 1);
    let before = uc
        .get_delta_commits(&id, 0)
        .with_end_version(0)
        .await
        .unwrap();
    assert!(before.commits.is_empty());
    assert_eq!(before.latest_table_version, 1);
    let loaded = uc.load_delta_table("c", "s", "m").await.unwrap();
    assert_eq!(loaded.latest_table_version, Some(1));
    assert_eq!(loaded.commits[0].file_name, "c1.json");
    let table = uc.table("c.s.m").get().await.unwrap();
    assert_eq!(table.comment.as_deref(), Some("sales, v1"));

    uc.commit_delta(&id, &uri)
        .with_latest_backfilled_version(1)
        .await
        .unwrap();
    let commits = uc.get_delta_commits(&id, 0).await.unwrap();
    assert!(commits.commits.is_empty());
    assert_eq!(commits.latest_table_version, 1);
    rejected(
        uc.get_delta_commits(MISSING, 0).await,
        404,
        "TABLE_NOT_FOUND",
    );
}

/// The updates of a MANAGED table: a commit, a backfill, the protocol and
/// the domain metadata.
#[tokio::test]
async fn updates_of_managed_tables() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let table = managed(&uc, "m", None).await;
    let committed = uc
        .update_delta_table(
            "c",
            "s",
            "m",
            requirements(&table),
            vec![update("add-commit", |u| u.commit = api_commit(1).into())],
        )
        .await
        .unwrap();
    assert_eq!(committed.commits, [api_commit(1)]);
    assert_eq!(committed.latest_table_version, Some(1));
    let backfilled = uc
        .update_delta_table(
            "c",
            "s",
            "m",
            requirements(&committed),
            vec![update("set-latest-backfilled-version", |u| {
                u.latest_published_version = Some(1)
            })],
        )
        .await
        .unwrap();
    assert!(backfilled.commits.is_empty());

    // The protocol of the table, with the `domainMetadata` writer feature.
    let reader = [
        "catalogManaged",
        "v2Checkpoint",
        "vacuumProtocolCheck",
        "deletionVectors",
    ];
    let writer = ["inCommitTimestamp", "rowTracking", "domainMetadata"];
    let protocol = DeltaProtocol {
        min_reader_version: Some(3),
        min_writer_version: Some(7),
        reader_features: reader.map(String::from).into(),
        writer_features: reader
            .iter()
            .chain(&writer)
            .map(|f| f.to_string())
            .collect(),
        ..Default::default()
    };
    let watermark = Struct::from_fields([("rowIdHighWaterMark", 20)]);
    let set = uc
        .update_delta_table(
            "c",
            "s",
            "m",
            requirements(&backfilled),
            vec![
                update("set-protocol", |u| u.protocol = protocol.into()),
                update("set-domain-metadata", |u| {
                    u.updates
                        .insert("delta.rowTracking".into(), watermark.into());
                }),
            ],
        )
        .await
        .unwrap();
    let properties = &set.metadata.properties;
    assert_eq!(properties["delta.feature.domainMetadata"], "supported");
    assert_eq!(properties["delta.rowTracking.rowIdHighWaterMark"], "20");
    let removed = uc
        .update_delta_table(
            "c",
            "s",
            "m",
            requirements(&set),
            vec![update("remove-domain-metadata", |u| {
                u.domains = vec!["delta.rowTracking".into()]
            })],
        )
        .await
        .unwrap();
    let properties = &removed.metadata.properties;
    assert!(!properties.contains_key("delta.rowTracking.rowIdHighWaterMark"));
}

/// UniForm tables: their Iceberg metadata at create time, with a commit of
/// the Delta API, and with a commit of the commit coordinator.
#[tokio::test]
async fn uniform_tables() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let created = managed(&uc, "uni", Some(("_uniform/v1.json", 0))).await;
    let location = created.metadata.location.clone();
    let iceberg = created.uniform.iceberg.as_option().unwrap();
    let first = format!("{location}/_uniform/v1.json");
    assert_eq!(iceberg.metadata_location.as_deref(), Some(first.as_str()));
    assert_eq!(iceberg.converted_delta_version, Some(0));
    assert_eq!(iceberg.converted_delta_timestamp, Some(1_700_000_000_000));

    let committed = uc
        .update_delta_table(
            "c",
            "s",
            "uni",
            requirements(&created),
            vec![update("add-commit", |u| {
                u.commit = api_commit(1).into();
                u.uniform = uniform_metadata(&location, "v2.json", 1).into();
            })],
        )
        .await
        .unwrap();
    let iceberg = committed.uniform.iceberg.as_option().unwrap();
    let second = format!("{location}/v2.json");
    assert_eq!(iceberg.metadata_location.as_deref(), Some(second.as_str()));
    assert_eq!(iceberg.converted_delta_version, Some(1));

    let commit = DeltaCommitInfo {
        version: 2,
        timestamp: 1,
        file_name: "2.json".into(),
        file_size: 1,
        file_modification_timestamp: 1,
        ..Default::default()
    };
    let third = format!("{location}/v3.json");
    let uniform = DeltaUniform {
        iceberg: DeltaUniformIceberg {
            metadata_location: Some(third.clone()),
            converted_delta_version: Some(2),
            converted_delta_timestamp: Some("2025-01-04T03:13:11.423Z".into()),
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };
    uc.commit_delta(&created.metadata.table_uuid, &location)
        .with_commit_info(commit)
        .with_uniform(uniform)
        .await
        .unwrap();
    let loaded = uc.load_delta_table("c", "s", "uni").await.unwrap();
    let iceberg = loaded.uniform.iceberg.as_option().unwrap();
    assert_eq!(iceberg.metadata_location.as_deref(), Some(third.as_str()));
    assert_eq!(iceberg.converted_delta_timestamp, Some(1_735_960_391_423));
}

/// An EXTERNAL table: exists, load, rename and delete.
#[tokio::test]
async fn external_tables() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    let created = external(&uc, &storage, "ext").await;
    assert_eq!(created.metadata.table_type, DeltaTableType::External);
    assert!(uc.delta_table_exists("c", "s", "ext").await.unwrap());
    assert!(!uc.delta_table_exists("c", "s", "nope").await.unwrap());
    let loaded = uc.load_delta_table("c", "s", "ext").await.unwrap();
    assert_eq!(loaded.metadata.columns.as_option(), Some(&columns(false)));
    assert_eq!(loaded.metadata.table_uuid, created.metadata.table_uuid);
    assert_eq!(
        loaded.allowed_maintenance_operations,
        [
            DeltaMaintenanceOperation::DataReorganization,
            DeltaMaintenanceOperation::DataCleanup,
            DeltaMaintenanceOperation::MetadataCleanup,
        ]
    );

    uc.rename_delta_table("c", "s", "ext", "renamed")
        .await
        .unwrap();
    assert!(uc.delta_table_exists("c", "s", "renamed").await.unwrap());
    uc.delete_delta_table("c", "s", "renamed").await.unwrap();
    let error = rejected(
        uc.delete_delta_table("c", "s", "renamed").await,
        404,
        "NoSuchTableException",
    );
    assert_eq!(error.message, "Table not found: c.s.renamed");
    assert!(!uc.delta_table_exists("c", "s", "renamed").await.unwrap());
}

/// The updates of an EXTERNAL table: properties, comment, columns,
/// partition columns and the version of its metadata.
#[tokio::test]
async fn updates_of_external_tables() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    let table = external(&uc, &storage, "ext").await;
    let updated = uc
        .update_delta_table(
            "c",
            "s",
            "ext",
            requirements(&table),
            vec![
                update("set-properties", |u| {
                    u.updates.insert("k".into(), "v".into());
                    u.updates.insert("gone".into(), "soon".into());
                }),
                update("set-table-comment", |u| u.comment = Some("edited".into())),
                update("set-columns", |u| u.columns = columns(true).into()),
                update("set-partition-columns", |u| {
                    u.partition_columns = vec!["flag".into()]
                }),
                update("update-metadata-snapshot-version", |u| {
                    u.last_commit_version = Some(42);
                    u.last_commit_timestamp_ms = Some(1_700_000_000_001);
                }),
            ],
        )
        .await
        .unwrap();
    let metadata = updated.metadata.as_option().unwrap();
    assert_eq!(metadata.properties["k"], "v");
    assert_eq!(metadata.columns.as_option(), Some(&columns(true)));
    assert_eq!(metadata.partition_columns, ["flag"]);
    assert_eq!(metadata.last_commit_version, Some(42));
    assert_eq!(metadata.last_commit_timestamp_ms, Some(1_700_000_000_001));
    assert_ne!(metadata.etag, table.metadata.etag);
    let comment = uc.table("c.s.ext").get().await.unwrap().comment;
    assert_eq!(comment.as_deref(), Some("edited"));

    let removed = uc
        .update_delta_table(
            "c",
            "s",
            "ext",
            requirements(&updated),
            vec![update("remove-properties", |u| {
                u.removals = vec!["gone".into()]
            })],
        )
        .await
        .unwrap();
    assert!(!removed.metadata.properties.contains_key("gone"));
    // The etag of an earlier load no longer holds.
    rejected(
        uc.update_delta_table(
            "c",
            "s",
            "ext",
            requirements(&updated),
            vec![update("set-table-comment", |u| {
                u.comment = Some("x".into())
            })],
        )
        .await,
        409,
        "UpdateRequirementConflictException",
    );
}

/// Metrics reports and the credentials of a table.
#[tokio::test]
async fn metrics_and_credentials() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    let table = external(&uc, &storage, "ext").await;
    let report = DeltaMetricsReport {
        commit_report: DeltaCommitReport {
            num_files_added: Some(1),
            num_bytes_added: Some(10),
            num_rows_inserted: Some(3),
            file_size_histogram: DeltaFileSizeHistogram {
                sorted_bin_boundaries: vec![0, 1024],
                file_counts: vec![1, 0],
                total_bytes: vec![10, 0],
                commit_version: Some(1),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };
    uc.report_delta_table_metrics("c", "s", "ext", &table.metadata.table_uuid)
        .with_report(report)
        .await
        .unwrap();
    let error = rejected(
        uc.report_delta_table_metrics("c", "s", "ext", MISSING)
            .await,
        400,
        "InvalidParameterValueException",
    );
    let message = format!("table-id {MISSING} does not match table c.s.ext.");
    assert_eq!(error.message, message);

    let credentials = uc
        .get_delta_table_credentials("c", "s", "ext", DeltaCredentialOperation::Read)
        .await
        .unwrap();
    let [credential] = credentials.storage_credentials.as_slice() else {
        panic!("one credential: {credentials:?}");
    };
    assert_eq!(credential.prefix, table.metadata.location);
    assert_eq!(credential.operation, DeltaCredentialOperation::Read);
    rejected(
        uc.get_delta_table_credentials("c", "s", "nope", DeltaCredentialOperation::ReadWrite)
            .await,
        404,
        "NoSuchTableException",
    );
}
