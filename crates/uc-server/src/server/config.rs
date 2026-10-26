//! uc-server's configuration, from its settings (`uc_common::settings`):
//! `uc.toml` and the `UC_<GROUP>_<KEY>` variables. Here the settings become
//! the server's types; the flags (`-p`, `--obs-port`) win over them.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use uc_common::settings::{Settings, invalid};

use crate::authn::sign_in::SignIn;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// `server.data_dir`: the embedded database, unless `db.url` names
    /// another.
    pub data_dir: PathBuf,
    pub bind: SocketAddr,
    /// `server.obs_port`: `/livez`, `/readyz` and `/metrics` on their own
    /// port; none without it.
    pub obs_port: Option<u16>,
    /// Where the catalog lives: see [`database_config`].
    pub database: uc_metastore::DatabaseConfig,
    /// `log.dir`: the rolling log file; none without it.
    pub log_dir: Option<PathBuf>,
    /// What the API listener serves, and its limits.
    pub http: crate::server::app::HttpSettings,
    /// `storage.tables_root` and `storage.models_root`.
    pub storage: uc_metastore::ManagedStorageConfig,
    /// `tables.managed` and `tables.allow_missing_deletion_vectors`.
    pub tables: uc_metastore::TableSettings,
    /// `tables.delta_api_only`.
    pub delta_api_only: bool,
    /// `tables.iceberg_writes`.
    pub iceberg_writes_enabled: bool,
    /// `storage.external_local_roots`.
    pub external_local_roots: Vec<PathBuf>,
    /// `server.readiness_interval` and `server.readiness_db_timeout`.
    pub readiness_interval: Duration,
    pub readiness_timeout: Duration,
    /// `server.workers`: the threads that serve requests
    /// (`crate::server::serve`); one per CPU without the setting.
    pub workers: usize,
    /// `server.io_threads`: the worker threads of the IO runtime
    /// (`uc_storage::runtime`).
    pub io_threads: usize,
    /// `storage.cleanup_*`.
    pub cleanup: crate::storage::cleanup::CleanupConfig,
    /// `[identity]` and `[locations]`: the server's cloud identities, and
    /// the identities of paths without an external location.
    pub credentials: crate::storage::vendor::StorageConfig,
    /// `identity.aws.arn`.
    pub aws_identity_arn: Option<String>,
    /// `[auth]`: sign-in, privilege checks and the web UI's sign-in
    /// ([`Self::sign_in`]).
    pub auth: uc_common::settings::Auth,
    /// `[secrets]`: the store of the secrets of storage credentials.
    pub secrets: crate::storage::secrets::SecretsConfig,
    /// The `uc.toml` that was read.
    pub file: Option<PathBuf>,
}

impl ServerConfig {
    /// The configuration of the environment and of `uc.toml`: the file
    /// `config` (`--config`), else `UC_CONFIG`'s, else the first `uc.toml`
    /// in the working directory or a folder above it.
    pub fn load(config: Option<&Path>) -> Result<Self, String> {
        Self::from_settings(Settings::load(config)?)
    }

