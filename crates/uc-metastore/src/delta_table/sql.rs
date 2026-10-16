//! [`DeltaTableOps`] in SQL: the Delta API's tables and the commit
//! coordinator. The rules live in [`uc_delta`]; this module reads the
//! tracked commits and applies a plan in one transaction.

use async_trait::async_trait;
use tracing::debug;

use uc_api::delta_api::{DeltaCreateTableRequest, DeltaLoadTableResponse, DeltaUpdateTableRequest};
use uc_api::{DeltaCommit, DeltaGetCommits, DeltaGetCommitsResponse, TableType};
use uc_common::{Error, ErrorCode, Result, three_part_full_name};

use crate::DeltaTableOps;
use crate::sql::rows::{DELTA_COMMIT, TBL};
use crate::sql::{SqlMetastore, by_id, db_err, insert, select, touch_audit};
use crate::table::sql::TableRow;
use uc_delta::commits::{
    self as rules, COMMITS_PER_BATCH, CommitOutcome, CommitPlan, Planned, StoredCommit,
};
use uc_storage::Value;

/// Attempts before a commit that keeps racing other writers gives up.
const COMMIT_ATTEMPTS: usize = 3;

fn to_json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| Error::internal(e.to_string()))
}

/// The tracked commits with the lowest and highest version, and the one at
/// the request's version.
struct Bounds {
    first: Option<StoredCommit>,
    last: Option<StoredCommit>,
    at_version: Option<StoredCommit>,
}

impl SqlMetastore {
    async fn table_by_id(&self, id: &str) -> Result<Option<TableRow>> {
        by_id(self.db.as_ref(), &TBL, id).await
    }

    async fn commit_bounds(&self, table_id: &str, version: i64) -> Result<Bounds> {
        let commits: Vec<StoredCommit> = select(
            self.db.as_ref(),
            &DELTA_COMMIT,
            "WHERE table_id = $1 AND (\
                 commit_version = (SELECT MIN(commit_version) FROM delta_commit WHERE table_id = $1) \
                 OR commit_version = (SELECT MAX(commit_version) FROM delta_commit WHERE table_id = $1) \
                 OR commit_version = $2)",
            &[table_id.into(), version.into()],
        )
        .await?;
        Ok(Bounds {
            first: commits.iter().min_by_key(|c| c.commit_version).cloned(),
            last: commits.iter().max_by_key(|c| c.commit_version).cloned(),
            at_version: commits.into_iter().find(|c| c.commit_version == version),
        })
    }

