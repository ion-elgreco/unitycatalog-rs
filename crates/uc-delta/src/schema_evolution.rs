//! Delta's schema evolution rules for a `set-columns` that renames a
//! column. Upstream `DeltaSchemaEvolutionValidator`, which calls Delta
//! Kernel's `SchemaUtils.validateSchemaEvolutionById`.
//!
//! The kernel's [`SchemaDiff`] pairs the fields of both schemas by
//! column-mapping id. It refuses a field without an id, an id that two
//! fields share, and a new or missing physical name under a kept id, and it
//! reports which kept fields changed nullability or type. The changes it
//! does not check yet are refused in [`crate::unsupported_changes`].

use std::collections::HashMap;

use delta_kernel::schema::diff::{FieldChangeType, FieldUpdate, SchemaDiff};
use delta_kernel::schema::{MetadataValue, StructField, StructType};
use serde_json::Value;

use uc_api::ColumnInfo;
use uc_common::{Error, Result, SecurablePropertiesMap};

use crate::column_mapping::{PHYSICAL_NAME, column_id};
use crate::fields::{kernel_message, nested_fields};
use crate::table::struct_field;
use crate::unsupported_changes;

const ENABLE_ICEBERG_WRITER_COMPAT_V1: &str = "delta.enableIcebergWriterCompatV1";

pub(crate) fn invalid(message: impl AsRef<str>) -> Error {
    Error::invalid_argument(format!("set-columns: {}", message.as_ref()))
}

/// Run the check when the request renames a top-level column, or keeps the
/// id of some field while its name or physical name changes at any depth.
/// Other requests skip it: an `ALTER COLUMN TYPE` writes a new
/// `delta.typeChanges` entry, which the check refuses, while a rename
/// carries the stored entries unchanged.
///
/// `properties` are the table properties before the request.
pub fn validate(
    current_columns: &[ColumnInfo],
    incoming: &[StructField],
    properties: &SecurablePropertiesMap,
    renames_top_level_column: bool,
) -> Result<()> {
    if !renames_top_level_column && !keeps_an_id_under_a_new_identity(current_columns, incoming) {
        return Ok(());
    }
    let current = current_columns
        .iter()
        .map(struct_field)
        .collect::<std::result::Result<Vec<_>, _>>()
        .and_then(|fields| StructType::try_new(fields).map_err(kernel_message))
        .map_err(|e| invalid(format!("the table schema cannot be read: {e}")))?;
    let new = StructType::try_new(incoming.iter().cloned()).map_err(|e| {
        invalid(format!(
            "the requested schema cannot be read: {}",
            kernel_message(e)
        ))
    })?;
    let iceberg_writer_compat_v1 = properties
        .get(ENABLE_ICEBERG_WRITER_COMPAT_V1)
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));

    let diff = SchemaDiff::new(&current, &new).map_err(|e| invalid(e.to_string()))?;
    let (_, _, updated) = diff.all_changes();
    // Changes that Delta refuses.
    unsupported_changes::reject_moves(&current, &new)?;
    for update in updated {
        unsupported_changes::reject_new_type_changes(update)?;
        reject_tightened_nullability(update)?;
    }
    if iceberg_writer_compat_v1 {
        unsupported_changes::reject_map_key_changes(&current, &new)?;
    }
    // Changes that Delta can take after a check the kernel lacks.
    unsupported_changes::reject_added_fields(&diff)?;
    for update in updated {
        unsupported_changes::reject_type_change(update)?;
    }
    unsupported_changes::reject_dropped_fields(&diff)
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

/// A field's name and physical name: what a rename or a new physical name
/// changes under an id.
type Identity = (String, Option<MetadataValue>);

fn keeps_an_id_under_a_new_identity(
    current_columns: &[ColumnInfo],
    incoming: &[StructField],
) -> bool {
    let mut current = HashMap::new();
    // A stored column that cannot be read has no identity to compare.
    for field in current_columns.iter().filter_map(|c| struct_field(c).ok()) {
        collect_identities(&field, &mut current);
    }
    let mut requested = HashMap::new();
    for field in incoming {
        collect_identities(field, &mut requested);
    }
    requested
        .iter()
        .any(|(id, identity)| current.get(id).is_some_and(|c| c != identity))
}

