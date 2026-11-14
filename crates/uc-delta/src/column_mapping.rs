//! Column-mapping rules of `set-columns`. Upstream
//! `DeltaUpdateTableMapper.validateColumnMappingGuards` and
//! `resolvePartitionColumns`.
//!
//! With column mapping, Delta reads a column by an identity in the field
//! metadata, not by its name: the column-mapping id in id mode, the physical
//! name in name mode. A `set-columns` that loses or reuses an identity makes
//! the data files unreadable, so the request is refused.

use std::collections::{HashMap, HashSet};

use delta_kernel::schema::{DataType, MetadataValue, StructField, StructType};
use delta_kernel::table_features::validate_schema_column_mapping;
use serde_json::Value;
use tracing::warn;

use uc_api::{ColumnInfo, TableType};
use uc_common::{Error, Result, SecurablePropertiesMap};

use crate::fields::{all_fields, kernel_message};
use crate::table::struct_field;

pub use delta_kernel::table_features::ColumnMappingMode;

pub const MODE: &str = "delta.columnMapping.mode";
pub const ID: &str = "delta.columnMapping.id";
pub const PHYSICAL_NAME: &str = "delta.columnMapping.physicalName";
pub const MAX_COLUMN_ID: &str = "delta.columnMapping.maxColumnId";

fn invalid(message: impl Into<String>) -> Error {
    Error::invalid_argument(message)
}

/// The mode of a `delta.columnMapping.mode` value, case ignored. An absent
/// value means no column mapping; an unknown value is refused.
pub fn parse_mode(raw: Option<&str>) -> Result<ColumnMappingMode> {
    let Some(raw) = raw else {
        return Ok(ColumnMappingMode::None);
    };
    raw.trim()
        .to_lowercase()
        .parse()
        .map_err(|_| invalid(format!("Invalid column mapping mode: {raw}")))
}

/// The metadata key that identifies a column in `mode`. A table without
/// column mapping is checked by physical name.
fn identity_metadata_key(mode: ColumnMappingMode) -> &'static str {
    match mode {
        ColumnMappingMode::Id => ID,
        ColumnMappingMode::Name | ColumnMappingMode::None => PHYSICAL_NAME,
    }
}

/// Whether the request sets the mode to name or id while the stored mode is
/// neither. Neither value is parsed, so an unknown mode does not block
/// other updates.
pub fn turns_on(stored: Option<&str>, requested: Option<&str>) -> bool {
    let name_or_id = |mode: Option<&str>| {
        mode.map(str::trim)
            .is_some_and(|m| m.eq_ignore_ascii_case("name") || m.eq_ignore_ascii_case("id"))
    };
    !name_or_id(stored) && name_or_id(requested)
}

/// The column-mapping id of a field: an integer in the 32-bit range.
pub(crate) fn column_id(field: &StructField) -> Option<i32> {
    match field.metadata().get(ID)? {
        MetadataValue::Number(id) => i32::try_from(*id).ok(),
        _ => None,
    }
}

/// The value of `key` in the metadata of a field, if it is a string.
fn metadata_str<'a>(field: &'a StructField, key: &str) -> Option<&'a str> {
    match field.metadata().get(key)? {
        MetadataValue::String(value) => Some(value),
        _ => None,
    }
}

/// The physical name of a field: a non-empty string.
pub(crate) fn physical_name(field: &StructField) -> Option<&str> {
    metadata_str(field, PHYSICAL_NAME).filter(|s| !s.is_empty())
}

/// Whether some field, at any depth, has `key` in its metadata.
fn carries_key(fields: &[StructField], key: &str) -> bool {
    all_fields(fields)
        .iter()
        .any(|f| f.metadata().contains_key(key))
}

/// The table's stored columns as fields. A stored column that cannot be
/// read is skipped with a warning, as loading a table does.
pub(crate) fn readable_stored_fields(columns: &[ColumnInfo]) -> Vec<StructField> {
    let mut fields = vec![];
    for column in columns {
        match struct_field(column) {
            Ok(field) => fields.push(field),
            Err(error) => warn!(
                column = column.name.as_deref().unwrap_or_default(),
                %error,
                "skipping a column: its stored type cannot be read"
            ),
        }
    }
    fields
}

/// Whether the metadata of a field has `key` with a value that is not null.
fn has_key(field: &StructField, key: &str) -> bool {
    field
        .metadata()
        .get(key)
        .is_some_and(|v| *v != MetadataValue::Other(Value::Null))
}