    /// Upstream `postCommit`.
    async fn post_delta_commit(
        &self,
        commit: DeltaCommit,
        updated_by: Option<&str>,
    ) -> Result<CommitOutcome> {
        self.check_managed_enabled()?;
        rules::validate_commit(&commit)?;
        let uniform = uc_delta::uniform::from_commit(
            commit.uniform.as_ref(),
            commit.commit_info.as_ref().and_then(|i| i.version),
        )?;
        let raw_id = commit.table_id.clone().unwrap_or_default();
        // Upstream parses the id inside its transaction, which reports a bad
        // id as INTERNAL.
        let failed = |e: Error| format!("Error committing to table: {raw_id}: {}", e.message());
        let table_id = rules::parse_table_id(&raw_id).map_err(|e| Error::internal(failed(e)))?;
        let version = commit
            .commit_info
            .as_ref()
            .and_then(|i| i.version)
            .unwrap_or(-1);
        let mut attempt = 0;
        loop {
            attempt += 1;
            let table = self
                .table_by_id(&table_id)
                .await?
                .ok_or_else(|| Error::table_not_found(format!("Table not found: {raw_id}")))?;
            rules::validate_table(
                &table_id,
                &table.table_type,
                table.data_source_format.as_deref(),
                table.storage_location.as_deref(),
            )?;
            let location = table.storage_location.clone().unwrap_or_default();
            rules::validate_table_uri(commit.table_uri.as_deref().unwrap_or_default(), &location)?;
            // Upstream `validateTableForCommit`: UniForm metadata lives inside
            // the table, and comes exactly when the (new) properties enable it.
            if let Some(fields) = &uniform {
                uc_delta::uniform::require_subpath(
                    &fields.metadata_location,
                    &uc_common::NormalizedUrl::parse(&location)?,
                )?;
            }
            let effective_properties =
                match commit.metadata.as_ref().and_then(|m| m.properties.as_ref()) {
                    Some(p) => p
                        .properties
                        .iter()
                        .flatten()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    None => table.properties.clone().unwrap_or_default(),
                };
            uc_delta::contract::validate_uniform_consistency(
                &effective_properties,
                uniform.is_some(),
            )?;
            let bounds = self.commit_bounds(&table_id, version).await?;
            let planned = rules::plan_commit(
                &table_id,
                &commit,
                bounds.first.as_ref(),
                bounds.last.as_ref(),
                bounds.at_version.as_ref(),
            )?;
            let plan = match planned {
                Planned::Replay => return Ok(CommitOutcome::Done),
                Planned::ContentCheck {
                    version,
                    staged_file_name,
                } => {
                    return Ok(CommitOutcome::ContentCheck {
                        table_location: location,
                        version,
                        staged_file_name,
                    });
                }
                Planned::Apply(plan) => plan,
            };
            // The commit's metadata replaces the table's properties, columns
            // and comment.
            let uniform_applied = uniform.is_some() && plan.save.is_some();
            let new_row = match (plan.update_table || uniform_applied, &commit.metadata) {
                (true, metadata) => {
                    let mut row = table.clone();
                    if let (true, Some(fields)) = (uniform_applied, &uniform) {
                        row.set_uniform(fields);
                    }
                    let metadata = metadata.clone().unwrap_or_default();
                    let metadata = &metadata;
                    let (properties, columns, description) = rules::metadata_changes(metadata);
                    if properties.is_some() {
                        row.properties = properties;
                    }
                    if columns.is_some() {
                        row.columns = columns;
                    }
                    if description.is_some() {
                        row.comment = description;
                    }
                    touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);
                    Some(row)
                }
                _ => None,
            };
            let row = new_row.as_ref().map(|r| (r, table.updated_at));
            match self
                .write_commit_changes(&table_id, Some(&plan), &bounds, row)
                .await
            {
                Ok(()) => return Ok(CommitOutcome::Done),
                // Another writer changed the log between the read and the
                // write: plan again against the new log.
                Err(e) if attempt < COMMIT_ATTEMPTS => {
                    debug!(%table_id, attempt, error = %e.message(), "delta commit raced; retrying");
                }
                Err(e) => return Err(Error::new(ErrorCode::CommitStateUnknown, failed(e))),
            }
        }
    }

    /// Write a commit plan and/or a new table row in one transaction that
    /// first checks that the commit log and the row are still the ones the
    /// changes were planned from. `row` carries the `updated_at` it was read
    /// with.
    async fn write_commit_changes(
        &self,
        table_id: &str,
        plan: Option<&CommitPlan>,
        bounds: &Bounds,
        row: Option<(&TableRow, i64)>,
    ) -> Result<()> {
        let failed = db_err("delta commit");
        let tx = self.db.begin().await.map_err(&failed)?;
        let id: Value = table_id.into();
        // The table first: on PostgreSQL the write (or the lock) makes the
        // writers of one table take turns.
        match row {
            Some((row, read_at)) => {
                let mut values = TBL.encode(row)?;
                values.push(read_at.into());
                let sql = format!("{} AND updated_at = ${}", TBL.update(), values.len());
                if tx.execute(&sql, &values).await.map_err(&failed)? != 1 {
                    return Err(Error::aborted("the table changed"));
                }
            }
            None => {
                let sql = format!(
                    "SELECT id_string FROM tbl WHERE id_string = $1{}",
                    tx.for_update()
                );
                tx.query(&sql, std::slice::from_ref(&id))
                    .await
                    .map_err(&failed)?;
            }
        }
        if let Some(plan) = plan {
            let log = tx
                .query(
                    "SELECT MIN(commit_version), MAX(commit_version) FROM delta_commit \
                     WHERE table_id = $1",
                    std::slice::from_ref(&id),
                )
                .await
                .map_err(&failed)?;
            let (first, last) = match log.first() {
                Some(r) => (
                    r.opt_int(0).map_err(&failed)?,
                    r.opt_int(1).map_err(&failed)?,
                ),
                None => (None, None),
            };
            if first != bounds.first.as_ref().map(|c| c.commit_version)
                || last != bounds.last.as_ref().map(|c| c.commit_version)
            {
                return Err(Error::aborted("the commit log changed"));
            }
            if let Some(saved) = &plan.save {
                insert(tx.as_ref(), &DELTA_COMMIT, saved)
                    .await
                    .map_err(&failed)?;
            }
            if let Some(up_to) = plan.delete_up_to {
                tx.execute(
                    "DELETE FROM delta_commit WHERE table_id = $1 AND commit_version <= $2",
                    &[id.clone(), up_to.into()],
                )
                .await
                .map_err(&failed)?;
            }
            if let Some(marked) = plan.mark_backfilled {
                tx.execute(
                    "UPDATE delta_commit SET is_backfilled_latest_commit = $3 \
                     WHERE table_id = $1 AND commit_version = $2",
                    &[id.clone(), marked.into(), true.into()],
                )
                .await
                .map_err(&failed)?;
            }
        }
        tx.commit().await.map_err(&failed)
    }

    /// Upstream `updateTableForDelta`: apply a Delta API update, with any
    /// commit-log part, in one transaction. A replay of an accepted commit
    /// makes the whole request a no-op.
    async fn update_delta_table(
        &self,
        full_name: &str,
        req: &uc_api::delta_api::DeltaUpdateTableRequest,
        updated_by: Option<&str>,
        allow_missing_dv: bool,
    ) -> Result<CommitOutcome> {
        use uc_delta::update::{self as update_rules, TableState};
        let collected = update_rules::collect(req)?;
        let mut attempt = 0;
        loop {
            attempt += 1;
            let (_, row) = self.resolve_table(full_name).await?;
            if row.data_source_format.as_deref() != Some("DELTA") {
                return Err(Error::new(
                    ErrorCode::UnsupportedTableFormat,
                    format!("Table is not a Delta table: {full_name}"),
                ));
            }
            let table_id = row.id_string.clone();
            collected.check_table_uuid(&table_id)?;
            let pre_apply_etag = format!("etag-{}", row.updated_at);
            let mut state = TableState {
                table_type: crate::sql::parse_enum(&row.table_type)?,
                columns: row.columns.clone().unwrap_or_default(),
                properties: row.properties.clone().unwrap_or_default(),
                comment: row.comment.clone(),
            };
            let dispatch = update_rules::apply_updates(&mut state, &collected, allow_missing_dv)?;
            let mut plan = None;
            let mut bounds = Bounds {
                first: None,
                last: None,
                at_version: None,
            };
            let mut uniform = None;
            if let Some(dispatch) = dispatch {
                // Upstream `applyCommitAndBackfillInSession`: the commit rules
                // of `/delta/preview/commits`, without the URI check.
                self.check_managed_enabled()?;
                if let Some(fields) = &dispatch.uniform {
                    let location = row.storage_location.as_deref().unwrap_or_default();
                    uc_delta::uniform::require_subpath(
                        &fields.metadata_location,
                        &uc_common::NormalizedUrl::parse(location)?,
                    )?;
                }
                uniform = dispatch.uniform.clone();
                if let Some(info) = &dispatch.commit {
                    rules::validate_commit_info(info)?;
                }
                let version = dispatch
                    .commit
                    .as_ref()
                    .and_then(|i| i.version)
                    .unwrap_or(-1);
                bounds = self.commit_bounds(&table_id, version).await?;
                let commit = DeltaCommit {
                    table_id: Some(table_id.clone()),
                    commit_info: dispatch.commit,
                    latest_backfilled_version: dispatch.latest_backfilled_version,
                    ..Default::default()
                };
                let planned = rules::plan_commit(
                    &table_id,
                    &commit,
                    bounds.first.as_ref(),
                    bounds.last.as_ref(),
                    bounds.at_version.as_ref(),
                )?;
                match planned {
                    Planned::Replay => return Ok(CommitOutcome::Done),
                    Planned::ContentCheck {
                        version,
                        staged_file_name,
                    } => {
                        return Ok(CommitOutcome::ContentCheck {
                            table_location: row.storage_location.clone().unwrap_or_default(),
                            version,
                            staged_file_name,
                        });
                    }
                    Planned::Apply(p) => plan = Some(p),
                }
            }
            collected.check_etag(&pre_apply_etag)?;
            let mut new_row = row.clone();
            new_row.columns = Some(state.columns);
            new_row.properties = (row.properties.is_some() || !state.properties.is_empty())
                .then_some(state.properties);
            new_row.comment = state.comment;
            // A new commit carries the UniForm metadata it was converted
            // with (upstream `updateTableFromCommit`).
            let uniform_applied = match (&uniform, &plan) {
                (Some(fields), Some(plan)) if plan.save.is_some() => {
                    new_row.set_uniform(fields);
                    true
                }
                _ => false,
            };
            if collected.changes_table_metadata() || uniform_applied {
                touch_audit(&mut new_row.updated_at, &mut new_row.updated_by, updated_by);
                // The etag follows updated_at, so make sure it moves even
                // within one millisecond.
                new_row.updated_at = new_row.updated_at.max(row.updated_at + 1);
            }
            let changed = to_json(&new_row)? != to_json(&row)?;
            let write_row = changed.then_some((&new_row, row.updated_at));
            if plan.is_none() && write_row.is_none() {
                return Ok(CommitOutcome::Done);
            }
            match self
                .write_commit_changes(&table_id, plan.as_ref(), &bounds, write_row)
                .await
            {
                Ok(()) => return Ok(CommitOutcome::Done),
                Err(e) if attempt < COMMIT_ATTEMPTS => {
                    debug!(%table_id, attempt, error = %e.message(), "delta update raced; retrying");
                }
                Err(e) => {
                    return Err(Error::new(
                        ErrorCode::CommitStateUnknown,
                        format!("Failed to update table {full_name}: {}", e.message()),
                    ));
                }
            }
        }
    }

    /// The newest tracked commits of a table, newest first.
    async fn newest_commits(&self, table_id: &str) -> Result<Vec<StoredCommit>> {
        select(
            self.db.as_ref(),
            &DELTA_COMMIT,
            "WHERE table_id = $1 ORDER BY commit_version DESC LIMIT $2",
            &[table_id.into(), (COMMITS_PER_BATCH as i64).into()],
        )
        .await
    }

    /// Upstream `getCommits`.
    async fn get_delta_commits(&self, req: DeltaGetCommits) -> Result<DeltaGetCommitsResponse> {
        self.check_managed_enabled()?;
        let table_id = rules::validate_get_commits(&req)?;
        let table = self
            .table_by_id(&table_id)
            .await?
            .ok_or_else(|| Error::table_not_found(format!("Table not found: {table_id}")))?;
        rules::validate_table(
            &table_id,
            &table.table_type,
            table.data_source_format.as_deref(),
            table.storage_location.as_deref(),
        )?;
        let newest_first = self.newest_commits(&table_id).await?;
        Ok(rules::commits_response(&req, &newest_first))
    }
}

