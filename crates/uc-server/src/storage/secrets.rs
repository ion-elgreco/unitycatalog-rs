//! The store of the secrets of storage credentials (`[secrets]`), and the
//! cache in front of it.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::{info, warn};

use uc_common::settings::{self, SecretStoreKind, invalid};
use uc_metastore::{CachedStore, DatabaseStore, DynMetastore, SecretStore};

/// `[secrets]`, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretsConfig {
    pub store: StoreConfig,
    /// None when `secrets.cache_capacity` is 0.
    pub cache: Option<CacheConfig>,
}

/// `secrets.store` and its settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreConfig {
    /// In the database, sealed with the keys of `key_file`, else with a new
    /// key in memory at each start.
    Database { key_file: Option<PathBuf> },
}

/// `secrets.cache_capacity` and `secrets.cache_ttl`: how many secrets, or
/// Azure sign-ins, stay in memory, and for how long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheConfig {
    pub capacity: u64,
    pub ttl: Duration,
}

impl SecretsConfig {
    pub fn from_settings(secrets: &settings::Secrets) -> Result<Self, String> {
        if secrets.cache_ttl.is_zero() {
            return Err(invalid(
                "secrets.cache_ttl",
                "0s",
                "Expected a positive duration; set secrets.cache_capacity to 0 for no cache.",
            ));
        }
        let store = match secrets.store {
            SecretStoreKind::Database => StoreConfig::Database {
                key_file: secrets.key_file.clone(),
            },
        };
        let cache = (secrets.cache_capacity > 0).then_some(CacheConfig {
            capacity: secrets.cache_capacity,
            ttl: secrets.cache_ttl,
        });
        Ok(Self { store, cache })
    }
}

/// The secret store of the settings.
pub struct Secrets {
    /// What the metastore uses: the store, behind the cache if there is
    /// one.
    pub store: Arc<dyn SecretStore>,
    /// What only the store of the settings knows, for its logs.
    kind: Kind,
    /// The timer that drops expired secrets from the cache.
    expiry: Option<JoinHandle<()>>,
}

enum Kind {
    /// `store = "database"`, and where its keys come from.
    Database {
        store: Arc<DatabaseStore>,
        keys: KeySource,
    },
}

/// Where the keys of the database store come from.
enum KeySource {
    /// `secrets.key_file`.
    File(PathBuf),
    /// Without `secrets.key_file`: a new key in memory at each start.
    Memory,
}

impl Drop for Secrets {
    fn drop(&mut self) {
        if let Some(expiry) = &self.expiry {
            expiry.abort();
        }
    }
}

impl Secrets {
    /// The store of `config`, behind the cache of `config`, whose expired
    /// secrets a timer drops. Without `secrets.key_file`, the database
    /// store gets a new key in memory, as the signing key does; with a
    /// catalog that outlives the server (`lasting_catalog`), the server
    /// warns about it.
    pub fn open(config: &SecretsConfig, lasting_catalog: bool) -> Result<Self, String> {
        let StoreConfig::Database { key_file } = &config.store;
        let (database, keys) = match key_file {
            Some(file) => {
                let cannot =
                    |e: String| format!("cannot load secrets.key_file {}: {e}", file.display());
                let pem = std::fs::read_to_string(file).map_err(|e| cannot(e.to_string()))?;
                let database =
                    DatabaseStore::from_pem(&pem).map_err(|e| cannot(e.message().to_string()))?;
                let key_ids: Vec<&str> = database.ids().collect();
                info!(file = %file.display(), ?key_ids, cache = ?config.cache, "secrets keys loaded");
                (database, KeySource::File(file.clone()))
            }
            None => {
                let database = DatabaseStore::generate()
                    .map_err(|e| format!("cannot make a secrets key: {}", e.message()))?;
                match lasting_catalog {
                    true => warn!(
                        "secrets.key_file is not set: the secrets key is in memory only, so the \
                         secrets of credentials made now cannot be opened after the server \
                         stops. Set it for production."
                    ),
                    false => info!("secrets key in memory (secrets.key_file is not set)"),
                }
                (database, KeySource::Memory)
            }
        };
        let database = Arc::new(database);
        let (store, expiry) = with_cache(database.clone(), config.cache);
        Ok(Self {
            store,
            kind: Kind::Database {
                store: database,
                keys,
            },
            expiry,
        })
    }