    pub fn from_settings(settings: Settings) -> Result<Self, String> {
        let Settings {
            server,
            db,
            auth,
            storage,
            tables,
            identity,
            ui,
            log,
            file,
            ..
        } = &settings;
        let data_dir = server.data_dir.clone();
        if server.obs_port == Some(0) {
            return Err(invalid("server.obs_port", 0, "Expected 1 to 65535."));
        }
        if server.io_threads == 0 {
            return Err(invalid("server.io_threads", 0, "Expected 1 or more."));
        }
        if server.workers == Some(0) {
            return Err(invalid("server.workers", 0, "Expected 1 or more."));
        }
        let workers = server.workers.unwrap_or_else(|| {
            std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
        });
        let database = database_config(db, &data_dir)?;
        let http = crate::server::app::HttpSettings {
            ui_dir: match ui.enabled {
                true => uc_common::install::ui_dir(ui.dir.clone()),
                false => None,
            },
            ..crate::server::app::HttpSettings::new(server)
        };

        let root = |key: &str, value: &Option<String>| match value {
            None => Ok(None),
            Some(value) => uc_common::NormalizedUrl::parse(value)
                .map(Some)
                .map_err(|e| invalid(key, value, e.message())),
        };
        let managed = uc_metastore::ManagedStorageConfig {
            tables_root: root("storage.tables_root", &storage.tables_root)?,
            models_root: root("storage.models_root", &storage.models_root)?,
        };
        let roots = &storage.external_local_roots;
        let external_local_roots = crate::iceberg::location::parse_local_roots(roots)
            .map_err(|e| invalid("storage.external_local_roots", roots.join(","), e.message()))?;
        for (key, value) in [
            ("server.readiness_interval", server.readiness_interval),
            ("server.readiness_db_timeout", server.readiness_db_timeout),
        ] {
            if value.is_zero() {
                return Err(invalid(key, "0s", "Expected a positive duration."));
            }
        }
        let cleanup = crate::storage::cleanup::CleanupConfig::from_settings(storage)?;
        let secrets = crate::storage::secrets::SecretsConfig::from_settings(&settings.secrets)?;
        let credentials = crate::storage::vendor::StorageConfig {
            sign_in_cache: secrets.cache,
            ..crate::storage::vendor::StorageConfig::from_settings(&settings)?
        };

        Ok(Self {
            data_dir,
            bind: server.bind,
            obs_port: server.obs_port,
            database,
            log_dir: log.dir.clone(),
            http,
            storage: managed,
            tables: uc_metastore::TableSettings {
                managed_tables_enabled: tables.managed,
                allow_missing_deletion_vectors: tables.allow_missing_deletion_vectors,
            },
            delta_api_only: tables.delta_api_only,
            iceberg_writes_enabled: tables.iceberg_writes,
            external_local_roots,
            readiness_interval: server.readiness_interval,
            readiness_timeout: server.readiness_db_timeout,
            workers,
            io_threads: server.io_threads,
            cleanup,
            credentials,
            aws_identity_arn: identity.aws.arn.clone(),
            auth: auth.clone(),
            secrets,
            file: file.clone(),
        })
    }

    /// The web UI's sign-in: `auth.mode` and its settings (`crate::authn::sign_in`).
    pub fn sign_in(&self) -> Result<Option<SignIn>, String> {
        SignIn::new(&self.auth)
    }

    /// Token exchange and sign-in settings derived from this config.
    pub fn auth_settings(&self) -> crate::authn::idp::AuthSettings {
        crate::authn::idp::AuthSettings::new(&self.auth)
    }

    /// Metastore settings derived from this config, with the secret store
    /// that the server opened at start.
    pub fn metastore_config(
        &self,
        secrets: Arc<dyn uc_metastore::SecretStore>,
    ) -> uc_metastore::MetastoreConfig {
        uc_metastore::MetastoreConfig {
            database: self.database.clone(),
            storage: self.storage.clone(),
            tables: self.tables.clone(),
            secrets,
        }
    }
}