#[async_trait]
impl DeltaTableOps for SqlMetastore {
    async fn create_table(
        &self,
        catalog: &str,
        schema: &str,
        req: &DeltaCreateTableRequest,
        created_by: Option<&str>,
    ) -> Result<DeltaLoadTableResponse> {
        let allow_missing_dv = self.tables.allow_missing_deletion_vectors;
        let (create, uniform) =
            uc_delta::table::create_table_request(catalog, schema, req, allow_missing_dv)?;
        let full_name = three_part_full_name(catalog, schema, &create.name);
        self.persist_table(create, uniform, None, created_by)
            .await?;
        DeltaTableOps::load_table(self, &full_name).await
    }

    async fn load_table(&self, full_name: &str) -> Result<DeltaLoadTableResponse> {
        let (schema, row) = self.resolve_table(full_name).await?;
        let uniform = row.uniform();
        let info = row.into_info(&schema)?;
        uc_delta::table::require_delta(&info, full_name)?;
        let commits = match (info.table_type, info.table_id.as_deref()) {
            (Some(TableType::Managed), Some(id)) => Some(uc_delta::table::unbackfilled(
                self.newest_commits(id).await?,
            )),
            _ => None,
        };
        let mut response = uc_delta::table::load_response(&info, full_name, commits);
        response.uniform = uniform;
        Ok(response)
    }

    async fn commit(&self, commit: DeltaCommit, updated_by: Option<&str>) -> Result<CommitOutcome> {
        self.post_delta_commit(commit, updated_by).await
    }

    async fn get_commits(&self, req: DeltaGetCommits) -> Result<DeltaGetCommitsResponse> {
        self.get_delta_commits(req).await
    }

    async fn update_table(
        &self,
        full_name: &str,
        req: DeltaUpdateTableRequest,
        updated_by: Option<&str>,
    ) -> Result<CommitOutcome> {
        let allow_missing_dv = self.tables.allow_missing_deletion_vectors;
        self.update_delta_table(full_name, &req, updated_by, allow_missing_dv)
            .await
    }

    async fn unbackfilled_commits(&self, table_id: &str) -> Result<(Vec<StoredCommit>, i64)> {
        Ok(uc_delta::table::unbackfilled(
            self.newest_commits(table_id).await?,
        ))
    }
}
