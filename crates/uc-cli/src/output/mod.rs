//! Output as upstream's CLI prints it (`CliUtils.postProcessAndPrintOutput`):
//! compact JSON for `--output json`, Jackson's pretty printer for
//! `jsonPretty`, otherwise a table (a list as rows, an object as KEY and
//! VALUE), drawn by comfy-table. [`models`] gives the fields that print, as
//! upstream's client models.

pub mod models;

use std::io::IsTerminal;

use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};
use serde_json::Value;

/// Upstream's minimum table width; `cli.output_width` overrides.
const TABLE_WIDTH: usize = 120;

/// `--output`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Table,
    Json,
    #[value(name = "jsonPretty")]
    JsonPretty,
}

impl Format {
    /// A result as JSON text: pretty for `jsonPretty`, else compact.
    pub fn json(self, value: &Value) -> String {
        match self {
            Self::JsonPretty => jackson_pretty(value),
            _ => value.to_string(),
        }
    }
}

/// Jackson's `DefaultPrettyPrinter`: objects indent by two spaces and
/// separate keys with ` : `; arrays stay on the line (`[ 1, 2 ]`); empty
/// ones are `{ }` and `[ ]`.
pub fn jackson_pretty(value: &Value) -> String {
    let mut out = String::new();
    pretty(&mut out, value, 0);
    out
}

fn pretty(out: &mut String, value: &Value, level: usize) {
    match value {
        Value::Object(map) if map.is_empty() => out.push_str("{ }"),
        Value::Object(map) => {
            out.push('{');
            for (i, (key, item)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&"  ".repeat(level + 1));
                out.push_str(&Value::String(key.clone()).to_string());
                out.push_str(" : ");
                pretty(out, item, level + 1);
            }
            out.push('\n');
            out.push_str(&"  ".repeat(level));
            out.push('}');
        }
        Value::Array(items) if items.is_empty() => out.push_str("[ ]"),
        Value::Array(items) => {
            out.push_str("[ ");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty(out, item, level);
            }
            out.push_str(" ]");
        }
        other => out.push_str(&other.to_string()),
    }
}

/// What a command prints for its result `output` (JSON text, or text);
/// `width` is the `cli.output_width` setting.
pub fn print(format: Format, width: Option<usize>, output: &str) -> String {
    if format != Format::Table {
        return output.to_string();
    }
    // No content (a delete) is a table without rows, as upstream.
    let node = if output.trim().is_empty() {
        Value::Object(Default::default())
    } else {
        match serde_json::from_str::<Value>(output) {
            Ok(node) => node,
            Err(_) => return output.to_string(),
        }
    };
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(u16::try_from(width.unwrap_or_else(output_width)).unwrap_or(u16::MAX));
    match &node {
        Value::Array(items) if items.is_empty() => return output.to_string(),
        Value::Array(items) => rows(&mut table, items),
        _ => keys_and_values(&mut table, &node),
    }
    table.to_string()
}

/// Upstream's minimum, or the terminal's width when wider: `COLUMNS`, else
/// the width of the terminal that shows the table, as upstream asks its
/// terminal. Shells do not pass `COLUMNS` on to programs.
fn output_width() -> usize {
    let terminal = || {
        let shown = std::io::stdout().is_terminal();
        let size = shown.then(crossterm::terminal::size)?.ok()?;
        Some(usize::from(size.0))
    };
    table_width(std::env::var("COLUMNS").ok().as_deref(), terminal)
}

/// [`output_width`] of the `COLUMNS` variable and the terminal's width.
fn table_width(columns: Option<&str>, terminal: impl FnOnce() -> Option<usize>) -> usize {
    let width = columns
        .and_then(|c| c.trim().parse().ok())
        .or_else(terminal);
    TABLE_WIDTH.max(width.unwrap_or(0))
}

/// A JSON value as a cell: text as is, anything else as JSON.
fn cell(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Upper-case headers, centered.
fn header<'a>(table: &mut Table, names: impl IntoIterator<Item = &'a str>) {
    table.set_header(
        names
            .into_iter()
            .map(|n| Cell::new(n.to_uppercase()).set_alignment(CellAlignment::Center)),
    );
}

/// A list of objects as rows, a column for each field of the first; a list
/// of other values as one VALUE column.
fn rows(table: &mut Table, items: &[Value]) {
    let Value::Object(first) = &items[0] else {
        header(table, ["value"]);
        for item in items {
            table.add_row([cell(item)]);
        }
        return;
    };
    header(table, first.keys().map(String::as_str));
    for item in items {
        table.add_row(
            first
                .keys()
                .map(|field| cell(item.get(field).unwrap_or(&Value::Null))),
        );
    }
}

