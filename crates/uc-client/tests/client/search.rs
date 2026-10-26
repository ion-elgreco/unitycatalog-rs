//! Search.

use uc_client::models::volumes::v1::VolumeType;

use crate::common::{catalog_and_schema, client, rejected};

#[tokio::test]
async fn search() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    for name in ["orders", "sales_orders"] {
        uc.create_volume("c", "s", name, VolumeType::Managed)
            .with_comment("raw files".to_string())
            .await
            .unwrap();
    }
    let found = uc.search_objects("orders").await.unwrap();
    let names: Vec<_> = found.results.iter().map(|r| r.full_name.as_str()).collect();
    assert_eq!(names, ["c.s.orders", "c.s.sales_orders"]);
    let first = &found.results[0];
    assert_eq!(first.securable_type, "volume");
    assert_eq!(
        (first.catalog_name.as_str(), first.schema_name.as_deref()),
        ("c", Some("s"))
    );
    assert_eq!(first.comment.as_deref(), Some("raw files"));
    assert!(!first.id.is_empty() && first.created_at > 0);
    assert_eq!(found.next_page_token, None);

    let page = uc
        .search_objects("raw files")
        .with_securable_type("volume".to_string())
        .with_max_results(1)
        .await
        .unwrap();
    assert_eq!(page.results[0].name, "orders");
    let next = uc
        .search_objects("raw files")
        .with_securable_type("volume".to_string())
        .with_max_results(1)
        .with_page_token(page.next_page_token)
        .await
        .unwrap();
    assert_eq!(next.results[0].name, "sales_orders");
    let tables = uc
        .search_objects("orders")
        .with_securable_type("table".to_string())
        .await
        .unwrap();
    assert!(tables.results.is_empty());
    rejected(
        uc.search_objects("orders")
            .with_securable_type("credential".to_string())
            .await,
        400,
        "INVALID_ARGUMENT",
    );
}
