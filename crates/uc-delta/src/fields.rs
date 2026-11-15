//! The struct fields of Delta schemas: upstream `ColumnUtils`' checks and
//! column conversions.
//!
//! A field is JSON in one of two key styles: Spark's camelCase
//! (`elementType`, `containsNull`), which a column's `type_json` and the
//! Delta log use, and the Delta REST API's kebab-case (`element-type`,
//! `contains-null`). [`normalize`] moves a field from one style to the
//! other, and [`check`] gives upstream's message for each missing part. The
//! kernel reads the types and the fields, and all other rules work on its
//! [`StructField`].

use delta_kernel::KernelError;
use delta_kernel::schema::{DataType, MetadataValue, PrimitiveType, StructField, StructType};
use serde::Deserialize;
use serde_json::{Map, Value};
use uc_api::ColumnTypeName;

/// The JSON key style of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStyle {
    /// Spark `StructField` JSON (`elementType`), as a column stores it in
    /// `type_json`.
    Camel,
    /// Delta REST API JSON (`element-type`).
    Kebab,
}

impl KeyStyle {
    fn key(self, camel: &'static str, kebab: &'static str) -> &'static str {
        match self {
            Self::Camel => camel,
            Self::Kebab => kebab,
        }
    }
}

/// The struct type of a request after [`check_fields`].
#[derive(Debug, Clone)]
pub struct CheckedSchema {
    /// The struct type as the kernel reads it.
    pub schema: StructType,
    /// Each top-level field in camelCase, as a column stores it in
    /// `type_json`. It keeps the key order of the request, which the
    /// kernel's metadata map does not.
    pub type_jsons: Vec<Value>,
}

/// The message of a kernel error, without the kernel's prefix for schema
/// and protocol errors.
pub(crate) fn kernel_message(error: KernelError) -> String {
    match error {
        KernelError::Schema(message) | KernelError::InvalidProtocol(message) => message,
        error => error.to_string(),
    }
}

/// Read a field in `from` style and write it in `to` style. Only the known
/// keys stay. A decimal object with a precision and a scale becomes a
/// `decimal(p,s)` string, as upstream `DeltaTypeSerializer` writes it. A
/// missing part stays missing, for [`check`] to name.
pub fn normalize(field: &Value, from: KeyStyle, to: KeyStyle) -> Result<Value, String> {
    let obj = field
        .as_object()
        .ok_or("struct field must be a JSON object")?;
    let mut out = Map::new();
    match obj.get("name") {
        None | Some(Value::Null) => {}
        Some(name @ Value::String(_)) => {
            out.insert("name".into(), name.clone());
        }
        Some(_) => return Err("name must be a string".into()),
    }
    if let Some(t) = obj.get("type").filter(|t| !t.is_null()) {
        out.insert("type".into(), normalize_type(t, from, to)?);
    }
    copy_bool(obj, &mut out, ("nullable", "nullable"), from, to)?;
    match obj.get("metadata") {
        None | Some(Value::Null) => {}
        Some(metadata @ Value::Object(_)) => {
            out.insert("metadata".into(), metadata.clone());
        }
        Some(_) => return Err("metadata must be a JSON object".into()),
    }
    Ok(Value::Object(out))
}

fn normalize_type(t: &Value, from: KeyStyle, to: KeyStyle) -> Result<Value, String> {
    let obj = match t {
        Value::String(_) => return Ok(t.clone()),
        Value::Object(obj) => obj,
        _ => return Err("type must be a string or an object".into()),
    };
    let kind = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or("type object needs a string \"type\" field")?;
    let mut out = Map::new();
    out.insert("type".into(), kind.into());
    match kind {
        "array" => {
            copy_type(obj, &mut out, ("elementType", "element-type"), from, to)?;
            copy_bool(obj, &mut out, ("containsNull", "contains-null"), from, to)?;
        }
        "map" => {
            copy_type(obj, &mut out, ("keyType", "key-type"), from, to)?;
            copy_type(obj, &mut out, ("valueType", "value-type"), from, to)?;
            let key = ("valueContainsNull", "value-contains-null");
            copy_bool(obj, &mut out, key, from, to)?;
        }
        "struct" => match obj.get("fields") {
            None | Some(Value::Null) => {}
            Some(Value::Array(fields)) => {
                let fields = fields
                    .iter()
                    .map(|f| normalize(f, from, to))
                    .collect::<Result<_, _>>()?;
                out.insert("fields".into(), Value::Array(fields));
            }
            Some(_) => return Err("struct fields must be an array".into()),
        },
        "decimal" => {
            let integer = |key: &str| match obj.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => v
                    .as_i64()
                    .map(Some)
                    .ok_or_else(|| format!("{key} must be an integer")),
            };
            match (integer("precision")?, integer("scale")?) {
                (Some(precision), Some(scale)) => {
                    return Ok(format!("decimal({precision},{scale})").into());
                }
                (precision, scale) => {
                    if let Some(precision) = precision {
                        out.insert("precision".into(), precision.into());
                    }
                    if let Some(scale) = scale {
                        out.insert("scale".into(), scale.into());
                    }
                }
            }
        }
        // A primitive in an object, such as `{"type": "long"}`.
        other => return Ok(other.into()),
    }
    Ok(Value::Object(out))
}

