//! Upstream `CliUtils.parseColumns` and `parseInputParams`: `--columns
//! "id INT, price DECIMAL(10,2)"` and `--input_params "a INT, b STRING"`.

use std::sync::LazyLock;

use delta_kernel::schema::{DataType, PrimitiveType, StructField};
use regex::Regex;
use uc_client::models::functions::v1::FunctionParameterInfo;
use uc_client::models::tables::v1::{ColumnInfo, ColumnTypeName};

use super::options::{from_wire, wire_name};

/// Upstream's `ColumnTypeName` values.
pub const TYPE_NAMES: &[&str] = &[
    "BOOLEAN",
    "BYTE",
    "SHORT",
    "INT",
    "LONG",
    "FLOAT",
    "DOUBLE",
    "DATE",
    "TIMESTAMP",
    "TIMESTAMP_NTZ",
    "STRING",
    "BINARY",
    "DECIMAL",
    "INTERVAL",
    "ARRAY",
    "STRUCT",
    "MAP",
    "VARIANT",
    "CHAR",
    "NULL",
    "USER_DEFINED_TYPE",
    "TABLE_TYPE",
];

/// The type of one of [`TYPE_NAMES`].
fn type_named(type_name: &str) -> Option<ColumnTypeName> {
    TYPE_NAMES
        .contains(&type_name)
        .then(|| from_wire(type_name))
        .flatten()
}

/// The SQL type text: `LONG` is `bigint`, `SHORT` `smallint`, `BYTE`
/// `tinyint`, any other the name in lower case.
pub fn type_text(type_name: &str) -> String {
    match type_name {
        "LONG" => "bigint".into(),
        "SHORT" => "smallint".into(),
        "BYTE" => "tinyint".into(),
        other => other.to_lowercase(),
    }
}

/// Upstream `findBasicTypeFromString`: the Delta type of a basic type,
/// none for the others. A `DECIMAL` without a precision is `decimal(10,0)`.
pub fn basic_type(
    type_name: ColumnTypeName,
    precision: Option<i32>,
    scale: Option<i32>,
) -> anyhow::Result<Option<DataType>> {
    Ok(Some(match type_name {
        ColumnTypeName::Int => DataType::INTEGER,
        ColumnTypeName::String => DataType::STRING,
        ColumnTypeName::Double => DataType::DOUBLE,
        ColumnTypeName::Boolean => DataType::BOOLEAN,
        ColumnTypeName::Long => DataType::LONG,
        ColumnTypeName::Float => DataType::FLOAT,
        ColumnTypeName::Short => DataType::SHORT,
        ColumnTypeName::Byte => DataType::BYTE,
        ColumnTypeName::Date => DataType::DATE,
        ColumnTypeName::Timestamp => DataType::TIMESTAMP,
        ColumnTypeName::TimestampNtz => DataType::TIMESTAMP_NTZ,
        ColumnTypeName::Binary => DataType::BINARY,
        ColumnTypeName::Decimal => DataType::decimal(
            precision.filter(|p| *p > 0).map_or(10, |p| p as u8),
            scale.map_or(0, |s| s as u8),
        )?,
        _ => return Ok(None),
    }))
}

/// The Spark JSON of a nullable field, written by the kernel. A type
/// without a Delta type, such as a bare `ARRAY`, gets its name in lower
/// case.
fn type_json(
    name: &str,
    type_name: ColumnTypeName,
    precision: Option<i32>,
    scale: Option<i32>,
) -> anyhow::Result<String> {
    if let Some(data_type) = basic_type(type_name, precision, scale)? {
        return Ok(serde_json::to_string(&StructField::nullable(
            name, data_type,
        ))?);
    }
    let type_name = wire_name(&type_name).to_lowercase();
    let field = serde_json::json!({
        "name": name, "type": type_name, "nullable": true, "metadata": {}
    });
    Ok(field.to_string())
}

/// Upstream's `(\w+)\s+(\w+)(?:\(([^)]+)\)|(?<=(?i:decimal))\s*\(([^)]*)\))?`:
/// name, type, and the text in parentheses. Only a `DECIMAL` type takes
/// blanks before the parentheses or nothing in them. The regex crate has no
/// lookbehind. Upstream's lookbehind can only see the type, so the first
/// branch takes a type that ends in `decimal`. `[[:word:]]` and `[[:space:]]`
/// are ASCII, as Java's `\w` and `\s`.
static COLUMN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"([[:word:]]+)[[:space:]]+",
        r"(?:([[:word:]]*(?i:decimal))[[:space:]]*\(([^)]*)\)",
        r"|([[:word:]]+)(?:\(([^)]+)\))?)",
    ))
    .expect("valid regex")
});

