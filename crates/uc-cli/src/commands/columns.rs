//! Upstream `CliUtils.parseColumns` and `parseInputParams`: `--columns
//! "id INT, price DECIMAL(10,2)"` and `--input_params "a INT, b STRING"`.

use std::sync::LazyLock;

use regex::Regex;
use uc_client::models::functions::v1::FunctionParameterInfo;
use uc_client::models::tables::v1::{ColumnInfo, ColumnTypeName};

use super::options::from_wire;

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

/// The Spark JSON type: `INT` is `integer`.
fn json_type(type_name: &str) -> String {
    match type_name {
        "INT" => "integer".into(),
        other => other.to_lowercase(),
    }
}

fn type_json(name: &str, json_type: &str) -> String {
    format!("{{\"name\":\"{name}\",\"type\":\"{json_type}\",\"nullable\":true,\"metadata\":{{}}}}")
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

/// Upstream's `\s*[(\[\d]`: the text after a column starts like parameters.
static PARAMS_FOLLOW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[[:space:]]*[(\[0-9]").expect("valid regex"));

/// Upstream's `\d{1,9}`: one parameter of `DECIMAL`.
static DECIMAL_PARAM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[0-9]{1,9}$").expect("valid regex"));

/// Upstream `setPrecisionAndScale`: the precision and scale of a column,
/// and the `(p,s)` that its type text gets. A bare `DECIMAL` is
/// `DECIMAL(10,0)`, as in Spark. Other types take no parameters.
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
    let parts: Vec<&str> = params.map_or(vec![], |p| p.split(',').collect());
    let params = params.unwrap_or_default();
    if parts.len() > 2 {
        anyhow::bail!("DECIMAL takes at most 2 parameters: ({params})");
    }
    let parse = |part: &str| {
        // Java's trim: all characters up to U+0020.
        let part = part.trim_matches(|c| c <= ' ');
        if !DECIMAL_PARAM.is_match(part) {
            anyhow::bail!("Invalid DECIMAL parameters: ({params})");
        }
        Ok(part.parse::<i32>()?)
    };
    let precision = parts.first().map_or(Ok(10), |p| parse(p))?;
    let scale = parts.get(1).map_or(Ok(0), |s| parse(s))?;
    if !(1..=38).contains(&precision) {
        anyhow::bail!("DECIMAL precision must be between 1 and 38: {precision}");
    }
    if !(0..=precision).contains(&scale) {
        anyhow::bail!("DECIMAL scale must be between 0 and precision {precision}: {scale}");
    }
    Ok((
        Some(precision),
        Some(scale),
        format!("({precision},{scale})"),
    ))
}

/// The columns of `--columns`, as upstream's `ColumnInfo`s.
pub fn parse_columns(text: &str) -> anyhow::Result<Vec<ColumnInfo>> {
    let mut columns = Vec::new();
    for (position, column) in COLUMN.captures_iter(text).enumerate() {
        let whole = column.get(0).expect("group 0 is the match");
        let matched = whole.as_str();
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
        if decimal && params.is_none() && PARAMS_FOLLOW.is_match(&text[whole.end()..]) {
            return Err(invalid(anyhow::anyhow!(
                "Malformed DECIMAL parameters, expected DECIMAL(p) or DECIMAL(p,s)"
            )));
        }
        let (precision, scale, suffix) = precision_and_scale(decimal, params).map_err(invalid)?;
        columns.push(ColumnInfo {
            type_text: Some(format!("{}{suffix}", type_text(&type_name))),
            type_json: type_json(name, &format!("{}{suffix}", json_type(&type_name))),
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
            type_json: Some(type_json(name, &json_type(&type_name))),
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

    /// After upstream `CliUtilsTest`.
    #[test]
    fn decimals_take_a_precision_and_a_scale() {
        let decimal = |text: &str| {
            let columns = parse_columns(text).unwrap();
            let c = &columns[0];
            (
                c.type_text.clone().unwrap(),
                c.type_precision,
                c.type_scale,
                c.type_json.clone(),
            )
        };
        let json =
            |t: &str| format!(r#"{{"name":"d","type":"{t}","nullable":true,"metadata":{{}}}}"#);
        assert_eq!(
            decimal("d DECIMAL(12, 3)"),
            (
                "decimal(12,3)".into(),
                Some(12),
                Some(3),
                json("decimal(12,3)")
            )
        );
        assert_eq!(decimal("d decimal (10,2)").0, "decimal(10,2)");
        assert_eq!(
            decimal("d DECIMAL(7)"),
            (
                "decimal(7,0)".into(),
                Some(7),
                Some(0),
                json("decimal(7,0)")
            )
        );
        assert_eq!(
            decimal("d DECIMAL"),
            (
                "decimal(10,0)".into(),
                Some(10),
                Some(0),
                json("decimal(10,0)")
            )
        );
        assert_eq!(decimal("d DECIMAL(38,38)").0, "decimal(38,38)");
        assert_eq!(decimal("d DECIMAL(1,0)").0, "decimal(1,0)");
        let columns = parse_columns("id INT, d DECIMAL(5,1), s STRING").unwrap();
        assert_eq!(columns[1].type_text.as_deref(), Some("decimal(5,1)"));
        assert_eq!(columns[2].position, Some(2));

        for (text, reason) in [
            (
                "d DECIMAL(12,3,4)",
                "DECIMAL takes at most 2 parameters: (12,3,4)",
            ),
            ("d DECIMAL(12,)", "Invalid DECIMAL parameters: (12,)"),
            ("d DECIMAL(,)", "Invalid DECIMAL parameters: (,)"),
            ("d DECIMAL()", "Invalid DECIMAL parameters: ()"),
            ("d DECIMAL(a,1)", "Invalid DECIMAL parameters: (a,1)"),
            ("d DECIMAL(-1,0)", "Invalid DECIMAL parameters: (-1,0)"),
            (
                "d DECIMAL(39,0)",
                "DECIMAL precision must be between 1 and 38: 39",
            ),
            (
                "d DECIMAL(0)",
                "DECIMAL precision must be between 1 and 38: 0",
            ),
            (
                "d DECIMAL(5,6)",
                "DECIMAL scale must be between 0 and precision 5: 6",
            ),
        ] {
            let err = parse_columns(text).unwrap_err().to_string();
            assert_eq!(err, format!("Invalid column definition: {text}: {reason}"));
        }
        let malformed = "Malformed DECIMAL parameters, expected DECIMAL(p) or DECIMAL(p,s)";
        for text in ["d DECIMAL(12,3", "d DECIMAL[10,2]", "d DECIMAL 10"] {
            let err = parse_columns(text).unwrap_err().to_string();
            assert_eq!(
                err,
                format!("Invalid column definition: d DECIMAL: {malformed}"),
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
        let columns = parse_columns("price DECIMAL extra").unwrap();
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].type_text.as_deref(), Some("decimal(10,0)"));
    }

    /// Upstream's Java patterns, at their edges.
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
        // `\w` is ASCII: the type ends before `é`, and the rest is ignored.
        let columns = parse_columns("x decimalé (1,2)").unwrap();
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].type_text.as_deref(), Some("decimal(10,0)"));
        // `trim` removes control characters, not a no-break space.
        let columns = parse_columns("d DECIMAL(\x0112,3)").unwrap();
        assert_eq!(columns[0].type_text.as_deref(), Some("decimal(12,3)"));
        let err = parse_columns("d DECIMAL(\u{a0}12,3)")
            .unwrap_err()
            .to_string();
        assert!(
            err.ends_with("Invalid DECIMAL parameters: (\u{a0}12,3)"),
            "{err}"
        );
    }

    #[test]
    fn input_params_parse_as_upstream() {
        let params = parse_input_params("a INT, b string").unwrap();
        assert_eq!(params[1].type_name, ColumnTypeName::String);
        assert_eq!(params[1].position, Some(1));
        assert!(parse_input_params("a").is_err());
    }
}
