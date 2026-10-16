//! `POST /delta/v1/.../tables/{table}`: the rules of a Delta API table
//! update. Upstream `DeltaUpdateTableMapper`.
//!
//! A request carries requirements (`assert-table-uuid` is mandatory) and at
//! most one of each update action. Actions apply in a fixed order to the
//! table's columns, properties and comment; `add-commit` and
//! `set-latest-backfilled-version` become a commit-log dispatch that goes
//! through the same rules as `/delta/preview/commits`.

use std::collections::{BTreeMap, BTreeSet};

use uc_api::{
    ColumnInfo, DeltaCommitInfo, TableType,
    delta_api::{
        DeltaCommitWire, DeltaDomainMetadataUpdates, DeltaProtocol, DeltaTableRequirement,
        DeltaTableUpdate, DeltaUniformMetadata, DeltaUpdateTableRequest,
    },
};
use uc_common::{Error, ErrorCode, Result};

use crate::contract::{
    self as contract, CLUSTERING_DOMAIN, FEATURE_PREFIX, LAST_COMMIT_TIMESTAMP,
    LAST_UPDATE_VERSION, ROW_TRACKING_DOMAIN,
};
use crate::table::{apply_partition_columns, column_infos, struct_fields};

fn invalid(message: impl Into<String>) -> Error {
    Error::invalid_argument(message)
}

/// The requirements and actions of one request, at most one of each.
#[derive(Debug, Default)]
pub struct Collected {
    assert_table_uuid: Option<Option<String>>,
    assert_etag: Option<Option<String>>,
    set_properties: Option<Option<BTreeMap<String, String>>>,
    remove_properties: Option<Option<Vec<String>>>,
    set_protocol: Option<Option<DeltaProtocol>>,
    set_columns: Option<Option<serde_json::Value>>,
    set_partition_columns: Option<Option<Vec<String>>>,
    set_table_comment: Option<Option<String>>,
    set_domain_metadata: Option<Option<DeltaDomainMetadataUpdates>>,
    remove_domain_metadata: Option<Option<Vec<String>>>,
    update_snapshot_version: Option<(Option<i64>, Option<i64>)>,
    add_commit: Option<(Option<DeltaCommitWire>, Option<DeltaUniformMetadata>)>,
    set_latest_backfilled_version: Option<Option<i64>>,
}

fn put_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<()> {
    if slot.is_some() {
        return Err(invalid(format!(
            "At most one {name} is allowed per request."
        )));
    }
    *slot = Some(value);
    Ok(())
}

/// Java's `TreeSet.toString`: `[a, b]`.
fn java_set(items: &BTreeSet<String>) -> String {
    format!("[{}]", items.iter().cloned().collect::<Vec<_>>().join(", "))
}