/// The precision and scale of a column, and the `(p,s)` that its type text
/// gets. The kernel's Delta type parser reads a `DECIMAL(p,s)`, so a
/// `DECIMAL` needs both. Other types take no parameters. As upstream, they
/// get a precision and scale of 0 when they have none.
fn precision_and_scale(
    decimal: bool,
    params: Option<&str>,
) -> anyhow::Result<(Option<i32>, Option<i32>, String)> {
    if !decimal {
        return Ok(match params {
            Some(_) => (None, None, String::new()),
            None => (Some(0), Some(0), String::new()),
        });
    }
    let Some(params) = params else {
        anyhow::bail!("DECIMAL needs a precision and a scale: DECIMAL(p,s)");
    };
    match serde_json::from_value(serde_json::Value::String(format!("decimal({params})")))? {
        PrimitiveType::Decimal(d) => Ok((
            Some(d.precision().into()),
            Some(d.scale().into()),
            format!("({},{})", d.precision(), d.scale()),
        )),
        other => anyhow::bail!("{other} is not a DECIMAL"),
    }
}

/// The columns of `--columns`, as upstream's `ColumnInfo`s.
pub fn parse_columns(text: &str) -> anyhow::Result<Vec<ColumnInfo>> {
    let mut columns = Vec::new();
    for (position, column) in COLUMN.captures_iter(text).enumerate() {
        let matched = &column[0];
        let name = &column[1];
        let type_name = column
            .get(2)
            .or(column.get(4))
            .map_or("", |m| m.as_str())
            .to_uppercase();
        let params = column.get(3).or(column.get(5)).map(|m| m.as_str());
        let invalid =
            |e: anyhow::Error| anyhow::anyhow!("Invalid column definition: {matched}: {e}");
        let Some(column_type) = type_named(&type_name) else {
            anyhow::bail!("Invalid column definition: {matched}");
        };
        let decimal = column_type == ColumnTypeName::Decimal;
        let (precision, scale, suffix) = precision_and_scale(decimal, params).map_err(invalid)?;
        columns.push(ColumnInfo {
            type_text: Some(format!("{}{suffix}", type_text(&type_name))),
            type_json: type_json(name, column_type, precision, scale)?,
            name: name.to_string(),
            type_name: Some(column_type.into()),
            type_precision: precision,
            type_scale: scale,
            position: Some(position as i32),
            // As in type_json, and as upstream's client sends by default;
            // without it, the server stores false.
            nullable: Some(true),
            ..Default::default()
        });
    }
    Ok(columns)
}