/// Copy the nested type under `key`, in `to` style.
fn copy_type(
    obj: &Map<String, Value>,
    out: &mut Map<String, Value>,
    (camel, kebab): (&'static str, &'static str),
    from: KeyStyle,
    to: KeyStyle,
) -> Result<(), String> {
    if let Some(t) = obj.get(from.key(camel, kebab)).filter(|t| !t.is_null()) {
        out.insert(to.key(camel, kebab).into(), normalize_type(t, from, to)?);
    }
    Ok(())
}

/// Copy the flag under `key`, in `to` style.
fn copy_bool(
    obj: &Map<String, Value>,
    out: &mut Map<String, Value>,
    (camel, kebab): (&'static str, &'static str),
    from: KeyStyle,
    to: KeyStyle,
) -> Result<(), String> {
    let key = from.key(camel, kebab);
    match obj.get(key) {
        None | Some(Value::Null) => Ok(()),
        Some(flag @ Value::Bool(_)) => {
            out.insert(to.key(camel, kebab).into(), flag.clone());
            Ok(())
        }
        Some(_) => Err(format!("{key} must be a boolean")),
    }
}

/// Upstream `ColumnUtils.validateStructField`, on a camelCase field from
/// [`normalize`], then the kernel reads it. `path` names the field in
/// messages (`type_json`, `columns.fields[0]`). Paths name nested types in
/// kebab case, as upstream.
pub fn check(field: &Value, path: &str) -> Result<StructField, String> {
    check_parts(field, path)?;
    serde_json::from_value(field.clone()).map_err(|e| format!("{path}: {e}"))
}

/// Upstream `ColumnUtils.validateStructType` for the camelCase fields of a
/// struct at `path`, then the kernel reads them. The kernel refuses two
/// fields whose names differ only in case.
pub fn check_fields(fields: Option<&[Value]>, path: &str) -> Result<StructType, String> {
    check_field_parts(fields, path)?;
    let fields = fields
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, field)| {
            StructField::deserialize(field).map_err(|e| format!("{path}.fields[{i}]: {e}"))
        })
        .collect::<Result<Vec<StructField>, _>>()?;
    StructType::try_new(fields).map_err(|e| format!("{path}: {e}"))
}

fn check_parts(field: &Value, path: &str) -> Result<(), String> {
    let part = |key: &str| field.get(key).filter(|v| !v.is_null());
    if part("name")
        .and_then(Value::as_str)
        .is_none_or(|name| name.trim().is_empty())
    {
        return Err(format!("{path}.name is required."));
    }
    let data_type = part("type").ok_or_else(|| format!("{path}.type is required."))?;
    for key in ["nullable", "metadata"] {
        if part(key).is_none() {
            return Err(format!("{path}.{key} is required."));
        }
    }
    check_type(data_type, &format!("{path}.type"))
}

fn check_field_parts(fields: Option<&[Value]>, path: &str) -> Result<(), String> {
    let fields = fields.ok_or_else(|| format!("{path}.fields is required."))?;
    if fields.is_empty() {
        return Err(format!("{path}.fields must contain at least one field."));
    }
    for (i, field) in fields.iter().enumerate() {
        check_parts(field, &format!("{path}.fields[{i}]"))?;
    }
    Ok(())
}

