//! The authorizers of privileges: Cedar's default "permit when granted"
//! policy decides as the grants do; operators' policies see the resource,
//! its owner and the context of the request; any other authorizer plugs in.

use std::sync::Arc;
use std::time::SystemTime;

use rstest::{fixture, rstest};
use uc_api::Privilege;
use uc_auth::authz::{AuthContext, AuthorizeRequest, AuthorizeResponse, Authorizer};
use uc_common::SecurableType;
use uc_metastore::{Lineage, Metastore, MetastoreConfig, SqlMetastore};
use uc_server::{
    AppState,
    authn::principal::Principal,
    authz::{Access, Level, Node},
};

#[fixture]
async fn metastore() -> Arc<dyn Metastore> {
    SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap()
}

/// A metastore with the catalog `main`, owned by `alice`, who has `USE
/// CATALOG` on it.
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
    metastore
}

/// The lineage of `main`.
async fn main(metastore: &Arc<dyn Metastore>) -> Lineage {
    metastore
        .permissions()
        .lineage(SecurableType::Catalog, "main")
        .await
        .unwrap()
}

/// The access of `who` with `state`, from `context`.
async fn access(state: &AppState, who: &str, context: AuthContext) -> Access {
    let principal = Principal::authenticated(who).with_context(context);
    Access::load(state, &principal).await.unwrap()
}

fn state(metastore: &Arc<dyn Metastore>) -> AppState {
    AppState::new(metastore.clone()).with_authz(true)
}

fn cedar(metastore: &Arc<dyn Metastore>, policies: &[&str]) -> AppState {
    let files: Vec<tempfile::NamedTempFile> = (policies.iter())
        .map(|policy| {
            let file = tempfile::NamedTempFile::new().unwrap();
            std::fs::write(file.path(), policy).unwrap();
            file
        })
        .collect();
    let paths: Vec<_> = files.iter().map(|f| f.path().to_path_buf()).collect();
    state(metastore).with_cedar(&paths).unwrap()
}

#[rstest]
#[tokio::test]
async fn the_default_policy_permits_what_is_granted(
    #[future(awt)] metastore_with_main: Arc<dyn Metastore>,
) {
    let metastore = metastore_with_main;
    let main = main(&metastore).await;
    // A schema of `main`: the catalog's grant counts for it.
    let schema = Node {
        kind: SecurableType::Schema,
        id: "s-1".into(),
        owner: None,
    };
    let in_main = Lineage {
        schema: Some(schema.clone()),
        ..main.clone()
    };
    let elsewhere = Lineage {
        schema: Some(schema),
        ..Default::default()
    };
    for state in [state(&metastore), cedar(&metastore, &[])] {
        let alice = access(&state, "alice", AuthContext::new()).await;
        assert!(
            alice
                .has(&main, Level::Catalog, Privilege::UseCatalog)
                .await
        );
        assert!(!alice.has(&main, Level::Catalog, Privilege::Modify).await);
        assert!(
            alice
                .has(&in_main, Level::Schema, Privilege::UseCatalog)
                .await
        );
        assert!(
            !alice
                .has(&elsewhere, Level::Schema, Privilege::UseCatalog)
                .await
        );
        let bob = access(&state, "bob", AuthContext::new()).await;
        assert!(!bob.has(&main, Level::Catalog, Privilege::UseCatalog).await);
    }
}

