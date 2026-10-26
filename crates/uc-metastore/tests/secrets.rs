//! The secrets of storage credentials: in the database store, sealed with
//! a key of `secrets.key_file`, and sealed again with a new first key. On
//! Turso, or on PostgreSQL when `UC_TEST_POSTGRES_URL` is set.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use p256::{SecretKey, elliptic_curve::Generate};
use uc_api::{
    AwsIamRoleRequest, AzureServicePrincipalRequest, CreateCredentialRequest, CredentialPurpose,
    GcpServiceAccountKeyRequest, UpdateCredentialRequest,
};
use uc_common::ErrorCode;
use uc_metastore::{
    DatabaseStore, Metastore, MetastoreConfig, RefreshReport, SecretRef, SecretStore, SecretValue,
    SqlMetastore,
};
use uc_storage::{self as storage, Storage};

/// A catalog on `db` with the secret store `store`.
async fn with_store(db: &Arc<dyn Storage>, store: Arc<dyn SecretStore>) -> Arc<SqlMetastore> {
    SqlMetastore::new(db.clone(), Default::default(), Default::default(), store)
        .await
        .unwrap()
}

/// A catalog on `db` with a database store of the keys `keys`, newest
/// first.
async fn metastore(db: &Arc<dyn Storage>, keys: &[&SecretKey]) -> Arc<SqlMetastore> {
    let keys = keys.iter().map(|&key| key.clone()).collect();
    with_store(db, Arc::new(DatabaseStore::new(keys).unwrap())).await
}

fn new_key() -> SecretKey {
    SecretKey::try_generate().unwrap()
}

fn request(name: &str) -> CreateCredentialRequest {
    CreateCredentialRequest {
        name: name.into(),
        comment: None,
        aws_iam_role: None,
        purpose: Some(CredentialPurpose::Storage),
        azure_service_principal: None,
        azure_managed_identity: None,
        gcp_service_account: None,
        gcp_service_account_key: None,
    }
}

fn azure(name: &str) -> CreateCredentialRequest {
    CreateCredentialRequest {
        azure_service_principal: Some(AzureServicePrincipalRequest {
            directory_id: "tenant".into(),
            application_id: "app".into(),
            client_secret: "s3cret".into(),
        }),
        ..request(name)
    }
}

/// The stored text of the secret of credential `name`.
async fn stored(db: &Arc<dyn Storage>, name: &str) -> String {
    let rows = db
        .query(
            "SELECT azure_client_secret, gcp_json_key FROM credential WHERE name = $1",
            &[name.into()],
        )
        .await
        .unwrap();
    let row = &rows[0];
    row.opt_text(0)
        .unwrap()
        .or(row.opt_text(1).unwrap())
        .unwrap()
}

async fn secret(metastore: &SqlMetastore, name: &str) -> uc_common::Result<Option<String>> {
    metastore
        .credentials()
        .get_with_secret(name)
        .await
        .map(|(_, secret)| secret.map(|secret| secret.as_sensitive_str().to_string()))
}

#[tokio::test]
async fn secrets_are_sealed_and_need_their_key() {
    let db = storage::open(&MetastoreConfig::for_tests().database)
        .await
        .unwrap();
    let with_key = metastore(&db, &[&new_key()]).await;
    // Another key, as after a restart with a key in memory.
    let other = metastore(&db, &[&new_key()]).await;

    // An AWS role has no secret.
    let role = CreateCredentialRequest {
        aws_iam_role: Some(AwsIamRoleRequest {
            role_arn: "arn:aws:iam::123456789012:role/uc".into(),
        }),
        ..request("aws")
    };
    with_key.credentials().create(role, None).await.unwrap();
    assert_eq!(secret(&other, "aws").await.unwrap(), None);

    // The database holds only the sealed secret.
    with_key
        .credentials()
        .create(azure("az"), None)
        .await
        .unwrap();
    let json_key = serde_json::json!({
        "type": "service_account",
        "client_email": "uc@project.iam.gserviceaccount.com",
        "private_key": "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n",
    })
    .to_string();
    let gcp = CreateCredentialRequest {
        gcp_service_account_key: Some(GcpServiceAccountKeyRequest {
            json_key: json_key.clone(),
        }),
        ..request("gcp")
    };
    with_key.credentials().create(gcp, None).await.unwrap();
    for (name, plain) in [("az", "s3cret"), ("gcp", json_key.as_str())] {
        let sealed = stored(&db, name).await;
        assert!(
            sealed.starts_with("v1.") && !sealed.contains(plain),
            "{sealed}"
        );
        assert_eq!(
            secret(&with_key, name).await.unwrap().as_deref(),
            Some(plain)
        );
    }

    // Another key cannot open them.
    let error = secret(&other, "az").await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::FailedPrecondition);
    assert!(error.message().contains("secrets.key_file"), "{error:?}");
    assert_eq!(
        other.credentials().refresh_secrets().await.unwrap(),
        RefreshReport {
            refreshed: 0,
            unreadable: vec!["az".into(), "gcp".into()],
        }
    );
}