/// The dotted paths of the fields that stay in `incoming` (by name, case
/// ignored) but lost `key`. `prefix` is the path of the enclosing field.
fn stripped_keys(
    stored: &[&StructField],
    incoming: &[&StructField],
    key: &str,
    prefix: &str,
) -> Vec<String> {
    let incoming_by_name: HashMap<String, &StructField> = incoming
        .iter()
        .map(|f| (f.name().to_lowercase(), *f))
        .collect();
    let mut stripped = vec![];
    for stored in stored {
        // A column that is gone was dropped, not stripped.
        let Some(incoming) = incoming_by_name.get(&stored.name().to_lowercase()) else {
            continue;
        };
        let path = format!("{prefix}{}", stored.name());
        if has_key(stored, key) && !has_key(incoming, key) {
            stripped.push(path.clone());
        }
        stripped.extend(stripped_keys_in_types(
            stored.data_type(),
            incoming.data_type(),
            key,
            &format!("{path}."),
        ));
    }
    stripped
}

fn stripped_keys_in_types(
    stored: &DataType,
    incoming: &DataType,
    key: &str,
    prefix: &str,
) -> Vec<String> {
    match (stored, incoming) {
        (DataType::Struct(stored), DataType::Struct(incoming)) => {
            let stored: Vec<&StructField> = stored.fields().collect();
            let incoming: Vec<&StructField> = incoming.fields().collect();
            stripped_keys(&stored, &incoming, key, prefix)
        }
        (DataType::Array(stored), DataType::Array(incoming)) => {
            stripped_keys_in_types(stored.element_type(), incoming.element_type(), key, prefix)
        }
        (DataType::Map(stored), DataType::Map(incoming)) => {
            let mut stripped = stripped_keys_in_types(
                stored.key_type(),
                incoming.key_type(),
                key,
                &format!("{prefix}key."),
            );
            stripped.extend(stripped_keys_in_types(
                stored.value_type(),
                incoming.value_type(),
                key,
                &format!("{prefix}value."),
            ));
            stripped
        }
        _ => vec![],
    }
}

/// Refuse an empty physical name at any depth, and a physical name that two
/// top-level columns share with case ignored. Renames and partition columns
/// are found by physical name in lowercase, and Delta reads partition values
/// and statistics by physical name in both modes.
fn reject_unusable_physical_names(fields: &[StructField]) -> Result<()> {
    let empty: Vec<&str> = all_fields(fields)
        .into_iter()
        .filter(|f| metadata_str(f, PHYSICAL_NAME) == Some(""))
        .map(|f| f.name().as_str())
        .collect();
    if !empty.is_empty() {
        return Err(invalid(format!(
            "column(s) [{}] have an empty column mapping physical name.",
            empty.join(", ")
        )));
    }
    let mut seen = HashSet::new();
    for field in fields {
        if let Some(physical) = metadata_str(field, PHYSICAL_NAME)
            && !seen.insert(physical.to_lowercase())
        {
            return Err(invalid(format!(
                "Duplicate column mapping physicalName '{physical}' on top-level columns; each \
                 top-level column must have a unique physical name."
            )));
        }
    }
    Ok(())
}

/// Refuse a schema that a table in `mode` cannot store. The kernel's check
/// refuses a field at any depth without a physical name or an id, a value of
/// the wrong type, an id out of range or shared by two fields, and a
/// physical name that two fields of one struct share. Create and
/// `set-columns` both call it.
pub fn validate_identities(fields: &[StructField], mode: ColumnMappingMode) -> Result<()> {
    if mode == ColumnMappingMode::None {
        return Ok(());
    }
    let schema = StructType::try_new(fields.iter().cloned())
        .map_err(|e| invalid(format!("the schema cannot be read: {}", kernel_message(e))))?;
    validate_schema_column_mapping(&schema, mode).map_err(|e| invalid(kernel_message(e)))?;
    reject_unusable_physical_names(fields)
}

/// Whether an incoming field's physical name is the name of a stored field
/// of the same parent while its own name differs. Enabling column mapping
/// keeps each stored name as the physical name, so such a field was
/// renamed.
fn renames_while_enabling(current: &[&StructField], incoming: &[&StructField]) -> bool {
    let mut current_by_name: HashMap<String, &StructField> = HashMap::new();
    for field in current {
        current_by_name
            .entry(field.name().to_lowercase())
            .or_insert(field);
    }
    incoming.iter().any(|inc| {
        let Some(physical) = physical_name(inc) else {
            return false;
        };
        current_by_name
            .get(&physical.to_lowercase())
            .is_some_and(|cur| {
                cur.name().to_lowercase() != inc.name().to_lowercase()
                    || type_renames_while_enabling(cur.data_type(), inc.data_type())
            })
    })
}