/// Upstream `collectRequest`: classify and shape-check the request.
pub fn collect(req: &DeltaUpdateTableRequest) -> Result<Collected> {
    let updates = req.updates.as_deref().unwrap_or_default();
    if updates.is_empty() {
        return Err(invalid("At least one update is required."));
    }
    let mut c = Collected::default();
    for requirement in req.requirements.iter().flatten() {
        match requirement {
            DeltaTableRequirement::AssertTableUuid { uuid } => {
                put_once(&mut c.assert_table_uuid, uuid.clone(), "assert-table-uuid")?
            }
            DeltaTableRequirement::AssertEtag { etag } => {
                put_once(&mut c.assert_etag, etag.clone(), "assert-etag")?
            }
        }
    }
    if c.assert_table_uuid.is_none() {
        return Err(invalid("assert-table-uuid requirement is required."));
    }
    for update in updates {
        let name = update.action();
        match update.clone() {
            DeltaTableUpdate::SetProperties { updates } => {
                put_once(&mut c.set_properties, updates, name)?
            }
            DeltaTableUpdate::RemoveProperties { removals } => {
                put_once(&mut c.remove_properties, removals, name)?
            }
            DeltaTableUpdate::SetProtocol { protocol } => {
                put_once(&mut c.set_protocol, protocol, name)?
            }
            DeltaTableUpdate::SetColumns { columns } => {
                put_once(&mut c.set_columns, columns, name)?
            }
            DeltaTableUpdate::SetPartitionColumns { partition_columns } => {
                put_once(&mut c.set_partition_columns, partition_columns, name)?
            }
            DeltaTableUpdate::SetTableComment { comment } => {
                put_once(&mut c.set_table_comment, comment, name)?
            }
            DeltaTableUpdate::SetDomainMetadata { updates } => {
                put_once(&mut c.set_domain_metadata, updates, name)?
            }
            DeltaTableUpdate::RemoveDomainMetadata { domains } => {
                put_once(&mut c.remove_domain_metadata, domains, name)?
            }
            DeltaTableUpdate::UpdateMetadataSnapshotVersion {
                last_commit_version,
                last_commit_timestamp_ms,
            } => put_once(
                &mut c.update_snapshot_version,
                (last_commit_version, last_commit_timestamp_ms),
                name,
            )?,
            DeltaTableUpdate::AddCommit { commit, uniform } => {
                put_once(&mut c.add_commit, (commit, uniform), name)?
            }
            DeltaTableUpdate::SetLatestBackfilledVersion {
                latest_published_version,
            } => put_once(
                &mut c.set_latest_backfilled_version,
                latest_published_version,
                name,
            )?,
        }
    }
    // Contradictory set/remove pairs fail fast.
    if let (Some(Some(set)), Some(Some(remove))) = (&c.set_properties, &c.remove_properties) {
        let overlap: BTreeSet<String> = remove
            .iter()
            .filter(|k| set.contains_key(*k))
            .cloned()
            .collect();
        if !overlap.is_empty() {
            return Err(invalid(format!(
                "set-properties and remove-properties overlap on keys: {}",
                java_set(&overlap)
            )));
        }
    }
    if let (Some(Some(set)), Some(Some(remove))) =
        (&c.set_domain_metadata, &c.remove_domain_metadata)
    {
        let mut set_domains = BTreeSet::new();
        if set.delta_clustering.is_some() {
            set_domains.insert(CLUSTERING_DOMAIN.to_string());
        }
        if set.delta_row_tracking.is_some() {
            set_domains.insert(ROW_TRACKING_DOMAIN.to_string());
        }
        let overlap: BTreeSet<String> = remove
            .iter()
            .filter(|d| set_domains.contains(*d))
            .cloned()
            .collect();
        if !overlap.is_empty() {
            return Err(invalid(format!(
                "set-domain-metadata and remove-domain-metadata overlap on domains: {}",
                java_set(&overlap)
            )));
        }
    }
    Ok(c)
}

impl Collected {
    /// A MANAGED-applicable metadata change. A `set-domain-metadata` of only
    /// the row-tracking domain is per-commit bookkeeping, not a change.
    fn has_managed_table_metadata_change(&self) -> bool {
        let beyond_row_tracking = matches!(
            &self.set_domain_metadata,
            Some(Some(dm)) if dm.delta_clustering.is_some()
        );
        self.set_properties.is_some()
            || self.remove_properties.is_some()
            || self.set_protocol.is_some()
            || self.set_columns.is_some()
            || self.set_partition_columns.is_some()
            || self.set_table_comment.is_some()
            || beyond_row_tracking
            || self.remove_domain_metadata.is_some()
    }

    /// Whether the request changes catalog-visible metadata, which moves
    /// `updated_at` (and so the etag).
    pub fn changes_table_metadata(&self) -> bool {
        self.has_managed_table_metadata_change() || self.update_snapshot_version.is_some()
    }

