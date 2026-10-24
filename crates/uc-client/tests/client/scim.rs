//! SCIM users and groups of the control API, the signed-in user, bulk
//! requests and the discovery endpoints.

use buffa_types::google::protobuf::{ListValue, Struct, Value};

use uc_client::Client;
use uc_client::models::scim::v1::{
    ScimBulkOp, ScimGroup, ScimGroupMember, ScimPatchOperation, ScimPhoto, ScimUser,
};

use crate::common::{api_error, client, email, rejected, server_with_keys, token};

const GROUP: &str = "urn:ietf:params:scim:schemas:core:2.0:Group";
const PATCH_OP: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
const BULK_REQUEST: &str = "urn:ietf:params:scim:api:messages:2.0:BulkRequest";

/// A SCIM PATCH operation.
fn operation(op: &str, path: Option<&str>, value: Option<Value>) -> ScimPatchOperation {
    ScimPatchOperation {
        op: op.into(),
        path: path.map(str::to_string),
        value: value.into(),
        ..Default::default()
    }
}

/// The group member `user`.
fn member(user: &ScimUser) -> ScimGroupMember {
    ScimGroupMember {
        value: user.id.clone(),
        display: user.display_name.clone(),
        member_type: Some("User".into()),
        ..Default::default()
    }
}

fn member_ids(group: &ScimGroup) -> Vec<&str> {
    group.members.iter().map(|m| m.value.as_str()).collect()
}

async fn user(uc: &Client, address: &str) -> ScimUser {
    uc.create_user()
        .with_emails([email(address)])
        .await
        .unwrap()
}