fn type_renames_while_enabling(current: &DataType, incoming: &DataType) -> bool {
    match (current, incoming) {
        (DataType::Struct(current), DataType::Struct(incoming)) => {
            let current: Vec<&StructField> = current.fields().collect();
            let incoming: Vec<&StructField> = incoming.fields().collect();
            renames_while_enabling(&current, &incoming)
        }
        (DataType::Array(current), DataType::Array(incoming)) => {
            type_renames_while_enabling(current.element_type(), incoming.element_type())
        }
        (DataType::Map(current), DataType::Map(incoming)) => {
            type_renames_while_enabling(current.key_type(), incoming.key_type())
                || type_renames_while_enabling(current.value_type(), incoming.value_type())
        }
        _ => false,
    }
}

/// The identity Delta reads a field by in `mode`: the id as text in id
/// mode, else the physical name in lowercase.
fn identity_key(field: &StructField, mode: ColumnMappingMode) -> Option<String> {
    if mode == ColumnMappingMode::Id {
        return column_id(field).map(|id| id.to_string());
    }
    metadata_str(field, PHYSICAL_NAME).map(str::to_lowercase)
}

/// The top-level renames: columns whose identity stayed while the name
/// changed. A change of case only is not a rename.
fn top_level_renames(
    current: &[StructField],
    incoming: &[StructField],
    mode: ColumnMappingMode,
) -> Vec<(String, String)> {
    let incoming_by_key: HashMap<String, &str> = incoming
        .iter()
        .filter_map(|f| Some((identity_key(f, mode)?, f.name().as_str())))
        .collect();
    current
        .iter()
        .filter_map(|f| {
            let new_name = incoming_by_key.get(&identity_key(f, mode)?)?;
            (new_name.to_lowercase() != f.name().to_lowercase())
                .then(|| (f.name().clone(), new_name.to_string()))
        })
        .collect()
}

/// Check a `set-columns` against the stored column-mapping mode
/// (`table_mode`) and the mode the request sets (`requested_mode`).
///
/// - A field that stays keeps its identity, and a schema with identities
///   keeps them.
/// - An identity that is present must be usable, and unique where Delta
///   needs it to be.
/// - With column mapping, stored or being enabled, every field at any depth
///   has a physical name and an id.
/// - Enabling column mapping and renaming a column are separate requests.
/// - At most one top-level column is renamed per request. On a MANAGED
///   table, a rename is also checked with Delta's schema evolution rules
///   ([`crate::schema_evolution`]).
///
/// `properties` are the table properties before the request.
pub fn validate_set_columns(
    table_type: TableType,
    stored_columns: &[ColumnInfo],
    incoming: &[StructField],
    table_mode: ColumnMappingMode,
    requested_mode: ColumnMappingMode,
    properties: &SecurablePropertiesMap,
) -> Result<()> {
    let key = identity_metadata_key(table_mode);
    let current = readable_stored_fields(stored_columns);
    let current_has_keys = table_mode != ColumnMappingMode::None && carries_key(&current, key);
    let incoming_has_keys = carries_key(incoming, key);
    if current_has_keys {
        let stored: Vec<&StructField> = current.iter().collect();
        let requested: Vec<&StructField> = incoming.iter().collect();
        let mut stripped = stripped_keys(&stored, &requested, key, "");
        if !stripped.is_empty() {
            stripped.sort();
            return Err(invalid(format!(
                "set-columns dropped column mapping metadata for column(s): {}",
                stripped.join(", ")
            )));
        }
        if !incoming_has_keys {
            return Err(invalid(
                "set-columns dropped column mapping metadata; removing column mapping is not \
                 supported.",
            ));
        }
    }
    validate_identities(incoming, table_mode)?;
    let enabling = table_mode == ColumnMappingMode::None
        && (requested_mode != ColumnMappingMode::None || incoming_has_keys);
    if enabling {
        let mode = match requested_mode {
            ColumnMappingMode::None => ColumnMappingMode::Name,
            mode => mode,
        };
        validate_identities(incoming, mode)?;
    }
    if !incoming_has_keys {
        // No identity to find a rename by: a missing name is a drop and an
        // add.
        return Ok(());
    }
    if !current_has_keys {
        // The identities were checked above, against the stored mode or the
        // mode being enabled.
        let stored: Vec<&StructField> = current.iter().collect();
        let requested: Vec<&StructField> = incoming.iter().collect();
        if renames_while_enabling(&stored, &requested) {
            return Err(invalid(
                "set-columns: renaming a column while enabling column mapping is not supported \
                 in a single request. Enable column mapping in a separate statement, then rename \
                 the column.",
            ));
        }
        return Ok(());
    }
    let mode = match table_mode {
        ColumnMappingMode::None => ColumnMappingMode::Name,
        mode => mode,
    };
    let renames = top_level_renames(&current, incoming, mode);
    if renames.len() > 1 {
        let mut list: Vec<String> = renames
            .iter()
            .map(|(old, new)| format!("{old}->{new}"))
            .collect();
        list.sort();
        return Err(invalid(format!(
            "Only a single top-level column rename is supported per request; this request \
             renames {} columns [{}]. Split them into separate ALTER statements.",
            renames.len(),
            list.join(", ")
        )));
    }
    // The Delta log of an EXTERNAL table is the source of truth, and the
    // stored schema can lag behind it, so only a MANAGED table gets the
    // schema evolution check.
    if table_type == TableType::Managed {
        crate::schema_evolution::validate(
            stored_columns,
            incoming,
            properties,
            !renames.is_empty(),
        )?;
    }
    Ok(())
}

