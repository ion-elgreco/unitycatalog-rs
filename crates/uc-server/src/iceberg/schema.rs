//! Iceberg schemas as UC columns, so that tables made through the Iceberg
//! REST catalog are browsable through the UC API. Upstream
//! `IcebergSchemaConverter`: the metadata file stays the source of truth;
//! `type_json` takes the Spark `StructField` shape UC stores for every
//! format.

use delta_kernel::schema::{ArrayType, DataType, MapType, StructField, StructType};
use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};

use uc_api::ColumnInfo;
use uc_common::{Error, Result};

/// The UC columns of `schema`'s top-level fields, as Delta's columns:
/// Spark's catalog strings, type names and `StructField` JSON.
pub fn to_column_infos(schema: &Schema) -> Result<Vec<ColumnInfo>> {
    let mut columns = Vec::new();
    for (position, field) in schema.as_struct().fields().iter().enumerate() {
        let delta_field = struct_field(field)?;
        let type_json = serde_json::to_value(&delta_field)
            .map_err(|e| Error::internal(format!("column {}: {e}", field.name)))?;
        let mut column = uc_delta::table::column_info(&delta_field, &type_json, position as i32)?;
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

fn unsupported(iceberg_type: &impl std::fmt::Display) -> Error {
    Error::invalid_argument(format!(
        "Iceberg type {} is not supported by Unity Catalog",
        iceberg_type.to_string().to_uppercase()
    ))
}

/// An Iceberg field as a Delta struct field, its doc as the comment.
fn struct_field(field: &NestedField) -> Result<StructField> {
    let delta_field = StructField::new(
        field.name.clone(),
        data_type(&field.field_type)?,
        !field.required,
    );
    Ok(match &field.doc {
        Some(doc) => delta_field.with_metadata([("comment", doc.as_str())]),
        None => delta_field,
    })
}

fn data_type(field_type: &Type) -> Result<DataType> {
    Ok(match field_type {
        Type::Struct(struct_type) => {
            let fields = struct_type
                .fields()
                .iter()
                .map(|f| struct_field(f))
                .collect::<Result<Vec<_>>>()?;
            StructType::try_new(fields)
                .map_err(|e| Error::invalid_argument(e.to_string()))?
                .into()
        }
        Type::List(list) => ArrayType::new(
            data_type(&list.element_field.field_type)?,
            !list.element_field.required,
        )
        .into(),
        Type::Map(map) => MapType::new(
            data_type(&map.key_field.field_type)?,
            data_type(&map.value_field.field_type)?,
            !map.value_field.required,
        )
        .into(),
        Type::Primitive(p) => match p {
            PrimitiveType::Boolean => DataType::BOOLEAN,
            PrimitiveType::Int => DataType::INTEGER,
            PrimitiveType::Long => DataType::LONG,
            PrimitiveType::Float => DataType::FLOAT,
            PrimitiveType::Double => DataType::DOUBLE,
            PrimitiveType::Date => DataType::DATE,
            PrimitiveType::Timestamptz => DataType::TIMESTAMP,
            PrimitiveType::Timestamp => DataType::TIMESTAMP_NTZ,
            PrimitiveType::String | PrimitiveType::Uuid => DataType::STRING,
            PrimitiveType::Fixed(_) | PrimitiveType::Binary => DataType::BINARY,
            PrimitiveType::Decimal { precision, scale } => {
                let digits = |n: u32| u8::try_from(n).map_err(|_| unsupported(p));
                DataType::decimal(digits(*precision)?, digits(*scale)?)
                    .map_err(|e| Error::invalid_argument(e.to_string()))?
            }
            PrimitiveType::Time
            | PrimitiveType::TimestampNs
            | PrimitiveType::TimestamptzNs
            | PrimitiveType::Unknown
            | PrimitiveType::Geometry(_)
            | PrimitiveType::Geography(_) => {
                return Err(unsupported(p));
            }
        },
        Type::Variant(_) => return Err(unsupported(field_type)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
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
