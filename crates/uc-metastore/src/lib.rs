//! The Unity Catalog metastore: all the metadata one server keeps.
//!
//! The [`Metastore`] trait is the only contract the server depends on. It
//! gives one `Ops` trait per domain: [`CatalogOps`], [`TableOps`],
//! [`AuditLogOps`], ... Each domain is a folder: the `Ops` trait in
//! `mod.rs`, its SQL in `sql.rs`. [`SqlMetastore`] implements them all once,
//! in SQL, on any [`Storage`]: Turso, embedded (in memory or in a file), or
//! PostgreSQL, which several server replicas can share.

pub mod audit_log;
pub mod backup;
pub mod catalog;
pub mod cleanup;
mod config;
pub mod credential;
pub mod delta_table;
pub mod external_location;
pub mod function;
pub mod iceberg_table;
pub mod migrations;
pub mod model;
pub mod permissions;
pub mod schema;
pub mod scim;
pub mod secrets;
pub mod sharing;
mod sql;
pub mod table;
pub mod token;
pub mod volume;

pub use audit_log::{AuditLogOps, AuditQuery};
pub use backup::{BackupOps, RestoreReport, Snapshot};
pub use catalog::CatalogOps;
pub use cleanup::{CleanupClaim, CleanupOps, CleanupTask};
pub use config::{ManagedStorageConfig, MetastoreConfig, TableSettings};
pub use credential::{CredentialOps, RefreshReport};
pub use delta_table::DeltaTableOps;
pub use external_location::ExternalLocationOps;
pub use function::FunctionOps;
pub use iceberg_table::{IcebergTableOps, IcebergTableRef, IcebergTableState};
pub use model::ModelOps;
pub use permissions::{Lineage, Node, PathOwner, PermissionOps};
pub use schema::{PropertyChanges, SchemaOps};
pub use scim::{ScimOps, SeedReport, SeedUser};
pub use secrets::{CachedStore, DatabaseStore, OpenError, SecretRef, SecretStore, SecretValue};
pub use sharing::SharingOps;
pub use sql::SqlMetastore;
pub use table::TableOps;
pub use token::TokenOps;
pub use uc_storage::{DatabaseConfig, PostgresConfig, Storage};
pub use volume::VolumeOps;

use std::sync::Arc;

/// The metastore: one `Ops` trait per domain, so that a single `Metastore`
/// value gives handlers access to everything.
pub trait Metastore: Send + Sync + 'static {
    fn catalogs(&self) -> &dyn CatalogOps;
    fn schemas(&self) -> &dyn SchemaOps;
    fn tables(&self) -> &dyn TableOps;
    fn delta_tables(&self) -> &dyn DeltaTableOps;
    fn iceberg_tables(&self) -> &dyn IcebergTableOps;
    fn volumes(&self) -> &dyn VolumeOps;
    fn functions(&self) -> &dyn FunctionOps;
    fn models(&self) -> &dyn ModelOps;
    fn credentials(&self) -> &dyn CredentialOps;
    fn external_locations(&self) -> &dyn ExternalLocationOps;
    fn permissions(&self) -> &dyn PermissionOps;
    fn tokens(&self) -> &dyn TokenOps;
    fn scim(&self) -> &dyn ScimOps;
    fn audit_log(&self) -> &dyn AuditLogOps;
    fn backups(&self) -> &dyn BackupOps;
    fn sharing(&self) -> &dyn SharingOps;
    fn cleanup(&self) -> &dyn CleanupOps;
}

/// The metastore as the shared trait object that handlers hold.
pub type DynMetastore = Arc<dyn Metastore>;