/// The stored top-level identities by column name in lowercase; empty
/// without column mapping. Two stored columns with one identity are
/// refused.
fn stored_keys_by_name(
    stored_columns: &[ColumnInfo],
    mode: ColumnMappingMode,
) -> Result<HashMap<String, String>> {
    let mut key_by_name = HashMap::new();
    if mode == ColumnMappingMode::None {
        return Ok(key_by_name);
    }
    let mut name_by_key: HashMap<String, String> = HashMap::new();
    for field in readable_stored_fields(stored_columns) {
        let Some(key) = identity_key(&field, mode) else {
            continue;
        };
        if let Some(other) = name_by_key.get(&key) {
            return Err(invalid(format!(
                "stored columns '{other}' and '{}' share the column mapping identity '{key}'; \
                 send set-partition-columns with this update.",
                field.name()
            )));
        }
        name_by_key.insert(key.clone(), field.name().clone());
        key_by_name.insert(field.name().to_lowercase(), key);
    }
    Ok(key_by_name)
}

/// The current partition columns under their names in the incoming schema,
/// so a renamed partition column stays a partition column. Columns are
/// matched by the stored identity, or by name when the table has no column
/// mapping or no stored column has an identity. A column that was replaced
/// under the same name with a new identity also keeps the partition, as
/// `REPLACE TABLE` sends it.
pub fn resolve_partition_columns(
    stored_columns: &[ColumnInfo],
    incoming: &[ColumnInfo],
    mode: ColumnMappingMode,
) -> Result<Vec<String>> {
    let mut partition_columns: Vec<&ColumnInfo> = stored_columns
        .iter()
        .filter(|c| c.partition_index.is_some())
        .collect();
    partition_columns.sort_by_key(|c| c.partition_index);
    if partition_columns.is_empty() {
        return Ok(vec![]);
    }
    let mut incoming_name_by_name: HashMap<String, &str> = HashMap::new();
    let mut incoming_name_by_key: HashMap<String, &str> = HashMap::new();
    let mut incoming_key_by_name: HashMap<String, String> = HashMap::new();
    for column in incoming {
        let column_name = column.name.as_deref().unwrap_or_default();
        let lower = column_name.to_lowercase();
        incoming_name_by_name.insert(lower.clone(), column_name);
        let key = struct_field(column)
            .ok()
            .and_then(|field| identity_key(&field, mode));
        if let Some(key) = key {
            incoming_name_by_key.insert(key.clone(), column_name);
            incoming_key_by_name.insert(lower, key);
        }
    }
    let current_key_by_name = stored_keys_by_name(stored_columns, mode)?;

    let mut resolved = vec![];
    for column in partition_columns {
        let column_name = column.name.as_deref().unwrap_or_default();
        let lower = column_name.to_lowercase();
        let new_name = if let Some(key) = current_key_by_name.get(&lower) {
            let by_key = incoming_name_by_key.get(key).copied();
            if let Some(moved_to) = by_key
                && moved_to.to_lowercase() != lower
                && incoming_name_by_name.contains_key(&lower)
            {
                // The identity says the column was renamed; the kept name
                // says it was not.
                return Err(invalid(format!(
                    "partition column '{column_name}' keeps its name, but its column mapping \
                     identity moved to '{moved_to}'; send set-partition-columns with this update."
                )));
            }
            // Replaced in place under the same name with an identity that
            // no stored column has.
            by_key.or_else(|| {
                incoming_key_by_name
                    .get(&lower)
                    .filter(|key| !current_key_by_name.values().any(|k| k == *key))
                    .and_then(|_| incoming_name_by_name.get(&lower).copied())
            })
        } else if current_key_by_name.is_empty() {
            incoming_name_by_name.get(&lower).copied()
        } else {
            return Err(invalid(format!(
                "partition column '{column_name}' has no column mapping identity while the \
                 table uses column mapping."
            )));
        };
        let new_name = new_name.ok_or_else(|| {
            invalid(format!(
                "partition column '{column_name}' dropped: not in the new schema."
            ))
        })?;
        resolved.push(new_name.to_string());
    }
    Ok(resolved)
}
