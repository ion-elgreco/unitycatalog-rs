//! Where the server keeps the secrets of storage credentials: the client
//! secret of an Azure service principal and the key of a GCP service
//! account.
//!
//! A [`SecretStore`] stores a secret and gives a [`SecretRef`], which the
//! credential's row keeps. [`DatabaseStore`] keeps the secret itself in the
//! row, sealed with the server's key. [`CachedStore`] keeps what a store
//! reads in memory for a short time.

mod database;

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache;

pub use database::{DatabaseStore, OpenError};
pub use secret_vault_value::SecretValue;
use uc_common::{Error, Result};

/// What a row keeps of a stored secret: for [`DatabaseStore`], the sealed
/// secret itself; for a store outside the database, where the secret is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretRef(String);

impl SecretRef {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// A store of secrets. `owner` is the id of the credential that the secret
/// belongs to, and a store may bind the secret to it.
///
/// A reference never changes: a new secret gets a new reference, so that a
/// cache keyed on references never holds an old secret.
#[async_trait]
pub trait SecretStore: Send + Sync + fmt::Debug {
    /// Store `secret` for `owner`; the reference that the row keeps.
    async fn put(&self, owner: &str, secret: &SecretValue) -> Result<SecretRef>;

    /// The secret of `reference`, stored for `owner`.
    async fn get(&self, owner: &str, reference: &SecretRef) -> Result<SecretValue>;

    /// Remove the secret of `reference`.
    async fn delete(&self, owner: &str, reference: &SecretRef) -> Result<()>;

    /// Whether the store keeps secrets in forms that [`Self::refresh`]
    /// renews, such as sealed with an older key. A store that does not is
    /// never asked.
    fn refreshes(&self) -> bool {
        false
    }

    /// A new reference to the same secret when the store keeps it in an
    /// older form, such as sealed with an older key; `None` when it is
    /// current, as always in a store that does not refresh.
    async fn refresh(&self, _owner: &str, _reference: &SecretRef) -> Result<Option<SecretRef>> {
        Ok(None)
    }
}

/// A store that keeps the secrets it reads in memory, each for at most the
/// TTL. Its entries are keyed on the owner and the reference, so a changed
/// credential never gets its old secret, and a dropped entry wipes its
/// secret.
pub struct CachedStore {
    inner: Arc<dyn SecretStore>,
    cache: Cache<(String, SecretRef), SecretValue>,
}

impl fmt::Debug for CachedStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedStore")
            .field("inner", &self.inner)
            .field("entries", &self.cache.entry_count())
            .finish()
    }
}

impl CachedStore {
    /// `inner`, with at most `capacity` secrets in memory, each for at most
    /// `ttl`.
    pub fn new(inner: Arc<dyn SecretStore>, capacity: u64, ttl: Duration) -> Self {
        Self {
            inner,
            cache: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(ttl)
                .build(),
        }
    }

    /// Drop the entries that have expired. The cache drops them only when
    /// it is used, so the server calls this on a timer.
    pub async fn drop_expired(&self) {
        self.cache.run_pending_tasks().await;
    }

    /// The number of secrets in memory.
    pub async fn entry_count(&self) -> u64 {
        self.cache.run_pending_tasks().await;
        self.cache.entry_count()
    }
}

#[async_trait]
impl SecretStore for CachedStore {
    async fn put(&self, owner: &str, secret: &SecretValue) -> Result<SecretRef> {
        self.inner.put(owner, secret).await
    }

    async fn get(&self, owner: &str, reference: &SecretRef) -> Result<SecretValue> {
        // Concurrent misses of one key share one read of the store.
        self.cache
            .try_get_with(
                (owner.to_string(), reference.clone()),
                self.inner.get(owner, reference),
            )
            .await
            .map_err(|e| Error::new(e.code(), e.message()))
    }

    async fn delete(&self, owner: &str, reference: &SecretRef) -> Result<()> {
        self.cache
            .invalidate(&(owner.to_string(), reference.clone()))
            .await;
        self.inner.delete(owner, reference).await
    }

    fn refreshes(&self) -> bool {
        self.inner.refreshes()
    }

    async fn refresh(&self, owner: &str, reference: &SecretRef) -> Result<Option<SecretRef>> {
        let refreshed = self.inner.refresh(owner, reference).await?;
        if refreshed.is_some() {
            self.cache
                .invalidate(&(owner.to_string(), reference.clone()))
                .await;
        }
        Ok(refreshed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A store that counts its reads; the secret of a reference is the
    /// reference.
    #[derive(Debug, Default)]
    struct Counting {
        reads: AtomicUsize,
    }

    #[async_trait]
    impl SecretStore for Counting {
        async fn put(&self, _: &str, secret: &SecretValue) -> Result<SecretRef> {
            Ok(SecretRef::new(secret.as_sensitive_str()))
        }
        async fn get(&self, _: &str, reference: &SecretRef) -> Result<SecretValue> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            match reference.as_str() {
                "missing" => Err(Error::not_found("no such secret")),
                text => Ok(SecretValue::from(text)),
            }
        }
        async fn delete(&self, _: &str, _: &SecretRef) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn the_cache_reads_a_secret_once_until_it_expires() {
        let inner = Arc::new(Counting::default());
        let ttl = Duration::from_millis(200);
        let cached = CachedStore::new(inner.clone(), 10, ttl);
        let (a, b) = (SecretRef::new("a"), SecretRef::new("b"));
        let reads = || inner.reads.load(Ordering::SeqCst);

        for _ in 0..3 {
            let secret = cached.get("credential-1", &a).await.unwrap();
            assert_eq!(secret.as_sensitive_str(), "a");
        }
        assert_eq!(reads(), 1);
        // Another reference or another owner is another entry.
        cached.get("credential-1", &b).await.unwrap();
        cached.get("credential-2", &a).await.unwrap();
        assert_eq!(reads(), 3);
        // A failed read is not kept.
        for _ in 0..2 {
            let error = cached
                .get("credential-1", &SecretRef::new("missing"))
                .await
                .unwrap_err();
            assert_eq!(error.code(), uc_common::ErrorCode::NotFound);
        }
        assert_eq!(reads(), 5);
        assert_eq!(cached.entry_count().await, 3);

        // A deleted secret is dropped at once.
        cached.delete("credential-1", &b).await.unwrap();
        assert_eq!(cached.entry_count().await, 2);

        // After the TTL, every secret is gone from memory.
        tokio::time::sleep(ttl + Duration::from_millis(100)).await;
        cached.drop_expired().await;
        assert_eq!(cached.entry_count().await, 0);
        cached.get("credential-1", &a).await.unwrap();
        assert_eq!(reads(), 6);
    }

    #[test]
    fn a_secret_never_shows_in_debug_output() {
        let secret = SecretValue::from("s3cret");
        assert!(!format!("{secret:?}").contains("s3cret"));
    }
}
