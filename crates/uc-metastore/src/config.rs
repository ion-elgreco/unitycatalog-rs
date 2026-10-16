//! The metastore's settings.

use std::path::PathBuf;

use uc_storage::{DatabaseConfig, PostgresConfig};

/// The metastore's settings: where the catalog lives, and the server
/// settings it applies.
#[derive(Debug, Clone)]
pub struct MetastoreConfig {
    pub database: DatabaseConfig,
    /// Fallback storage roots for managed entities.
    pub storage: ManagedStorageConfig,
    /// Managed-table server settings.
    pub tables: TableSettings,
}

impl MetastoreConfig {
    pub fn new(database: DatabaseConfig) -> Self {
        Self {
            database,
            storage: ManagedStorageConfig::default(),
            tables: TableSettings::default(),
        }
    }

    /// Turso in memory.
    pub fn in_memory() -> Self {
        Self::new(DatabaseConfig::Memory)
    }

    /// Turso in the file `path`.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::new(DatabaseConfig::File(path.into()))
    }

    /// For tests: Turso in memory; or, when `UC_TEST_POSTGRES_URL` names a
    /// PostgreSQL database, a new schema in it, so that the whole test
    /// suite runs on either engine.
    pub fn for_tests() -> Self {
        match std::env::var("UC_TEST_POSTGRES_URL") {
            Ok(url) if !url.trim().is_empty() => {
                let mut pg = PostgresConfig::new(url.trim());
                pg.schema = Some(format!("uc_test_{}", uuid::Uuid::new_v4().simple()));
                pg.pool_size = Some(4);
                Self::new(DatabaseConfig::Postgres(pg))
            }
            _ => Self::in_memory(),
        }
    }

    pub fn with_storage(mut self, storage: ManagedStorageConfig) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_tables(mut self, tables: TableSettings) -> Self {
        self.tables = tables;
        self
    }
}

/// The `[tables]` settings of managed tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSettings {
    /// `tables.managed` (default true): without it, MANAGED
    /// tables, staging tables and the commit coordinator are refused.
    pub managed_tables_enabled: bool,
    /// `tables.allow_missing_deletion_vectors` (default
    /// false): IcebergCompatV2 tables may skip deletion vectors.
    pub allow_missing_deletion_vectors: bool,
}

impl Default for TableSettings {
    fn default() -> Self {
        Self {
            managed_tables_enabled: true,
            allow_missing_deletion_vectors: false,
        }
    }
}

/// Fallback roots for managed data when neither the schema nor the catalog
/// has a storage location. Upstream server properties `storage-root.tables`
/// and `storage-root.models` (deprecated upstream, still honoured).
#[derive(Debug, Clone, Default)]
pub struct ManagedStorageConfig {
    pub tables_root: Option<uc_common::NormalizedUrl>,
    pub models_root: Option<uc_common::NormalizedUrl>,
}
