//! Column-mapping rules of `set-columns`. Upstream
//! `DeltaUpdateTableMapper.validateColumnMappingGuards` and
//! `resolvePartitionColumns`.
//!
//! With column mapping, Delta reads a column by an identity in the field
//! metadata, not by its name: the column-mapping id in id mode, the physical
//! name in name mode. A `set-columns` that loses or reuses an identity makes
//! the data files unreadable, so the request is refused.

use std::collections::{HashMap, HashSet};

use delta_kernel::schema::StructType;
use delta_kernel::table_features::validate_schema_column_mapping;
use serde_json::{Map, Value, json};
use tracing::warn;

use uc_api::{
    ColumnInfo, TableType,
    delta_types::{DeltaDataType, DeltaStructField, KeyStyle},
};
use uc_common::{Error, Result, SecurablePropertiesMap};

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

/// The fields as a kernel schema, read from their Delta schema JSON.
pub(crate) fn kernel_schema(
    fields: &[DeltaStructField],
) -> std::result::Result<StructType, String> {
    let fields: Vec<Value> = fields.iter().map(|f| f.to_json(KeyStyle::Camel)).collect();
    serde_json::from_value(json!({ "type": "struct", "fields": fields })).map_err(|e| e.to_string())
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

fn metadata(field: &DeltaStructField) -> Option<&Map<String, Value>> {
    field.metadata.as_ref()
}

fn name(field: &DeltaStructField) -> &str {
    field.name.as_deref().unwrap_or_default()
}

/// The column-mapping id of a field: a JSON integer in the 32-bit range.
pub(crate) fn column_id(field: &DeltaStructField) -> Option<i32> {
    metadata(field)?
        .get(ID)?
        .as_i64()
        .and_then(|id| i32::try_from(id).ok())
}

/// The physical name of a field: a non-empty string.
pub(crate) fn physical_name(field: &DeltaStructField) -> Option<&str> {
    metadata(field)?
        .get(PHYSICAL_NAME)?
        .as_str()
        .filter(|s| !s.is_empty())
}

/// The struct fields directly under a type, through array elements and map
/// keys and values.
fn nested_fields(data_type: Option<&DeltaDataType>) -> Vec<&DeltaStructField> {
    match data_type {
        Some(DeltaDataType::Struct { fields }) => fields.iter().flatten().collect(),
        Some(DeltaDataType::Array { element_type, .. }) => nested_fields(element_type.as_deref()),
        Some(DeltaDataType::Map {
            key_type,
            value_type,
            ..
        }) => {
            let mut fields = nested_fields(key_type.as_deref());
            fields.extend(nested_fields(value_type.as_deref()));
            fields
        }
        _ => vec![],
    }
}

/// Every field at every depth, each before the fields nested in it.
fn all_fields<'a>(
    fields: impl IntoIterator<Item = &'a DeltaStructField>,
) -> Vec<&'a DeltaStructField> {
    let mut all = vec![];
    for field in fields {
        all.push(field);
        all.extend(all_fields(nested_fields(field.data_type.as_ref())));
    }
    all
}

/// Whether some field, at any depth, has `key` in its metadata.
fn carries_key(fields: &[DeltaStructField], key: &str) -> bool {
    all_fields(fields)
        .iter()
        .any(|f| metadata(f).is_some_and(|m| m.contains_key(key)))
}

/// The table's stored columns as fields. A stored column that cannot be
/// read is skipped with a warning, as loading a table does.
pub(crate) fn readable_stored_fields(columns: &[ColumnInfo]) -> Vec<DeltaStructField> {
    let mut fields = vec![];
    for column in columns {
        match struct_field(column) {
            Ok(field) if field.name.is_some() && field.data_type.is_some() => fields.push(field),
            Ok(_) => warn!(
                column = column.name.as_deref().unwrap_or_default(),
                "skipping a column: its stored type is an incomplete field"
            ),
            Err(error) => warn!(
                column = column.name.as_deref().unwrap_or_default(),
                %error,
                "skipping a column: its stored type cannot be read"
            ),
        }
    }
    fields
}

