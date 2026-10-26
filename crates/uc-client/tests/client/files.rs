//! Files: the files of volumes and tables.

use uc_client::models::tables::v1::{DataSourceFormat, TableType};
use uc_client::models::volumes::v1::VolumeType;

use crate::common::{api_error, catalog_and_schema, client, file_url, rejected};

#[tokio::test]
async fn volume_files() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    uc.create_volume("c", "s", "v", VolumeType::Managed)
        .await
        .unwrap();

    // Paths with or without their leading `/`.
    uc.upload_file("/Volumes/c/s/v/dir/a.txt", b"hello".to_vec())
        .await
        .unwrap();
    uc.upload_file("Volumes/c/s/v/b.txt", b"b".to_vec())
        .await
        .unwrap();
    rejected(
        uc.upload_file("/Volumes/c/s/v/b.txt", b"again".to_vec())
            .await,
        409,
        "ALREADY_EXISTS",
    );
    uc.upload_file("/Volumes/c/s/v/b.txt", b"again".to_vec())
        .with_overwrite(true)
        .await
        .unwrap();

    let first = uc
        .list_directory_contents("/Volumes/c/s/v")
        .with_page_size(1)
        .await
        .unwrap();
    assert_eq!(first.contents.len(), 1);
    assert!(first.contents[0].is_directory);
    assert_eq!(first.contents[0].path, "/Volumes/c/s/v/dir/");
    let second = uc
        .list_directory_contents("/Volumes/c/s/v")
        .with_page_size(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.contents[0].name, "b.txt");
    assert_eq!(second.contents[0].file_size, Some(5));
    assert!(second.next_page_token.is_none());

    let file = uc.download_file("/Volumes/c/s/v/dir/a.txt").await.unwrap();
    assert_eq!(file.contents, b"hello");
    assert_eq!(file.content_length, Some(5));
    assert_eq!(
        file.content_type.as_deref(),
        Some("application/octet-stream")
    );
    assert!(file.last_modified.is_some());
    let meta = uc.get_file_metadata("/Volumes/c/s/v/b.txt").await.unwrap();
    assert_eq!(meta.content_length, Some(5));
    // A HEAD answer has no body: only the status.
    let missing = api_error(uc.get_file_metadata("/Volumes/c/s/v/none.txt").await);
    assert_eq!(missing.status, 404);

    uc.create_directory("/Volumes/c/s/v/empty").await.unwrap();
    assert!(
        uc.get_directory_metadata("/Volumes/c/s/v/empty")
            .await
            .unwrap()
    );
    uc.delete_directory("/Volumes/c/s/v/empty").await.unwrap();
    assert!(
        !uc.get_directory_metadata("/Volumes/c/s/v/empty")
            .await
            .unwrap()
    );
    uc.delete_file("/Volumes/c/s/v/dir/a.txt").await.unwrap();
    rejected(
        uc.delete_file("/Volumes/c/s/v/dir/a.txt").await,
        404,
        "NOT_FOUND",
    );

    // A directory that holds files only with `recursive`.
    uc.upload_file("/Volumes/c/s/v/full/sub/x.txt", b"x".to_vec())
        .await
        .unwrap();
    rejected(
        uc.delete_directory("/Volumes/c/s/v/full").await,
        400,
        "FAILED_PRECONDITION",
    );
    uc.delete_directory("/Volumes/c/s/v/full")
        .with_recursive(true)
        .await
        .unwrap();
    assert!(
        !uc.get_directory_metadata("/Volumes/c/s/v/full")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn table_files() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    let location = storage.path().join("t");
    std::fs::create_dir_all(location.join("_delta_log")).unwrap();
    std::fs::write(location.join("part-0.parquet"), "data").unwrap();
    uc.create_table("t", "c", "s", TableType::External)
        .with_data_source_format(DataSourceFormat::Delta)
        .with_storage_location(format!("{}/t", file_url(&storage)))
        .await
        .unwrap();

    let listing = uc.list_directory_contents("/Tables/c/s/t").await.unwrap();
    let names: Vec<_> = listing.contents.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["_delta_log", "part-0.parquet"]);
    let file = uc
        .download_file("/Tables/c/s/t/part-0.parquet")
        .await
        .unwrap();
    assert_eq!(file.contents, b"data");
    rejected(
        uc.upload_file("/Tables/c/s/t/new.parquet", b"x".to_vec())
            .await,
        400,
        "INVALID_ARGUMENT",
    );
}
