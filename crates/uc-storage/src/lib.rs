//! The storage the catalog runs on, behind the engine-agnostic [`Storage`]
//! trait: [`TursoStorage`], embedded in the server (in memory or in a file),
//! and [`PostgresStorage`], a server that several replicas can share.
//!
//! The metastore writes its SQL once, in PostgreSQL's dialect with `$1`
//! placeholders, kept to what Turso also runs. Where the engines differ
//! (the clock, row locks, DDL, the migration lock), [`Exec`] and
//! [`Transaction`] ask the engine. A [`Value`] binds and a [`Row`] reads the
//! few types the catalog stores: integers, booleans and text (JSON
//! documents are text too).
//!
//! Reads outside a transaction see committed data only. A transaction that
//! is dropped without [`Transaction::commit`] rolls back.

mod postgres;
mod tls;
mod turso;

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

pub use postgres::{PostgresConfig, PostgresStorage, ddl as postgres_ddl};
pub use turso::TursoStorage;

/// A parameter or column value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Int(i64),
    Bool(bool),
    Text(String),
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl From<&String> for Value {
    fn from(v: &String) -> Self {
        Value::Text(v.clone())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        v.map_or(Value::Null, Into::into)
    }
}

/// One row of a result: its values in column order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row(pub(crate) Vec<Value>);

impl From<Vec<Value>> for Row {
    fn from(values: Vec<Value>) -> Self {
        Row(values)
    }
}

impl Row {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_values(self) -> Vec<Value> {
        self.0
    }

    fn get(&self, i: usize) -> StorageResult<&Value> {
        self.0.get(i).ok_or_else(|| {
            StorageError::Other(format!("no column {i} in a row of {}", self.0.len()))
        })
    }

    pub fn opt_text(&self, i: usize) -> StorageResult<Option<String>> {
        match self.get(i)? {
            Value::Null => Ok(None),
            Value::Text(s) => Ok(Some(s.clone())),
            v => Err(StorageError::Other(format!(
                "column {i} is not text: {v:?}"
            ))),
        }
    }

    pub fn text(&self, i: usize) -> StorageResult<String> {
        self.opt_text(i)?
            .ok_or_else(|| StorageError::Other(format!("column {i} is null")))
    }

    pub fn opt_int(&self, i: usize) -> StorageResult<Option<i64>> {
        match self.get(i)? {
            Value::Null => Ok(None),
            Value::Int(n) => Ok(Some(*n)),
            v => Err(StorageError::Other(format!(
                "column {i} is not an integer: {v:?}"
            ))),
        }
    }

    pub fn int(&self, i: usize) -> StorageResult<i64> {
        self.opt_int(i)?
            .ok_or_else(|| StorageError::Other(format!("column {i} is null")))
    }
}

/// A storage error, by what the caller can do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// A unique key (or primary key) already holds the value.
    Unique(String),
    /// Another writer holds the lock, or the transaction lost a race:
    /// trying again can work.
    Busy(String),
    Other(String),
}

