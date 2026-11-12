//! Sharing: shares and recipients, and what a recipient reads with a Delta
//! Sharing client, signed in with its bearer token.

use futures::TryStreamExt;
use olai_http::CloudClient;
use opensharing_client::client::DeltaSharingClient;
use url::Url;

use uc_client::Client;
use uc_client::models::recipients::v1::AuthenticationType;
use uc_client::models::shares::v1::{HistoryDataSharingStatus, SharedDataObject, SharedObjectType};

use crate::common::{catalog_and_schema, client, external_table, rejected, server};

/// The table `c.s.<name>`, as `default.<name>` of a share.
fn shared(name: &str) -> SharedDataObject {
    SharedDataObject {
        source_full_name: format!("c.s.{name}"),
        data_object_type: Some(SharedObjectType::Table.into()),
        history_data_sharing_status: Some(HistoryDataSharingStatus::Disabled.into()),
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

/// The recipient `partner` of the share `analytics`: the client of the API,
/// the root URL of the server and the bearer token of the recipient.
async fn partner() -> (Client, Url, String) {
    let url = server().await;
    let uc = Client::new_unauthenticated(url.clone());
    uc.create_share("analytics").await.unwrap();
    let recipient = uc.create_recipient("partner").await.unwrap();
    uc.recipient("partner")
        .update()
        .with_add_shares(["analytics".to_string()])
        .await
        .unwrap();
    (uc, url, recipient.bearer_token.unwrap())
}

fn reader(url: &Url, token: &str) -> DeltaSharingClient {
    DeltaSharingClient::new(CloudClient::new_with_token(token), url.clone())
}

/// Whether the recipient's token `token` signs in.
async fn signs_in(url: &Url, token: &str) -> bool {
    let shares: Result<Vec<_>, _> = reader(url, token).list_shares(None).try_collect().await;
    shares.is_ok()
}

/// A Delta table of two versions in `dir`: version 1 adds one file.
fn delta_table(dir: &std::path::Path) {
    let log = dir.join("_delta_log");
    std::fs::create_dir_all(&log).unwrap();
    let v0 = concat!(
        r#"{"protocol":{"minReaderVersion":1,"minWriterVersion":2}}"#,
        "\n",
        r#"{"metaData":{"id":"5fba94ed-9794-4965-ba6e-6ee3c0d22af9","format":{"provider":"parquet","options":{}},"#,
        r#""schemaString":"{\"type\":\"struct\",\"fields\":[{\"name\":\"id\",\"type\":\"long\",\"nullable\":true,\"metadata\":{}}]}","#,
        r#""partitionColumns":[],"configuration":{},"createdTime":1700000000000}}"#,
        "\n",
    );
    let v1 = concat!(
        r#"{"add":{"path":"part-0.parquet","partitionValues":{},"size":100,"modificationTime":1700000000000,"#,
        r#""dataChange":true,"stats":"{\"numRecords\":3}"}}"#,
        "\n",
    );
    std::fs::write(log.join("00000000000000000000.json"), v0).unwrap();
    std::fs::write(log.join("00000000000000000001.json"), v1).unwrap();
}

/// What a recipient reads with a Delta Sharing client.
#[tokio::test]
async fn the_protocol() {
    let (uc, url, token) = partner().await;
    let storage = catalog_and_schema(&uc).await;
    delta_table(&storage.path().join("events"));
    let table = external_table(&uc, &storage, "events").await;
    uc.share("analytics")
        .update()
        .with_add_objects([shared("events")])
        .await
        .unwrap();

    let partner = reader(&url, &token);
    let shares: Vec<_> = partner.list_shares(None).try_collect().await.unwrap();
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].name, "analytics");
    let schemas: Vec<_> = (partner
        .list_share_schemas("analytics", None)
        .try_collect()
        .await)
        .unwrap();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0].name, "default");
    let tables: Vec<_> = (partner
        .list_all_tables("analytics", None)
        .try_collect()
        .await)
        .unwrap();
    assert_eq!(tables.len(), 1);
    assert_eq!(
        (tables[0].schema.as_str(), tables[0].name.as_str()),
        ("default", "events")
    );

    let version = partner.get_table_version("analytics", "default", "events", None);
    assert_eq!(version.await.unwrap(), 1);
    let (protocol, metadata) = (partner
        .get_table_metadata("analytics", "default", "events")
        .await)
        .unwrap();
    assert_eq!(protocol.min_reader_version(), 1);
    assert_eq!(metadata.id(), "5fba94ed-9794-4965-ba6e-6ee3c0d22af9");
    let query = (partner
        .query_table("analytics", "default", "events", &Default::default())
        .await)
        .unwrap();
    let query = String::from_utf8(query).unwrap();
    let file = format!("{}/part-0.parquet", table.storage_location.unwrap());
    assert!(
        query.lines().count() == 3 && query.contains(&file),
        "{query}"
    );

    assert!(
        partner
            .get_table_version("analytics", "default", "nope", None)
            .await
            .is_err()
    );
    // A share that the recipient was not granted.
    uc.create_share("secret").await.unwrap();
    let secret: Result<Vec<_>, _> = partner
        .list_share_schemas("secret", None)
        .try_collect()
        .await;
    assert!(secret.is_err());
}

/// A new bearer token, or a profile with one, ends the token before.
#[tokio::test]
async fn new_tokens() {
    let (uc, url, token) = partner().await;
    assert!(signs_in(&url, &token).await);
    let rotated = uc
        .recipient("partner")
        .rotate_recipient_token()
        .await
        .unwrap();
    assert!(!signs_in(&url, &token).await);
    let token = rotated.bearer_token.unwrap();
    assert!(signs_in(&url, &token).await);

    let profile = uc
        .recipient("partner")
        .create_recipient_profile()
        .await
        .unwrap();
    assert_eq!(profile.share_credentials_version, 1);
    assert_eq!(profile.endpoint, format!("{url}api/v1/delta-sharing"));
    assert_eq!(profile.expiration_time, None);
    assert!(!signs_in(&url, &token).await);
    assert!(signs_in(&url, &profile.bearer_token).await);
}
