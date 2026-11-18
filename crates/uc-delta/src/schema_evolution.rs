//! Delta's schema evolution rules for a `set-columns` that renames a
//! column. Upstream `DeltaSchemaEvolutionValidator`, which calls Delta
//! Kernel's `SchemaUtils.validateSchemaEvolutionById`.
//!
//! The kernel's [`SchemaDiff`] pairs the fields of both schemas by
//! column-mapping id. It refuses a field without an id, an id that two
//! fields share, and a new or missing physical name under a kept id, and it
//! reports which kept fields changed nullability or type. The rules it
//! does not check are here and in [`crate::unsupported_changes`]. A type
//! change under type widening is not supported, as its check is missing.

use delta_kernel::schema::diff::{FieldChange, FieldChangeType, FieldUpdate, SchemaDiff};
use delta_kernel::schema::{DataType, StructField, StructType};

use uc_common::{Error, Result, SecurablePropertiesMap};

use crate::column_mapping::physical_name;
use crate::contract::{MAX_COLUMN_ID, stored_clustering_columns};
use crate::unsupported_changes::{self, fields_by_id};

const ENABLE_ICEBERG_WRITER_COMPAT_V1: &str = "delta.enableIcebergWriterCompatV1";
const ENABLE_TYPE_WIDENING: &str = "delta.enableTypeWidening";

pub(crate) fn invalid(message: impl AsRef<str>) -> Error {
    Error::invalid_argument(format!("set-columns: {}", message.as_ref()))
}

fn unsupported(message: impl AsRef<str>) -> Error {
    Error::unimplemented(format!("set-columns: {}", message.as_ref()))
}

/// Run the check when the request renames a field at any depth, also by
/// case only. Other requests skip it: an `ALTER COLUMN TYPE` writes a new
/// `delta.typeChanges` entry, which the check refuses, while a rename
/// carries the stored entries unchanged.
///
/// `current` holds the stored columns that can be read, `unreadable` the
/// error of the first one that cannot, and `diff` goes from `current` to
/// `new`. `properties` are the table properties before the request.
pub fn validate(
    current: &StructType,
    new: &StructType,
    diff: &SchemaDiff,
    unreadable: Option<String>,
    properties: &SecurablePropertiesMap,
) -> Result<()> {
    let (added, _, updated) = diff.all_changes();
    let renames = updated
        .iter()
        .any(|u| u.change_types.contains(&FieldChangeType::Renamed));
    if !renames {
        return Ok(());
    }
    if let Some(error) = unreadable {
        return Err(invalid(format!("the table schema cannot be read: {error}")));
    }
    let enabled = |key: &str| {
        properties
            .get(key)
            .is_some_and(|v| v.eq_ignore_ascii_case("true"))
    };
    let iceberg_writer_compat_v1 = enabled(ENABLE_ICEBERG_WRITER_COMPAT_V1);
    let type_widening = enabled(ENABLE_TYPE_WIDENING);

    unsupported_changes::reject_moves(current, new)?;
    for update in updated {
        unsupported_changes::reject_new_type_changes(update)?;
        reject_tightened_nullability(update)?;
    }
    if iceberg_writer_compat_v1 {
        unsupported_changes::reject_map_key_changes(current, new)?;
    }
    reject_invalid_added_fields(added, properties)?;
    reject_dropped_clustering_columns(current, new, properties)?;
    for update in updated {
        reject_type_change(update, type_widening)?;
    }
    Ok(())
}

/// A new field, also one nested in a kept struct, is nullable and its id is
/// above `delta.columnMapping.maxColumnId`. The fields inside a new struct
/// are not checked, as Delta does not check them either.
fn reject_invalid_added_fields(
    added: &[FieldChange],
    properties: &SecurablePropertiesMap,
) -> Result<()> {
    if added.is_empty() {
        return Ok(());
    }
    let max_column_id = properties
        .get(MAX_COLUMN_ID)
        .map(|value| {
            value.parse::<i64>().map_err(|_| {
                invalid(format!(
                    "the stored property {MAX_COLUMN_ID} is not a valid number: {value}"
                ))
            })
        })
        .transpose()?
        .unwrap_or(0);
    let mut added: Vec<&FieldChange> = added.iter().collect();
    added.sort_by_cached_key(|change| change.path.to_string());
    for change in added {
        if !change.field.is_nullable() {
            return Err(invalid(format!(
                "new field {} must be nullable",
                change.path
            )));
        }
        // The kernel's schema diff refused fields without an id before.
        if let Some(id) = change.field.column_mapping_id()
            && id <= max_column_id
        {
            return Err(invalid(format!(
                "new field {} has fieldId={id}, which must be above \
                 {MAX_COLUMN_ID}={max_column_id}",
                change.path
            )));
        }
    }
    Ok(())
}