/// An object as KEY and VALUE rows; a list's elements on lines of their
/// own.
fn keys_and_values(table: &mut Table, node: &Value) {
    header(table, ["key", "value"]);
    for (key, value) in node.as_object().into_iter().flatten() {
        let text = match value {
            Value::Array(items) => items.iter().map(cell).collect::<Vec<_>>().join("\n"),
            other => cell(other),
        };
        table.add_row([key.to_uppercase(), text]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pretty_prints_like_jackson() {
        let v = json!({ "name": "c", "properties": {}, "list": [], "nested": { "a": 1 }, "items": [{ "x": 1 }, 2] });
        assert_eq!(
            jackson_pretty(&v),
            "{\n  \"name\" : \"c\",\n  \"properties\" : { },\n  \"list\" : [ ],\n  \"nested\" : {\n    \"a\" : 1\n  },\n  \"items\" : [ {\n    \"x\" : 1\n  }, 2 ]\n}"
        );
        assert_eq!(
            jackson_pretty(&json!([{ "a": null }])),
            "[ {\n  \"a\" : null\n} ]"
        );
    }

    fn table(output: &Value, width: usize) -> String {
        print(Format::Table, Some(width), &output.to_string())
    }

    #[test]
    fn lists_render_as_rows() {
        let rows = json!([{ "name": "unity", "comment": "Main catalog", "properties": {}, "owner": null }]);
        assert_eq!(
            table(&rows, 120),
            "┌───────┬──────────────┬────────────┬───────┐\n\
             │  NAME ┆    COMMENT   ┆ PROPERTIES ┆ OWNER │\n\
             ╞═══════╪══════════════╪════════════╪═══════╡\n\
             │ unity ┆ Main catalog ┆ {}         ┆ null  │\n\
             └───────┴──────────────┴────────────┴───────┘"
        );
        // A list of other values is one column.
        assert_eq!(
            table(&json!(["a", 1]), 120),
            "┌───────┐\n│ VALUE │\n╞═══════╡\n│ a     │\n├╌╌╌╌╌╌╌┤\n│ 1     │\n└───────┘"
        );
        // An empty list prints as is.
        assert_eq!(table(&json!([]), 120), "[]");
    }

    #[test]
    fn tables_fit_the_width() {
        let items = json!([{ "name": "n".repeat(100), "comment": "Managed table" }]);
        let out = table(&items, 60);
        assert!(out.lines().all(|l| l.chars().count() <= 60), "{out}");
        assert!(out.contains("┆ Managed table │"), "{out}");
    }

    #[test]
    fn tables_take_the_terminal_width() {
        // COLUMNS, else the terminal's width; at least upstream's 120.
        assert_eq!(table_width(Some("200"), || Some(150)), 200);
        assert_eq!(table_width(None, || Some(150)), 150);
        assert_eq!(table_width(Some("wide"), || Some(150)), 150);
        assert_eq!(table_width(None, || Some(80)), 120);
        // Not in a terminal: 120.
        assert_eq!(table_width(None, || None), 120);
    }

    #[test]
    fn objects_render_as_keys_and_values() {
        let object = json!({ "name": "c", "list": ["USE CATALOG", "CREATE SCHEMA"], "def": "t = x\nreturn t" });
        assert_eq!(
            table(&object, 120),
            "┌──────┬───────────────┐\n\
             │  KEY ┆     VALUE     │\n\
             ╞══════╪═══════════════╡\n\
             │ NAME ┆ c             │\n\
             ├╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤\n\
             │ LIST ┆ USE CATALOG   │\n\
             │      ┆ CREATE SCHEMA │\n\
             ├╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤\n\
             │ DEF  ┆ t = x         │\n\
             │      ┆ return t      │\n\
             └──────┴───────────────┘"
        );
        // No result (a delete) prints the header only.
        assert_eq!(
            print(Format::Table, Some(120), ""),
            "┌─────┬───────┐\n│ KEY ┆ VALUE │\n╞═════╪═══════╡\n└─────┴───────┘"
        );
    }

    #[test]
    fn other_formats_and_text_print_as_is() {
        let json = r#"{"name":"c"}"#;
        assert_eq!(print(Format::Json, Some(120), json), json);
        assert_eq!(print(Format::Table, Some(120), "not JSON"), "not JSON");
    }
}