#[rstest]
#[tokio::test]
async fn policies_see_the_resource_and_the_request(
    #[future(awt)] metastore_with_main: Arc<dyn Metastore>,
) {
    let metastore = metastore_with_main;
    let main = main(&metastore).await;
    let state = cedar(
        &metastore,
        &[
            // Alice may MODIFY any catalog, whatever the grants.
            r#"permit (principal == User::"alice", action == Action::"MODIFY", resource is Catalog);"#,
            // The owner of a resource may read its files.
            r#"permit (principal, action == Action::"READ FILES", resource)
               when { resource has owner && resource.owner == "bob" };"#,
            // Nothing from outside the office network.
            r#"forbid (principal, action, resource)
               unless { context has source_ip && context.source_ip.isInRange(ip("10.0.0.0/8")) };"#,
        ],
    );
    let office = AuthContext::new()
        .source_ip("10.1.2.3".parse().unwrap())
        .request_time(SystemTime::now());
    let alice = access(&state, "alice", office.clone()).await;
    assert!(alice.has(&main, Level::Catalog, Privilege::Modify).await);
    assert!(
        alice
            .has(&main, Level::Catalog, Privilege::UseCatalog)
            .await
    );
    // `main` is alice's, not bob's.
    let bob = access(&state, "bob", office).await;
    assert!(!bob.has(&main, Level::Catalog, Privilege::Modify).await);
    assert!(!bob.has(&main, Level::Catalog, Privilege::ReadFiles).await);
    // From elsewhere, nothing.
    let home = AuthContext::new().source_ip("192.0.2.1".parse().unwrap());
    let alice = access(&state, "alice", home).await;
    assert!(
        !alice
            .has(&main, Level::Catalog, Privilege::UseCatalog)
            .await
    );
}

#[rstest]
#[tokio::test]
async fn a_policy_file_that_does_not_parse_is_named(#[future(awt)] metastore: Arc<dyn Metastore>) {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "permit (").unwrap();
    let Err(error) = state(&metastore).with_cedar(&[file.path().to_path_buf()]) else {
        panic!("the policy file parsed");
    };
    assert!(error.message().contains(&file.path().display().to_string()));
}

#[rstest]
#[tokio::test]
async fn a_policy_file_that_repeats_an_id_is_named(#[future(awt)] metastore: Arc<dyn Metastore>) {
    let policy = "@id(\"office\")\nforbid (principal, action, resource);";
    let (first, second) = (
        tempfile::NamedTempFile::new().unwrap(),
        tempfile::NamedTempFile::new().unwrap(),
    );
    std::fs::write(first.path(), policy).unwrap();
    std::fs::write(second.path(), policy).unwrap();
    let paths = [first.path().to_path_buf(), second.path().to_path_buf()];
    let Err(error) = state(&metastore).with_cedar(&paths) else {
        panic!("two policies with one id passed");
    };
    let message = error.message();
    assert!(
        message.contains(&second.path().display().to_string()),
        "{message}"
    );
    assert!(message.contains("office"), "{message}");
}

/// Denies everything, and remembers the last request.
#[derive(Clone, Default)]
struct Nobody(Arc<std::sync::Mutex<Option<AuthorizeRequest>>>);

#[uc_auth::async_trait]
impl Authorizer for Nobody {
    async fn authorize(&self, request: &AuthorizeRequest) -> AuthorizeResponse {
        *self.0.lock().unwrap() = Some(request.clone());
        AuthorizeResponse::deny("nobody may")
    }
}

#[rstest]
#[tokio::test]
async fn any_authorizer_plugs_in(#[future(awt)] metastore_with_main: Arc<dyn Metastore>) {
    let metastore = metastore_with_main;
    let main = main(&metastore).await;
    let nobody = Nobody::default();
    let state = state(&metastore).with_authorizer(nobody.clone());
    let context = AuthContext::new().with("auth_method", "jwt");
    let alice = access(&state, "alice", context).await;
    assert!(
        !alice
            .has(&main, Level::Catalog, Privilege::UseCatalog)
            .await
    );
    // What it was asked: the resource with its parents, owner and grants,
    // and the request's context.
    let asked = nobody.0.lock().unwrap().clone().unwrap();
    assert_eq!(
        (asked.principal.id.as_str(), asked.action.id.as_str()),
        ("alice", "USE CATALOG")
    );
    assert_eq!(asked.resource.entity_type, "Catalog");
    let entity = asked.resource_entity.unwrap();
    assert_eq!(entity["attrs"]["owner"], "alice");
    assert_eq!(entity["parents"][0]["type"], "Metastore");
    assert_eq!(
        entity["attrs"]["granted_privileges"][0]["__entity"]["id"],
        "USE CATALOG"
    );
    assert_eq!(asked.context["auth_method"], "jwt");
}
