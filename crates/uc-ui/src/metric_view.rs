//! A metric view's YAML definition: its dimensions and measures. The "New"
//! dialog checks them against the columns and writes each one into its
//! column's metadata, as the console's bridge does.

use serde_json::Value;
use uc_client::models::tables::v1::TableInfo;
use uc_client::models::tables::v1::TableType;

/// Whether a table is a metric view. The API lists both as tables; the UI
/// shows them apart.
pub fn is_metric_view(table: &TableInfo) -> bool {
    table.table_type.as_known() == Some(TableType::MetricView)
}

/// The full names of the tables and functions that a metric view reads.
pub fn dependencies(table: &TableInfo) -> Vec<String> {
    let Some(list) = table.view_dependencies.as_option() else {
        return Vec::new();
    };
    list.dependencies
        .iter()
        .filter_map(|dependency| {
            let table = dependency
                .table
                .as_option()
                .map(|table| table.table_full_name.clone());
            let function = dependency
                .function
                .as_option()
                .map(|function| function.function_full_name.clone());
            table.or(function)
        })
        .collect()
}

/// A dimension or a measure of a definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    /// `dimension` or `measure`, as the column's metadata names it.
    pub kind: &'static str,
    pub expr: String,
}

/// The dimensions, then the measures, of the YAML `definition`.
pub fn fields(definition: &str) -> Result<Vec<Field>, String> {
    let document: Value = serde_saphyr::from_str(definition)
        .map_err(|error| format!("Invalid metric view YAML definition: {error}"))?;
    let mut fields = Vec::new();
    collect(&document, "dimensions", "dimension", &mut fields)?;
    collect(&document, "measures", "measure", &mut fields)?;
    Ok(fields)
}

/// The entries of `section`, each with a name and an expression.
fn collect(
    document: &Value,
    section: &str,
    kind: &'static str,
    fields: &mut Vec<Field>,
) -> Result<(), String> {
    let Some(entries) = document.get(section) else {
        return Ok(());
    };
    let entries = entries
        .as_array()
        .ok_or_else(|| format!("{section} must be a YAML list"))?;
    for entry in entries {
        let text = |key: &str| entry.get(key).and_then(Value::as_str);
        let name = text("name").ok_or_else(|| format!("{section} entries require a name"))?;
        let expr = text("expr").ok_or_else(|| format!("{section} entries require an expr"))?;
        if fields.iter().any(|field| field.name == name) {
            return Err(format!("Metric view field names must be unique: {name}"));
        }
        fields.push(Field {
            name: name.to_string(),
            kind,
            expr: expr.to_string(),
        });
    }
    Ok(())
}

/// The field of each of `columns`, in the columns' order: a metric view has
/// one column per dimension and measure, with the same name.
pub fn fields_of<'a>(fields: &'a [Field], columns: &[String]) -> Result<Vec<&'a Field>, String> {
    let mut matched: Vec<&Field> = Vec::with_capacity(columns.len());
    for column in columns {
        if matched.iter().any(|field| field.name == *column) {
            return Err(format!("Metric view column names must be unique: {column}"));
        }
        let field = fields
            .iter()
            .find(|field| field.name == *column)
            .ok_or_else(|| {
                format!("Column {column} is not declared in the metric view definition")
            })?;
        matched.push(field);
    }
    let missing: Vec<&str> = fields
        .iter()
        .filter(|field| !columns.contains(&field.name))
        .map(|field| field.name.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Metric view columns are missing YAML fields: {}",
            missing.join(", ")
        ));
    }
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::{Field, fields, fields_of};

    const SALES: &str = "version: 0.1
source: main.sales.orders
dimensions:
  - name: region
    expr: region
measures:
  - name: revenue
    expr: SUM(amount)
";

    fn names(columns: &[&str]) -> Vec<String> {
        columns.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn a_definition_names_its_dimensions_and_measures() {
        let found = fields(SALES).unwrap();
        assert_eq!(
            found,
            vec![
                Field {
                    name: "region".into(),
                    kind: "dimension",
                    expr: "region".into(),
                },
                Field {
                    name: "revenue".into(),
                    kind: "measure",
                    expr: "SUM(amount)".into(),
                },
            ]
        );
        assert_eq!(fields("source: main.sales.orders").unwrap(), Vec::new());
        assert_eq!(fields("").unwrap(), Vec::new());
    }

    #[test]
    fn a_bad_definition_says_what_is_wrong() {
        let error = fields("dimensions: [").unwrap_err();
        assert!(error.starts_with("Invalid metric view YAML definition: "));
        assert_eq!(
            fields("dimensions: region").unwrap_err(),
            "dimensions must be a YAML list"
        );
        assert_eq!(
            fields("measures:\n  - expr: SUM(amount)").unwrap_err(),
            "measures entries require a name"
        );
        assert_eq!(
            fields("measures:\n  - name: revenue").unwrap_err(),
            "measures entries require an expr"
        );
        let twice =
            "dimensions:\n  - name: a\n    expr: a\nmeasures:\n  - name: a\n    expr: SUM(a)";
        assert_eq!(
            fields(twice).unwrap_err(),
            "Metric view field names must be unique: a"
        );
    }

    #[test]
    fn the_columns_are_the_fields() {
        let found = fields(SALES).unwrap();
        let matched = fields_of(&found, &names(&["revenue", "region"])).unwrap();
        assert_eq!(matched[0].kind, "measure");
        assert_eq!(matched[1].kind, "dimension");
        assert_eq!(
            fields_of(&found, &names(&["region", "value"])).unwrap_err(),
            "Column value is not declared in the metric view definition"
        );
        assert_eq!(
            fields_of(&found, &names(&["region"])).unwrap_err(),
            "Metric view columns are missing YAML fields: revenue"
        );
        assert_eq!(
            fields_of(&found, &names(&["region", "region"])).unwrap_err(),
            "Metric view column names must be unique: region"
        );
    }
}
