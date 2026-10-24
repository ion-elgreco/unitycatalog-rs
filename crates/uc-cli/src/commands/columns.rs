//! Upstream `CliUtils.parseColumns` and `parseInputParams`: `--columns
//! "id INT, price DECIMAL(10,2)"` and `--input_params "a INT, b STRING"`.

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

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Each match of `(\w+)\s+(\w+)(\(([^)]+)\))?`: name, type and the text in
/// parentheses.
fn matches(text: &str) -> Vec<(String, String, Option<String>, String)> {
    let chars: Vec<char> = text.chars().collect();
    let take = |from: usize, keep: fn(char) -> bool| {
        let mut end = from;
        while end < chars.len() && keep(chars[end]) {
            end += 1;
        }
        end
    };
    let mut found = Vec::new();
    let mut pos = 0;
    while pos < chars.len() {
        if !is_word(chars[pos]) {
            pos += 1;
            continue;
        }
        let name_end = take(pos, is_word);
        let type_start = take(name_end, |c| c.is_ascii_whitespace());
        let type_end = take(type_start, is_word);
        if type_start == name_end || type_end == type_start {
            pos = name_end;
            continue;
        }
        let mut end = type_end;
        let mut params = None;
        if chars.get(type_end) == Some(&'(')
            && let Some(close) = chars[type_end + 1..].iter().position(|c| *c == ')')
            && close > 0
        {
            params = Some(chars[type_end + 1..type_end + 1 + close].iter().collect());
            end = type_end + close + 2;
        }
        found.push((
            chars[pos..name_end].iter().collect(),
            chars[type_start..type_end].iter().collect(),
            params,
            chars[pos..end].iter().collect(),
        ));
        pos = end;
    }
    found
}

/// The columns of `--columns`, as upstream's `ColumnInfo`s.
pub fn parse_columns(text: &str) -> anyhow::Result<Vec<ColumnInfo>> {
    let mut columns = Vec::new();
    for (position, (name, type_name, params, matched)) in matches(text).into_iter().enumerate() {
        let type_name = type_name.to_uppercase();
        let Some(column_type) = type_named(&type_name) else {
            anyhow::bail!("Invalid column definition: {matched}");
        };
        // Upstream `setPrecisionAndScale`.
        let (mut precision, mut scale) = (0, 0);
        let mut suffix = String::new();
        if let Some(params) = params {
            let parts: Vec<&str> = params.split(',').map(str::trim).collect();
            if column_type == ColumnTypeName::Decimal && parts.len() == 2 {
                let parse = |p: &str| {
                    p.parse::<i32>()
                        .map_err(|_| anyhow::anyhow!("Invalid column definition: {matched}"))
                };
                (precision, scale) = (parse(parts[0])?, parse(parts[1])?);
                suffix = format!("({},{})", parts[0], parts[1]);
            }
        }
        columns.push(ColumnInfo {
            type_text: Some(type_text(&type_name)),
            type_json: type_json(&name, &format!("{}{suffix}", json_type(&type_name))),
            name,
            type_name: Some(column_type.into()),
            type_precision: Some(precision),
            type_scale: Some(scale),
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
        assert_eq!(columns[2].type_text.as_deref(), Some("decimal"));
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

    #[test]
    fn input_params_parse_as_upstream() {
        let params = parse_input_params("a INT, b string").unwrap();
        assert_eq!(params[1].type_name, ColumnTypeName::String);
        assert_eq!(params[1].position, Some(1));
        assert!(parse_input_params("a").is_err());
    }
}