fn collect_identities(field: &StructField, by_id: &mut HashMap<i32, Identity>) {
    if let Some(id) = column_id(field) {
        let physical = field
            .metadata()
            .get(PHYSICAL_NAME)
            .filter(|v| **v != MetadataValue::Other(Value::Null))
            .cloned();
        by_id.insert(id, (field.name().clone(), physical));
    }
    for nested in nested_fields(field.data_type()) {
        collect_identities(nested, by_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    fn check(
        properties: &SecurablePropertiesMap,
        rename: bool,
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
        let incoming: Vec<StructField> = incoming
            .into_iter()
            .map(|f| serde_json::from_value(f).unwrap())
            .collect();
        validate(&current, &incoming, properties, rename)
    }

    fn rejected(
        properties: &SecurablePropertiesMap,
        rename: bool,
        current: Vec<Value>,
        incoming: Vec<Value>,
    ) -> String {
        let error = check(properties, rename, current, incoming).unwrap_err();
        assert_eq!(error.code(), uc_common::ErrorCode::InvalidArgument);
        error.message().to_string()
    }

    fn unsupported(
        properties: &SecurablePropertiesMap,
        rename: bool,
        current: Vec<Value>,
        incoming: Vec<Value>,
    ) -> String {
        let error = check(properties, rename, current, incoming).unwrap_err();
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
                unsupported(
                    &p,
                    true,
                    current(),
                    vec![fld("b", json!("string"), Some(1), Some("col-1"))]
                )
                .contains("changing the type of field b from long to string")
            );
            assert!(
                rejected(
                    &p,
                    true,
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
                rejected(&p, true, current(), vec![long("b", 1, "col-9")])
                    .contains("inconsistent physical names: 'col-1' -> 'col-9'")
            );
            assert!(check(&p, true, current(), vec![long("b", 1, "col-1")]).is_ok());
        }
    }

    #[test]
    fn a_new_field_is_not_supported_with_a_rename() {
        for mode in ["name", "id"] {
            let p = properties(mode, &[]);
            let current = || vec![long("a", 1, "col-1")];
            assert!(
                unsupported(
                    &p,
                    true,
                    current(),
                    vec![long("b", 1, "col-1"), long("c", 6, "col-6")]
                )
                .contains("adding field c in a request that renames a column is not supported")
            );
            assert!(
                rejected(
                    &p,
                    true,
                    current(),
                    vec![
                        long("b", 1, "col-1"),
                        fld("c", json!("long"), None, Some("col-c"))
                    ]
                )
                .contains("is missing column mapping ID")
            );
        }
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
        assert!(
            unsupported(
                &p,
                true,
                current(),
                vec![fld(
                    "t",
                    strukt(vec![long("x2", 6, "col-x2")]),
                    Some(1),
                    Some("col-s")
                )]
            )
            .contains("adding field t.x2")
        );
        assert!(
            check(
                &p,
                true,
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
            unsupported(
                &p,
                false,
                current(),
                vec![fld(
                    "s",
                    strukt(vec![fld("y", json!("string"), Some(2), Some("col-x"))]),
                    Some(1),
                    Some("col-s")
                )]
            )
            .contains("changing the type of field s.y from long to string")
        );
    }

    #[test]
    fn a_kept_id_with_a_new_physical_name_is_refused_without_a_rename() {
        let id = properties("id", &[]);
        assert!(
            rejected(
                &id,
                false,
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
            rejected(&name, false, in_array("col-x"), in_array("col-z"))
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
            rejected(&name, false, in_map("col-x"), in_map("col-z"))
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
            assert!(check(&p, true, schema("a"), schema("b")).is_ok());
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
            assert!(check(&p, true, schema("x"), schema("y")).is_ok());
        }
    }

    #[test]
    fn a_dropped_field_is_not_supported_with_a_rename() {
        let p = properties("name", &[]);
        assert!(
            unsupported(
                &p,
                true,
                vec![long("a", 1, "col-1"), long("c", 2, "col-c")],
                vec![long("b", 1, "col-1")]
            )
            .contains("dropping field c in a request that renames a column is not supported")
        );
        // A rename onto the name of a dropped column.
        assert!(
            unsupported(
                &p,
                true,
                vec![long("a", 1, "col-1"), long("b", 2, "col-2")],
                vec![long("b", 1, "col-1")]
            )
            .contains("dropping field b")
        );
        let s = |fields| fld("s", strukt(fields), Some(2), Some("col-s"));
        assert!(
            unsupported(
                &p,
                true,
                vec![
                    long("a", 1, "col-1"),
                    s(vec![long("x", 3, "col-x"), long("y", 4, "col-y")])
                ],
                vec![long("b", 1, "col-1"), s(vec![long("y", 4, "col-y")])]
            )
            .contains("dropping field s.x")
        );
    }

    #[test]
    fn a_field_stays_in_its_struct() {
        let p = properties("name", &[]);
        let message = rejected(
            &p,
            true,
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
                true,
                vec![type_change(long("a", 1, "col-1"), "integer", "long")],
                vec![type_change(long("b", 1, "col-1"), "short", "long")]
            )
            .contains("field b changes delta.typeChanges from")
        );
        assert!(
            check(
                &p,
                true,
                vec![type_change(long("a", 1, "col-1"), "integer", "long")],
                vec![type_change(long("b", 1, "col-1"), "integer", "long")]
            )
            .is_ok()
        );
        assert!(
            rejected(
                &p,
                true,
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
                false,
                vec![long("a", 1, "col-1")],
                vec![fld("a", json!("string"), Some(1), Some("col-1"))]
            )
            .is_ok()
        );
        assert!(
            check(
                &properties("id", &[]),
                false,
                vec![long("a", 1, "col-1")],
                vec![long("a", 2, "col-1")]
            )
            .is_ok()
        );
        // A stored column that cannot be read has no identity to compare.
        assert!(
            check(
                &properties("name", &[]),
                false,
                vec![fld("a", json!(12345), Some(1), Some("col-1"))],
                vec![long("b", 1, "col-1")]
            )
            .is_ok()
        );
        // A change of case only runs the check and passes.
        assert!(
            check(
                &properties("name", &[]),
                false,
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
                true,
                vec![long("a", 1, "col-1"), unknown()],
                vec![long("b", 1, "col-1"), long("v", 2, "col-v")]
            )
            .contains("the table schema cannot be read")
        );
        assert!(
            rejected(
                &p,
                true,
                vec![long("a", 1, "col-1")],
                vec![long("b", 1, "col-1"), long("B", 2, "col-2")]
            )
            .contains("the requested schema cannot be read: Duplicate field name")
        );
        assert!(
            check(
                &p,
                false,
                vec![long("a", 1, "col-1"), unknown()],
                vec![long("a", 1, "col-1"), long("v", 2, "col-v")]
            )
            .is_ok()
        );
    }

    #[test]
    fn other_stored_state_does_not_block_a_rename() {
        let rename = |p: &SecurablePropertiesMap, current: Vec<Value>| {
            check(p, true, current, vec![long("b", 1, "col-1")])
        };
        let p = properties(
            "name",
            &[("delta.rowTracking.rowIdHighWaterMark", "not a number")],
        );
        assert!(rename(&p, vec![long("a", 1, "col-1")]).is_ok());
    }

    #[test]
    fn a_type_change_is_not_supported_with_a_rename() {
        let int = || vec![fld("a", json!("integer"), Some(1), Some("col-1"))];
        let double = || vec![fld("b", json!("double"), Some(1), Some("col-1"))];
        let widening = properties("name", &[("delta.enableTypeWidening", "true")]);
        assert!(
            unsupported(&widening, true, int(), double())
                .contains("changing the type of field b from integer to double")
        );
    }
}
