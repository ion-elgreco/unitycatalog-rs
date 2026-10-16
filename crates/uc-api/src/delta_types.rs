//! Delta / Spark data types — upstream `DeltaDataType`, `DeltaStructField`
//! and the checks in `ColumnUtils`.
//!
//! The same types travel in two JSON dialects: Spark's camelCase
//! (`elementType`, `containsNull`; used for a column's `type_json`) and the
//! Delta REST API's kebab-case (`element-type`, `contains-null`). A type is
//! either a string (`"long"`, `"decimal(10,2)"`) or an object with a `type`
//! discriminator (`array`, `map`, `struct`).

use serde_json::{Map, Value};

/// The JSON key style of a type document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStyle {
    /// Spark `StructField` JSON (`elementType`), as stored in `type_json`.
    Camel,
    /// UC Delta REST API JSON (`element-type`).
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

#[derive(Debug, Clone, PartialEq)]
pub enum DeltaDataType {
    /// A primitive such as `long` or `timestamp_ntz`.
    Primitive(String),
    /// A `decimal` object may leave out its precision or scale; the checks
    /// then name the missing key, as upstream.
    Decimal {
        precision: Option<i64>,
        scale: Option<i64>,
    },
    Array {
        element_type: Option<Box<DeltaDataType>>,
        contains_null: Option<bool>,
    },
    Map {
        key_type: Option<Box<DeltaDataType>>,
        value_type: Option<Box<DeltaDataType>>,
        value_contains_null: Option<bool>,
    },
    Struct {
        fields: Option<Vec<DeltaStructField>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeltaStructField {
    pub name: Option<String>,
    pub data_type: Option<DeltaDataType>,
    pub nullable: Option<bool>,
    pub metadata: Option<Map<String, Value>>,
}

/// Primitive names of the Delta protocol (Kernel's primitive registry).
pub const DELTA_PRIMITIVES: &[&str] = &[
    "boolean",
    "byte",
    "short",
    "integer",
    "long",
    "float",
    "double",
    "date",
    "timestamp",
    "timestamp_ntz",
    "string",
    "binary",
    "variant",
];

impl DeltaStructField {
    /// Parse a struct field document.
    pub fn from_json(v: &Value, style: KeyStyle) -> Result<Self, String> {
        let obj = v.as_object().ok_or("struct field must be a JSON object")?;
        Ok(Self {
            name: opt_str(obj, "name")?,
            data_type: obj
                .get("type")
                .filter(|t| !t.is_null())
                .map(|t| DeltaDataType::from_json(t, style))
                .transpose()?,
            nullable: opt_bool(obj, "nullable")?,
            metadata: match obj.get("metadata") {
                None | Some(Value::Null) => None,
                Some(Value::Object(m)) => Some(m.clone()),
                Some(_) => return Err("metadata must be a JSON object".into()),
            },
        })
    }

    /// Serialize as JSON in the given key style.
    pub fn to_json(&self, style: KeyStyle) -> Value {
        let mut obj = Map::new();
        if let Some(n) = &self.name {
            obj.insert("name".into(), Value::String(n.clone()));
        }
        if let Some(t) = &self.data_type {
            obj.insert("type".into(), t.to_json(style));
        }
        if let Some(n) = self.nullable {
            obj.insert("nullable".into(), Value::Bool(n));
        }
        obj.insert(
            "metadata".into(),
            Value::Object(self.metadata.clone().unwrap_or_default()),
        );
        Value::Object(obj)
    }

    /// The `comment` entry of the field metadata, if any.
    pub fn comment(&self) -> Option<&str> {
        self.metadata.as_ref()?.get("comment")?.as_str()
    }
}

impl DeltaDataType {
    /// Parse a type: a string (primitive or `decimal(p,s)`) or an object with
    /// a `type` discriminator.
    pub fn from_json(v: &Value, style: KeyStyle) -> Result<Self, String> {
        match v {
            Value::String(s) => Ok(parse_type_string(s)),
            Value::Object(obj) => {
                let kind = obj
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or("type object needs a string \"type\" field")?;
                let sub = |camel, kebab| -> Result<Option<Box<DeltaDataType>>, String> {
                    match obj.get(style.key(camel, kebab)) {
                        None | Some(Value::Null) => Ok(None),
                        Some(t) => Ok(Some(Box::new(Self::from_json(t, style)?))),
                    }
                };
                match kind {
                    "array" => Ok(Self::Array {
                        element_type: sub("elementType", "element-type")?,
                        contains_null: opt_bool(obj, style.key("containsNull", "contains-null"))?,
                    }),
                    "map" => Ok(Self::Map {
                        key_type: sub("keyType", "key-type")?,
                        value_type: sub("valueType", "value-type")?,
                        value_contains_null: opt_bool(
                            obj,
                            style.key("valueContainsNull", "value-contains-null"),
                        )?,
                    }),
                    "struct" => Ok(Self::Struct {
                        fields: match obj.get("fields") {
                            None | Some(Value::Null) => None,
                            Some(Value::Array(items)) => Some(
                                items
                                    .iter()
                                    .map(|f| DeltaStructField::from_json(f, style))
                                    .collect::<Result<_, _>>()?,
                            ),
                            Some(_) => return Err("struct fields must be an array".into()),
                        },
                    }),
                    "decimal" => Ok(Self::Decimal {
                        precision: opt_i64(obj, "precision")?,
                        scale: opt_i64(obj, "scale")?,
                    }),
                    other => Ok(parse_type_string(other)),
                }
            }
            _ => Err("type must be a string or an object".into()),
        }
    }

    /// Serialize as JSON in the given key style.
    pub fn to_json(&self, style: KeyStyle) -> Value {
        match self {
            Self::Primitive(p) => Value::String(p.clone()),
            Self::Decimal {
                precision: Some(precision),
                scale: Some(scale),
            } => Value::String(format!("decimal({precision},{scale})")),
            // An incomplete decimal stays an object.
            Self::Decimal { precision, scale } => {
                let mut o = Map::new();
                o.insert("type".into(), "decimal".into());
                if let Some(p) = precision {
                    o.insert("precision".into(), (*p).into());
                }
                if let Some(s) = scale {
                    o.insert("scale".into(), (*s).into());
                }
                Value::Object(o)
            }
            Self::Array {
                element_type,
                contains_null,
            } => {
                let mut o = Map::new();
                o.insert("type".into(), "array".into());
                if let Some(t) = element_type {
                    o.insert(
                        style.key("elementType", "element-type").into(),
                        t.to_json(style),
                    );
                }
                if let Some(c) = contains_null {
                    o.insert(
                        style.key("containsNull", "contains-null").into(),
                        (*c).into(),
                    );
                }
                Value::Object(o)
            }
            Self::Map {
                key_type,
                value_type,
                value_contains_null,
            } => {
                let mut o = Map::new();
                o.insert("type".into(), "map".into());
                if let Some(t) = key_type {
                    o.insert(style.key("keyType", "key-type").into(), t.to_json(style));
                }
                if let Some(t) = value_type {
                    o.insert(
                        style.key("valueType", "value-type").into(),
                        t.to_json(style),
                    );
                }
                if let Some(c) = value_contains_null {
                    o.insert(
                        style.key("valueContainsNull", "value-contains-null").into(),
                        (*c).into(),
                    );
                }
                Value::Object(o)
            }
            Self::Struct { fields } => {
                let mut o = Map::new();
                o.insert("type".into(), "struct".into());
                o.insert(
                    "fields".into(),
                    Value::Array(fields.iter().flatten().map(|f| f.to_json(style)).collect()),
                );
                Value::Object(o)
            }
        }
    }

    /// Upstream `ColumnUtils.toCatalogString`: Spark's `catalogString`
    /// (`bigint`, `decimal(10,2)`, `array<int>`, ...).
    pub fn catalog_string(&self) -> Result<String, String> {
        Ok(match self {
            Self::Decimal { precision, scale } => format!(
                "decimal({},{})",
                precision.ok_or("decimal precision missing")?,
                scale.ok_or("decimal scale missing")?
            ),
            Self::Array { element_type, .. } => format!(
                "array<{}>",
                element_type
                    .as_ref()
                    .ok_or("array element type missing")?
                    .catalog_string()?
            ),
            Self::Map {
                key_type,
                value_type,
                ..
            } => format!(
                "map<{},{}>",
                key_type
                    .as_ref()
                    .ok_or("map key type missing")?
                    .catalog_string()?,
                value_type
                    .as_ref()
                    .ok_or("map value type missing")?
                    .catalog_string()?
            ),
            Self::Struct { fields } => {
                let parts: Vec<String> = fields
                    .iter()
                    .flatten()
                    .map(|f| {
                        let t = f.data_type.as_ref().ok_or("struct field type missing")?;
                        Ok::<_, String>(format!(
                            "{}:{}",
                            f.name.as_deref().unwrap_or_default(),
                            t.catalog_string()?
                        ))
                    })
                    .collect::<Result<_, _>>()?;
                format!("struct<{}>", parts.join(","))
            }
            Self::Primitive(_) => match column_type_name(self)? {
                "LONG" => "bigint".to_string(),
                "SHORT" => "smallint".to_string(),
                "BYTE" => "tinyint".to_string(),
                name => name.to_ascii_lowercase(),
            },
        })
    }
}

/// Upstream `ColumnUtils.resolveColumnTypeName`: the UC `ColumnTypeName`
/// wire name for a Delta type.
pub fn column_type_name(t: &DeltaDataType) -> Result<&'static str, String> {
    Ok(match t {
        DeltaDataType::Array { .. } => "ARRAY",
        DeltaDataType::Decimal { .. } => "DECIMAL",
        DeltaDataType::Map { .. } => "MAP",
        DeltaDataType::Struct { .. } => "STRUCT",
        DeltaDataType::Primitive(p) => match p.as_str() {
            "boolean" => "BOOLEAN",
            "byte" => "BYTE",
            "short" => "SHORT",
            "integer" => "INT",
            "long" => "LONG",
            "float" => "FLOAT",
            "double" => "DOUBLE",
            "date" => "DATE",
            "timestamp" => "TIMESTAMP",
            "timestamp_ntz" => "TIMESTAMP_NTZ",
            "string" => "STRING",
            "binary" => "BINARY",
            "variant" => "VARIANT",
            other => return Err(format!("Unsupported Delta primitive type: {other}")),
        },
    })
}

/// Upstream `ColumnUtils.validateStructField`, with its messages. `path`
/// names the field in messages (`type_json`, `columns.fields[0]`).
pub fn validate_struct_field(field: &DeltaStructField, path: &str) -> Result<(), String> {
    if field.name.as_deref().is_none_or(|n| n.trim().is_empty()) {
        return Err(format!("{path}.name is required."));
    }
    let data_type = field
        .data_type
        .as_ref()
        .ok_or_else(|| format!("{path}.type is required."))?;
    if field.nullable.is_none() {
        return Err(format!("{path}.nullable is required."));
    }
    if field.metadata.is_none() {
        return Err(format!("{path}.metadata is required."));
    }
    validate_type(data_type, &format!("{path}.type"))
}

/// Upstream `ColumnUtils.validateStructType` for a list of fields at `path`.
pub fn validate_struct_fields(
    fields: Option<&[DeltaStructField]>,
    path: &str,
) -> Result<(), String> {
    let fields = fields.ok_or_else(|| format!("{path}.fields is required."))?;
    if fields.is_empty() {
        return Err(format!("{path}.fields must contain at least one field."));
    }
    let mut seen = std::collections::HashSet::new();
    for (i, field) in fields.iter().enumerate() {
        let field_path = format!("{path}.fields[{i}]");
        validate_struct_field(field, &field_path)?;
        let name = field.name.as_deref().unwrap_or_default();
        if !seen.insert(name.to_lowercase()) {
            return Err(format!(
                "Duplicate field name (case-insensitive) in {field_path}: {name}"
            ));
        }
    }
    Ok(())
}

fn validate_type(t: &DeltaDataType, path: &str) -> Result<(), String> {
    match t {
        DeltaDataType::Struct { fields } => validate_struct_fields(fields.as_deref(), path),
        DeltaDataType::Array {
            element_type,
            contains_null,
        } => {
            let element = element_type
                .as_ref()
                .ok_or_else(|| format!("{path}.element-type is required."))?;
            if contains_null.is_none() {
                return Err(format!("{path}.contains-null is required."));
            }
            validate_type(element, &format!("{path}.element-type"))
        }
        DeltaDataType::Map {
            key_type,
            value_type,
            value_contains_null,
        } => {
            let key = key_type
                .as_ref()
                .ok_or_else(|| format!("{path}.key-type is required."))?;
            let value = value_type
                .as_ref()
                .ok_or_else(|| format!("{path}.value-type is required."))?;
            if value_contains_null.is_none() {
                return Err(format!("{path}.value-contains-null is required."));
            }
            validate_type(key, &format!("{path}.key-type"))?;
            validate_type(value, &format!("{path}.value-type"))
        }
        DeltaDataType::Decimal { precision, scale } => {
            let precision = precision.ok_or_else(|| format!("{path}.precision is required."))?;
            let scale = scale.ok_or_else(|| format!("{path}.scale is required."))?;
            if !(0..=38).contains(&precision) {
                return Err(format!(
                    "{path}.precision must be in [0, 38], got {precision}."
                ));
            }
            if scale < 0 || scale > precision {
                return Err(format!(
                    "{path}.scale must be in [0, precision={precision}], got {scale}."
                ));
            }
            Ok(())
        }
        DeltaDataType::Primitive(_) => column_type_name(t)
            .map(|_| ())
            .map_err(|e| format!("{path}: {e}")),
    }
}

fn parse_type_string(s: &str) -> DeltaDataType {
    // `decimal(p,s)` (Java `decimal\((\d+),\s*(\d+)\)`).
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if let Some(inner) = s.strip_prefix("decimal(").and_then(|r| r.strip_suffix(')'))
        && let Some((p, sc)) = inner.split_once(',')
    {
        let sc = sc.trim_start();
        if digits(p)
            && digits(sc)
            && let (Ok(precision), Ok(scale)) = (p.parse(), sc.parse())
        {
            return DeltaDataType::Decimal {
                precision: Some(precision),
                scale: Some(scale),
            };
        }
    }
    DeltaDataType::Primitive(s.to_string())
}

fn opt_str(obj: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

fn opt_bool(obj: &Map<String, Value>, key: &str) -> Result<Option<bool>, String> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(format!("{key} must be a boolean")),
    }
}

