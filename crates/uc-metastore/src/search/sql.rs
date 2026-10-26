use async_trait::async_trait;

use uc_api::SearchResult;
use uc_common::{Error, Result, SecurableType, schema_full_name, three_part_full_name};
use uc_storage::{Row, StorageResult, Value};

use crate::permissions::{Lineage, Node};
use crate::search::{SearchHit, SearchOps};
use crate::sql::rows::{CATALOG, FUNC, REGISTERED_MODEL, SCHEMA, TBL, VOLUME};
use crate::sql::{SqlMetastore, char_len, db_err, starts_with};

/// The types of the objects that a search finds, and their tables: the
/// `kind` column of a result is its index.
const SOURCES: [(SecurableType, &str); 6] = [
    (SecurableType::Catalog, CATALOG.name),
    (SecurableType::Schema, SCHEMA.name),
    (SecurableType::Table, TBL.name),
    (SecurableType::Volume, VOLUME.name),
    (SecurableType::Function, FUNC.name),
    (SecurableType::RegisteredModel, REGISTERED_MODEL.name),
];

/// The group of a name in the order of the results, with the parameters
/// `$2` (the text in lowercase), `$3` (its length) and `$4` (a `LIKE`
/// pattern of it). Both engines make only ASCII letters lowercase here.
fn rank() -> String {
    format!(
        "CASE WHEN lower(o.name) = $2 THEN 0 WHEN {} THEN 1 \
         WHEN lower(o.name) LIKE $4 ESCAPE '\\' THEN 2 ELSE 3 END",
        starts_with("lower(o.name)", 2, 3)
    )
}

/// The `SELECT` of the objects `o` of `SOURCES[i]` that the condition
/// `found` finds, with the name, id and owner of their catalog and schema.
fn part(i: usize, found: &str) -> String {
    let (kind, table) = SOURCES[i];
    // The rows of the catalog and the schema: a catalog or schema is its
    // own.
    let (parents, catalog, schema) = match kind {
        SecurableType::Catalog => ("", "o", None),
        SecurableType::Schema => (
            " JOIN catalog c ON c.id_string = o.catalog_id",
            "c",
            Some("o"),
        ),
        _ => (
            " JOIN schema s ON s.id_string = o.schema_id \
             JOIN catalog c ON c.id_string = s.catalog_id",
            "c",
            Some("s"),
        ),
    };
    let schema = |column: &str| match schema {
        Some(schema) => format!("{schema}.{column}"),
        None => "CAST(NULL AS TEXT)".to_string(),
    };
    format!(
        "SELECT {i} AS kind, o.id_string, {catalog}.name AS catalog_name, {} AS schema_name, \
         o.name AS name, o.comment, o.owner, o.created_at, o.updated_at, {} AS rank, \
         {catalog}.id_string AS catalog_id, {catalog}.owner AS catalog_owner, \
         {} AS schema_id, {} AS schema_owner FROM {table} o{parents} WHERE {found}",
        schema("name"),
        rank(),
        schema("id_string"),
        schema("owner"),
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

fn hit(row: &Row) -> StorageResult<SearchHit> {
    let kind = SOURCES[usize::try_from(row.int(0)?).unwrap_or_default()].0;
    let id = row.text(1)?;
    let catalog_name = row.text(2)?;
    let schema_name = row.opt_text(3)?;
    let name = row.text(4)?;
    let owner = row.opt_text(6)?;
    let full_name = match (kind, &schema_name) {
        (SecurableType::Catalog, _) => name.clone(),
        (SecurableType::Schema, _) | (_, None) => schema_full_name(&catalog_name, &name),
        (_, Some(schema)) => three_part_full_name(&catalog_name, schema, &name),
    };
    let lineage = Lineage {
        catalog: Some(Node {
            kind: SecurableType::Catalog,
            id: row.text(10)?,
            owner: row.opt_text(11)?,
        }),
        schema: match row.opt_text(12)? {
            Some(id) => Some(Node {
                kind: SecurableType::Schema,
                id,
                owner: row.opt_text(13)?,
            }),
            None => None,
        },
        object: Some(Node {
            kind,
            id: id.clone(),
            owner: owner.clone(),
        }),
    };
    let result = SearchResult {
        securable_type: kind,
        full_name,
        catalog_name,
        schema_name,
        name,
        comment: row.opt_text(5)?,
        owner,
        created_at: row.int(7)?,
        updated_at: row.int(8)?,
        id,
    };
    Ok(SearchHit { result, lineage })
}

#[async_trait]
impl SearchOps for SqlMetastore {
    async fn search(
        &self,
        text: &str,
        kind: Option<SecurableType>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<SearchHit>> {
        if let Some(kind) = kind.filter(|k| !SOURCES.iter().any(|(t, _)| t == k)) {
            return Err(Error::invalid_argument(format!(
                "Search cannot find objects of the type {kind}"
            )));
        }
        let text = text.trim();
        let Some(query) = self.db.text_query(text) else {
            return Ok(Vec::new());
        };
        let found = self.db.text_match(&["o.name", "o.comment"], "$1");
        let parts: Vec<String> = (0..SOURCES.len())
            .filter(|&i| kind.is_none_or(|k| k == SOURCES[i].0))
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
            .map(|row| hit(row).map_err(db_err("search")))
            .collect()
    }
}
