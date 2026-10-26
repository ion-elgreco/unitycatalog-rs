//! Protection against deletes.

use crate::common::{catalog_and_schema, client, rejected};

#[tokio::test]
async fn protection() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let none = uc.get_protection("schema", "c.s").await.unwrap();
    assert!(!none.protected);
    let on = uc.update_protection("schema", "c.s", true).await.unwrap();
    assert!(on.protected && on.protected_at.is_some());
    assert_eq!(uc.get_protection("schema", "c.s").await.unwrap(), on);

    let error = rejected(uc.schema("c.s").delete().await, 409, "RESOURCE_PROTECTED");
    assert_eq!(
        error.message,
        "Cannot delete schema c.s: it is protected. Remove its protection first."
    );
    let off = uc.update_protection("schema", "c.s", false).await.unwrap();
    assert!(!off.protected);
    uc.schema("c.s").delete().await.unwrap();
}
