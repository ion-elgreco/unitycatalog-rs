//! Schema changes that delta-kernel-rs does not check yet. A `set-columns`
//! that renames a column of a MANAGED table runs these after the kernel's
//! [`SchemaDiff`], and each error names the gap.
//!
//! - A change that Delta can allow, but only after a check the kernel lacks,
//!   is not supported (`UNIMPLEMENTED`): a type change and a dropped field.
//! - A change that Delta never allows is an invalid argument: a field that
//!   moves to another struct, new `delta.typeChanges`, and a changed map key
//!   under Iceberg writer compatibility.

use std::collections::HashMap;

use delta_kernel::schema::diff::{FieldChange, FieldChangeType, FieldUpdate, SchemaDiff};
use delta_kernel::schema::{ColumnMetadataKey, DataType, MetadataValue, StructField, StructType};
use serde_json::Value;

use uc_common::{Error, Result};

use crate::fields::all_fields;
use crate::schema_evolution::invalid;

fn unsupported(message: impl AsRef<str>) -> Error {
    Error::unimplemented(format!("set-columns: {}", message.as_ref()))
}

/// Refuse a type change of a kept field when `delta.enableTypeWidening` is
/// on. Delta then takes the changes of type widening, and fewer under
/// Iceberg writer compatibility.
pub(crate) fn reject_type_change(update: &FieldUpdate) -> Result<()> {
    if !update.change_types.contains(&FieldChangeType::TypeChanged) {
        return Ok(());
    }
    Err(unsupported(format!(
        "changing the type of field {} from {} to {} in a request that renames a column is not \
         supported, as delta-kernel-rs cannot check type changes yet. Change the type in a \
         separate request.",
        update.path,
        update.before.data_type(),
        update.after.data_type()
    )))
}

/// Refuse a dropped field. Delta drops a field unless it is a clustering
/// column.
pub(crate) fn reject_dropped_fields(diff: &SchemaDiff) -> Result<()> {
    let (_, removed, _) = diff.all_changes();
    match first_path(removed) {
        Some(path) => Err(unsupported(format!(
            "dropping field {path} in a request that renames a column is not supported, as \
             delta-kernel-rs cannot check clustering columns yet. Drop it in a separate request."
        ))),
        None => Ok(()),
    }
}

/// Refuse a field that keeps its id but moves to another struct. The kernel
/// pairs the field by id and does not report the move.
pub(crate) fn reject_moves(current: &StructType, new: &StructType) -> Result<()> {
    let before = parents(current);
    let mut after: Vec<(i64, Parent)> = parents(new).into_iter().collect();
    after.sort_by_key(|(id, _)| *id);
    let describe = |parent: &Parent| match parent {
        None => "the top level".to_string(),
        Some((id, path)) if path.is_empty() => format!("the field with fieldId={id}"),
        Some((id, path)) => format!("{path} of the field with fieldId={id}"),
    };
    for (id, parent) in after {
        if let Some(old) = before.get(&id)
            && *old != parent
        {
            return Err(invalid(format!(
                "field with fieldId={id} moves from {} to {}. Moving a field to another struct \
                 is not supported, as delta-kernel-rs does not check it yet.",
                describe(old),
                describe(&parent)
            )));
        }
    }
    Ok(())
}

/// Refuse new `delta.typeChanges` on a field: only a type change writes
/// them. A request without them keeps the stored ones.
pub(crate) fn reject_new_type_changes(update: &FieldUpdate) -> Result<()> {
    let stored = type_changes(&update.before);
    match type_changes(&update.after) {
        Some(requested) if stored != Some(requested) => Err(invalid(format!(
            "field {} changes delta.typeChanges from {} to {requested}. This is not supported, \
             as delta-kernel-rs does not check type change history yet.",
            update.path,
            stored.map_or("[]".to_string(), ToString::to_string),
        ))),
        _ => Ok(()),
    }
}

/// Under Iceberg writer compatibility, refuse a change to the key of a map,
/// also by the fields of a struct key.
pub(crate) fn reject_map_key_changes(current: &StructType, new: &StructType) -> Result<()> {
    let current = fields_by_id(current);
    for field in all_fields(new.fields()) {
        if let Some(old) = field.column_mapping_id().and_then(|id| current.get(&id))
            && map_struct_key_changed(old.data_type(), field.data_type())
        {
            return Err(invalid(format!(
                "the key of map field {} changes. This is not supported under Iceberg writer \
                 compatibility, as delta-kernel-rs does not check map keys yet.",
                field.name()
            )));
        }
    }
    Ok(())
}

/// The first path of the changes in sort order, so the error does not
/// depend on the order of the diff.
fn first_path(changes: &[FieldChange]) -> Option<String> {
    changes.iter().map(|change| change.path.to_string()).min()
}

/// The struct fields of a schema by id. The kernel's schema diff refused
/// fields without an id and ids that two fields share before.
fn fields_by_id(schema: &StructType) -> HashMap<i64, &StructField> {
    all_fields(schema.fields())
        .into_iter()
        .filter_map(|field| Some((field.column_mapping_id()?, field)))
        .collect()
}

/// Where a struct field sits: the id of the nearest enclosing struct field
/// and the path from it to the struct (empty, or like `element` or
/// `value.element`). None at the top level.
type Parent = Option<(i64, String)>;

fn parents(schema: &StructType) -> HashMap<i64, Parent> {
    let mut out = HashMap::new();
    collect_parents(schema, None, &mut out);
    out
}

fn collect_parents(fields: &StructType, parent: Parent, out: &mut HashMap<i64, Parent>) {
    for field in fields.fields() {
        let Some(id) = field.column_mapping_id() else {
            continue;
        };
        out.insert(id, parent.clone());
        collect_parents_in(field.data_type(), id, String::new(), out);
    }
}

fn collect_parents_in(data_type: &DataType, id: i64, path: String, out: &mut HashMap<i64, Parent>) {
    let join = |part: &str| match path.as_str() {
        "" => part.to_string(),
        path => format!("{path}.{part}"),
    };
    match data_type {
        DataType::Struct(fields) => collect_parents(fields, Some((id, path)), out),
        DataType::Array(array) => {
            collect_parents_in(array.element_type(), id, join("element"), out)
        }
        DataType::Map(map) => {
            collect_parents_in(map.key_type(), id, join("key"), out);
            collect_parents_in(map.value_type(), id, join("value"), out);
        }
        _ => {}
    }
}

/// The `delta.typeChanges` of a field, unless null or empty.
fn type_changes(field: &StructField) -> Option<&MetadataValue> {
    field
        .get_config_value(&ColumnMetadataKey::TypeChanges)
        .filter(|value| {
            !matches!(value, MetadataValue::Other(Value::Null))
                && !matches!(value, MetadataValue::Other(Value::Array(a)) if a.is_empty())
        })
}

/// Whether a map in the type has struct keys that differ.
fn map_struct_key_changed(old: &DataType, new: &DataType) -> bool {
    match (old, new) {
        (DataType::Map(old), DataType::Map(new)) => {
            let struct_keys = matches!(
                (old.key_type(), new.key_type()),
                (DataType::Struct(_), DataType::Struct(_))
            );
            (struct_keys && old.key_type() != new.key_type())
                || map_struct_key_changed(old.value_type(), new.value_type())
        }
        (DataType::Array(old), DataType::Array(new)) => {
            map_struct_key_changed(old.element_type(), new.element_type())
        }
        _ => false,
    }
}
