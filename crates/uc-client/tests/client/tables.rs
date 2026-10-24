//! Tables, views and staging tables, and the temporary credentials of
//! tables, volumes, paths and model versions.

use futures::TryStreamExt;

use uc_client::models::credentials::v1::{
    AwsIamRoleRequest, ModelVersionOperation, PathOperation, TableOperation, VolumeOperation,
};
use uc_client::models::tables::v1::{
    ColumnTypeName, DataSourceFormat, Dependency, DependencyList, TableDependency, TableType,
    UpdateColumn,
};
use uc_client::models::volumes::v1::VolumeType;

use crate::common::{catalog_and_schema, client, column, external_table, file_url, rejected};

#[tokio::test]
async fn managed_tables() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let staging = uc.create_staging_table("t", "c", "s").await.unwrap();
    assert_eq!(
        (staging.name.as_str(), staging.catalog_name.as_str()),
        ("t", "c")
    );
    assert_eq!(staging.schema_name, "s");
    assert!(!staging.id.is_empty() && !staging.staging_location.is_empty());

    let columns = vec![
        column("id", "bigint", "long", ColumnTypeName::Long, 0),
        column("email", "string", "string", ColumnTypeName::String, 1),
    ];
    let table = uc
        .create_table("t", "c", "s", TableType::Managed)
        .with_columns(columns.clone())
        .with_data_source_format(DataSourceFormat::Delta)
        .with_storage_location(staging.staging_location.clone())
        .with_comment("users".to_string())
        .with_properties([("io.unitycatalog.tableId", staging.id.as_str())])
        .await
        .unwrap();
    assert_eq!(table.table_type, TableType::Managed);
    assert_eq!(
        table.data_source_format,
        Some(DataSourceFormat::Delta.into())
    );
    assert_eq!(table.columns, columns);
    assert_eq!(table.table_id, staging.id);
    assert_eq!(table.comment.as_deref(), Some("users"));
    assert_eq!(
        table.storage_location.as_deref(),
        Some(staging.staging_location.as_str())
    );
    let fetched = uc
        .table("c.s.t")
        .get()
        .with_read_streaming_table_as_managed(true)
        .with_read_materialized_view_as_managed(false)
        .await
        .unwrap();
    assert_eq!(fetched, table);

    let edited = uc
        .table("c.s.t")
        .update()
        .with_comment("people".to_string())
        .with_columns([UpdateColumn {
            name: "email".into(),
            comment: "primary email".into(),
            ..Default::default()
        }])
        .await
        .unwrap();
    assert_eq!(edited.comment.as_deref(), Some("people"));
    assert_eq!(edited.columns[1].comment.as_deref(), Some("primary email"));
    assert_eq!(edited.columns[0].comment, None);

    uc.table("c.s.t").delete().await.unwrap();
    rejected(uc.table("c.s.t").get().await, 404, "TABLE_NOT_FOUND");
}

#[tokio::test]
async fn views_and_lists() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    external_table(&uc, &storage, "t").await;
    let dependencies = DependencyList {
        dependencies: vec![Dependency {
            table: TableDependency {
                table_full_name: "c.s.t".into(),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let view = uc
        .create_table("v", "c", "s", TableType::View)
        .with_columns([column("id", "bigint", "long", ColumnTypeName::Long, 0)])
        .with_view_definition("select * from t".to_string())
        .with_view_dependencies(dependencies.clone())
        .await
        .unwrap();
    assert_eq!(view.table_type, TableType::View);
    assert_eq!(view.view_definition.as_deref(), Some("select * from t"));
    assert_eq!(view.view_dependencies.as_option(), Some(&dependencies));

    let first = uc.list_tables("c", "s").with_max_results(1).await.unwrap();
    assert_eq!(first.tables[0].name, "t");
    let second = uc
        .list_tables("c", "s")
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.tables, [view]);
    let all: Vec<_> = uc
        .list_tables("c", "s")
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    let names: Vec<_> = all.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["t", "v"]);
}

/// Credentials that the server vends for a table, a volume and a path.
/// Local storage needs none, so they hold no cloud credentials.
#[tokio::test]
async fn temporary_credentials_of_local_storage() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    let table = external_table(&uc, &storage, "t").await;
    let credentials = uc
        .generate_temporary_table_credentials(&table.table_id, TableOperation::Read)
        .await
        .unwrap();
    assert!(credentials.aws_temp_credentials.is_unset());
    assert!(credentials.azure_user_delegation_sas.is_unset());
    assert!(credentials.gcp_oauth_token.is_unset());

    let volume = uc
        .create_volume("c", "s", "v", VolumeType::Managed)
        .await
        .unwrap();
    let credentials = uc
        .generate_temporary_volume_credentials(&volume.volume_id, VolumeOperation::WriteVolume)
        .await
        .unwrap();
    assert!(credentials.aws_temp_credentials.is_unset());

    let path = format!("{}/t", file_url(&storage));
    let credentials = uc
        .generate_temporary_path_credentials(&path, PathOperation::PathRead)
        .await
        .unwrap();
    assert!(credentials.aws_temp_credentials.is_unset());

    let missing = "00000000-0000-0000-0000-000000000000";
    rejected(
        uc.generate_temporary_table_credentials(missing, TableOperation::Read)
            .await,
        404,
        "TABLE_NOT_FOUND",
    );
}

/// Credentials for S3 storage below an external location: the server's
/// placeholder vendor puts the role of the location's credential in the
/// session token. Model versions take cloud storage only.
#[tokio::test]
async fn temporary_credentials_of_cloud_storage() {
    let uc = client().await;
    let role = "arn:aws:iam::123456789012:role/uc";
    uc.create_credential(
        "cred",
        AwsIamRoleRequest {
            role_arn: role.into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    uc.create_external_location("lake", "s3://bucket/lake", "cred")
        .await
        .unwrap();
    uc.create_catalog("c")
        .with_storage_root("s3://bucket/lake/root".to_string())
        .await
        .unwrap();
    uc.create_schema("s", "c").await.unwrap();
    uc.create_registered_model("m", "c", "s").await.unwrap();
    uc.create_model_version("m", "c", "s", "s3://bucket/runs/1")
        .await
        .unwrap();
    let credentials = uc
        .generate_temporary_model_version_credentials(
            "c",
            "s",
            "m",
            1,
            ModelVersionOperation::ReadWriteModelVersion,
        )
        .await
        .unwrap();
    let aws = credentials.aws_temp_credentials.as_option().unwrap();
    assert_eq!(aws.session_token, role);
    assert!(credentials.expiration_time.is_some());

    let credentials = uc
        .generate_temporary_path_credentials("s3://bucket/lake/data", PathOperation::PathReadWrite)
        .await
        .unwrap();
    assert_eq!(credentials.aws_temp_credentials.session_token, role);
}
