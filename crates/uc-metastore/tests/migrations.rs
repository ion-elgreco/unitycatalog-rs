//! Migrations: applied once, in one transaction, one server at a time. On
//! Turso, or on PostgreSQL when `UC_TEST_POSTGRES_URL` is set.

use std::sync::Arc;

use uc_metastore::migrations::{run_migrations, runner};
use uc_metastore::{DatabaseConfig, MetastoreConfig};
use uc_storage::{self as storage, Storage};

async fn new_storage() -> Arc<dyn Storage> {
    storage::open(&MetastoreConfig::for_tests().database)
        .await
        .unwrap()
}

fn migration_count() -> usize {
    runner().get_migrations().len()
}

#[tokio::test]
async fn migrations_apply_once_and_are_idempotent() {
    let db = new_storage().await;
    let applied = run_migrations(db.as_ref()).await.unwrap();
    assert_eq!(
        applied.applied_migrations().len(),
        migration_count(),
        "first run applies everything"
    );
    let applied_again = run_migrations(db.as_ref()).await.unwrap();
    assert!(
        applied_again.applied_migrations().is_empty(),
        "second run is a no-op"
    );

    let rows = db
        .query(
            "SELECT version, name FROM refinery_schema_history ORDER BY version",
            &[],
        )
        .await
        .unwrap();
    let mut migrations = runner().get_migrations().clone();
    migrations.sort();
    assert_eq!(rows.len(), migrations.len());
    for (row, mig) in rows.iter().zip(&migrations) {
        assert_eq!(row.int(0).unwrap(), i64::from(mig.version()));
        assert_eq!(row.text(1).unwrap(), mig.name());
    }
}

/// Servers that share a database migrate one at a time: of two that start
/// together, one applies everything and the other nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn servers_starting_together_migrate_once() {
    let db = new_storage().await;
    let (a, b) = tokio::join!(run_migrations(db.as_ref()), run_migrations(db.as_ref()));
    let mut counts = [
        a.unwrap().applied_migrations().len(),
        b.unwrap().applied_migrations().len(),
    ];
    counts.sort();
    assert_eq!(counts, [0, migration_count()]);
}

/// A migration that fails leaves nothing behind: no tables, no version.
#[tokio::test]
async fn a_failed_migration_changes_nothing() {
    let db = new_storage().await;
    // A table in the way of the first migration's.
    db.execute("CREATE TABLE volume (id TEXT)", &[])
        .await
        .unwrap();
    let err = run_migrations(db.as_ref()).await.unwrap_err();
    assert!(
        err.message()
            .contains("error applying migration V1__initial_schema"),
        "{}",
        err.message()
    );
    assert!(
        db.query("SELECT 1 FROM catalog", &[]).await.is_err(),
        "the tables before the failure are gone"
    );
    assert!(
        db.query("SELECT 1 FROM refinery_schema_history", &[])
            .await
            .is_err(),
        "no version was recorded"
    );
}

/// Text columns sort by byte, as on Turso, whatever the locale of the
/// PostgreSQL database. Turso sorts text by byte anyway.
#[tokio::test]
async fn text_columns_sort_by_byte_on_postgres() {
    if !matches!(
        MetastoreConfig::for_tests().database,
        DatabaseConfig::Postgres(_)
    ) {
        return;
    }
    let db = new_storage().await;
    run_migrations(db.as_ref()).await.unwrap();
    let rows = db
        .query(
            "SELECT table_name || '.' || column_name FROM information_schema.columns \
             WHERE table_schema = current_schema() \
             AND data_type IN ('text', 'character varying', 'character') \
             AND collation_name IS DISTINCT FROM 'C'",
            &[],
        )
        .await
        .unwrap();
    assert!(rows.is_empty(), "not by byte: {rows:?}");
}

/// An applied migration whose file has changed since stops the start.
#[tokio::test]
async fn a_changed_migration_is_refused() {
    let db = new_storage().await;
    run_migrations(db.as_ref()).await.unwrap();
    db.execute(
        "UPDATE refinery_schema_history SET checksum = '1' WHERE version = 1",
        &[],
    )
    .await
    .unwrap();
    let err = run_migrations(db.as_ref()).await.unwrap_err();
    assert!(
        err.message().contains("is different than filesystem"),
        "{}",
        err.message()
    );
}