    /// Upstream `checkTableUuidRequirement`.
    pub fn check_table_uuid(&self, table_id: &str) -> Result<()> {
        if let Some(Some(expected)) = &self.assert_table_uuid {
            let normalized = uuid::Uuid::parse_str(expected)
                .map(|u| u.to_string())
                .map_err(|_| invalid(format!("Invalid UUID string: {expected}")))?;
            if normalized != table_id {
                return Err(Error::new(
                    ErrorCode::UpdateRequirementConflict,
                    format!(
                        "assert-table-uuid failed: expected {normalized} but table has {table_id}"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Upstream `checkEtagRequirement`, against the etag before the update.
    pub fn check_etag(&self, pre_apply_etag: &str) -> Result<()> {
        if let Some(Some(expected)) = &self.assert_etag
            && expected != pre_apply_etag
        {
            return Err(Error::new(
                ErrorCode::UpdateRequirementConflict,
                format!("assert-etag failed: expected {expected} but table has {pre_apply_etag}"),
            ));
        }
        Ok(())
    }
}

/// The parts of a table an update can change.
#[derive(Debug, Clone)]
pub struct TableState {
    pub table_type: TableType,
    pub columns: Vec<ColumnInfo>,
    pub properties: BTreeMap<String, String>,
    pub comment: Option<String>,
}

/// The commit-log part of an update, for the commit rules.
#[derive(Debug, Clone, Default)]
pub struct CommitDispatch {
    pub commit: Option<DeltaCommitInfo>,
    pub latest_backfilled_version: Option<i64>,
    pub uniform: Option<crate::uniform::UniformIceberg>,
}

/// Upstream `applyUpdates`: apply the actions in canonical order and return
/// the commit-log dispatch, if any.
pub fn apply_updates(
    state: &mut TableState,
    c: &Collected,
    allow_missing_dv: bool,
) -> Result<Option<CommitDispatch>> {
    apply_columns(state, c)?;
    if let Some(protocol) = &c.set_protocol {
        let protocol = protocol
            .as_ref()
            .ok_or_else(|| invalid("set-protocol requires a protocol."))?;
        state
            .properties
            .retain(|k, _| !k.starts_with(FEATURE_PREFIX));
        contract::derive_from_protocol(&mut state.properties, Some(protocol));
    }
    if let Some(Some(updates)) = &c.set_properties {
        state.properties.extend(updates.clone());
    }
    if let Some(Some(removals)) = &c.remove_properties {
        for key in removals {
            state.properties.remove(key);
        }
    }
    if let Some(updates) = &c.set_domain_metadata {
        let updates = updates
            .as_ref()
            .ok_or_else(|| invalid("set-domain-metadata requires an updates block."))?;
        let mut derived = BTreeMap::new();
        contract::derive_from_domain_metadata(&mut derived, Some(updates));
        if derived.is_empty() {
            return Err(invalid(format!(
                "set-domain-metadata requires at least one domain entry. Supported domains: \
                 [{CLUSTERING_DOMAIN}, {ROW_TRACKING_DOMAIN}]"
            )));
        }
        state.properties.extend(derived);
    }
    if let Some(Some(domains)) = &c.remove_domain_metadata {
        for domain in domains {
            let key = contract::domain_property(domain).ok_or_else(|| {
                invalid(format!(
                    "Unknown domain in remove-domain-metadata: {domain}"
                ))
            })?;
            state.properties.remove(key);
        }
    }
    if let Some(comment) = &c.set_table_comment {
        let comment = comment
            .clone()
            .ok_or_else(|| invalid("set-table-comment requires a comment."))?;
        state.comment = Some(comment);
    }
    if let Some((version, timestamp)) = c.update_snapshot_version {
        if state.table_type != TableType::External {
            return Err(invalid(
                "update-metadata-snapshot-version is only supported for EXTERNAL Delta tables; \
                 for MANAGED tables, use the Delta commit endpoint.",
            ));
        }
        let version = version.ok_or_else(|| {
            invalid("update-metadata-snapshot-version requires last-commit-version.")
        })?;
        let timestamp = timestamp.ok_or_else(|| {
            invalid("update-metadata-snapshot-version requires last-commit-timestamp-ms.")
        })?;
        state
            .properties
            .insert(LAST_UPDATE_VERSION.into(), version.to_string());
        state
            .properties
            .insert(LAST_COMMIT_TIMESTAMP.into(), timestamp.to_string());
    }
    // Re-check the MANAGED contract against the final state.
    if state.table_type == TableType::Managed {
        let dm = contract::domain_metadata_from_properties(&state.properties)?;
        if let Some(Some(protocol)) = &c.set_protocol {
            contract::validate(
                Some(protocol),
                Some(&dm),
                &state.properties,
                allow_missing_dv,
            )?;
        } else if c.set_domain_metadata.is_some() {
            let features = contract::features_from_properties(&state.properties);
            let features: BTreeSet<&str> = features.iter().map(String::as_str).collect();
            contract::validate_domain_metadata(&features, Some(&dm))?;
        }
    }
    if c.add_commit.is_none() && c.set_latest_backfilled_version.is_none() {
        return Ok(None);
    }
    prepare_commit(state, c).map(Some)
}

/// Upstream `applySchemaAndPartitionColumns`: build the new column list
/// once. The action that is absent keeps its current state.
fn apply_columns(state: &mut TableState, c: &Collected) -> Result<()> {
    if c.set_columns.is_none() && c.set_partition_columns.is_none() {
        return Ok(());
    }
    let mut columns = match &c.set_columns {
        Some(columns) => {
            let columns = columns
                .as_ref()
                .ok_or_else(|| invalid("set-columns requires a columns block."))?;
            match columns.get("fields") {
                None | Some(serde_json::Value::Null) => {
                    return Err(invalid("set-columns requires columns.fields."));
                }
                Some(serde_json::Value::Array(f)) if f.is_empty() => {
                    return Err(invalid("set-columns requires at least one column."));
                }
                _ => {}
            }
            column_infos(&struct_fields(Some(columns), "columns")?)?
        }
        None => {
            let mut current = state.columns.clone();
            for column in &mut current {
                column.partition_index = None;
            }
            current
        }
    };
    let partitions = match &c.set_partition_columns {
        Some(names) => names
            .clone()
            .ok_or_else(|| invalid("set-partition-columns requires a partition-columns list."))?,
        None => crate::table::partition_columns(&state.columns),
    };
    apply_partition_columns(&mut columns, &partitions)?;
    state.columns = columns;
    Ok(())
}

/// Upstream `prepareCommitAndBackfill`.
fn prepare_commit(state: &mut TableState, c: &Collected) -> Result<CommitDispatch> {
    if state.table_type != TableType::Managed {
        return Err(invalid(
            "add-commit and set-latest-backfilled-version require a MANAGED Delta table.",
        ));
    }
    let mut dispatch = CommitDispatch::default();
    if let Some((commit, uniform)) = &c.add_commit {
        let commit = commit
            .as_ref()
            .ok_or_else(|| invalid("add-commit requires a commit block."))?;
        let uniform = crate::uniform::from_delta_api(uniform.as_ref())?;
        if let Some(fields) = &uniform {
            crate::uniform::require_version_equals(fields, commit.version.unwrap_or_default())?;
        }
        contract::validate_uniform_consistency(&state.properties, uniform.is_some())?;
        // UC coordinates the commits, so a metadata-changing commit
        // stamps the snapshot properties itself.
        if c.has_managed_table_metadata_change() {
            let stamp = |v: Option<i64>| v.map_or("null".to_string(), |v| v.to_string());
            state
                .properties
                .insert(LAST_UPDATE_VERSION.into(), stamp(commit.version));
            state
                .properties
                .insert(LAST_COMMIT_TIMESTAMP.into(), stamp(commit.timestamp));
        }
        dispatch.commit = Some(DeltaCommitInfo {
            version: commit.version,
            timestamp: commit.timestamp,
            file_name: commit.file_name.clone(),
            file_size: commit.file_size,
            file_modification_timestamp: commit.file_modification_timestamp,
        });
        dispatch.uniform = uniform;
    }
    if let Some(version) = c.set_latest_backfilled_version {
        dispatch.latest_backfilled_version = Some(version.ok_or_else(|| {
            invalid("set-latest-backfilled-version requires latest-published-version.")
        })?);
    }
    Ok(dispatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(updates: serde_json::Value) -> DeltaUpdateTableRequest {
        serde_json::from_value(json!({
            "requirements": [{ "type": "assert-table-uuid", "uuid": "00000000-0000-0000-0000-000000000001" }],
            "updates": updates,
        }))
        .unwrap()
    }

    #[test]
    fn requests_are_shape_checked() {
        let err = |r: DeltaUpdateTableRequest| collect(&r).unwrap_err().message().to_string();
        assert_eq!(err(request(json!([]))), "At least one update is required.");
        assert_eq!(
            err(serde_json::from_value(
                json!({ "updates": [{ "action": "set-table-comment", "comment": "c" }] })
            )
            .unwrap()),
            "assert-table-uuid requirement is required."
        );
        assert_eq!(
            err(request(
                json!([{ "action": "set-table-comment", "comment": "a" },
                               { "action": "set-table-comment", "comment": "b" }])
            )),
            "At most one set-table-comment is allowed per request."
        );
        assert_eq!(
            err(request(
                json!([{ "action": "set-properties", "updates": { "a": "1", "b": "2" } },
                               { "action": "remove-properties", "removals": ["b", "a"] }])
            )),
            "set-properties and remove-properties overlap on keys: [a, b]"
        );
    }

    #[test]
    fn row_tracking_alone_is_not_a_metadata_change() {
        let c = collect(&request(json!([{ "action": "set-domain-metadata",
            "updates": { "delta.rowTracking": { "rowIdHighWaterMark": 5 } } }])))
        .unwrap();
        assert!(!c.changes_table_metadata());
        let c = collect(&request(
            json!([{ "action": "set-table-comment", "comment": "x" }]),
        ))
        .unwrap();
        assert!(c.changes_table_metadata());
    }
}