/// Where the catalog lives: `db.url` (`memory`, `file:<path>`,
/// `postgresql://…` or `jdbc:postgresql://…`), else the Turso file `uc.db`
/// in the data folder. A PostgreSQL user, password and pool size come from
/// `db.user`, `db.password` and `db.pool_size`, else from the URL.
pub fn database_config(
    db: &uc_common::settings::Db,
    data_dir: &Path,
) -> Result<uc_metastore::DatabaseConfig, String> {
    let url = db
        .url
        .clone()
        .unwrap_or_else(|| format!("file:{}", data_dir.join("uc.db").display()));
    let mut database =
        uc_metastore::DatabaseConfig::parse(&url).map_err(|e| invalid("db.url", &url, e))?;
    if let uc_metastore::DatabaseConfig::Postgres(pg) = &mut database {
        pg.user = db.user.clone();
        pg.password = db.password.clone();
        pg.pool_size = match db.pool_size {
            Some(0) => return Err(invalid("db.pool_size", 0, "Expected at least 1.")),
            size => size,
        };
    }
    Ok(database)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str) -> Result<ServerConfig, String> {
        ServerConfig::from_settings(Settings::from_toml(text)?)
    }

    #[test]
    fn reads_the_settings() {
        let cfg = load(
            r#"
            [server]
            data_dir = "/var/lib/uc"
            obs_port = 8081

            [db]
            url = "memory"

            [auth]
            enabled = true
            allowed_issuers = ["https://idp", "https://other"]
            audiences = "uc"
            access_token_timeout = "1h"
            cookie_timeout = "1d"

            [tables]
            managed = false
            iceberg_writes = false
            delta_api_only = true

            [storage]
            tables_root = "/tmp/uc-tables"
            "#,
        )
        .unwrap();
        assert_eq!(cfg.data_dir, PathBuf::from("/var/lib/uc"));
        assert_eq!(cfg.obs_port, Some(8081));
        assert_eq!(cfg.database, uc_metastore::DatabaseConfig::Memory);
        assert!(cfg.auth.enabled);
        assert_eq!(cfg.auth.allowed_issuers, ["https://idp", "https://other"]);
        assert_eq!(cfg.auth.audiences, ["uc"]);
        assert_eq!(cfg.auth.access_token_timeout, Duration::from_secs(3600));
        assert_eq!(cfg.auth.cookie_timeout, Duration::from_secs(86400));
        assert!(!cfg.tables.managed_tables_enabled);
        assert!(!cfg.iceberg_writes_enabled);
        assert!(cfg.delta_api_only);
        assert_eq!(
            cfg.storage.tables_root.unwrap().as_str(),
            "file:///tmp/uc-tables"
        );
    }

    #[test]
    fn defaults_keep_the_state_in_the_data_folder() {
        let cfg = load("").unwrap();
        let dir = PathBuf::from(".uc");
        assert_eq!(cfg.data_dir, dir);
        assert_eq!(
            cfg.database,
            uc_metastore::DatabaseConfig::File(dir.join("uc.db"))
        );
        assert_eq!(cfg.auth.signing_key_file, None);
        assert_eq!(cfg.log_dir, None);
        assert_eq!(cfg.bind, "0.0.0.0:8080".parse::<SocketAddr>().unwrap());
        assert_eq!(cfg.obs_port, None);
        assert!(!cfg.auth.enabled);
        assert!(cfg.tables.managed_tables_enabled && cfg.iceberg_writes_enabled);
        assert_eq!(
            cfg.auth.access_token_timeout,
            Duration::from_secs(24 * 3600)
        );
        assert_eq!(cfg.http.max_request_length, 10 * 1024 * 1024);
        assert_eq!(cfg.http.request_timeout, Some(Duration::from_secs(10)));
    }

    #[test]
    fn refuses_bad_values() {
        let error = |text: &str| load(text).unwrap_err();
        assert!(error("[auth]\nenabled = \"maybe\"").starts_with("auth.enabled: 'maybe'"));
        assert!(
            error("[auth]\ncookie_timeout = \"P5D\"")
                .starts_with("auth.cookie_timeout: 'P5D' is not a duration")
        );
        assert_eq!(
            error("[server]\nreadiness_db_timeout = \"0s\""),
            "Invalid value for server.readiness_db_timeout (UC_SERVER_READINESS_DB_TIMEOUT): \
             '0s'. Expected a positive duration."
        );
        assert!(error("[server]\nbind = \"localhost\"").starts_with("server.bind: "));
        assert!(
            error("[db]\nurl = \"postgresql://db/uc\"\npool_size = 0")
                .starts_with("Invalid value for db.pool_size (UC_DB_POOL_SIZE): '0'.")
        );
        assert!(
            error("[storage]\ntables_root = \"nope://x\"")
                .starts_with("Invalid value for storage.tables_root (UC_STORAGE_TABLES_ROOT)")
        );
    }
}
