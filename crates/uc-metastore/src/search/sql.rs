use async_trait::async_trait;

use uc_api::SearchResult;
use uc_common::{Error, Result, SecurableType};
use uc_storage::{Row, StorageResult, Value};

use crate::search::{SEARCHABLE, SearchOps};
use crate::sql::{SqlMetastore, char_len, db_err};

/// The table of each of [`SEARCHABLE`], in that order: the `kind` column
/// of a result is its index.
const TABLES: [&str; 6] = [
    "catalog",
    "schema",
    "tbl",
    "volume",
    "func",
    "registered_model",
];

/// The group of a name in the order of the results, with the parameters
/// `$2` (the text in lowercase), `$3` (its length) and `$4` (a `LIKE`
/// pattern of it). Both engines make only ASCII letters lowercase here.
const RANK: &str = "CASE WHEN lower(o.name) = $2 THEN 0 \
                    WHEN substr(lower(o.name), 1, $3) = $2 THEN 1 \
                    WHEN lower(o.name) LIKE $4 ESCAPE '\\' THEN 2 \
                    ELSE 3 END";

/// The `SELECT` of the objects `o` of the type `SEARCHABLE[i]` that the
/// condition `found` finds, with the names of their catalog and schema.
fn part(i: usize, found: &str) -> String {
    let (parents, catalog, schema) = match SEARCHABLE[i] {
        SecurableType::Catalog => ("", "o.name", "CAST(NULL AS TEXT)"),
        SecurableType::Schema => (
            " JOIN catalog c ON c.id_string = o.catalog_id",
            "c.name",
            "o.name",
        ),
        _ => (
            " JOIN schema s ON s.id_string = o.schema_id \
             JOIN catalog c ON c.id_string = s.catalog_id",
            "c.name",
            "s.name",
        ),
    };
    format!(
        "SELECT {i} AS kind, o.id_string, {catalog} AS catalog_name, {schema} AS schema_name, \
         o.name AS name, o.comment, o.owner, o.created_at, o.updated_at, {RANK} AS rank \
         FROM {} o{parents} WHERE {found}",
        TABLES[i]
    )
}

/// `text` in a `LIKE` pattern that finds it anywhere.
fn contains_pattern(text: &str) -> String {
    let mut pattern = String::from("%");
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

fn result(row: &Row) -> StorageResult<SearchResult> {
    let kind = SEARCHABLE[usize::try_from(row.int(0)?).unwrap_or_default()];
    let catalog_name = row.text(2)?;
    let schema_name = row.opt_text(3)?;
    let name = row.text(4)?;
    let full_name = match (kind, &schema_name) {
        (SecurableType::Catalog, _) => name.clone(),
        (SecurableType::Schema, _) | (_, None) => format!("{catalog_name}.{name}"),
        (_, Some(schema)) => format!("{catalog_name}.{schema}.{name}"),
    };
    Ok(SearchResult {
        securable_type: kind,
        full_name,
        catalog_name,
        schema_name,
        name,
        comment: row.opt_text(5)?,
        owner: row.opt_text(6)?,
        created_at: row.int(7)?,
        updated_at: row.int(8)?,
        id: row.text(1)?,
    })
}

#[async_trait]
impl SearchOps for SqlMetastore {
    async fn search(
        &self,
        text: &str,
        kind: Option<SecurableType>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<SearchResult>> {
        if let Some(kind) = kind.filter(|k| !SEARCHABLE.contains(k)) {
            return Err(Error::invalid_argument(format!(
                "Search cannot find objects of the type {kind}"
            )));
        }
        let text = text.trim();
        let Some(query) = self.db.text_query(text) else {
            return Ok(Vec::new());
        };
        let found = self.db.text_match(&["o.name", "o.comment"], "$1");
        let parts: Vec<String> = (0..SEARCHABLE.len())
            .filter(|&i| kind.is_none_or(|k| k == SEARCHABLE[i]))
            .map(|i| part(i, &found))
            .collect();
        let sql = format!(
            "{} ORDER BY rank, name, catalog_name, kind, schema_name LIMIT $5 OFFSET $6",
            parts.join(" UNION ALL ")
        );
        let lower = text.to_ascii_lowercase();
        let params = [
            Value::from(query),
            Value::from(lower.as_str()),
            char_len(&lower),
            Value::from(contains_pattern(&lower)),
            Value::Int(limit),
            Value::Int(offset),
        ];
        let rows = self
            .db
            .query(&sql, &params)
            .await
            .map_err(db_err("search"))?;
        rows.iter()
            .map(|row| result(row).map_err(db_err("search")))
            .collect()
    }
}
