//! Volumes.

use futures::TryStreamExt;

use uc_client::models::volumes::v1::VolumeType;

use crate::common::{catalog_and_schema, client, rejected};

#[tokio::test]
async fn volumes() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let created = uc
        .create_volume("c", "s", "v", VolumeType::Managed)
        .with_comment("scratch".to_string())
        .await
        .unwrap();
    assert_eq!(created.full_name, "c.s.v");
    assert_eq!(created.volume_type, VolumeType::Managed);
    assert_eq!(created.comment.as_deref(), Some("scratch"));
    assert!(!created.storage_location.is_empty() && !created.volume_id.is_empty());
    assert_eq!(uc.volume("c.s.v").get().await.unwrap(), created);
    uc.create_volume("c", "s", "w", VolumeType::Managed)
        .await
        .unwrap();
    let first = uc.list_volumes("c", "s").with_max_results(1).await.unwrap();
    assert_eq!(first.volumes, std::slice::from_ref(&created));
    let second = uc
        .list_volumes("c", "s")
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.volumes[0].name, "w");

    let updated = uc
        .volume("c.s.v")
        .update()
        .with_comment("edited".to_string())
        .with_new_name("v2".to_string())
        .await
        .unwrap();
    assert_eq!(updated.full_name, "c.s.v2");
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    assert_eq!(updated.volume_id, created.volume_id);

    uc.volume("c.s.v2").delete().await.unwrap();
    rejected(uc.volume("c.s.v2").get().await, 404, "NOT_FOUND");
    let left: Vec<_> = uc
        .list_volumes("c", "s")
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(left.len(), 1);
}