/// A clustering column stays in the schema: its id is in `new`. The stored
/// `clusteringColumns` property lists each column as a path of physical
/// names, compared without case. A path that names no field of `current` is
/// skipped.
fn reject_dropped_clustering_columns(
    current: &StructType,
    new: &StructType,
    properties: &SecurablePropertiesMap,
) -> Result<()> {
    let Some(paths) = stored_clustering_columns(properties)? else {
        return Ok(());
    };
    let new_fields = fields_by_id(new);
    for path in paths {
        let Some((field, names)) = field_at(current, &path) else {
            continue;
        };
        if field
            .column_mapping_id()
            .is_some_and(|id| !new_fields.contains_key(&id))
        {
            return Err(invalid(format!(
                "clustering column {} cannot be dropped",
                names.join(".")
            )));
        }
    }
    Ok(())
}

/// The field at a path of physical names, through structs only, with the
/// names of the fields on the way.
fn field_at<'a>(
    schema: &'a StructType,
    path: &[String],
) -> Option<(&'a StructField, Vec<&'a str>)> {
    let find = |level: &'a StructType, physical: &str| {
        let physical = physical.to_lowercase();
        level
            .fields()
            .find(|field| physical_name(field).unwrap_or(field.name()).to_lowercase() == physical)
    };
    let (last, parents) = path.split_last()?;
    let mut level = schema;
    let mut names = vec![];
    for physical in parents {
        let field = find(level, physical)?;
        let DataType::Struct(fields) = field.data_type() else {
            return None;
        };
        names.push(field.name().as_str());
        level = fields;
    }
    let field = find(level, last)?;
    names.push(field.name().as_str());
    Some((field, names))
}

/// A kept field keeps its type unless `delta.enableTypeWidening` is on.
/// Delta then takes the changes of type widening, and fewer under Iceberg
/// writer compatibility, which delta-kernel-rs cannot check yet.
fn reject_type_change(update: &FieldUpdate, type_widening: bool) -> Result<()> {
    if !update.change_types.contains(&FieldChangeType::TypeChanged) {
        return Ok(());
    }
    let (path, before, after) = (
        &update.path,
        update.before.data_type(),
        update.after.data_type(),
    );
    if !type_widening {
        return Err(invalid(format!(
            "field {path} changes type from {before} to {after}. A type change needs \
             {ENABLE_TYPE_WIDENING}=true."
        )));
    }
    Err(unsupported(format!(
        "changing the type of field {path} from {before} to {after} in a request that renames \
         a column is not supported, as delta-kernel-rs cannot check type changes yet. Change \
         the type in a separate request."
    )))
}

