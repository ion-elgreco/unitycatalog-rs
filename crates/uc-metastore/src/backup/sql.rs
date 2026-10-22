//! Backups: the catalog's tables as JSON objects, column by column.

use async_trait::async_trait;
use serde_json::Value as JsonValue;

use uc_common::{Error, Result};

use crate::backup::{BACKUP_VERSION, BackupOps, RestoreReport, Snapshot};
use crate::sql::rows::{
    CATALOG, CREDENTIAL, DELTA_COMMIT, EXTERNAL_LOCATION, FUNC, METASTORE, MODEL_VERSION, PAT,
    PERMISSION, RECIPIENT, REGISTERED_MODEL, SCHEMA, SCIM_GROUP, SCIM_GROUP_MEMBER, SCIM_USER,
    SHARE, STAGING_TABLE, STORAGE_CLEANUP_TASK, TBL, TableDef, VOLUME,
};
use crate::sql::{SqlMetastore, db_err, placeholders};
use uc_storage::{StorageError, Value};

/// The tables of a backup. The audit log is left out: it is operational
/// telemetry, not catalog state, and a restored deployment starts a fresh
/// one.
const BACKUP_TABLES: &[&TableDef] = &[
    &CATALOG,
    &SCHEMA,
    &TBL,
    &STAGING_TABLE,
    &DELTA_COMMIT,
    &VOLUME,
    &FUNC,
    &REGISTERED_MODEL,
    &MODEL_VERSION,
    &CREDENTIAL,
    &EXTERNAL_LOCATION,
    &PERMISSION,
    &METASTORE,
    &PAT,
    &SCIM_USER,
    &SCIM_GROUP,
    &SCIM_GROUP_MEMBER,
    &SHARE,
    &RECIPIENT,
    &STORAGE_CLEANUP_TASK,
];

/// Most parameters in one statement: SQLite's oldest limit.
const MAX_PARAMS: usize = 999;

fn restore_err(e: StorageError) -> Error {
    match e {
        StorageError::Unique(m) => Error::invalid_argument(format!("restore: duplicate key: {m}")),
        e => Error::internal(format!("restore: {e}")),
    }
}

#[async_trait]
impl BackupOps for SqlMetastore {
    async fn export_snapshot(&self) -> Result<Snapshot> {
        let failed = db_err("backup");
        let tx = self.db.begin_snapshot().await.map_err(&failed)?;
        let mut snap = Snapshot::empty();
        for t in BACKUP_TABLES {
            let sql = format!("{} ORDER BY {}", t.select(), t.key.join(", "));
            let rows = tx.query(&sql, &[]).await.map_err(&failed)?;
            let objects = rows
                .into_iter()
                .map(|r| t.object(r).map(JsonValue::Object))
                .collect::<Result<Vec<_>>>()?;
            snap.tables.insert(t.name.to_string(), objects);
        }
        tx.commit().await.map_err(&failed)?;
        Ok(snap)
    }

    async fn import_snapshot(&self, snap: Snapshot) -> Result<RestoreReport> {
        if snap.version != BACKUP_VERSION {
            return Err(Error::invalid_argument(format!(
                "unsupported backup version {} (expected {BACKUP_VERSION})",
                snap.version
            )));
        }
        let mut report = RestoreReport::default();
        // Every table is replaced, in one transaction.
        let tx = self.db.begin().await.map_err(restore_err)?;
        for t in BACKUP_TABLES {
            tx.execute(&format!("DELETE FROM {}", t.name), &[])
                .await
                .map_err(restore_err)?;
            let mut rows: Vec<Vec<Value>> = Vec::new();
            for row in snap.tables.get(t.name).into_iter().flatten() {
                let keyed = row.as_object().filter(|object| {
                    t.key
                        .iter()
                        .all(|k| object.get(*k).is_some_and(|v| !v.is_null()))
                });
                let Some(object) = keyed else {
                    report
                        .skipped
                        .push(format!("{}: row missing reconstructable key", t.name));
                    continue;
                };
                let values = t
                    .values_of(object)
                    .map_err(|e| Error::invalid_argument(format!("restore: {}", e.message())))?;
                rows.push(values);
            }
            // Many rows per statement: one round trip for a chunk.
            let per_statement = (MAX_PARAMS / t.columns.len()).max(1);
            for chunk in rows.chunks(per_statement) {
                let width = t.columns.len();
                let tuples: Vec<String> = (0..chunk.len())
                    .map(|i| format!("({})", placeholders(i * width + 1, width)))
                    .collect();
                let sql = format!(
                    "INSERT INTO {} ({}) VALUES {}",
                    t.name,
                    t.column_list(),
                    tuples.join(", ")
                );
                let params: Vec<Value> = chunk.iter().flatten().cloned().collect();
                tx.execute(&sql, &params).await.map_err(restore_err)?;
            }
            report.imported.insert(t.name.to_string(), rows.len());
        }
        tx.commit().await.map_err(restore_err)?;
        Ok(report)
    }
}