impl StorageError {
    pub fn message(&self) -> &str {
        match self {
            StorageError::Unique(m) | StorageError::Busy(m) | StorageError::Other(m) => m,
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for StorageError {}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Statements, on a [`Storage`] (each in a transaction of its own) or in a
/// [`Transaction`].
#[async_trait]
pub trait Exec: Send + Sync {
    /// The rows of a statement that reads. Outside a transaction it must
    /// not write: Turso runs it on a reader connection.
    async fn query(&self, sql: &str, params: &[Value]) -> StorageResult<Vec<Row>>;

    /// The number of rows a statement changed.
    async fn execute(&self, sql: &str, params: &[Value]) -> StorageResult<u64>;

    /// The database clock in epoch milliseconds, as an SQL expression.
    fn now_millis(&self) -> &'static str;

    /// Appended to a `SELECT` in a transaction: lock the rows it reads
    /// until the transaction ends.
    fn for_update(&self) -> &'static str;
}

/// A database the catalog runs on.
#[async_trait]
pub trait Storage: Exec {
    /// Start a transaction.
    async fn begin(&self) -> StorageResult<Box<dyn Transaction>>;

    /// Start a transaction whose reads all see one snapshot of the
    /// database, for backups.
    async fn begin_snapshot(&self) -> StorageResult<Box<dyn Transaction>> {
        self.begin().await
    }
}

/// A transaction. Dropping it without [`Transaction::commit`] rolls it back.
#[async_trait]
pub trait Transaction: Exec {
    /// Apply DDL written for both engines: several statements without
    /// parameters.
    async fn apply_ddl(&self, sql: &str) -> StorageResult<()>;

    /// Hold the migration lock until the transaction ends, so that servers
    /// sharing the database migrate one at a time.
    async fn lock_migrations(&self) -> StorageResult<()>;

    async fn commit(self: Box<Self>) -> StorageResult<()>;

    async fn rollback(self: Box<Self>) -> StorageResult<()>;
}

/// Where the catalog lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseConfig {
    /// Turso in memory: the catalog is gone when the server stops.
    Memory,
    /// Turso in a file, which one server opens at a time.
    File(PathBuf),
    /// A PostgreSQL server, which several replicas can share.
    Postgres(PostgresConfig),
}

impl DatabaseConfig {
    /// A database URL: `memory`, `file:<path>`, a PostgreSQL URL
    /// (`postgres://…`, `postgresql://…`), or one of upstream's JDBC URLs:
    /// `jdbc:postgresql://…`, `jdbc:h2:mem:…` (memory) and
    /// `jdbc:h2:file:<path>` (the Turso file `<path>.db`; H2's own files
    /// are not read).
    pub fn parse(url: &str) -> Result<Self, String> {
        let url = url.trim();
        if matches!(url, "memory" | "mem" | "mem://" | ":memory:") {
            return Ok(DatabaseConfig::Memory);
        }
        if let Some(path) = url.strip_prefix("file:") {
            let path = path.strip_prefix("//").unwrap_or(path);
            if path.is_empty() {
                return Err(format!("Invalid database URL '{url}': no file path"));
            }
            return Ok(DatabaseConfig::File(path.into()));
        }
        if let Some(h2) = url.strip_prefix("jdbc:h2:") {
            // H2 settings follow the name after `;`.
            let name = h2.split(';').next().unwrap_or_default();
            if name.starts_with("mem:") {
                return Ok(DatabaseConfig::Memory);
            }
            let path = name.strip_prefix("file:").unwrap_or(name);
            if path.is_empty() || path.contains(':') {
                return Err(format!(
                    "Invalid database URL '{url}': only H2's mem: and file: URLs map to Turso"
                ));
            }
            return Ok(DatabaseConfig::File(format!("{path}.db").into()));
        }
        if postgres::is_url(url) {
            return Ok(DatabaseConfig::Postgres(PostgresConfig::new(url)));
        }
        Err(format!(
            "Invalid database URL '{url}': expected memory, file:<path>, postgresql://… or \
             jdbc:postgresql://…"
        ))
    }

    /// What the server logs at start: no credentials.
    pub fn describe(&self) -> String {
        match self {
            DatabaseConfig::Memory => "Turso in memory".to_string(),
            DatabaseConfig::File(path) => format!("Turso file {}", path.display()),
            DatabaseConfig::Postgres(pg) => format!("PostgreSQL {}", pg.describe()),
        }
    }
}

/// Open the storage `config` names.
pub async fn open(config: &DatabaseConfig) -> StorageResult<Arc<dyn Storage>> {
    Ok(match config {
        DatabaseConfig::Memory => Arc::new(TursoStorage::open(None).await?),
        DatabaseConfig::File(path) => Arc::new(TursoStorage::open(Some(path)).await?),
        DatabaseConfig::Postgres(pg) => Arc::new(PostgresStorage::open(pg).await?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_urls() {
        let parse = |u: &str| DatabaseConfig::parse(u).unwrap();
        assert_eq!(parse("memory"), DatabaseConfig::Memory);
        assert_eq!(
            parse("jdbc:h2:mem:testdb;DB_CLOSE_DELAY=-1"),
            DatabaseConfig::Memory
        );
        assert_eq!(
            parse("file:/data/uc.db"),
            DatabaseConfig::File("/data/uc.db".into())
        );
        assert_eq!(
            parse("jdbc:h2:file:./etc/db/h2db;DB_CLOSE_DELAY=-1"),
            DatabaseConfig::File("./etc/db/h2db.db".into())
        );
        assert!(matches!(
            parse("postgresql://u@db:5432/uc"),
            DatabaseConfig::Postgres(_)
        ));
        assert!(matches!(
            parse("jdbc:postgresql://db/uc"),
            DatabaseConfig::Postgres(_)
        ));
        assert!(DatabaseConfig::parse("mysql://db/uc").is_err());
        assert!(DatabaseConfig::parse("jdbc:h2:tcp://h/db").is_err());
    }
}
