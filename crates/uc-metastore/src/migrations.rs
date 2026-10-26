//! Versioned schema migrations, run by refinery.
//!
//! The migrations are the files `migrations/V{n}__{name}.sql`: SQL that
//! both engines run. Integers are `BIGINT`, booleans `BOOLEAN` (integers on
//! Turso) and JSON documents `TEXT`. For PostgreSQL, sqlparser reads the
//! SQL and gives text columns `COLLATE "C"`, so both engines sort by byte,
//! and makes Turso's full-text indexes (`USING fts`) GIN indexes (see
//! [`Transaction::apply_ddl`]).
//!
//! refinery creates its history table (`refinery_schema_history`), reads
//! the versions applied so far and applies the missing ones, in order.
//! [`run_migrations`] runs all of it in one transaction, not refinery's
//! transaction per migration: a failed migration leaves nothing behind. The
//! transaction holds the migration lock, so servers that share a database
//! migrate one at a time.
//!
//! Migrations are append-only and immutable once shipped: refinery refuses
//! a database where an applied migration has changed or is missing. To
//! change a shipped migration, add one with the corrective DDL.

use refinery::{Migration, Report, SchemaVersion};
use refinery_core::traits::r#async::{AsyncMigrate, AsyncQuery, AsyncTransaction};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::info;

use uc_common::{Error, Result};

use uc_storage::{Row, Storage, StorageError, StorageResult, Transaction};

mod embedded {
    refinery::embed_migrations!("migrations");
}

/// refinery's runner of the migrations in `migrations/`.
pub use embedded::migrations::runner;

fn failed(what: &str) -> impl Fn(StorageError) -> Error + '_ {
    move |e| Error::internal(format!("migrations: {what}: {e}"))
}

/// Apply the missing migrations; the report lists them.
pub async fn run_migrations(storage: &dyn Storage) -> Result<Report> {
    let tx = storage.begin().await.map_err(failed("begin"))?;
    tx.lock_migrations().await.map_err(failed("lock"))?;
    let mut migrator = Migrator(tx);
    let report = runner()
        .run_async(&mut migrator)
        .await
        .map_err(|e| Error::internal(format!("migrations: {e}")))?;
    migrator.0.commit().await.map_err(failed("commit"))?;
    for migration in report.applied_migrations() {
        info!(%migration, "applied migration");
    }
    Ok(report)
}

/// refinery's statements, in the transaction that [`run_migrations`]
/// commits. Dropped, it rolls back.
struct Migrator(Box<dyn Transaction>);

impl AsyncTransaction for Migrator {
    type Error = StorageError;

    async fn execute<'a, T: Iterator<Item = &'a str> + Send + 'a>(
        &mut self,
        queries: T,
    ) -> StorageResult<usize> {
        let mut count = 0;
        for sql in queries {
            self.0.apply_ddl(sql).await?;
            count += 1;
        }
        Ok(count)
    }
}

impl AsyncQuery<Vec<Migration>> for Migrator {
    async fn query(&mut self, query: &str) -> StorageResult<Vec<Migration>> {
        let rows = self.0.query(query, &[]).await?;
        rows.iter().map(applied).collect()
    }
}

impl AsyncMigrate for Migrator {}

/// A row of refinery's history: version, name, when (RFC 3339), checksum.
fn applied(row: &Row) -> StorageResult<Migration> {
    let invalid = |what: &str| StorageError::Other(format!("migration history: invalid {what}"));
    let version = SchemaVersion::try_from(row.int(0)?).map_err(|_| invalid("version"))?;
    let name = row.text(1)?;
    let applied_on =
        OffsetDateTime::parse(&row.text(2)?, &Rfc3339).map_err(|_| invalid("applied_on"))?;
    let checksum = row.text(3)?.parse().map_err(|_| invalid("checksum"))?;
    Ok(Migration::applied(version, name, applied_on, checksum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_reads_every_migration() {
        for migration in runner().get_migrations() {
            if let Err(e) = uc_storage::postgres_ddl(migration.sql().unwrap_or_default()) {
                panic!("{migration}: {e}");
            }
        }
    }
}
