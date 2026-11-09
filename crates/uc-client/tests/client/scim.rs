//! SCIM users and groups of the control API, the signed-in user, bulk
//! requests and the discovery endpoints.

use buffa_types::google::protobuf::{ListValue, Struct, Value};

use uc_client::Client;
use uc_client::models::scim::v1::{
    ScimBulkOp, ScimGroup, ScimGroupMember, ScimPatchOperation, ScimPhoto, ScimUser,
};

use crate::common::{api_error, client, email, server_with_keys, token};

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
    uc.create_user(address)
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
        .create_user("ada")
        .with_display_name("Ada".to_string())
        .with_external_id("ada-at-idp".to_string())
        .with_active(true)
        .with_emails([email("ada@example.com")])
        .with_photos([photo.clone()])
        .await
        .unwrap();
    assert_eq!(created.user_name, "ada");
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
        .with_filter(r#"userName eq "ada""#.to_string())
        .await
        .unwrap();
    assert_eq!(found.resources, std::slice::from_ref(&created));
    assert_eq!((found.total_results, found.start_index), (1, 1));
    let searched = uc
        .search_users()
        .with_filter(r#"emails.value eq "ada@example.com""#.to_string())
        .with_attributes(["userName".to_string()])
        .await
        .unwrap();
    assert_eq!(searched.total_results, 1);
    assert_eq!(searched.resources[0].user_name, "ada");
    assert!(searched.resources[0].emails.is_empty());
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
    assert_eq!(names, ["ada", "bob@example.com"]);

    // A replace keeps what it does not name.
    let replaced = uc
        .replace_user(&created.id, "ada.l")
        .with_display_name("Ada L".to_string())
        .with_external_id("ada-2".to_string())
        .await
        .unwrap();
    assert_eq!(replaced.display_name.as_deref(), Some("Ada L"));
    assert_eq!(replaced.external_id.as_deref(), Some("ada-2"));
    assert_eq!(replaced.user_name, "ada.l");
    assert_eq!(replaced.emails, created.emails);

    uc.delete_user(&created.id).await.unwrap();
    assert_eq!(api_error(uc.get_user(&created.id).await).status, 404);
}

/// A patch changes the attributes of a user, and refuses a malformed
/// operation.
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
    let work = Struct::from_fields([("value", "ada@work.example"), ("type", "work")]);
    let patched = uc
        .patch_user(
            &id,
            vec![operation("add", Some("emails"), Some(work.into()))],
        )
        .await
        .unwrap();
    let emails: Vec<_> = patched.emails.iter().map(|e| e.value.as_str()).collect();
    assert_eq!(emails, ["ada@example.com", "ada@work.example"]);
    let error = api_error(
        uc.patch_user(&id, vec![operation("remove", None, None)])
            .await,
    );
    assert_eq!(
        (error.status, error.code.as_deref()),
        (400, Some("INVALID_ARGUMENT"))
    );
    let body: serde_json::Value = serde_json::from_str(&error.body).unwrap();
    assert_eq!(body["scimType"], "noTarget");
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
    let searched = uc
        .search_groups()
        .with_filter(r#"displayName eq "admins""#.to_string())
        .with_excluded_attributes(["members".to_string()])
        .await
        .unwrap();
    assert_eq!(searched.total_results, 1);
    assert!(searched.resources[0].members.is_empty());
    // Users and groups at once.
    let everything = uc.search_resources().await.unwrap();
    assert_eq!(everything.total_results, 4);
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
    assert_eq!(api_error(uc.get_group(&created.id).await).status, 404);
}

/// `me` is the user of the client's token. Without a token the server
/// knows no caller: a SCIM 400 error.
#[tokio::test]
async fn me() {
    let url = server_with_keys().await;
    let alice = Client::new_with_token(url.clone(), token("alice@example.com"));
    alice
        .create_user("alice@example.com")
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
        (400, "No user found.")
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
    let location = created.location.as_deref().unwrap();
    assert!(location.contains("/scim2/Users/"), "{location}");
    let failed = response.operations[2].response.as_option().unwrap();
    assert_eq!(failed.get("status").and_then(Value::as_str), Some("404"));
    let users = uc.list_users().await.unwrap().resources;
    assert_eq!(users[0].user_name, "ada@example.com");
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
    assert_eq!(user.attributes[4].name, "emails");
    assert_eq!(user.attributes[4].sub_attributes[0].name, "value");
    let group = uc.get_scim_schema(GROUP).await.unwrap();
    assert_eq!(group.name, "Group");
    let users = uc.get_scim_resource_type("User").await.unwrap();
    assert_eq!(users.endpoint, "/Users");

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
