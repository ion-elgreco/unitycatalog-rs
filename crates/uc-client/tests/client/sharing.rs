//! Delta Sharing: shares and recipients, and the protocol that a recipient
//! reads its shares with, signed in with its bearer token.

use futures::TryStreamExt;
use url::Url;

use uc_client::Client;
use uc_client::models::recipients::v1::AuthenticationType;
use uc_client::models::shares::v1::SharedDataObject;
use uc_client::models::sharing::v1::SharingTable;
use uc_client::models::tables::v1::TableInfo;

use crate::common::{catalog_and_schema, client, external_table, rejected, server};

/// The table `c.s.<name>`, as `default.<name>` of a share.
fn shared(name: &str) -> SharedDataObject {
    SharedDataObject {
        source_table_full_name: format!("c.s.{name}"),
        shared_as_schema: "default".into(),
        shared_as_name: name.into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn shares() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    external_table(&uc, &storage, "events").await;
    let created = uc
        .create_share("analytics")
        .with_comment("for partners".to_string())
        .await
        .unwrap();
    assert_eq!(created.name, "analytics");
    assert_eq!(created.comment.as_deref(), Some("for partners"));
    assert!(created.objects.is_empty() && !created.id.is_empty());

    let updated = uc
        .share("analytics")
        .update()
        .with_add_objects([shared("events")])
        .with_comment("edited".to_string())
        .await
        .unwrap();
    assert_eq!(updated.objects, [shared("events")]);
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    assert_eq!(uc.share("analytics").get().await.unwrap(), updated);
    uc.create_share("other").await.unwrap();
    let first = uc.list_shares().with_max_results(1).await.unwrap();
    assert_eq!(first.items, std::slice::from_ref(&updated));
    let second = uc
        .list_shares()
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.items[0].name, "other");

    let renamed = uc
        .share("analytics")
        .update()
        .with_remove_objects([shared("events")])
        .with_new_name("renamed".to_string())
        .await
        .unwrap();
    assert_eq!(renamed.name, "renamed");
    assert!(renamed.objects.is_empty());
    uc.share("renamed").delete().await.unwrap();
    rejected(uc.share("renamed").get().await, 404, "NOT_FOUND");
    let left: Vec<_> = uc.list_shares().into_stream().try_collect().await.unwrap();
    assert_eq!(left.len(), 1);
}

#[tokio::test]
async fn recipients() {
    let uc = client().await;
    uc.create_share("analytics").await.unwrap();
    let created = uc
        .create_recipient("partner")
        .with_authentication_type(AuthenticationType::Token)
        .with_comment("partner x".to_string())
        .await
        .unwrap();
    assert_eq!(created.authentication_type, AuthenticationType::Token);
    assert_eq!(created.comment.as_deref(), Some("partner x"));
    let token = created.bearer_token.clone().expect("the token, once");
    assert!(token.starts_with("dsr_") && token.starts_with(&created.token_prefix));

    let granted = uc
        .recipient("partner")
        .update()
        .with_add_shares(["analytics".to_string()])
        .with_comment("edited".to_string())
        .await
        .unwrap();
    assert_eq!(granted.shares, ["analytics"]);
    assert_eq!(granted.comment.as_deref(), Some("edited"));
    assert_eq!(granted.bearer_token, None);
    assert_eq!(uc.recipient("partner").get().await.unwrap(), granted);
    uc.create_recipient("other").await.unwrap();
    let first = uc.list_recipients().with_max_results(1).await.unwrap();
    let second = uc
        .list_recipients()
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    let mut names = vec![first.items[0].name.clone(), second.items[0].name.clone()];
    names.sort();
    assert_eq!(names, ["other", "partner"]);

    let revoked = uc
        .recipient("partner")
        .update()
        .with_remove_shares(["analytics".to_string()])
        .await
        .unwrap();
    assert!(revoked.shares.is_empty());
    uc.recipient("partner").delete().await.unwrap();
    rejected(uc.recipient("partner").get().await, 404, "NOT_FOUND");
    let left: Vec<_> = uc
        .list_recipients()
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(left.len(), 1);
}