/// A kept field does not become non-nullable, also not by the elements or
/// values it contains.
fn reject_tightened_nullability(update: &FieldUpdate) -> Result<()> {
    let tightened = update.change_types.iter().any(|change| {
        matches!(
            change,
            FieldChangeType::NullabilityTightened | FieldChangeType::ContainerNullabilityTightened
        )
    });
    if tightened {
        return Err(invalid(format!(
            "Cannot tighten the nullability of existing field {}",
            update.path
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use uc_api::ColumnInfo;

    use crate::column_mapping::readable_stored_fields;

    fn fld(name: &str, data_type: Value, id: Option<i32>, physical: Option<&str>) -> Value {
        fld_nullable(name, data_type, true, id, physical)
    }

    fn fld_nullable(
        name: &str,
        data_type: Value,
        nullable: bool,
        id: Option<i32>,
        physical: Option<&str>,
    ) -> Value {
        let mut metadata = serde_json::Map::new();
        if let Some(id) = id {
            metadata.insert("delta.columnMapping.id".into(), json!(id));
        }
        if let Some(physical) = physical {
            metadata.insert("delta.columnMapping.physicalName".into(), json!(physical));
        }
        json!({ "name": name, "type": data_type, "nullable": nullable, "metadata": metadata })
    }

    fn strukt(fields: Vec<Value>) -> Value {
        json!({ "type": "struct", "fields": fields })
    }

    fn array(element: Value) -> Value {
        json!({ "type": "array", "elementType": element, "containsNull": true })
    }

    fn map(key: Value, value: Value) -> Value {
        json!({ "type": "map", "keyType": key, "valueType": value, "valueContainsNull": true })
    }

    fn type_change(mut field: Value, from: &str, to: &str) -> Value {
        field["metadata"]["delta.typeChanges"] = json!([{ "fromType": from, "toType": to }]);
        field
    }

    fn properties(mode: &str, more: &[(&str, &str)]) -> SecurablePropertiesMap {
        let mut properties: SecurablePropertiesMap = [
            ("delta.columnMapping.mode", mode),
            ("delta.columnMapping.maxColumnId", "5"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        for (key, value) in more {
            properties.insert(key.to_string(), value.to_string());
        }
        properties
    }

    /// The check as `set-columns` runs it: on the stored columns that can be
    /// read, after the kernel's schema diff.
    fn check(
        properties: &SecurablePropertiesMap,
        current: Vec<Value>,
        incoming: Vec<Value>,
    ) -> Result<()> {
        let current: Vec<ColumnInfo> = current
            .iter()
            .map(|f| ColumnInfo {
                name: f["name"].as_str().map(str::to_string),
                type_json: Some(f.to_string()),
                ..Default::default()
            })
            .collect();
        let (current, unreadable) = readable_stored_fields(&current);
        let current = StructType::try_new(current).unwrap();
        let new = StructType::try_new(
            incoming
                .into_iter()
                .map(|f| serde_json::from_value(f).unwrap()),
        )
        .unwrap();
        let diff = SchemaDiff::new(&current, &new).map_err(|e| invalid(e.to_string()))?;
        validate(&current, &new, &diff, unreadable, properties)
    }

    fn rejected(
        properties: &SecurablePropertiesMap,
        current: Vec<Value>,
        incoming: Vec<Value>,
    ) -> String {
        let error = check(properties, current, incoming).unwrap_err();
        assert_eq!(error.code(), uc_common::ErrorCode::InvalidArgument);
        error.message().to_string()
    }

    fn unsupported(
        properties: &SecurablePropertiesMap,
        current: Vec<Value>,
        incoming: Vec<Value>,
    ) -> String {
        let error = check(properties, current, incoming).unwrap_err();
        assert_eq!(error.code(), uc_common::ErrorCode::Unimplemented);
        error.message().to_string()
    }

    fn long(name: &str, id: i32, physical: &str) -> Value {
        fld(name, json!("long"), Some(id), Some(physical))
    }

    #[test]
    fn a_rename_keeps_type_nullability_and_physical_name() {
        for mode in ["name", "id"] {
            let p = properties(mode, &[]);
            let current = || vec![long("a", 1, "col-1")];
            assert!(
                rejected(
                    &p,
                    current(),
                    vec![fld("b", json!("string"), Some(1), Some("col-1"))]
                )
                .contains(
                    "field b changes type from long to string. A type change needs \
                     delta.enableTypeWidening=true."
                )
            );
            assert!(
                rejected(
                    &p,
                    current(),
                    vec![fld_nullable(
                        "b",
                        json!("long"),
                        false,
                        Some(1),
                        Some("col-1")
                    )]
                )
                .contains("Cannot tighten the nullability")
            );
            assert!(
                rejected(&p, current(), vec![long("b", 1, "col-9")])
                    .contains("inconsistent physical names: 'col-1' -> 'col-9'")
            );
            assert!(check(&p, current(), vec![long("b", 1, "col-1")]).is_ok());
        }
    }

    #[test]
    fn a_new_field_with_a_rename_is_checked() {
        for mode in ["name", "id"] {
            let p = properties(mode, &[]);
            let current = || vec![long("a", 1, "col-1")];
            let with = |field: Value| vec![long("b", 1, "col-1"), field];
            assert!(check(&p, current(), with(long("c", 6, "col-6"))).is_ok());
            assert!(
                rejected(
                    &p,
                    current(),
                    with(fld_nullable(
                        "c",
                        json!("long"),
                        false,
                        Some(6),
                        Some("col-6")
                    ))
                )
                .contains("new field c must be nullable")
            );
            assert!(
                rejected(&p, current(), with(long("c", 5, "col-5"))).contains(
                    "new field c has fieldId=5, which must be above \
                     delta.columnMapping.maxColumnId=5"
                )
            );
            assert!(
                rejected(
                    &p,
                    current(),
                    with(fld("c", json!("long"), None, Some("col-c")))
                )
                .contains("is missing column mapping ID")
            );
        }
    }

    #[test]
    fn fields_inside_a_new_struct_are_not_checked() {
        let p = properties("name", &[]);
        let inner = fld_nullable("m", json!("long"), false, Some(2), Some("col-m"));
        assert!(
            check(
                &p,
                vec![long("a", 1, "col-1")],
                vec![
                    long("b", 1, "col-1"),
                    fld("n", strukt(vec![inner]), Some(6), Some("col-n"))
                ]
            )
            .is_ok()
        );
    }

    #[test]
    fn the_stored_max_column_id_is_read() {
        let current = || vec![long("a", 1, "col-1")];
        let incoming = || vec![long("b", 1, "col-1"), long("c", 2, "col-2")];
        let mut p = properties("name", &[]);
        p.remove("delta.columnMapping.maxColumnId");
        assert!(check(&p, current(), incoming()).is_ok());
        let p = properties("name", &[("delta.columnMapping.maxColumnId", "two")]);
        assert!(rejected(&p, current(), incoming()).contains(
            "the stored property delta.columnMapping.maxColumnId is not a valid number: two"
        ));
        // Without a new field, the property is not read.
        assert!(check(&p, current(), vec![long("b", 1, "col-1")]).is_ok());
    }

    #[test]
    fn nested_fields_are_checked() {
        let p = properties("id", &[]);
        let current = || {
            vec![fld(
                "s",
                strukt(vec![long("x", 2, "col-x")]),
                Some(1),
                Some("col-s"),
            )]
        };
        let renamed = |field: Value| {
            let fields = vec![long("x", 2, "col-x"), field];
            vec![fld("t", strukt(fields), Some(1), Some("col-s"))]
        };
        assert!(
            rejected(&p, current(), renamed(long("x2", 4, "col-x2")))
                .contains("new field t.x2 has fieldId=4")
        );
        assert!(check(&p, current(), renamed(long("x2", 6, "col-x2"))).is_ok());
        assert!(
            check(
                &p,
                current(),
                vec![fld(
                    "t",
                    strukt(vec![long("x2", 2, "col-x")]),
                    Some(1),
                    Some("col-s")
                )]
            )
            .is_ok()
        );
        // A nested rename alone runs the check.
        let p = properties("name", &[]);
        assert!(
            rejected(
                &p,
                current(),
                vec![fld(
                    "s",
                    strukt(vec![fld("y", json!("string"), Some(2), Some("col-x"))]),
                    Some(1),
                    Some("col-s")
                )]
            )
            .contains("field s.y changes type from long to string")
        );
    }

    #[test]
    fn a_kept_id_with_a_new_physical_name_is_refused_without_a_rename() {
        let id = properties("id", &[]);
        assert!(
            rejected(
                &id,
                vec![long("a", 1, "col-1")],
                vec![long("a", 1, "col-new")]
            )
            .contains("inconsistent physical names: 'col-1' -> 'col-new'")
        );
        let name = properties("name", &[]);
        let in_array = |physical| {
            vec![fld(
                "e",
                array(strukt(vec![long("x", 2, physical)])),
                Some(1),
                Some("col-e"),
            )]
        };
        assert!(
            rejected(&name, in_array("col-x"), in_array("col-z"))
                .contains("inconsistent physical names: 'col-x' -> 'col-z'")
        );
        let in_map = |physical| {
            vec![fld(
                "m",
                map(json!("string"), strukt(vec![long("x", 2, physical)])),
                Some(1),
                Some("col-m"),
            )]
        };
        assert!(
            rejected(&name, in_map("col-x"), in_map("col-z"))
                .contains("inconsistent physical names: 'col-x' -> 'col-z'")
        );
    }

    #[test]
    fn repeated_nested_names_are_not_mixed_up() {
        for mode in ["name", "id"] {
            let p = properties(mode, &[]);
            let schema = |first: &str| {
                vec![
                    long(first, 1, "col-1"),
                    fld(
                        "s",
                        strukt(vec![long("x", 3, "col-x")]),
                        Some(2),
                        Some("col-s"),
                    ),
                    fld(
                        "t",
                        strukt(vec![long("x", 5, "col-x")]),
                        Some(4),
                        Some("col-t"),
                    ),
                ]
            };
            assert!(check(&p, schema("a"), schema("b")).is_ok());
            let schema = |first: &str| {
                vec![
                    long(first, 1, "col-1"),
                    fld(
                        "s",
                        strukt(vec![long("x", 3, "col-x")]),
                        Some(2),
                        Some("col-s"),
                    ),
                ]
            };
            assert!(check(&p, schema("x"), schema("y")).is_ok());
        }
    }

    #[test]
    fn a_dropped_field_with_a_rename_is_taken() {
        let p = properties("name", &[]);
        assert!(
            check(
                &p,
                vec![long("a", 1, "col-1"), long("c", 2, "col-c")],
                vec![long("b", 1, "col-1")]
            )
            .is_ok()
        );
        // A rename onto the name of a dropped column.
        assert!(
            check(
                &p,
                vec![long("a", 1, "col-1"), long("b", 2, "col-2")],
                vec![long("b", 1, "col-1")]
            )
            .is_ok()
        );
    }

    #[test]
    fn a_clustering_column_stays() {
        let s = |name: &str, fields| fld(name, strukt(fields), Some(2), Some("col-s"));
        let current = || {
            vec![
                long("a", 1, "col-1"),
                s("s", vec![long("x", 3, "col-x"), long("y", 4, "col-y")]),
            ]
        };
        let clustered = |paths: &str| properties("name", &[("clusteringColumns", paths)]);
        let p = clustered(r#"[["COL-1"], ["col-s", "col-x"]]"#);
        assert!(
            rejected(&p, current(), vec![long("b", 1, "col-1")])
                .contains("clustering column s.x cannot be dropped")
        );
        assert!(
            rejected(
                &p,
                current(),
                vec![long("b", 1, "col-1"), s("s", vec![long("y", 4, "col-y")])]
            )
            .contains("clustering column s.x cannot be dropped")
        );
        // Renamed, also with its struct, the column stays.
        assert!(
            check(
                &p,
                current(),
                vec![long("b", 1, "col-1"), s("t", vec![long("z", 3, "col-x")])]
            )
            .is_ok()
        );
        // Another dropped field is taken.
        assert!(
            check(
                &p,
                current(),
                vec![long("b", 1, "col-1"), s("s", vec![long("x", 3, "col-x")])]
            )
            .is_ok()
        );
        // Paths that name no field are skipped.
        let p = clustered(r#"[[], ["col-1", "x"], ["gone"]]"#);
        assert!(check(&p, current(), vec![long("b", 1, "col-1")]).is_ok());
        let error = check(
            &clustered("not json"),
            current(),
            vec![long("b", 1, "col-1")],
        );
        assert_eq!(error.unwrap_err().code(), uc_common::ErrorCode::Internal);
    }

    #[test]
    fn a_field_stays_in_its_struct() {
        let p = properties("name", &[]);
        let message = rejected(
            &p,
            vec![
                fld(
                    "s1",
                    strukt(vec![long("x", 3, "col-x")]),
                    Some(1),
                    Some("col-s1"),
                ),
                fld(
                    "s2",
                    strukt(vec![long("y", 4, "col-y")]),
                    Some(2),
                    Some("col-s2"),
                ),
            ],
            vec![
                fld(
                    "t1",
                    strukt(vec![long("z", 6, "col-z")]),
                    Some(1),
                    Some("col-s1"),
                ),
                fld(
                    "s2",
                    strukt(vec![long("y", 4, "col-y"), long("x", 3, "col-x")]),
                    Some(2),
                    Some("col-s2"),
                ),
            ],
        );
        assert!(
            message.contains(
                "field with fieldId=3 moves from the field with fieldId=1 to the field with \
                 fieldId=2. Moving a field to another struct is not supported"
            ),
            "{message}"
        );
    }

    #[test]
    fn stored_type_changes_and_ids_are_kept() {
        let p = properties("name", &[]);
        assert!(
            rejected(
                &p,
                vec![type_change(long("a", 1, "col-1"), "integer", "long")],
                vec![type_change(long("b", 1, "col-1"), "short", "long")]
            )
            .contains("field b changes delta.typeChanges from")
        );
        assert!(
            check(
                &p,
                vec![type_change(long("a", 1, "col-1"), "integer", "long")],
                vec![type_change(long("b", 1, "col-1"), "integer", "long")]
            )
            .is_ok()
        );
        assert!(
            rejected(
                &p,
                vec![long("a", 1, "col-1"), long("c", 1, "col-c")],
                vec![long("b", 1, "col-1"), long("c", 1, "col-c")]
            )
            .contains("Duplicate field ID 1")
        );
    }

    #[test]
    fn requests_without_a_rename_skip_the_check() {
        assert!(
            check(
                &properties("name", &[]),
                vec![long("a", 1, "col-1")],
                vec![fld("a", json!("string"), Some(1), Some("col-1"))]
            )
            .is_ok()
        );
        assert!(
            check(
                &properties("id", &[]),
                vec![long("a", 1, "col-1")],
                vec![long("a", 2, "col-1")]
            )
            .is_ok()
        );
        // A stored column that cannot be read has no identity to compare.
        assert!(
            check(
                &properties("name", &[]),
                vec![fld("a", json!(12345), Some(1), Some("col-1"))],
                vec![long("b", 1, "col-1")]
            )
            .is_ok()
        );
        // A change of case only runs the check and passes.
        assert!(
            check(
                &properties("name", &[]),
                vec![long("a", 1, "col-1")],
                vec![long("A", 1, "col-1")]
            )
            .is_ok()
        );
    }

    #[test]
    fn a_schema_that_cannot_be_read_fails_the_check() {
        let p = properties("name", &[]);
        let unknown = || fld("v", json!("hyperdecimal"), Some(2), Some("col-v"));
        assert!(
            rejected(
                &p,
                vec![long("a", 1, "col-1"), unknown()],
                vec![long("b", 1, "col-1"), long("v", 2, "col-v")]
            )
            .contains("the table schema cannot be read")
        );
        assert!(
            check(
                &p,
                vec![long("a", 1, "col-1"), unknown()],
                vec![long("a", 1, "col-1"), long("v", 2, "col-v")]
            )
            .is_ok()
        );
    }

    #[test]
    fn other_stored_state_does_not_block_a_rename() {
        let rename = |p: &SecurablePropertiesMap, current: Vec<Value>| {
            check(p, current, vec![long("b", 1, "col-1")])
        };
        let p = properties(
            "name",
            &[("delta.rowTracking.rowIdHighWaterMark", "not a number")],
        );
        assert!(rename(&p, vec![long("a", 1, "col-1")]).is_ok());
    }

    #[test]
    fn a_type_change_with_a_rename_needs_type_widening() {
        let int = || vec![fld("a", json!("integer"), Some(1), Some("col-1"))];
        let double = || vec![fld("b", json!("double"), Some(1), Some("col-1"))];
        for value in ["true", "TRUE"] {
            let widening = properties("name", &[("delta.enableTypeWidening", value)]);
            assert!(
                unsupported(&widening, int(), double())
                    .contains("changing the type of field b from integer to double")
            );
        }
        for p in [
            properties("name", &[]),
            properties("name", &[("delta.enableTypeWidening", "false")]),
        ] {
            assert!(
                rejected(&p, int(), double())
                    .contains("field b changes type from integer to double")
            );
        }
    }
}