#[tokio::test]
async fn a_new_first_key_seals_the_secrets_again() {
    let db = storage::open(&MetastoreConfig::for_tests().database)
        .await
        .unwrap();
    let (old, new) = (new_key(), new_key());
    let before = metastore(&db, &[&old]).await;
    before
        .credentials()
        .create(azure("az"), None)
        .await
        .unwrap();
    let sealed_by_old = stored(&db, "az").await;
    assert_eq!(
        before.credentials().refresh_secrets().await.unwrap(),
        RefreshReport::default()
    );

    // The new key first: the start seals the secret again, once.
    let rotated = metastore(&db, &[&new, &old]).await;
    assert_eq!(
        rotated.credentials().refresh_secrets().await.unwrap(),
        RefreshReport {
            refreshed: 1,
            unreadable: vec![],
        }
    );
    assert_ne!(stored(&db, "az").await, sealed_by_old);
    assert_eq!(
        rotated.credentials().refresh_secrets().await.unwrap(),
        RefreshReport::default()
    );

    // Then the old key can go; a server with only the old key is stuck.
    let after = metastore(&db, &[&new]).await;
    assert_eq!(
        secret(&after, "az").await.unwrap().as_deref(),
        Some("s3cret")
    );
    let error = secret(&before, "az").await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::FailedPrecondition);
    assert!(
        error.message().contains("is not in secrets.key_file"),
        "{error:?}"
    );
}

/// A database store that records what it is asked to remove, and that
/// does not refresh.
#[derive(Debug)]
struct Recording {
    inner: DatabaseStore,
    deleted: Mutex<Vec<String>>,
}

#[async_trait]
impl SecretStore for Recording {
    async fn put(&self, owner: &str, secret: &SecretValue) -> uc_common::Result<SecretRef> {
        self.inner.put(owner, secret).await
    }
    async fn get(&self, owner: &str, reference: &SecretRef) -> uc_common::Result<SecretValue> {
        self.inner.get(owner, reference).await
    }
    async fn delete(&self, _: &str, reference: &SecretRef) -> uc_common::Result<()> {
        self.deleted
            .lock()
            .unwrap()
            .push(reference.as_str().to_string());
        Ok(())
    }
    async fn refresh(&self, _: &str, _: &SecretRef) -> uc_common::Result<Option<SecretRef>> {
        panic!("a store that does not refresh is never asked to")
    }
}

#[tokio::test]
async fn a_replaced_or_dropped_secret_leaves_the_store() {
    let db = storage::open(&MetastoreConfig::for_tests().database)
        .await
        .unwrap();
    let store = Arc::new(Recording {
        inner: DatabaseStore::new(vec![new_key()]).unwrap(),
        deleted: Mutex::default(),
    });
    let metastore = with_store(&db, store.clone()).await;
    let credentials = metastore.credentials();
    let deleted = || store.deleted.lock().unwrap().clone();

    credentials.create(azure("az"), None).await.unwrap();
    let first = stored(&db, "az").await;
    // A new comment keeps the secret.
    let comment = UpdateCredentialRequest {
        comment: Some("lake".into()),
        ..Default::default()
    };
    credentials.update("az", comment, None).await.unwrap();
    assert_eq!(deleted(), Vec::<String>::new());

    // A new secret replaces the old one in the store.
    let principal = UpdateCredentialRequest {
        azure_service_principal: Some(AzureServicePrincipalRequest {
            directory_id: "tenant".into(),
            application_id: "app".into(),
            client_secret: "n3w".into(),
        }),
        ..Default::default()
    };
    credentials.update("az", principal, None).await.unwrap();
    let second = stored(&db, "az").await;
    assert_eq!(deleted(), std::slice::from_ref(&first));
    assert_eq!(
        secret(&metastore, "az").await.unwrap().as_deref(),
        Some("n3w")
    );

    // A store that does not refresh: the start leaves its secrets alone.
    assert_eq!(
        credentials.refresh_secrets().await.unwrap(),
        RefreshReport::default()
    );

    credentials.delete("az", false).await.unwrap();
    assert_eq!(deleted(), [first, second]);
}