#[tokio::test]
async fn users() {
    let uc = client().await;
    let photo = ScimPhoto {
        value: "https://example.com/ada.png".into(),
        ..Default::default()
    };
    let created = uc
        .create_user()
        .with_display_name("Ada".to_string())
        .with_external_id("ada-at-idp".to_string())
        .with_active(true)
        .with_emails([email("ada@example.com")])
        .with_photos([photo.clone()])
        .await
        .unwrap();
    assert_eq!(created.user_name, "ada@example.com");
    assert_eq!(created.display_name.as_deref(), Some("Ada"));
    assert_eq!(created.external_id.as_deref(), Some("ada-at-idp"));
    assert_eq!(created.active, Some(true));
    assert_eq!(created.photos[0].value, photo.value);
    assert_eq!(created.meta.resource_type, "User");
    assert!(!created.id.is_empty());
    assert_eq!(uc.get_user(&created.id).await.unwrap(), created);
    user(&uc, "bob@example.com").await;

    let found = uc
        .list_users()
        .with_filter(r#"userName eq "ada@example.com""#.to_string())
        .await
        .unwrap();
    assert_eq!(found.resources, std::slice::from_ref(&created));
    assert_eq!((found.total_results, found.start_index), (1, 1));
    let mut names = Vec::new();
    for start in [1, 2] {
        let page = uc
            .list_users()
            .with_start_index(start)
            .with_count(1)
            .await
            .unwrap();
        assert_eq!((page.start_index, page.items_per_page), (start, 1));
        names.extend(page.resources.into_iter().map(|u| u.user_name));
    }
    names.sort();
    assert_eq!(names, ["ada@example.com", "bob@example.com"]);

    let replaced = uc
        .replace_user(&created.id)
        .with_display_name("Ada L".to_string())
        .with_external_id("ada-2".to_string())
        .await
        .unwrap();
    assert_eq!(replaced.display_name.as_deref(), Some("Ada L"));
    assert_eq!(replaced.external_id.as_deref(), Some("ada-2"));
    assert_eq!(replaced.user_name, "ada@example.com");

    // Deleting a user disables it, as upstream.
    uc.delete_user(&created.id).await.unwrap();
    assert_eq!(uc.get_user(&created.id).await.unwrap().active, Some(false));
    rejected(uc.get_user("missing").await, 404, "NOT_FOUND");
}

/// A patch disables and enables a user; other operations are not
/// supported (501).
#[tokio::test]
async fn user_patches() {
    let uc = client().await;
    let id = user(&uc, "ada@example.com").await.id;
    uc.patch_user(
        &id,
        vec![operation("replace", Some("active"), Some(false.into()))],
    )
    .with_schemas([PATCH_OP.to_string()])
    .await
    .unwrap();
    assert_eq!(uc.get_user(&id).await.unwrap().active, Some(false));
    let active = Struct::from_fields([("active", true)]);
    uc.patch_user(&id, vec![operation("replace", None, Some(active.into()))])
        .await
        .unwrap();
    assert_eq!(uc.get_user(&id).await.unwrap().active, Some(true));
    let email = Some("x@example.com".into());
    let error = api_error(
        uc.patch_user(&id, vec![operation("add", Some("emails"), email)])
            .await,
    );
    assert_eq!(error.status, 501);
}

#[tokio::test]
async fn groups() {
    let uc = client().await;
    let ada = user(&uc, "ada@example.com").await;
    let bob = user(&uc, "bob@example.com").await;
    let created = uc
        .create_group("admins")
        .with_schemas([GROUP.to_string()])
        .with_members([member(&ada)])
        .await
        .unwrap();
    assert_eq!(created.display_name, "admins");
    assert_eq!(member_ids(&created), [ada.id.as_str()]);
    assert!(!created.id.is_empty());
    assert_eq!(uc.get_group(&created.id).await.unwrap(), created);
    // Without `schemas`, the server fills them in.
    let readers = uc.create_group("readers").await.unwrap();
    assert_eq!(readers.schemas, [GROUP]);
    let found = uc
        .list_groups()
        .with_filter(r#"displayName eq "admins""#.to_string())
        .await
        .unwrap();
    assert_eq!(found.resources, std::slice::from_ref(&created));
    let mut names = Vec::new();
    for start in [1, 2] {
        let page = uc
            .list_groups()
            .with_start_index(start)
            .with_count(1)
            .await
            .unwrap();
        assert_eq!((page.start_index, page.items_per_page), (start, 1));
        names.extend(page.resources.into_iter().map(|g| g.display_name));
    }
    names.sort();
    assert_eq!(names, ["admins", "readers"]);

    let replaced = uc
        .replace_group(&created.id, "ops")
        .with_schemas([GROUP.to_string()])
        .with_members([member(&bob)])
        .await
        .unwrap();
    assert_eq!(replaced.display_name, "ops");
    assert_eq!(member_ids(&replaced), [bob.id.as_str()]);

    let ada_member = Struct::from_fields([("value", ada.id.as_str())]);
    let bob_path = format!(r#"members[value eq "{}"]"#, bob.id);
    let patched = uc
        .patch_group(
            &created.id,
            vec![
                operation(
                    "add",
                    Some("members"),
                    Some(ListValue::from_values([ada_member]).into()),
                ),
                operation("remove", Some(&bob_path), None),
                operation("replace", Some("displayName"), Some("admins".into())),
            ],
        )
        .with_schemas([PATCH_OP.to_string()])
        .await
        .unwrap();
    assert_eq!(patched.display_name, "admins");
    assert_eq!(member_ids(&patched), [ada.id.as_str()]);

    uc.delete_group(&created.id).await.unwrap();
    rejected(uc.get_group(&created.id).await, 404, "NOT_FOUND");
}

/// `me` is the user of the client's token. Without a token the server
/// knows no caller: as upstream, a SCIM 400 error in an HTTP 500.
#[tokio::test]
async fn me() {
    let url = server_with_keys().await;
    let alice = Client::new_with_token(url.clone(), token("alice@example.com"));
    alice
        .create_user()
        .with_display_name("Alice".to_string())
        .with_emails([email("alice@example.com")])
        .await
        .unwrap();
    let me = alice.get_me().await.unwrap();
    assert_eq!(me.user_name, "alice@example.com");
    assert_eq!(me.display_name.as_deref(), Some("Alice"));
    // With authorization off, everyone can do what the metastore admin does.
    let roles: Vec<_> = me.roles.iter().map(|r| r.value.as_str()).collect();
    assert_eq!(roles, ["admin"]);

    let anonymous = Client::new_unauthenticated(url);
    let error = api_error(anonymous.get_me().await);
    assert_eq!(
        (error.status, error.message.as_str()),
        (500, "No user found.")
    );
}

/// A bulk request runs its operations in order, each with its own status,
/// and stops after `failOnErrors` failures.
#[tokio::test]
async fn bulk() {
    let uc = client().await;
    let ada = Struct::from_fields([(
        "emails",
        ListValue::from_values([Struct::from_fields([
            ("value", Value::from("ada@example.com")),
            ("primary", Value::from(true)),
        ])]),
    )]);
    let group = Struct::from_fields([
        ("schemas", Value::from(ListValue::from_values([GROUP]))),
        ("displayName", Value::from("bulk")),
    ]);
    let op = |method: &str, path: &str, bulk_id: &str, data: Option<Struct>| ScimBulkOp {
        method: method.into(),
        path: path.into(),
        bulk_id: Some(bulk_id.into()),
        data: data.into(),
        ..Default::default()
    };
    let response = uc
        .scim_bulk()
        .with_schemas([BULK_REQUEST.to_string()])
        .with_fail_on_errors(1)
        .with_operations([
            op("POST", "/Users", "ada", Some(ada)),
            op("POST", "/Groups", "group", Some(group.clone())),
            op("DELETE", "/Groups/missing", "missing", None),
            op("POST", "/Groups", "not-run", Some(group)),
        ])
        .await
        .unwrap();
    let results: Vec<_> = response
        .operations
        .iter()
        .map(|r| (r.bulk_id.as_deref(), r.status.as_str()))
        .collect();
    assert_eq!(
        results,
        [
            (Some("ada"), "201"),
            (Some("group"), "201"),
            (Some("missing"), "404")
        ]
    );
    let created = &response.operations[0];
    assert_eq!(created.method, "POST");
    assert!(
        created
            .location
            .as_deref()
            .is_some_and(|l| l.starts_with("/scim2/Users/"))
    );
    let user = created.response.as_option().unwrap();
    assert_eq!(
        user.get("userName").and_then(Value::as_str),
        Some("ada@example.com")
    );
    let groups = uc.list_groups().await.unwrap().resources;
    assert_eq!(groups.len(), 1);
}

#[tokio::test]
async fn discovery() {
    let uc = client().await;
    let schemas = uc.list_scim_schemas().await.unwrap();
    let ids: Vec<_> = schemas.resources.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:core:2.0:Group"
        ]
    );
    assert_eq!((schemas.total_results, schemas.start_index), (2, 1));
    let user = &schemas.resources[0];
    assert_eq!(user.name, "User");
    assert_eq!(user.attributes[0].name, "userName");
    assert_eq!(user.attributes[3].sub_attributes[0].name, "value");

    let types = uc.list_scim_resource_types().await.unwrap();
    let endpoints: Vec<_> = types
        .resources
        .iter()
        .map(|t| t.endpoint.as_str())
        .collect();
    assert_eq!(endpoints, ["/Users", "/Groups"]);
    assert_eq!(types.resources[1].meta.resource_type, "ResourceType");

    let config = uc.get_service_provider_config().await.unwrap();
    assert!(config.patch.supported && config.bulk.supported);
    assert_eq!(config.bulk.max_operations, 100);
    assert_eq!(config.filter.max_results, 1000);
    assert!(!config.change_password.supported);
    assert_eq!(config.authentication_schemes[0].r#type, "oauthbearertoken");
}