    /// Give the store each secret that it keeps in an older form, such as
    /// sealed with an older key, so that old keys can go; and log what it
    /// did and the secrets that it cannot read.
    pub async fn refresh(&self, metastore: &DynMetastore) -> Result<(), String> {
        let report = metastore
            .credentials()
            .refresh_secrets()
            .await
            .map_err(|e| format!("cannot seal the secrets of credentials again: {e}"))?;
        let unreadable = &report.unreadable;
        let Kind::Database { store, keys } = &self.kind;
        if report.refreshed > 0 {
            info!(
                count = report.refreshed,
                key_id = store.current_id(),
                "secrets sealed again with the first key of secrets.key_file"
            );
        }
        match keys {
            KeySource::Memory if !unreadable.is_empty() => warn!(
                credentials = ?unreadable,
                "secrets.key_file is not set, and the new key in memory cannot open the secrets \
                 of these credentials: set secrets.key_file, then give them their secrets again"
            ),
            KeySource::File(file) if !unreadable.is_empty() => warn!(
                credentials = ?unreadable,
                file = %file.display(),
                "no key of secrets.key_file opens the secrets of these credentials"
            ),
            KeySource::File(_) if store.ids().count() > 1 => info!(
                "the first key of secrets.key_file seals every secret: the other keys can go \
                 once every server has the first one"
            ),
            KeySource::File(_) | KeySource::Memory => {}
        }
        Ok(())
    }
}

/// `store` behind a cache of `cache`, and the timer that drops expired
/// secrets every TTL, so that none stays in memory long after it expires.
fn with_cache(
    store: Arc<dyn SecretStore>,
    cache: Option<CacheConfig>,
) -> (Arc<dyn SecretStore>, Option<JoinHandle<()>>) {
    let Some(CacheConfig { capacity, ttl }) = cache else {
        return (store, None);
    };
    let cached = Arc::new(CachedStore::new(store, capacity, ttl));
    let expiry = tokio::spawn({
        let cached = cached.clone();
        async move {
            loop {
                tokio::time::sleep(ttl).await;
                cached.drop_expired().await;
            }
        }
    });
    (cached, Some(expiry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parse_into_a_store_and_a_cache() {
        let defaults = SecretsConfig::from_settings(&settings::Secrets::default()).unwrap();
        assert_eq!(defaults.store, StoreConfig::Database { key_file: None });
        assert_eq!(
            defaults.cache,
            Some(CacheConfig {
                capacity: 500,
                ttl: Duration::from_secs(600),
            })
        );
        let no_cache = settings::Secrets {
            cache_capacity: 0,
            ..Default::default()
        };
        assert_eq!(SecretsConfig::from_settings(&no_cache).unwrap().cache, None);
        let no_ttl = settings::Secrets {
            cache_ttl: Duration::ZERO,
            ..Default::default()
        };
        assert!(
            SecretsConfig::from_settings(&no_ttl)
                .unwrap_err()
                .starts_with("Invalid value for secrets.cache_ttl")
        );
    }

    #[tokio::test]
    async fn a_database_store_has_keys_from_the_file_or_in_memory() {
        let config = |key_file: Option<PathBuf>, cache_capacity| {
            SecretsConfig::from_settings(&settings::Secrets {
                key_file,
                cache_capacity,
                ..Default::default()
            })
            .unwrap()
        };
        let in_memory = Secrets::open(&config(None, 500), true).unwrap();
        let Kind::Database { keys, .. } = &in_memory.kind;
        assert!(matches!(keys, KeySource::Memory));

        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("secrets-key.pem");
        std::fs::write(&file, DatabaseStore::generate_key_pem().unwrap()).unwrap();
        let cached = Secrets::open(&config(Some(file.clone()), 500), true).unwrap();
        let Kind::Database { keys, .. } = &cached.kind;
        assert!(matches!(keys, KeySource::File(path) if *path == file));
        assert!(format!("{:?}", cached.store).starts_with("CachedStore"));
        assert!(cached.expiry.is_some());
        let uncached = Secrets::open(&config(Some(file.clone()), 0), true).unwrap();
        assert!(format!("{:?}", uncached.store).starts_with("DatabaseStore"));
        assert!(uncached.expiry.is_none());

        // The RSA signing key is no secrets key.
        let signing = crate::authn::security::generate_key_pem().unwrap();
        std::fs::write(&file, signing).unwrap();
        let error = Secrets::open(&config(Some(file), 0), true).err().unwrap();
        assert!(
            error.starts_with("cannot load secrets.key_file") && error.contains("P-256"),
            "{error}"
        );
    }
}
