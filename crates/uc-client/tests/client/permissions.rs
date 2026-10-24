//! Permissions, and the metastore summary.

use uc_client::models::permissions::v1::PermissionsChange;

use crate::common::{catalog_and_schema, client, email};

const ADA: &str = "ada@example.com";
const BOB: &str = "bob@example.com";

fn change(principal: &str, add: &[&str], remove: &[&str]) -> PermissionsChange {
    PermissionsChange {
        principal: principal.into(),
        add: add.iter().map(|p| p.to_string()).collect(),
        remove: remove.iter().map(|p| p.to_string()).collect(),
        ..Default::default()
    }
}

#[tokio::test]
async fn permissions() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    // Grants need users.
    for user in [ADA, BOB] {
        uc.create_user().with_emails([email(user)]).await.unwrap();
    }
    let none = uc.get_permissions("catalog", "c").await.unwrap();
    assert!(none.privilege_assignments.is_empty());

    let granted = uc
        .update_permissions(
            "catalog",
            "c",
            vec![
                change(ADA, &["USE CATALOG", "CREATE SCHEMA"], &[]),
                change(BOB, &["USE CATALOG"], &[]),
            ],
        )
        .await
        .unwrap();
    assert_eq!(granted.privilege_assignments.len(), 2);
    let ada = granted
        .privilege_assignments
        .iter()
        .find(|a| a.principal == ADA)
        .unwrap();
    let mut privileges = ada.privileges.clone();
    privileges.sort();
    assert_eq!(privileges, ["CREATE SCHEMA", "USE CATALOG"]);
    assert_eq!(uc.get_permissions("catalog", "c").await.unwrap(), granted);

    let of_bob = uc
        .get_permissions("catalog", "c")
        .with_principal(BOB.to_string())
        .await
        .unwrap();
    let [assignment] = of_bob.privilege_assignments.as_slice() else {
        panic!("one assignment: {of_bob:?}");
    };
    assert_eq!(assignment.principal, BOB);
    assert_eq!(assignment.privileges, ["USE CATALOG"]);

    // A schema is named by its full name.
    let on_schema = uc
        .update_permissions("schema", "c.s", vec![change(ADA, &["USE SCHEMA"], &[])])
        .await
        .unwrap();
    assert_eq!(
        on_schema.privilege_assignments[0].privileges,
        ["USE SCHEMA"]
    );

    let revoked = uc
        .update_permissions(
            "catalog",
            "c",
            vec![change(ADA, &[], &["USE CATALOG", "CREATE SCHEMA"])],
        )
        .await
        .unwrap();
    assert!(revoked.privilege_assignments.is_empty());
    let left = uc.get_permissions("catalog", "c").await.unwrap();
    assert_eq!(left, of_bob);
}

#[tokio::test]
async fn metastore_summary() {
    let uc = client().await;
    let summary = uc.get_metastore_summary().await.unwrap();
    assert!(!summary.metastore_id.is_empty());
    let again = uc.get_metastore_summary().await.unwrap();
    assert_eq!(again, summary);
}
