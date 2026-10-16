//! Iceberg schemas as UC columns, so that tables made through the Iceberg
//! REST catalog are browsable through the UC API. Upstream
//! `IcebergSchemaConverter`: the metadata file stays the source of truth;
//! `type_json` takes the Spark `StructField` shape UC stores for every
//! format.

use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};
use serde_json::{Map, json};

use uc_api::{
    ColumnInfo,
    delta_types::{DeltaDataType, DeltaStructField},
};
use uc_common::{Error, Result};

/// The UC columns of `schema`'s top-level fields, as Delta's columns:
/// Spark's catalog strings, type names and `StructField` JSON.
pub fn to_column_infos(schema: &Schema) -> Result<Vec<ColumnInfo>> {
    let mut columns = Vec::new();
    for (position, field) in schema.as_struct().fields().iter().enumerate() {
        let mut column = uc_delta::table::column_info(&struct_field(field)?, position as i32)?;
        if let Type::Primitive(PrimitiveType::Decimal { precision, scale }) =
            field.field_type.as_ref()
        {
            column.type_precision = Some(*precision as i32);
            column.type_scale = Some(*scale as i32);
        }
        columns.push(column);
    }
    Ok(columns)
}

fn unsupported(primitive: &PrimitiveType) -> Error {
    Error::invalid_argument(format!(
        "Iceberg type {} is not supported by Unity Catalog",
        primitive.to_string().to_uppercase()
    ))
}

/// An Iceberg field as a Spark `StructField`, its doc as the comment.
fn struct_field(field: &NestedField) -> Result<DeltaStructField> {
    Ok(DeltaStructField {
        name: Some(field.name.clone()),
        data_type: Some(data_type(&field.field_type)?),
        nullable: Some(!field.required),
        metadata: field
            .doc
            .as_ref()
            .map(|doc| Map::from_iter([("comment".to_string(), json!(doc))])),
    })
}

fn data_type(field_type: &Type) -> Result<DeltaDataType> {
    let primitive = |name: &str| DeltaDataType::Primitive(name.to_string());
    Ok(match field_type {
        Type::Struct(struct_type) => DeltaDataType::Struct {
            fields: Some(
                struct_type
                    .fields()
                    .iter()
                    .map(|f| struct_field(f))
                    .collect::<Result<_>>()?,
            ),
        },
        Type::List(list) => DeltaDataType::Array {
            element_type: Some(Box::new(data_type(&list.element_field.field_type)?)),
            contains_null: Some(!list.element_field.required),
        },
        Type::Map(map) => DeltaDataType::Map {
            key_type: Some(Box::new(data_type(&map.key_field.field_type)?)),
            value_type: Some(Box::new(data_type(&map.value_field.field_type)?)),
            value_contains_null: Some(!map.value_field.required),
        },
        Type::Primitive(p) => match p {
            PrimitiveType::Boolean => primitive("boolean"),
            PrimitiveType::Int => primitive("integer"),
            PrimitiveType::Long => primitive("long"),
            PrimitiveType::Float => primitive("float"),
            PrimitiveType::Double => primitive("double"),
            PrimitiveType::Date => primitive("date"),
            PrimitiveType::Timestamptz => primitive("timestamp"),
            PrimitiveType::Timestamp => primitive("timestamp_ntz"),
            PrimitiveType::String | PrimitiveType::Uuid => primitive("string"),
            PrimitiveType::Fixed(_) | PrimitiveType::Binary => primitive("binary"),
            PrimitiveType::Decimal { precision, scale } => DeltaDataType::Decimal {
                precision: Some(i64::from(*precision)),
                scale: Some(i64::from(*scale)),
            },
            PrimitiveType::Time | PrimitiveType::TimestampNs | PrimitiveType::TimestamptzNs => {
                return Err(unsupported(p));
            }
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use uc_api::ColumnTypeName;

    #[test]
    fn columns_of_a_schema() {
        let schema: Schema = serde_json::from_value(json!({
            "type": "struct", "schema-id": 0, "fields": [
                { "id": 1, "name": "id", "required": true, "type": "long" },
                { "id": 2, "name": "price", "required": false, "type": "decimal(10, 2)", "doc": "usd" },
                { "id": 3, "name": "tags", "required": false, "type": {
                    "type": "list", "element-id": 5, "element": "string", "element-required": false } },
                { "id": 4, "name": "at", "required": false, "type": "timestamptz" },
            ],
        }))
        .unwrap();
        let columns = to_column_infos(&schema).unwrap();
        let summary: Vec<_> = columns
            .iter()
            .map(|c| {
                (
                    c.name.clone().unwrap(),
                    c.type_text.clone().unwrap(),
                    c.type_name.unwrap(),
                    c.nullable.unwrap(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "id".to_string(),
                    "bigint".to_string(),
                    ColumnTypeName::Long,
                    false
                ),
                (
                    "price".into(),
                    "decimal(10,2)".into(),
                    ColumnTypeName::Decimal,
                    true
                ),
                (
                    "tags".into(),
                    "array<string>".into(),
                    ColumnTypeName::Array,
                    true
                ),
                (
                    "at".into(),
                    "timestamp".into(),
                    ColumnTypeName::Timestamp,
                    true
                ),
            ]
        );
        assert_eq!(
            (columns[1].type_precision, columns[1].type_scale),
            (Some(10), Some(2))
        );
        assert_eq!(columns[1].comment.as_deref(), Some("usd"));
        let price: Value = serde_json::from_str(columns[1].type_json.as_deref().unwrap()).unwrap();
        assert_eq!(
            price,
            json!({ "name": "price", "type": "decimal(10,2)", "nullable": true,
            "metadata": { "comment": "usd" } })
        );
        let tags: Value = serde_json::from_str(columns[2].type_json.as_deref().unwrap()).unwrap();
        assert_eq!(
            tags["type"],
            json!({ "type": "array", "elementType": "string", "containsNull": true })
        );

        let time: Schema =
            serde_json::from_value(json!({ "type": "struct", "schema-id": 0, "fields": [
            { "id": 1, "name": "t", "required": false, "type": "time" } ] }))
            .unwrap();
        assert_eq!(
            to_column_infos(&time).unwrap_err().message(),
            "Iceberg type TIME is not supported by Unity Catalog"
        );
    }
}