fn check_type(t: &Value, path: &str) -> Result<(), String> {
    let Some(obj) = t.as_object() else {
        // A type string: the kernel reads it, and UC has a column type for it.
        let data_type: DataType =
            serde_json::from_value(t.clone()).map_err(|e| format!("{path}: {e}"))?;
        return column_type_name(&data_type)
            .map(|_| ())
            .map_err(|e| format!("{path}: {e}"));
    };
    let required = |camel: &str, kebab: &str| {
        obj.get(camel)
            .filter(|v| !v.is_null())
            .ok_or_else(|| format!("{path}.{kebab} is required."))
    };
    match obj.get("type").and_then(Value::as_str) {
        Some("struct") => check_field_parts(
            obj.get("fields")
                .and_then(Value::as_array)
                .map(Vec::as_slice),
            path,
        ),
        Some("array") => {
            let element = required("elementType", "element-type")?;
            required("containsNull", "contains-null")?;
            check_type(element, &format!("{path}.element-type"))
        }
        Some("map") => {
            let key = required("keyType", "key-type")?;
            let value = required("valueType", "value-type")?;
            required("valueContainsNull", "value-contains-null")?;
            check_type(key, &format!("{path}.key-type"))?;
            check_type(value, &format!("{path}.value-type"))
        }
        // A decimal object without its precision or scale.
        _ => {
            required("precision", "precision")?;
            required("scale", "scale")?;
            Ok(())
        }
    }
}

/// Upstream `ColumnUtils.resolveColumnTypeName`: the UC column type of a
/// Delta type.
pub fn column_type_name(t: &DataType) -> Result<ColumnTypeName, String> {
    Ok(match t {
        DataType::Array(_) => ColumnTypeName::Array,
        DataType::Map(_) => ColumnTypeName::Map,
        DataType::Struct(_) => ColumnTypeName::Struct,
        DataType::Variant(_) => ColumnTypeName::Variant,
        DataType::Primitive(p) => match p {
            PrimitiveType::Boolean => ColumnTypeName::Boolean,
            PrimitiveType::Byte => ColumnTypeName::Byte,
            PrimitiveType::Short => ColumnTypeName::Short,
            PrimitiveType::Integer => ColumnTypeName::Int,
            PrimitiveType::Long => ColumnTypeName::Long,
            PrimitiveType::Float => ColumnTypeName::Float,
            PrimitiveType::Double => ColumnTypeName::Double,
            PrimitiveType::Date => ColumnTypeName::Date,
            PrimitiveType::Timestamp => ColumnTypeName::Timestamp,
            PrimitiveType::TimestampNtz => ColumnTypeName::TimestampNtz,
            PrimitiveType::String => ColumnTypeName::String,
            PrimitiveType::Binary => ColumnTypeName::Binary,
            PrimitiveType::Decimal(_) => ColumnTypeName::Decimal,
            other => return Err(format!("Unsupported Delta primitive type: {other}")),
        },
        // Types behind kernel features that UC does not turn on.
        #[allow(unreachable_patterns)]
        other => return Err(format!("Unsupported Delta type: {other}")),
    })
}

/// Upstream `ColumnUtils.toCatalogString`: Spark's `catalogString`
/// (`bigint`, `decimal(10,2)`, `array<int>`, ...).
pub fn catalog_string(t: &DataType) -> Result<String, String> {
    Ok(match t {
        DataType::Array(array) => format!("array<{}>", catalog_string(array.element_type())?),
        DataType::Map(map) => format!(
            "map<{},{}>",
            catalog_string(map.key_type())?,
            catalog_string(map.value_type())?
        ),
        DataType::Struct(fields) => {
            let fields = fields
                .fields()
                .map(|f| Ok(format!("{}:{}", f.name(), catalog_string(f.data_type())?)))
                .collect::<Result<Vec<_>, String>>()?;
            format!("struct<{}>", fields.join(","))
        }
        DataType::Primitive(PrimitiveType::Byte) => "tinyint".into(),
        DataType::Primitive(PrimitiveType::Short) => "smallint".into(),
        DataType::Primitive(PrimitiveType::Integer) => "int".into(),
        DataType::Primitive(PrimitiveType::Long) => "bigint".into(),
        // Spark names the other types as the kernel does.
        t => {
            column_type_name(t)?;
            t.to_string()
        }
    })
}

/// The `comment` entry of the field metadata, if it is a string.
pub fn comment(field: &StructField) -> Option<&str> {
    match field.metadata().get("comment")? {
        MetadataValue::String(comment) => Some(comment),
        _ => None,
    }
}

