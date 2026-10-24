//! Schemas.

use futures::TryStreamExt;

use crate::common::{client, file_url, rejected};

#[tokio::test]
async fn schemas() {
    let uc = client().await;
    let storage = tempfile::tempdir().unwrap();
    uc.create_catalog("c")
        .with_storage_root(file_url(&storage))
        .await
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let created = uc
        .create_schema("s", "c")
        .with_comment("raw data".to_string())
        .with_properties([("layer", "bronze")])
        .with_storage_root(file_url(&root))
        .await
        .unwrap();
    assert_eq!(created.full_name, "c.s");
    assert_eq!(created.catalog_name, "c");
    assert_eq!(created.comment.as_deref(), Some("raw data"));
    assert_eq!(created.properties["layer"], "bronze");
    assert!(!created.schema_id.is_empty());
    assert_eq!(created.storage_root, Some(file_url(&root)));
    assert!(created.storage_location.is_some());
    assert_eq!(uc.schema("c.s").get().await.unwrap(), created);
    let listed: Vec<_> = uc
        .list_schemas("c")
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(listed, std::slice::from_ref(&created));

    let updated = uc
        .schema("c.s")
        .update()
        .with_comment("edited".to_string())
        .with_properties([("layer", "silver")])
        .with_new_name("s2".to_string())
        .await
        .unwrap();
    assert_eq!(updated.full_name, "c.s2");
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    assert_eq!(updated.properties["layer"], "silver");
    assert_eq!(updated.schema_id, created.schema_id);

    // A schema with a model goes only with `force`.
    uc.create_registered_model("m", "c", "s2").await.unwrap();
    rejected(uc.schema("c.s2").delete().await, 400, "FAILED_PRECONDITION");
    uc.schema("c.s2").delete().with_force(true).await.unwrap();
    assert!(uc.list_schemas("c").await.unwrap().schemas.is_empty());
    rejected(uc.schema("c.s2").get().await, 404, "SCHEMA_NOT_FOUND");
}

/// A page holds `max_results` schemas and the token of the next page.
#[tokio::test]
async fn lists_page() {
    let uc = client().await;
    uc.create_catalog("c").await.unwrap();
    for name in ["a", "b"] {
        uc.create_schema(name, "c").await.unwrap();
    }
    let first = uc.list_schemas("c").with_max_results(1).await.unwrap();
    assert_eq!(first.schemas[0].name, "a");
    let second = uc
        .list_schemas("c")
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.schemas[0].name, "b");
}