fn opt_i64(obj: &Map<String, Value>, key: &str) -> Result<Option<i64>, String> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("{key} must be an integer")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn field(v: Value) -> DeltaStructField {
        DeltaStructField::from_json(&v, KeyStyle::Camel).unwrap()
    }

    #[test]
    fn parses_and_renders_nested_types() {
        let f = field(json!({
            "name": "m", "nullable": true, "metadata": {},
            "type": { "type": "map", "keyType": "string",
                "valueType": { "type": "array", "elementType": "decimal(10, 2)", "containsNull": false },
                "valueContainsNull": true },
        }));
        assert!(validate_struct_field(&f, "type_json").is_ok());
        let t = f.data_type.as_ref().unwrap();
        assert_eq!(
            t.catalog_string().unwrap(),
            "map<string,array<decimal(10,2)>>"
        );
        assert_eq!(column_type_name(t).unwrap(), "MAP");
        // Kebab round trip.
        let kebab = f.to_json(KeyStyle::Kebab);
        assert_eq!(kebab["type"]["value-type"]["element-type"], "decimal(10,2)");
        assert_eq!(
            DeltaStructField::from_json(&kebab, KeyStyle::Kebab).unwrap(),
            f
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
        let f = field(v.clone());
        assert_eq!(f.to_json(KeyStyle::Camel), v);
        assert_eq!(f.comment(), Some("primary key"));
        assert_eq!(
            f.data_type.unwrap().catalog_string().unwrap(),
            "map<string,array<struct<v:double>>>"
        );
    }

    #[test]
    fn writes_decimals_as_strings_and_always_writes_metadata() {
        // Upstream `DeltaTypeSerializer` writes decimals as strings, also
        // decimals that were sent as objects.
        let v = json!({ "name": "d", "nullable": true,
            "type": { "type": "decimal", "precision": 10, "scale": 2 } });
        let f = DeltaStructField::from_json(&v, KeyStyle::Kebab).unwrap();
        assert_eq!(
            f.data_type,
            Some(DeltaDataType::Decimal {
                precision: Some(10),
                scale: Some(2)
            })
        );
        assert_eq!(
            f.to_json(KeyStyle::Kebab),
            json!({ "name": "d", "type": "decimal(10,2)", "nullable": true, "metadata": {} })
        );
        // An incomplete decimal stays an object.
        let incomplete = json!({ "type": "decimal", "scale": 2 });
        let t = DeltaDataType::from_json(&incomplete, KeyStyle::Kebab).unwrap();
        assert_eq!(t.to_json(KeyStyle::Kebab), incomplete);
        let no_fields = DeltaDataType::Struct { fields: None };
        assert_eq!(
            no_fields.to_json(KeyStyle::Camel),
            json!({ "type": "struct", "fields": [] })
        );
    }

    #[test]
    fn decimal_strings_follow_the_upstream_pattern() {
        // Upstream `decimal\((\d+),\s*(\d+)\)`. Other spellings are
        // primitives, which the checks reject.
        let decimal = |precision, scale| DeltaDataType::Decimal {
            precision: Some(precision),
            scale: Some(scale),
        };
        assert_eq!(parse_type_string("decimal(10,2)"), decimal(10, 2));
        assert_eq!(parse_type_string("decimal(38,  0)"), decimal(38, 0));
        for s in [
            "decimal",
            "decimal(10)",
            "decimal( 10,2)",
            "decimal(10,2 )",
            "decimal(-1,0)",
            "decimal(1e1,0)",
            "decimal(10,2))",
            "DECIMAL(10,2)",
        ] {
            assert_eq!(
                parse_type_string(s),
                DeltaDataType::Primitive(s.into()),
                "{s}"
            );
        }
    }

    #[test]
    fn comment_is_the_metadata_string() {
        assert_eq!(
            field(json!({ "metadata": { "comment": "c" } })).comment(),
            Some("c")
        );
        for metadata in [json!({}), json!({ "comment": 1 }), Value::Null] {
            assert_eq!(field(json!({ "metadata": metadata })).comment(), None);
        }
    }

    #[test]
    fn rejects_malformed_documents() {
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
                DeltaStructField::from_json(&v, KeyStyle::Camel).unwrap_err(),
                message,
                "{v}"
            );
        }
        let v = json!({ "type": { "type": "array", "contains-null": "no" } });
        assert_eq!(
            DeltaStructField::from_json(&v, KeyStyle::Kebab).unwrap_err(),
            "contains-null must be a boolean"
        );
    }

    #[test]
    fn checks_follow_upstream() {
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
                "type_json.type: Unsupported Delta primitive type: int",
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
                "type_json.type.precision must be in [0, 38], got 40.",
            ),
            (
                with(
                    "type",
                    json!({ "type": "decimal", "precision": -1, "scale": 0 }),
                ),
                "type_json.type.precision must be in [0, 38], got -1.",
            ),
            (
                with("type", json!("decimal(3,5)")),
                "type_json.type.scale must be in [0, precision=3], got 5.",
            ),
            (
                with(
                    "type",
                    json!({ "type": "decimal", "precision": 10, "scale": -1 }),
                ),
                "type_json.type.scale must be in [0, precision=10], got -1.",
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
                "Duplicate field name (case-insensitive) in type_json.type.fields[1]: C",
            ),
            // Paths name nested types in kebab case, also in camel-case
            // documents.
            (
                with("type", nested),
                "type_json.type.value-type.element-type.fields[0].nullable is required.",
            ),
        ];
        for (v, message) in cases {
            assert_eq!(
                validate_struct_field(&field(v), "type_json").unwrap_err(),
                message
            );
        }
    }

    #[test]
    fn accepts_upstream_boundaries() {
        // Upstream `testValidateStructTypeAcceptsDecimalBoundary`.
        for t in [
            "decimal(0,0)",
            "decimal(38,0)",
            "decimal(38,38)",
            "decimal(10,10)",
        ] {
            let f = field(json!({ "name": "d", "type": t, "nullable": true, "metadata": {} }));
            assert_eq!(validate_struct_field(&f, "type_json"), Ok(()), "{t}");
        }
        let fields = [field(
            json!({ "name": "id", "type": "long", "nullable": false, "metadata": {} }),
        )];
        assert_eq!(validate_struct_fields(Some(&fields), "columns"), Ok(()));
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
        let primitives: Vec<&str> = expected.iter().map(|(p, ..)| *p).collect();
        assert_eq!(DELTA_PRIMITIVES, primitives);
        for (primitive, type_name, catalog_string) in expected {
            let t = parse_type_string(primitive);
            assert_eq!(column_type_name(&t).unwrap(), type_name);
            // Also a `ColumnTypeName` on the wire.
            serde_json::from_value::<crate::ColumnTypeName>(json!(type_name)).unwrap();
            assert_eq!(t.catalog_string().unwrap(), catalog_string);
        }
    }

    #[test]
    fn composites_follow_upstream() {
        // Upstream `testToColumnInfoComplex`.
        let parse = |v: Value| DeltaDataType::from_json(&v, KeyStyle::Camel).unwrap();
        let cases = [
            (json!("decimal(10,2)"), "DECIMAL", "decimal(10,2)"),
            (
                json!({ "type": "array", "elementType": "string" }),
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
                    { "name": "zip", "type": "integer" }, { "name": "city", "type": "string" } ] }),
                "STRUCT",
                "struct<zip:int,city:string>",
            ),
            (json!({ "type": "struct" }), "STRUCT", "struct<>"),
        ];
        for (v, type_name, catalog_string) in cases {
            let t = parse(v);
            assert_eq!(column_type_name(&t).unwrap(), type_name);
            assert_eq!(t.catalog_string().unwrap(), catalog_string);
        }
        let errors = [
            (json!("void"), "Unsupported Delta primitive type: void"),
            (
                json!({ "type": "array", "elementType": "hyperdecimal" }),
                "Unsupported Delta primitive type: hyperdecimal",
            ),
            (
                json!({ "type": "decimal", "scale": 2 }),
                "decimal precision missing",
            ),
            (
                json!({ "type": "decimal", "precision": 10 }),
                "decimal scale missing",
            ),
            (json!({ "type": "array" }), "array element type missing"),
            (
                json!({ "type": "map", "valueType": "long" }),
                "map key type missing",
            ),
            (
                json!({ "type": "map", "keyType": "long" }),
                "map value type missing",
            ),
            (
                json!({ "type": "struct", "fields": [{ "name": "a" }] }),
                "struct field type missing",
            ),
        ];
        for (v, message) in errors {
            assert_eq!(parse(v).catalog_string().unwrap_err(), message);
        }
    }
}