/// The struct fields directly under a type, through array elements and map
/// keys and values.
pub(crate) fn nested_fields(data_type: &DataType) -> Vec<&StructField> {
    match data_type {
        DataType::Struct(fields) => fields.fields().collect(),
        DataType::Array(array) => nested_fields(array.element_type()),
        DataType::Map(map) => {
            let mut fields = nested_fields(map.key_type());
            fields.extend(nested_fields(map.value_type()));
            fields
        }
        _ => vec![],
    }
}

/// Every struct field at every depth, each before the fields nested in it.
pub(crate) fn all_fields<'a>(
    fields: impl IntoIterator<Item = &'a StructField>,
) -> Vec<&'a StructField> {
    let mut all = vec![];
    for field in fields {
        all.push(field);
        all.extend(all_fields(nested_fields(field.data_type())));
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn camel(v: Value) -> Value {
        normalize(&v, KeyStyle::Camel, KeyStyle::Camel).unwrap()
    }

    fn data_type(v: Value) -> DataType {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn nested_types_move_between_key_styles() {
        let v = json!({
            "name": "m", "nullable": true, "metadata": {},
            "type": { "type": "map", "keyType": "string",
                "valueType": { "type": "array", "elementType": "decimal(10, 2)", "containsNull": false },
                "valueContainsNull": true },
        });
        let field = check(&camel(v.clone()), "type_json").unwrap();
        assert_eq!(
            catalog_string(field.data_type()).unwrap(),
            "map<string,array<decimal(10,2)>>"
        );
        assert_eq!(
            column_type_name(field.data_type()).unwrap(),
            ColumnTypeName::Map
        );
        let kebab = normalize(&v, KeyStyle::Camel, KeyStyle::Kebab).unwrap();
        assert_eq!(
            kebab["type"]["value-type"]["element-type"],
            "decimal(10, 2)"
        );
        assert_eq!(
            normalize(&kebab, KeyStyle::Kebab, KeyStyle::Camel).unwrap(),
            v
        );
    }

    /// Upstream `testRoundtripNestedPreservesCamelCaseAndStructure`, with the
    /// metadata of `testMetadataPreserved`.
    #[test]
    fn camel_case_round_trips() {
        let v = json!({
            "name": "data", "nullable": true,
            "metadata": { "comment": "primary key", "delta.columnMapping.id": 1 },
            "type": { "type": "map", "keyType": "string", "valueContainsNull": true,
                "valueType": { "type": "array", "containsNull": true,
                    "elementType": { "type": "struct", "fields": [
                        { "name": "v", "type": "double", "nullable": false, "metadata": {} } ] } } },
        });
        assert_eq!(camel(v.clone()), v);
        let field = check(&v, "type_json").unwrap();
        assert_eq!(comment(&field), Some("primary key"));
        assert_eq!(
            catalog_string(field.data_type()).unwrap(),
            "map<string,array<struct<v:double>>>"
        );
    }

    #[test]
    fn decimal_objects_become_strings() {
        // Upstream `DeltaTypeSerializer` writes decimals as strings, also
        // decimals that were sent as objects.
        let v = json!({ "name": "d", "nullable": true, "metadata": {},
            "type": { "type": "decimal", "precision": 10, "scale": 2 } });
        assert_eq!(
            normalize(&v, KeyStyle::Kebab, KeyStyle::Camel).unwrap(),
            json!({ "name": "d", "type": "decimal(10,2)", "nullable": true, "metadata": {} })
        );
        // An incomplete decimal stays an object, and unknown keys go.
        let incomplete = json!({ "name": "d", "type": { "type": "decimal", "scale": 2 }, "x": 1 });
        assert_eq!(
            camel(incomplete),
            json!({ "name": "d", "type": { "type": "decimal", "scale": 2 } })
        );
    }

    #[test]
    fn comment_is_the_metadata_string() {
        let field = |metadata: Value| {
            serde_json::from_value::<StructField>(
                json!({ "name": "c", "type": "long", "nullable": true, "metadata": metadata }),
            )
            .unwrap()
        };
        assert_eq!(comment(&field(json!({ "comment": "c" }))), Some("c"));
        for metadata in [json!({}), json!({ "comment": 1 })] {
            assert_eq!(comment(&field(metadata)), None);
        }
    }

    #[test]
    fn malformed_documents_are_refused() {
        let cases = [
            (json!("long"), "struct field must be a JSON object"),
            (json!({ "name": 1 }), "name must be a string"),
            (json!({ "nullable": "true" }), "nullable must be a boolean"),
            (json!({ "metadata": [] }), "metadata must be a JSON object"),
            (json!({ "type": 1 }), "type must be a string or an object"),
            (
                json!({ "type": { "elementType": "long" } }),
                "type object needs a string \"type\" field",
            ),
            (
                json!({ "type": { "type": "struct", "fields": {} } }),
                "struct fields must be an array",
            ),
            (
                json!({ "type": { "type": "array", "containsNull": 1 } }),
                "containsNull must be a boolean",
            ),
            (
                json!({ "type": { "type": "decimal", "precision": "10" } }),
                "precision must be an integer",
            ),
            (
                json!({ "type": { "type": "map", "keyType": ["string"] } }),
                "type must be a string or an object",
            ),
            (
                json!({ "type": { "type": "struct", "fields": [{ "name": false }] } }),
                "name must be a string",
            ),
        ];
        for (v, message) in cases {
            assert_eq!(
                normalize(&v, KeyStyle::Camel, KeyStyle::Camel).unwrap_err(),
                message,
                "{v}"
            );
        }
        let v = json!({ "type": { "type": "array", "contains-null": "no" } });
        assert_eq!(
            normalize(&v, KeyStyle::Kebab, KeyStyle::Camel).unwrap_err(),
            "contains-null must be a boolean"
        );
    }

    #[test]
    fn checks_follow_upstream_and_the_kernel() {
        let ok = json!({ "name": "c", "type": "long", "nullable": true, "metadata": {} });
        let with = |key: &str, value: Value| {
            let mut v = ok.clone();
            v[key] = value;
            v
        };
        let nested = json!({ "type": "map", "keyType": "string", "valueContainsNull": true,
            "valueType": { "type": "array", "containsNull": true,
                "elementType": { "type": "struct", "fields": [with("nullable", Value::Null)] } } });
        let cases = [
            (with("name", Value::Null), "type_json.name is required."),
            (with("name", json!(" ")), "type_json.name is required."),
            (with("type", Value::Null), "type_json.type is required."),
            (
                with("nullable", Value::Null),
                "type_json.nullable is required.",
            ),
            (
                json!({ "name": "c", "type": "long", "nullable": true }),
                "type_json.metadata is required.",
            ),
            (
                with("type", json!("int")),
                "type_json.type: Unsupported Delta table type: 'int'",
            ),
            (
                with("type", json!("void")),
                "type_json.type: Unsupported Delta primitive type: void",
            ),
            (
                with("type", json!({ "type": "decimal", "scale": 2 })),
                "type_json.type.precision is required.",
            ),
            (
                with("type", json!({ "type": "decimal", "precision": 10 })),
                "type_json.type.scale is required.",
            ),
            (
                with("type", json!("decimal(40,2)")),
                "type_json.type: Invalid decimal: precision must be in range 1..38 inclusive, \
                 found: 40.",
            ),
            (
                with(
                    "type",
                    json!({ "type": "decimal", "precision": -1, "scale": 0 }),
                ),
                "type_json.type: Invalid precision in decimal: decimal(-1,0)",
            ),
            (
                with("type", json!("decimal(3,5)")),
                "type_json.type: Invalid decimal: scale must be in range 0..3 inclusive, found: 5.",
            ),
            (
                with("type", json!({ "type": "array", "containsNull": true })),
                "type_json.type.element-type is required.",
            ),
            (
                with("type", json!({ "type": "array", "elementType": "long" })),
                "type_json.type.contains-null is required.",
            ),
            (
                with(
                    "type",
                    json!({ "type": "map", "valueType": "long", "valueContainsNull": true }),
                ),
                "type_json.type.key-type is required.",
            ),
            (
                with(
                    "type",
                    json!({ "type": "map", "keyType": "long", "valueContainsNull": true }),
                ),
                "type_json.type.value-type is required.",
            ),
            (
                with(
                    "type",
                    json!({ "type": "map", "keyType": "long", "valueType": "long" }),
                ),
                "type_json.type.value-contains-null is required.",
            ),
            (
                with("type", json!({ "type": "struct" })),
                "type_json.type.fields is required.",
            ),
            (
                with("type", json!({ "type": "struct", "fields": [] })),
                "type_json.type.fields must contain at least one field.",
            ),
            (
                with(
                    "type",
                    json!({ "type": "struct", "fields": [ok.clone(), with("name", json!("C"))] }),
                ),
                "type_json: Schema error: Duplicate field name (case-insensitive): 'C'",
            ),
            // Paths name nested types in kebab case, also in camel-case
            // documents.
            (
                with("type", nested),
                "type_json.type.value-type.element-type.fields[0].nullable is required.",
            ),
        ];
        for (v, message) in cases {
            assert_eq!(check(&camel(v), "type_json").unwrap_err(), message);
        }
        let fields = [ok.clone(), with("name", json!("C"))];
        assert_eq!(
            check_fields(Some(&fields), "columns").unwrap_err(),
            "columns: Schema error: Duplicate field name (case-insensitive): 'C'"
        );
    }

    #[test]
    fn decimal_bounds_are_the_kernel_s() {
        for t in [
            "decimal(1,0)",
            "decimal(38,0)",
            "decimal(38,38)",
            "decimal(10,10)",
        ] {
            let v = json!({ "name": "d", "type": t, "nullable": true, "metadata": {} });
            assert!(check(&v, "type_json").is_ok(), "{t}");
        }
        let v = json!({ "name": "d", "type": "decimal(0,0)", "nullable": true, "metadata": {} });
        assert_eq!(
            check(&v, "type_json").unwrap_err(),
            "type_json.type: Invalid decimal: precision must be in range 1..38 inclusive, found: 0."
        );
        let fields = [json!({ "name": "id", "type": "long", "nullable": false, "metadata": {} })];
        assert_eq!(
            check_fields(Some(&fields), "columns").unwrap().num_fields(),
            1
        );
    }

    #[test]
    fn primitives_follow_upstream() {
        // Upstream `resolveColumnTypeName` and `typeNameVsTypeText`.
        let expected = [
            ("boolean", "BOOLEAN", "boolean"),
            ("byte", "BYTE", "tinyint"),
            ("short", "SHORT", "smallint"),
            ("integer", "INT", "int"),
            ("long", "LONG", "bigint"),
            ("float", "FLOAT", "float"),
            ("double", "DOUBLE", "double"),
            ("date", "DATE", "date"),
            ("timestamp", "TIMESTAMP", "timestamp"),
            ("timestamp_ntz", "TIMESTAMP_NTZ", "timestamp_ntz"),
            ("string", "STRING", "string"),
            ("binary", "BINARY", "binary"),
            ("variant", "VARIANT", "variant"),
        ];
        for (primitive, type_name, catalog) in expected {
            let t = data_type(json!(primitive));
            assert_eq!(json!(column_type_name(&t).unwrap()), json!(type_name));
            assert_eq!(catalog_string(&t).unwrap(), catalog);
        }
    }

    #[test]
    fn composites_follow_upstream() {
        // Upstream `testToColumnInfoComplex`.
        let cases = [
            (json!("decimal(10,2)"), "DECIMAL", "decimal(10,2)"),
            (
                json!({ "type": "array", "elementType": "string", "containsNull": true }),
                "ARRAY",
                "array<string>",
            ),
            (
                json!({ "type": "map", "keyType": "string", "valueType": "double" }),
                "MAP",
                "map<string,double>",
            ),
            (
                json!({ "type": "struct", "fields": [
                    { "name": "zip", "type": "integer", "nullable": true, "metadata": {} },
                    { "name": "city", "type": "string", "nullable": true, "metadata": {} } ] }),
                "STRUCT",
                "struct<zip:int,city:string>",
            ),
            (
                json!({ "type": "struct", "fields": [] }),
                "STRUCT",
                "struct<>",
            ),
        ];
        for (v, type_name, catalog) in cases {
            let t = data_type(v);
            assert_eq!(json!(column_type_name(&t).unwrap()), json!(type_name));
            assert_eq!(catalog_string(&t).unwrap(), catalog);
        }
        let void =
            data_type(json!({ "type": "array", "elementType": "void", "containsNull": true }));
        assert_eq!(
            catalog_string(&void).unwrap_err(),
            "Unsupported Delta primitive type: void"
        );
    }
}
