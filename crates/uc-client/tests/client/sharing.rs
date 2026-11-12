//! Sharing: shares, their permissions and recipients, and what a recipient
//! reads with a Delta Sharing client, signed in with its bearer token.

use futures::TryStreamExt;
use olai_http::CloudClient;
use opensharing_client::client::DeltaSharingClient;
use url::Url;

use uc_client::Client;
use uc_client::models::recipients::v1::{AuthenticationType, Recipient};
use uc_client::models::shares::v1::{
    Action, DataObject, DataObjectType, DataObjectUpdate, HistoryStatus, SharePermissionsChange,
};

use crate::common::{catalog_and_schema, client, external_table, rejected, server};

/// The table `c.s.<name>`, as `default.<name>` of a share.
fn shared(name: &str) -> DataObject {
    DataObject {
        name: format!("c.s.{name}"),
        data_object_type: DataObjectType::Table.into(),
        shared_as: Some(format!("default.{name}")),
        ..Default::default()
    }
}

fn change(action: Action, object: DataObject) -> DataObjectUpdate {
    DataObjectUpdate {
        action: action.into(),
        data_object: object.into(),
        ..Default::default()
    }
}

/// Let `recipient` read `share`, or not any more.
async fn permission(uc: &Client, share: &str, recipient: &str, grant: bool) -> Vec<String> {
    let select = vec!["SELECT".to_string()];
    let change = SharePermissionsChange {
        principal: recipient.into(),
        add: if grant { select.clone() } else { Vec::new() },
        remove: if grant { Vec::new() } else { select },
        ..Default::default()
    };
    let answer = uc
        .share(share)
        .update_share_permissions()
        .with_changes([change])
        .await
        .unwrap();
    (answer.privilege_assignments.into_iter())
        .map(|assignment| assignment.principal)
        .collect()
}

/// The bearer token of the newest token of `recipient`, from its
/// activation link.
async fn activate(uc: &Client, recipient: &Recipient) -> String {
    let link = &recipient.tokens.last().unwrap().activation_url;
    let code = link.rsplit('/').next().unwrap();
    let profile = uc.activate_recipient_token(code).await.unwrap();
    assert_eq!(profile.share_credentials_version, 1);
    assert!(profile.endpoint.ends_with("/api/v1/delta-sharing"));
    profile.bearer_token
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
    assert!(created.objects.is_empty() && created.id.is_some());

    let updated = uc
        .share("analytics")
        .update()
        .with_updates([change(Action::Add, shared("events"))])
        .with_comment("edited".to_string())
        .await
        .unwrap();
    let object = &updated.objects[0];
    assert_eq!(object.name, "c.s.events");
    assert_eq!(object.shared_as.as_deref(), Some("default.events"));
    assert_eq!(
        object.history_data_sharing_status,
        Some(HistoryStatus::Disabled.into())
    );
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    let read = uc.share("analytics").get().with_include_shared_data(true);
    assert_eq!(read.await.unwrap(), updated);
    // Without its objects, as in the list.
    assert!(
        uc.share("analytics")
            .get()
            .await
            .unwrap()
            .objects
            .is_empty()
    );

    let history = DataObject {
        history_data_sharing_status: Some(HistoryStatus::Enabled.into()),
        start_version: Some(1),
        ..shared("events")
    };
    let with_history = uc
        .share("analytics")
        .update()
        .with_updates([change(Action::Update, history)])
        .await
        .unwrap();
    assert_eq!(with_history.objects[0].start_version, Some(1));
    let twice = uc
        .share("analytics")
        .update()
        .with_updates([change(Action::Add, shared("events"))])
        .await;
    rejected(twice, 409, "ALREADY_EXISTS");

    uc.create_share("other").await.unwrap();
    let first = uc.list_shares().with_max_results(1).await.unwrap();
    assert_eq!(first.shares[0].name, "analytics");
    let second = uc
        .list_shares()
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.shares[0].name, "other");

    let renamed = uc
        .share("analytics")
        .update()
        .with_updates([change(Action::Remove, shared("events"))])
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
    // The admin gets the activation link of the token, not the token.
    assert_eq!(created.tokens.len(), 1);
    assert!(created.tokens[0].activation_url.contains("/activations/"));

    assert_eq!(
        permission(&uc, "analytics", "partner", true).await,
        ["partner"]
    );
    let granted = uc.share("analytics").get_share_permissions().await.unwrap();
    assert_eq!(granted.privilege_assignments[0].privileges, ["SELECT"]);
    assert!(
        permission(&uc, "analytics", "partner", false)
            .await
            .is_empty()
    );

    let edited = uc
        .recipient("partner")
        .update()
        .with_comment("edited".to_string())
        .await
        .unwrap();
    assert_eq!(edited.comment.as_deref(), Some("edited"));
    assert_eq!(uc.recipient("partner").get().await.unwrap(), edited);
    uc.create_recipient("other").await.unwrap();
    let first = uc.list_recipients().with_max_results(1).await.unwrap();
    let second = uc
        .list_recipients()
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    let names = [
        first.recipients[0].name.as_str(),
        second.recipients[0].name.as_str(),
    ];
    assert_eq!(names, ["other", "partner"]);

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
    permission(&uc, "analytics", "partner", true).await;
    let token = activate(&uc, &recipient).await;
    (uc, url, token)
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
        .with_updates([change(Action::Add, shared("events"))])
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

/// A rotation gives a new token, and ends the older one at once or after
/// an overlap.
#[tokio::test]
async fn new_tokens() {
    let (uc, url, first) = partner().await;
    assert!(signs_in(&url, &first).await);
    let rotated = uc
        .recipient("partner")
        .rotate_recipient_token()
        .with_existing_token_expire_in_seconds(3600)
        .await
        .unwrap();
    assert_eq!(rotated.tokens.len(), 2);
    assert!(rotated.tokens[0].expiration_time > 0);
    let second = activate(&uc, &rotated).await;
    assert!(signs_in(&url, &first).await && signs_in(&url, &second).await);

    let rotated = uc
        .recipient("partner")
        .rotate_recipient_token()
        .await
        .unwrap();
    assert_eq!(rotated.tokens.len(), 1);
    let third = activate(&uc, &rotated).await;
    assert!(!signs_in(&url, &first).await && !signs_in(&url, &second).await);
    assert!(signs_in(&url, &third).await);

    // An activation link works once.
    let code = rotated.tokens[0].activation_url.rsplit('/').next().unwrap();
    rejected(uc.activate_recipient_token(code).await, 404, "NOT_FOUND");
}
