//! Backup, restore and the audit log, which take a signed-in admin, and
//! the probes of the server.

use std::time::Duration;

use buffa_types::google::protobuf::Value;

use uc_client::Client;
use uc_client::models::audit::v1::AuditEvent;
use uc_server::authn::security::ADMIN;

use crate::common::{client, rejected, server_with_keys, token};

const CATALOGS: &str = "/api/2.1/unity-catalog/catalogs";

#[tokio::test]
async fn backup_and_restore() {
    let url = server_with_keys().await;
    let admin = Client::new_with_token(url.clone(), token(ADMIN));
    admin
        .create_catalog("c")
        .with_comment("before".to_string())
        .await
        .unwrap();
    let backup = admin.get_backup().await.unwrap();
    assert_eq!(backup.version, 1);
    let named_c = |row: &Value| {
        row.as_struct()
            .and_then(|r| r.get("name"))
            .and_then(Value::as_str)
            == Some("c")
    };
    assert!(backup.tables["catalog"].iter().any(named_c), "{backup:?}");

    // The restore puts back what the backup holds.
    admin.catalog("c").delete().await.unwrap();
    let report = admin
        .restore_backup(backup.version, backup.tables.clone())
        .await
        .unwrap();
    assert!(report.skipped.is_empty(), "{report:?}");
    assert!(report.imported["catalog"] >= 1, "{report:?}");
    let restored = admin.catalog("c").get().await.unwrap();
    assert_eq!(restored.comment.as_deref(), Some("before"));
    let error = rejected(
        admin.restore_backup(99, backup.tables).await,
        400,
        "INVALID_ARGUMENT",
    );
    assert!(error.message.contains("99"), "{error:?}");

    let anonymous = Client::new_unauthenticated(url);
    rejected(anonymous.get_backup().await, 401, "UNAUTHENTICATED");
}

/// The audit log: the requests that could change the catalog, newest
/// first, filtered and in pages.
#[tokio::test]
async fn audit_events() {
    let url = server_with_keys().await;
    let admin = Client::new_with_token(url.clone(), token(ADMIN));
    for name in ["a", "b"] {
        admin.create_catalog(name).await.unwrap();
    }
    admin.catalog("a").delete().await.unwrap();
    let posts = until_logged(&admin, 2).await;
    assert!(
        posts
            .iter()
            .all(|e| e.method == "POST" && e.path == CATALOGS && e.principal == ADMIN),
        "{posts:?}"
    );
    assert!(posts.iter().all(|e| e.status == 200 && e.timestamp > 0));

    let (since, until) = (posts[1].timestamp, posts[0].timestamp);
    let filtered = |principal: &str, path_prefix: &str, since: i64, until: i64| {
        admin
            .list_audit_events()
            .with_principal(principal.to_string())
            .with_method("POST".to_string())
            .with_path_prefix(path_prefix.to_string())
            .with_since(since)
            .with_until(until)
    };
    let in_range = filtered(ADMIN, CATALOGS, since, until).await.unwrap();
    assert_eq!(in_range.events, posts);
    // Each filter counts.
    for other in [
        filtered("bob@example.com", CATALOGS, since, until),
        filtered(ADMIN, "/api/2.1/unity-catalog/schemas", since, until),
        filtered(ADMIN, CATALOGS, until + 1, i64::MAX),
        filtered(ADMIN, CATALOGS, 0, since - 1),
    ] {
        assert!(other.await.unwrap().events.is_empty());
    }
    let first = admin
        .list_audit_events()
        .with_method("POST".to_string())
        .with_max_results(1)
        .await
        .unwrap();
    assert_eq!(first.events, [posts[0].clone()]);
    let second = admin
        .list_audit_events()
        .with_method("POST".to_string())
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.events, [posts[1].clone()]);

    let anonymous = Client::new_unauthenticated(url);
    rejected(anonymous.list_audit_events().await, 401, "UNAUTHENTICATED");
}

/// The `POST` events of the catalogs, once `count` of them are in the log:
/// the server writes them in the background.
async fn until_logged(admin: &Client, count: usize) -> Vec<AuditEvent> {
    for _ in 0..100 {
        let events = admin
            .list_audit_events()
            .with_method("POST".to_string())
            .with_path_prefix(CATALOGS.to_string())
            .await
            .unwrap()
            .events;
        if events.len() >= count {
            return events;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the audit log has fewer than {count} events");
}

#[tokio::test]
async fn health_and_version() {
    let uc = client().await;
    assert_eq!(uc.get_health().await.unwrap().status, "ok");
    let version = uc.get_version().await.unwrap();
    assert_eq!(version.server, "unitycatalog-rs");
    assert_eq!(version.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(version.api_version, "2.1");
}
