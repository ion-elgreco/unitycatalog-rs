//! CedarEngine reproduces PermissionEnforcer behaviour for the default
//! "permit when granted" policy.

use std::sync::Arc;

use rstest::{fixture, rstest};
use uc_api::Privilege;
use uc_common::SecurableType;
use uc_metastore::{Metastore, MetastoreConfig, SqlMetastore};
use uc_server::{
    authz::cedar::CedarEngine,
    authz::{Node, PolicyEngine},
};

#[fixture]
async fn metastore() -> Arc<dyn Metastore> {
    SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap()
}

/// A metastore with the catalog `main`, owned by `alice`.
#[fixture]
async fn metastore_with_main(#[future(awt)] metastore: Arc<dyn Metastore>) -> Arc<dyn Metastore> {
    let _ = metastore
        .catalogs()
        .create(
            uc_api::CreateCatalog {
                name: "main".into(),
                comment: None,
                properties: None,
                storage_root: None,
            },
            Some("alice"),
        )
        .await
        .unwrap();
    // Grants go to existing users.
    metastore
        .scim()
        .create_user(uc_api::ScimUser {
            user_name: "alice".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    metastore
}

#[rstest]
#[tokio::test]
async fn cedar_engine_permits_only_granted_actions(
    #[future(awt)] metastore_with_main: Arc<dyn Metastore>,
) {
    let metastore = metastore_with_main;
    // Grant USE_CATALOG on `main` to alice.
    metastore
        .permissions()
        .update(
            SecurableType::Catalog,
            "main",
            uc_api::UpdatePermissions {
                changes: vec![uc_api::PermissionsChange {
                    principal: "alice".into(),
                    add: vec![Privilege::UseCatalog],
                    remove: vec![],
                }],
            },
        )
        .await
        .unwrap();

    let engine = CedarEngine::new(metastore.clone()).unwrap();
    let (catalog, metastore_node) = main_chain(&metastore).await;
    let grants = engine.grants("alice", &[]).await.unwrap();
    assert!(
        grants.holds(&[&catalog, &metastore_node], Privilege::UseCatalog),
        "alice has USE CATALOG"
    );
    assert!(
        !grants.holds(&[&catalog, &metastore_node], Privilege::Modify),
        "alice lacks MODIFY"
    );
    assert!(
        !engine
            .grants("bob", &[])
            .await
            .unwrap()
            .holds(&[&catalog, &metastore_node], Privilege::UseCatalog)
    );

    // A grant on a parent counts for its children.
    let schema = Node {
        kind: SecurableType::Schema,
        id: "s-1".into(),
        owner: None,
    };
    assert!(grants.holds(&[&schema, &catalog, &metastore_node], Privilege::UseCatalog));
    assert!(!grants.holds(&[&schema, &metastore_node], Privilege::UseCatalog));
}

/// The `main` catalog and the metastore, as an authorization chain.
async fn main_chain(metastore: &Arc<dyn Metastore>) -> (Node, Node) {
    let catalog = metastore
        .permissions()
        .lineage(SecurableType::Catalog, "main")
        .await
        .unwrap()
        .catalog
        .unwrap();
    let metastore_node = Node {
        kind: SecurableType::Metastore,
        id: metastore.permissions().metastore_id().await.unwrap(),
        owner: None,
    };
    (catalog, metastore_node)
}

#[rstest]
#[tokio::test]
async fn cedar_engine_loads_extra_policy_files(
    #[future(awt)] metastore_with_main: Arc<dyn Metastore>,
) {
    let metastore = metastore_with_main;
    // An operator's policy: alice can MODIFY any catalog, whatever the grants.
    let extra = r#"
        permit (
            principal == User::"alice",
            action == Action::"MODIFY",
            resource is Catalog
        );
    "#;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), extra).unwrap();

    let engine =
        CedarEngine::with_policy_files(metastore.clone(), &[tmp.path().to_path_buf()]).unwrap();

    // Alice gets MODIFY via the extra policy even though no permission row exists.
    let (catalog, metastore_node) = main_chain(&metastore).await;
    assert!(
        engine
            .grants("alice", &[])
            .await
            .unwrap()
            .holds(&[&catalog, &metastore_node], Privilege::Modify)
    );
    // Bob does not.
    assert!(
        !engine
            .grants("bob", &[])
            .await
            .unwrap()
            .holds(&[&catalog, &metastore_node], Privilege::Modify)
    );
    // Policies see the owner.
    let mut owned = CedarEngine::new(metastore.clone()).unwrap();
    owned.add_policies(r#"permit (principal, action == Action::"MODIFY", resource) when { resource has owner && resource.owner == "alice" };"#).unwrap();
    assert!(
        owned
            .grants("bob", &[])
            .await
            .unwrap()
            .holds(&[&catalog, &metastore_node], Privilege::Modify)
    );
}