/// The parameters of `--input_params`, as upstream's `FunctionParameterInfo`s.
pub fn parse_input_params(text: &str) -> anyhow::Result<Vec<FunctionParameterInfo>> {
    let mut parameters = Vec::new();
    for (position, spec) in text.split(',').enumerate() {
        let mut words = spec.split_whitespace();
        let (Some(name), Some(type_name)) = (words.next(), words.next()) else {
            anyhow::bail!("Invalid input parameter: {}", spec.trim());
        };
        let type_name = type_name.to_uppercase();
        let Some(parameter_type) = type_named(&type_name) else {
            anyhow::bail!("Unexpected value '{type_name}'");
        };
        parameters.push(FunctionParameterInfo {
            name: name.to_string(),
            type_text: type_text(&type_name),
            type_json: Some(type_json(name, parameter_type, None, None)?),
            type_name: parameter_type.into(),
            position: Some(position as i32),
            ..Default::default()
        });
    }
    Ok(parameters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_parse_as_upstream() {
        let columns = parse_columns("id INT, name string,price DECIMAL(10, 2), big LONG").unwrap();
        assert_eq!(columns.len(), 4);
        assert_eq!(columns[0].type_text.as_deref(), Some("int"));
        assert_eq!(
            columns[0].type_json,
            r#"{"name":"id","type":"integer","nullable":true,"metadata":{}}"#
        );
        assert_eq!(columns[0].type_precision, Some(0));
        assert_eq!(columns[0].nullable, Some(true));
        assert_eq!(columns[1].type_name, Some(ColumnTypeName::String.into()));
        assert_eq!(columns[2].type_text.as_deref(), Some("decimal(10,2)"));
        assert_eq!(columns[2].type_precision, Some(10));
        assert_eq!(columns[2].type_scale, Some(2));
        assert_eq!(
            columns[2].type_json,
            r#"{"name":"price","type":"decimal(10,2)","nullable":true,"metadata":{}}"#
        );
        assert_eq!(columns[3].type_text.as_deref(), Some("bigint"));
        assert_eq!(columns[3].position, Some(3));
        assert!(parse_columns("id NUMBER").is_err());
        assert!(parse_columns("lonely").unwrap().is_empty());
    }

    /// The kernel's Delta type parser reads each `DECIMAL(p,s)`.
    #[test]
    fn decimals_take_a_precision_and_a_scale() {
        let columns = parse_columns("d DECIMAL(12, 3)").unwrap();
        assert_eq!(columns[0].type_text.as_deref(), Some("decimal(12,3)"));
        assert_eq!(
            (columns[0].type_precision, columns[0].type_scale),
            (Some(12), Some(3))
        );
        assert_eq!(
            columns[0].type_json,
            r#"{"name":"d","type":"decimal(12,3)","nullable":true,"metadata":{}}"#
        );
        let type_text = |text: &str| parse_columns(text).unwrap()[0].type_text.clone();
        assert_eq!(
            type_text("d decimal (10,2)").as_deref(),
            Some("decimal(10,2)")
        );
        assert_eq!(
            type_text("d DECIMAL(38,38)").as_deref(),
            Some("decimal(38,38)")
        );
        assert_eq!(type_text("d DECIMAL(1,0)").as_deref(), Some("decimal(1,0)"));
        let columns = parse_columns("id INT, d DECIMAL(5,1), s STRING").unwrap();
        assert_eq!(columns[1].type_text.as_deref(), Some("decimal(5,1)"));
        assert_eq!(columns[2].position, Some(2));

        for (text, reason) in [
            (
                "d DECIMAL(12,3,4)",
                "Invalid decimal format (expected 2 parts): decimal(12,3,4)",
            ),
            ("d DECIMAL(7)", "Invalid scale in decimal: decimal(7)"),
            ("d DECIMAL(12,)", "Invalid scale in decimal: decimal(12,)"),
            ("d DECIMAL(,)", "Invalid precision in decimal: decimal(,)"),
            ("d DECIMAL()", "Invalid precision in decimal: decimal()"),
            (
                "d DECIMAL(a,1)",
                "Invalid precision in decimal: decimal(a,1)",
            ),
            (
                "d DECIMAL(-1,0)",
                "Invalid precision in decimal: decimal(-1,0)",
            ),
            (
                "d DECIMAL(300,0)",
                "Invalid precision in decimal: decimal(300,0)",
            ),
            (
                "d DECIMAL(39,0)",
                "Invalid decimal: precision must be in range 1..38 inclusive, found: 39.",
            ),
            (
                "d DECIMAL(0,0)",
                "Invalid decimal: precision must be in range 1..38 inclusive, found: 0.",
            ),
            (
                "d DECIMAL(5,6)",
                "Invalid decimal: scale must be in range 0..5 inclusive, found: 6.",
            ),
        ] {
            let err = parse_columns(text).unwrap_err().to_string();
            assert_eq!(err, format!("Invalid column definition: {text}: {reason}"));
        }
        let needs = "DECIMAL needs a precision and a scale: DECIMAL(p,s)";
        for text in [
            "d DECIMAL",
            "d DECIMAL(12,3",
            "d DECIMAL[10,2]",
            "d DECIMAL 10",
        ] {
            let err = parse_columns(text).unwrap_err().to_string();
            assert_eq!(
                err,
                format!("Invalid column definition: d DECIMAL: {needs}"),
                "{text}"
            );
        }
    }

    #[test]
    fn other_types_stay_lenient() {
        // A blank before, or nothing in, the parentheses: no parameters.
        for text in ["id INT (5)", "id INT()"] {
            let columns = parse_columns(text).unwrap();
            assert_eq!(columns[0].type_text.as_deref(), Some("int"), "{text}");
            assert_eq!(
                (columns[0].type_precision, columns[0].type_scale),
                (Some(0), Some(0))
            );
        }
        // Parameters of other types are dropped, with no precision.
        let columns = parse_columns("id INT(5)").unwrap();
        assert_eq!(columns[0].type_text.as_deref(), Some("int"));
        assert_eq!(columns[0].type_precision, None);
        // Text after a column is ignored, as upstream.
        let columns = parse_columns("id INT extra").unwrap();
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].type_text.as_deref(), Some("int"));
    }

    /// Upstream's Java pattern, at its edges.
    #[test]
    fn columns_match_as_java() {
        // `\s` takes a vertical tab.
        let columns = parse_columns("d\x0bDECIMAL\x0b(5,1)").unwrap();
        assert_eq!(columns[0].type_text.as_deref(), Some("decimal(5,1)"));
        // The lookbehind takes each type that ends in `decimal`.
        let err = parse_columns("x mydecimal (1,2)").unwrap_err().to_string();
        assert_eq!(err, "Invalid column definition: x mydecimal (1,2)");
        let err = parse_columns("x decimalx (1,2)").unwrap_err().to_string();
        assert_eq!(err, "Invalid column definition: x decimalx");
        // `\w` is ASCII: the type ends before `é`.
        let err = parse_columns("x decimalé(1,2)").unwrap_err().to_string();
        assert!(
            err.starts_with("Invalid column definition: x decimal: "),
            "{err}"
        );
    }

    #[test]
    fn input_params_parse_as_upstream() {
        let params = parse_input_params("a INT, b string").unwrap();
        assert_eq!(params[1].type_name, ColumnTypeName::String);
        assert_eq!(params[1].position, Some(1));
        assert!(parse_input_params("a").is_err());
        // A type without a Delta type keeps its name; the JSON escapes the
        // name.
        let params = parse_input_params(r#"a"b ARRAY, d DECIMAL"#).unwrap();
        assert_eq!(
            params[0].type_json.as_deref(),
            Some(r#"{"name":"a\"b","type":"array","nullable":true,"metadata":{}}"#)
        );
        assert_eq!(
            params[1].type_json.as_deref(),
            Some(r#"{"name":"d","type":"decimal(10,0)","nullable":true,"metadata":{}}"#)
        );
    }
}
