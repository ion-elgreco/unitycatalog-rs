//! Catalogs, and the pages of a list.

use futures::TryStreamExt;

use uc_client::models::catalogs::v1::CatalogInfo;

use crate::common::{client, file_url, rejected};

fn names(catalogs: &[CatalogInfo]) -> Vec<&str> {
    catalogs.iter().map(|c| c.name.as_str()).collect()
}

#[tokio::test]
async fn catalogs() {
    let uc = client().await;
    let storage = tempfile::tempdir().unwrap();
    let created = uc
        .create_catalog("c")
        .with_comment("primary".to_string())
        .with_properties([("team", "data")])
        .with_storage_root(file_url(&storage))
        .await
        .unwrap();
    assert_eq!(created.name, "c");
    assert_eq!(created.comment.as_deref(), Some("primary"));
    assert_eq!(created.properties["team"], "data");
    assert!(!created.id.is_empty() && created.created_at > 0);
    assert_eq!(created.storage_root, Some(file_url(&storage)));
    assert!(created.storage_location.is_some());
    assert_eq!(uc.catalog("c").get().await.unwrap(), created);
    assert_eq!(
        uc.list_catalogs().await.unwrap().catalogs,
        std::slice::from_ref(&created)
    );

    let updated = uc
        .catalog("c")
        .update()
        .with_comment("edited".to_string())
        .with_properties([("team", "ml")])
        .with_new_name("c2".to_string())
        .await
        .unwrap();
    assert_eq!(updated.name, "c2");
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    assert_eq!(updated.properties["team"], "ml");
    assert_eq!(updated.id, created.id);

    // A catalog with a schema goes only with `force`.
    uc.create_schema("s", "c2").await.unwrap();
    rejected(uc.catalog("c2").delete().await, 400, "FAILED_PRECONDITION");
    uc.catalog("c2").delete().with_force(true).await.unwrap();
    let error = rejected(uc.catalog("c2").get().await, 404, "CATALOG_NOT_FOUND");
    assert_eq!(error.message, "Catalog not found: c2");
}

/// A page holds `max_results` catalogs and the token of the next page.
#[tokio::test]
async fn lists_page() {
    let uc = client().await;
    for name in ["a", "b", "c"] {
        uc.create_catalog(name).await.unwrap();
    }
    let first = uc.list_catalogs().with_max_results(2).await.unwrap();
    assert_eq!(names(&first.catalogs), ["a", "b"]);
    assert!(first.next_page_token.is_some());
    let second = uc
        .list_catalogs()
        .with_max_results(2)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(names(&second.catalogs), ["c"]);
    assert_eq!(second.next_page_token, None);
}

/// A stream reads every page: the server sends at most 100 catalogs in a
/// page. Its `max_results` caps the catalogs over all pages.
#[tokio::test]
async fn streams_read_every_page() {
    let uc = client().await;
    for i in 0..101 {
        uc.create_catalog(format!("c{i:03}")).await.unwrap();
    }
    let all: Vec<_> = uc
        .list_catalogs()
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(all.len(), 101);
    assert_eq!(all[100].name, "c100");
    let capped: Vec<_> = uc
        .list_catalogs()
        .with_max_results(2)
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(names(&capped), ["c000", "c001"]);
}

/// The server caps a page at 100 catalogs. With a cap above that, the
/// stream sends the same `max_results` for the second page, and gets more
/// than the cap.
#[tokio::test]
async fn a_cap_above_the_page_size_holds_over_all_pages() {
    let uc = client().await;
    for i in 0..102 {
        uc.create_catalog(format!("c{i:03}")).await.unwrap();
    }
    let capped: Vec<_> = uc
        .list_catalogs()
        .with_max_results(101)
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(capped.len(), 101);
}
