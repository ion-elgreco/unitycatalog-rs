//! Storage cleanup tasks. Times come from the database clock, as upstream.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use uuid::Uuid;

use uc_common::{Error, ManagedResource, Result};

use crate::sql::rows::{STORAGE_CLEANUP_TASK, TableDef};
use crate::sql::{SqlMetastore, db_err, select, select_one};
use crate::{CleanupClaim, CleanupOps, CleanupTask};
use uc_storage::{Exec, Value};

/// Longest `last_error` kept, as upstream.
const MAX_ERROR_LEN: usize = 2048;

/// Queue a cleanup task for each row of `t` that `filter` picks (binding
/// `params`) and that has a storage location. Upstream
/// `StorageCleanupTaskRepository.create`: run it in the transaction that
/// drops the rows, before the drop. Model versions are named by number.
pub(crate) async fn queue_cleanup<E: Exec + ?Sized>(
    ex: &E,
    t: &TableDef,
    resource: ManagedResource,
    filter: &str,
    params: &[Value],
) -> Result<()> {
    let name = match resource {
        ManagedResource::ModelVersion => "CAST(version AS TEXT)",
        _ => "name",
    };
    let sql = format!(
        "INSERT INTO storage_cleanup_task \
           (id_string, resource_type, name, storage_location, deleted_at, failure_count) \
         SELECT id_string, '{kind}', {name}, storage_location, {now}, 0 FROM {table} \
         WHERE ({filter}) AND storage_location IS NOT NULL",
        kind = resource.as_str(),
        now = ex.now_millis(),
        table = t.name,
    );
    ex.execute(&sql, params)
        .await
        .map_err(db_err("queue storage cleanup"))?;
    Ok(())
}

/// A duration as millis, capped so `now + d` cannot overflow.
fn millis(d: Duration) -> i64 {
    const CAP: u128 = 1 << 50;
    d.as_millis().min(CAP) as i64
}

#[derive(Deserialize)]
struct ClaimRow {
    id_string: String,
    resource_type: String,
    storage_location: String,
}

#[async_trait]
impl CleanupOps for SqlMetastore {
    async fn claim(
        &self,
        lease: Duration,
        initial_delay: Duration,
    ) -> Result<Option<CleanupClaim>> {
        if lease < Duration::from_millis(1) {
            return Err(Error::invalid_argument(
                "Lease duration must be at least one millisecond",
            ));
        }
        let now = self.db.now_millis();
        let ready = format!(
            "deleted_at <= {now} - $1 AND (lease_expires_at IS NULL OR lease_expires_at <= {now})"
        );
        let delay = Value::Int(millis(initial_delay));
        let Some(task): Option<ClaimRow> = select_one(
            self.db.as_ref(),
            &STORAGE_CLEANUP_TASK,
            &format!("WHERE {ready} ORDER BY COALESCE(lease_expires_at, deleted_at) LIMIT 1"),
            std::slice::from_ref(&delay),
        )
        .await?
        else {
            return Ok(None);
        };
        // The claim holds only if no one claimed the task in between.
        let token = Uuid::new_v4().to_string();
        let claimed = self
            .db
            .execute(
                &format!(
                    "UPDATE storage_cleanup_task SET lease_token = $2, lease_expires_at = {now} + $3 \
                     WHERE id_string = $4 AND {ready}"
                ),
                &[
                    delay,
                    token.as_str().into(),
                    Value::Int(millis(lease)),
                    task.id_string.as_str().into(),
                ],
            )
            .await
            .map_err(db_err("cleanup claim"))?;
        Ok((claimed == 1).then_some(CleanupClaim {
            resource_type: task.resource_type,
            resource_id: task.id_string,
            storage_location: task.storage_location,
            lease_token: token,
        }))
    }

    async fn finish(&self, resource_id: &str, lease_token: &str) -> Result<bool> {
        let removed = self
            .db
            .execute(
                &format!(
                    "DELETE FROM storage_cleanup_task \
                     WHERE id_string = $1 AND lease_token = $2 AND lease_expires_at > {}",
                    self.db.now_millis()
                ),
                &[resource_id.into(), lease_token.into()],
            )
            .await
            .map_err(db_err("cleanup finish"))?;
        Ok(removed == 1)
    }

    async fn report_failure(
        &self,
        resource_id: &str,
        lease_token: &str,
        error: &str,
        retry_backoff: Duration,
    ) -> Result<bool> {
        let error: String = error.chars().take(MAX_ERROR_LEN).collect();
        let now = self.db.now_millis();
        let updated = self
            .db
            .execute(
                &format!(
                    "UPDATE storage_cleanup_task \
                     SET lease_token = NULL, lease_expires_at = {now} + $3, \
                         failure_count = failure_count + 1, last_error = $4 \
                     WHERE id_string = $1 AND lease_token = $2 AND lease_expires_at > {now}"
                ),
                &[
                    resource_id.into(),
                    lease_token.into(),
                    Value::Int(millis(retry_backoff)),
                    error.into(),
                ],
            )
            .await
            .map_err(db_err("cleanup report failure"))?;
        Ok(updated == 1)
    }

    async fn list_tasks(&self) -> Result<Vec<CleanupTask>> {
        select(
            self.db.as_ref(),
            &STORAGE_CLEANUP_TASK,
            "ORDER BY deleted_at, id_string",
            &[],
        )
        .await
    }
}