fn has_key(field: &DeltaStructField, key: &str) -> bool {
    metadata(field).is_some_and(|m| m.get(key).is_some_and(|v| !v.is_null()))
}

/// The dotted paths of the fields that stay in `incoming` (by name, case
/// ignored) but lost `key`. `prefix` is the path of the enclosing field.
fn stripped_keys(
    stored: &[&DeltaStructField],
    incoming: &[&DeltaStructField],
    key: &str,
    prefix: &str,
) -> Vec<String> {
    let incoming_by_name: HashMap<String, &DeltaStructField> = incoming
        .iter()
        .map(|f| (name(f).to_lowercase(), *f))
        .collect();
    let mut stripped = vec![];
    for stored in stored {
        // A column that is gone was dropped, not stripped.
        let Some(incoming) = incoming_by_name.get(&name(stored).to_lowercase()) else {
            continue;
        };
        let path = format!("{prefix}{}", name(stored));
        if has_key(stored, key) && !has_key(incoming, key) {
            stripped.push(path.clone());
        }
        stripped.extend(stripped_keys_in_types(
            stored.data_type.as_ref(),
            incoming.data_type.as_ref(),
            key,
            &format!("{path}."),
        ));
    }
    stripped
}

fn stripped_keys_in_types(
    stored: Option<&DeltaDataType>,
    incoming: Option<&DeltaDataType>,
    key: &str,
    prefix: &str,
) -> Vec<String> {
    match (stored, incoming) {
        (
            Some(DeltaDataType::Struct { fields: stored }),
            Some(DeltaDataType::Struct { fields: incoming }),
        ) => {
            // A nested stored field without a name or type has no identity
            // to keep.
            let stored: Vec<&DeltaStructField> = stored
                .iter()
                .flatten()
                .filter(|f| f.name.is_some() && f.data_type.is_some())
                .collect();
            let incoming: Vec<&DeltaStructField> = incoming.iter().flatten().collect();
            stripped_keys(&stored, &incoming, key, prefix)
        }
        (
            Some(DeltaDataType::Array {
                element_type: stored,
                ..
            }),
            Some(DeltaDataType::Array {
                element_type: incoming,
                ..
            }),
        ) => stripped_keys_in_types(stored.as_deref(), incoming.as_deref(), key, prefix),
        (
            Some(DeltaDataType::Map {
                key_type: stored_key,
                value_type: stored_value,
                ..
            }),
            Some(DeltaDataType::Map {
                key_type: incoming_key,
                value_type: incoming_value,
                ..
            }),
        ) => {
            let mut stripped = stripped_keys_in_types(
                stored_key.as_deref(),
                incoming_key.as_deref(),
                key,
                &format!("{prefix}key."),
            );
            stripped.extend(stripped_keys_in_types(
                stored_value.as_deref(),
                incoming_value.as_deref(),
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
fn reject_unusable_physical_names(fields: &[DeltaStructField]) -> Result<()> {
    let empty: Vec<&str> = all_fields(fields)
        .into_iter()
        .filter(|f| metadata(f).and_then(|m| m.get(PHYSICAL_NAME)) == Some(&json!("")))
        .map(name)
        .collect();
    if !empty.is_empty() {
        return Err(invalid(format!(
            "column(s) [{}] have an empty column mapping physical name.",
            empty.join(", ")
        )));
    }
    let mut seen = HashSet::new();
    for field in fields {
        let physical = metadata(field)
            .and_then(|m| m.get(PHYSICAL_NAME))
            .and_then(Value::as_str);
        if let Some(physical) = physical
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
pub fn validate_identities(fields: &[DeltaStructField], mode: ColumnMappingMode) -> Result<()> {
    if mode == ColumnMappingMode::None {
        return Ok(());
    }
    let schema =
        kernel_schema(fields).map_err(|e| invalid(format!("the schema cannot be read: {e}")))?;
    validate_schema_column_mapping(&schema, mode).map_err(|e| match e {
        delta_kernel::KernelError::Schema(message) => invalid(message),
        e => invalid(e.to_string()),
    })?;
    reject_unusable_physical_names(fields)
}

/// Whether an incoming field's physical name is the name of a stored field
/// of the same parent while its own name differs. Enabling column mapping
/// keeps each stored name as the physical name, so such a field was
/// renamed.
fn renames_while_enabling(current: &[&DeltaStructField], incoming: &[&DeltaStructField]) -> bool {
    let mut current_by_name: HashMap<String, &DeltaStructField> = HashMap::new();
    for field in current {
        current_by_name
            .entry(name(field).to_lowercase())
            .or_insert(field);
    }
    incoming.iter().any(|inc| {
        let Some(physical) = physical_name(inc) else {
            return false;
        };
        current_by_name
            .get(&physical.to_lowercase())
            .is_some_and(|cur| {
                name(cur).to_lowercase() != name(inc).to_lowercase()
                    || type_renames_while_enabling(cur.data_type.as_ref(), inc.data_type.as_ref())
            })
    })
}

fn type_renames_while_enabling(
    current: Option<&DeltaDataType>,
    incoming: Option<&DeltaDataType>,
) -> bool {
    match (current, incoming) {
        (
            Some(DeltaDataType::Struct {
                fields: Some(current),
            }),
            Some(DeltaDataType::Struct {
                fields: Some(incoming),
            }),
        ) => {
            let current: Vec<&DeltaStructField> = current.iter().collect();
            let incoming: Vec<&DeltaStructField> = incoming.iter().collect();
            renames_while_enabling(&current, &incoming)
        }
        (
            Some(DeltaDataType::Array {
                element_type: current,
                ..
            }),
            Some(DeltaDataType::Array {
                element_type: incoming,
                ..
            }),
        ) => type_renames_while_enabling(current.as_deref(), incoming.as_deref()),
        (
            Some(DeltaDataType::Map {
                key_type: current_key,
                value_type: current_value,
                ..
            }),
            Some(DeltaDataType::Map {
                key_type: incoming_key,
                value_type: incoming_value,
                ..
            }),
        ) => {
            type_renames_while_enabling(current_key.as_deref(), incoming_key.as_deref())
                || type_renames_while_enabling(current_value.as_deref(), incoming_value.as_deref())
        }
        _ => false,
    }
}

/// The identity Delta reads a field by in `mode`: the id as text in id
/// mode, else the physical name in lowercase.
fn identity_key(field: &DeltaStructField, mode: ColumnMappingMode) -> Option<String> {
    if mode == ColumnMappingMode::Id {
        return column_id(field).map(|id| id.to_string());
    }
    metadata(field)?
        .get(PHYSICAL_NAME)
        .and_then(Value::as_str)
        .map(str::to_lowercase)
}

/// The top-level renames: columns whose identity stayed while the name
/// changed. A change of case only is not a rename.
fn top_level_renames(
    current: &[DeltaStructField],
    incoming: &[DeltaStructField],
    mode: ColumnMappingMode,
) -> Vec<(String, String)> {
    let incoming_by_key: HashMap<String, &str> = incoming
        .iter()
        .filter_map(|f| Some((identity_key(f, mode)?, name(f))))
        .collect();
    current
        .iter()
        .filter_map(|f| {
            let new_name = incoming_by_key.get(&identity_key(f, mode)?)?;
            (new_name.to_lowercase() != name(f).to_lowercase())
                .then(|| (name(f).to_string(), new_name.to_string()))
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
    incoming: &[DeltaStructField],
    table_mode: ColumnMappingMode,
    requested_mode: ColumnMappingMode,
    properties: &SecurablePropertiesMap,
) -> Result<()> {
    let key = identity_metadata_key(table_mode);
    let current = readable_stored_fields(stored_columns);
    let current_has_keys = table_mode != ColumnMappingMode::None && carries_key(&current, key);
    let incoming_has_keys = carries_key(incoming, key);
    if current_has_keys {
        let stored: Vec<&DeltaStructField> = current.iter().collect();
        let requested: Vec<&DeltaStructField> = incoming.iter().collect();
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
        let stored: Vec<&DeltaStructField> = current.iter().collect();
        let requested: Vec<&DeltaStructField> = incoming.iter().collect();
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
                name(&field)
            )));
        }
        name_by_key.insert(key.clone(), name(&field).to_string());
        key_by_name.insert(name(&field).to_lowercase(), key);
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