/// The share `analytics` with the table `c.s.events` as `default.events`,
/// and its recipient `partner`: the client of the API, the root URL of the
/// server, the bearer token of the recipient and the table.
async fn shared_table() -> (Client, Url, String, TableInfo) {
    let url = server().await;
    let uc = Client::new_unauthenticated(url.clone());
    let storage = catalog_and_schema(&uc).await;
    let table = external_table(&uc, &storage, "events").await;
    uc.create_share("analytics").await.unwrap();
    uc.share("analytics")
        .update()
        .with_add_objects([shared("events")])
        .await
        .unwrap();
    let recipient = uc.create_recipient("partner").await.unwrap();
    uc.recipient("partner")
        .update()
        .with_add_shares(["analytics".to_string()])
        .await
        .unwrap();
    (uc, url, recipient.bearer_token.unwrap(), table)
}

/// What a recipient reads, signed in with its bearer token.
#[tokio::test]
async fn the_protocol() {
    let (uc, url, token, table) = shared_table().await;
    let partner = Client::new_with_token(url, token);
    let shares = partner.sharing_list_shares().await.unwrap();
    assert_eq!(shares.items.len(), 1);
    assert_eq!(shares.items[0].name, "analytics");
    let schemas = partner.sharing_list_schemas("analytics").await.unwrap();
    let [schema] = schemas.items.as_slice() else {
        panic!("one schema: {schemas:?}");
    };
    assert_eq!(
        (schema.name.as_str(), schema.share.as_str()),
        ("default", "analytics")
    );
    let events = SharingTable {
        name: "events".into(),
        schema: "default".into(),
        share: "analytics".into(),
        ..Default::default()
    };
    let tables = partner
        .sharing_list_tables("analytics", "default")
        .await
        .unwrap();
    assert_eq!(tables.items, std::slice::from_ref(&events));
    let all = partner.sharing_list_all_tables("analytics").await.unwrap();
    assert_eq!(all.items, [events]);

    let version = partner
        .sharing_get_table_version("analytics", "default", "events")
        .await
        .unwrap();
    assert_eq!(version.version, 0);
    let lines = partner
        .sharing_get_table_metadata("analytics", "default", "events")
        .await
        .unwrap();
    let [protocol, metadata] = lines.as_slice() else {
        panic!("a protocol and a metadata line: {lines:?}");
    };
    assert_eq!(protocol.protocol.as_option().unwrap().min_reader_version, 1);
    let metadata = metadata.metadata.as_option().unwrap();
    assert_eq!(
        (metadata.id.as_str(), metadata.name.as_str()),
        (table.table_id.as_str(), "events")
    );
    assert_eq!(metadata.format.provider, "parquet");
    assert!(metadata.schema_string.contains("\"id\""));
    let queried = partner
        .sharing_query_table("analytics", "default", "events")
        .await
        .unwrap();
    assert_eq!(queried, lines);
    rejected(
        partner
            .sharing_get_table_version("analytics", "default", "nope")
            .await,
        404,
        "NOT_FOUND",
    );
    // A share that the recipient may not read.
    uc.create_share("secret").await.unwrap();
    rejected(
        partner.sharing_list_schemas("secret").await,
        403,
        "PERMISSION_DENIED",
    );
}

/// A new bearer token, or a profile with one, ends the token before.
#[tokio::test]
async fn new_tokens() {
    let (uc, url, token, _) = shared_table().await;
    let partner = Client::new_with_token(url.clone(), token);
    let rotated = uc
        .recipient("partner")
        .rotate_recipient_token()
        .await
        .unwrap();
    rejected(partner.sharing_list_shares().await, 401, "UNAUTHENTICATED");
    let partner = Client::new_with_token(url.clone(), rotated.bearer_token.unwrap());
    partner.sharing_list_shares().await.unwrap();

    let profile = uc
        .recipient("partner")
        .create_recipient_profile()
        .await
        .unwrap();
    assert_eq!(profile.share_credentials_version, 1);
    let endpoint = format!("{url}api/2.1/unity-catalog/delta-sharing");
    assert_eq!(profile.endpoint, endpoint);
    assert_eq!(profile.expiration_time, None);
    rejected(partner.sharing_list_shares().await, 401, "UNAUTHENTICATED");
    let partner = Client::new_with_token(url, profile.bearer_token);
    partner.sharing_list_shares().await.unwrap();
}
