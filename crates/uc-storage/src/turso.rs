//! [`Storage`] on Turso, embedded in the server, in memory or in a file.
//! Turso writes one transaction at a time, so all writes share one
//! connection behind a lock; reads run on connections of their own and see
//! committed data. Turso locks the file, so a second server cannot open it.

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use turso::Connection;

use super::{Exec, Row, Storage, StorageError, StorageResult, Transaction, Value};

/// Idle reader connections kept for the next read.
const IDLE_READERS: usize = 16;

/// `unixepoch('subsec')` is fractional seconds.
const NOW_MILLIS: &str = "CAST(unixepoch('subsec') * 1000 AS BIGINT)";

/// The shortest word that a full-text index finds: the indexes hold the
/// n-grams of 2 and 3 characters of the text (`tokenizer = 'ngram'`).
const MIN_WORD: usize = 2;

fn map_err(e: turso::Error) -> StorageError {
    match e {
        turso::Error::Constraint(m) if m.starts_with("UNIQUE constraint failed") => {
            StorageError::Unique(m)
        }
        turso::Error::Busy(m) | turso::Error::BusySnapshot(m) => StorageError::Busy(m),
        e => StorageError::Other(e.to_string()),
    }
}

/// PostgreSQL's placeholders (`$1`) as Turso's (`?1`), outside quotes.
fn numbered(sql: &str) -> Cow<'_, str> {
    if !sql.contains('$') {
        return Cow::Borrowed(sql);
    }
    let mut out = String::with_capacity(sql.len());
    let mut quote: Option<char> = None;
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (None, '\'' | '"') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '$') if chars.peek().is_some_and(char::is_ascii_digit) => {
                out.push('?');
                continue;
            }
            _ => {}
        }
        out.push(c);
    }
    Cow::Owned(out)
}

/// Turso's full-text index: tantivy, on the n-grams of the text.
fn text_match(columns: &[&str], param: &str) -> String {
    format!("fts_match({}, {param})", columns.join(", "))
}

/// Tantivy's query: each word a phrase in quotes, all of them required.
/// Words keep `_`, which the n-grams hold too; any other character that is
/// not a letter or a digit splits them, so none of the query syntax is left.
fn text_query(text: &str) -> Option<String> {
    let words: Vec<String> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| w.chars().count() >= MIN_WORD)
        .map(|w| format!("\"{w}\""))
        .collect();
    (!words.is_empty()).then(|| words.join(" AND "))
}

/// Booleans are integers in Turso.
fn params(values: &[Value]) -> Vec<turso::Value> {
    values
        .iter()
        .map(|v| match v {
            Value::Null => turso::Value::Null,
            Value::Int(n) => turso::Value::Integer(*n),
            Value::Bool(b) => turso::Value::Integer(i64::from(*b)),
            Value::Text(s) => turso::Value::Text(s.clone()),
        })
        .collect()
}

fn value(v: turso::Value) -> StorageResult<Value> {
    Ok(match v {
        turso::Value::Null => Value::Null,
        turso::Value::Integer(n) => Value::Int(n),
        turso::Value::Text(s) => Value::Text(s),
        turso::Value::Real(f) if f.fract() == 0.0 => Value::Int(f as i64),
        v => return Err(StorageError::Other(format!("unsupported value: {v:?}"))),
    })
}

async fn query(conn: &Connection, sql: &str, values: &[Value]) -> StorageResult<Vec<Row>> {
    let mut stmt = conn.prepare_cached(numbered(sql)).await.map_err(map_err)?;
    let result = async {
        let mut rows = stmt.query(params(values)).await.map_err(map_err)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(map_err)? {
            let values = (0..row.column_count())
                .map(|i| row.get_value(i).map_err(map_err).and_then(value))
                .collect::<StorageResult<Vec<_>>>()?;
            out.push(Row::from(values));
        }
        Ok(out)
    }
    .await;
    if result.is_err() {
        // A statement stopped part way keeps its read open until reset.
        let _ = stmt.reset();
    }
    result
}

async fn execute(conn: &Connection, sql: &str, values: &[Value]) -> StorageResult<u64> {
    let mut stmt = conn.prepare_cached(numbered(sql)).await.map_err(map_err)?;
    let result = stmt.execute(params(values)).await.map_err(map_err);
    if result.is_err() {
        let _ = stmt.reset();
    }
    result
}

/// Turso, in memory or in a file.
pub struct TursoStorage {
    db: turso::Database,
    writer: Arc<AsyncMutex<Connection>>,
    readers: Mutex<Vec<Connection>>,
}

impl TursoStorage {
    /// `path` is the database file; `None` keeps the database in memory.
    pub async fn open(path: Option<&Path>) -> StorageResult<Self> {
        let file = match path {
            None => ":memory:".to_string(),
            Some(path) => {
                if path.is_dir() {
                    return Err(StorageError::Other(format!(
                        "{} is a directory, not a database file",
                        path.display()
                    )));
                }
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        StorageError::Other(format!("cannot create {}: {e}", parent.display()))
                    })?;
                }
                path.to_str()
                    .ok_or_else(|| {
                        StorageError::Other(format!("{} is not a UTF-8 path", path.display()))
                    })?
                    .to_string()
            }
        };
        let db = turso::Builder::new_local(&file)
            .experimental_index_method(true)
            .build()
            .await
            .map_err(map_err)?;
        let writer = db.connect().map_err(map_err)?;
        Ok(Self {
            db,
            writer: Arc::new(AsyncMutex::new(writer)),
            readers: Mutex::new(Vec::new()),
        })
    }

    /// The writer, with any transaction a dropped [`TursoTransaction`] left
    /// open rolled back.
    async fn writer(&self) -> StorageResult<OwnedMutexGuard<Connection>> {
        let writer = self.writer.clone().lock_owned().await;
        if !writer.is_autocommit().map_err(map_err)? {
            writer.execute("ROLLBACK", ()).await.map_err(map_err)?;
        }
        Ok(writer)
    }
}

#[async_trait]
impl Exec for TursoStorage {
    async fn query(&self, sql: &str, values: &[Value]) -> StorageResult<Vec<Row>> {
        let idle = self.readers.lock().expect("readers lock").pop();
        let conn = match idle {
            Some(conn) => conn,
            None => self.db.connect().map_err(map_err)?,
        };
        // A connection whose statement failed is not reused.
        let rows = query(&conn, sql, values).await?;
        let mut readers = self.readers.lock().expect("readers lock");
        if readers.len() < IDLE_READERS {
            readers.push(conn);
        }
        Ok(rows)
    }

    async fn execute(&self, sql: &str, values: &[Value]) -> StorageResult<u64> {
        let writer = self.writer().await?;
        execute(&writer, sql, values).await
    }

    fn now_millis(&self) -> &'static str {
        NOW_MILLIS
    }

    /// One writer at a time: a transaction's rows need no lock.
    fn for_update(&self) -> &'static str {
        ""
    }

    fn text_match(&self, columns: &[&str], param: &str) -> String {
        text_match(columns, param)
    }

    fn text_query(&self, text: &str) -> Option<String> {
        text_query(text)
    }
}

#[async_trait]
impl Storage for TursoStorage {
    async fn begin(&self) -> StorageResult<Box<dyn Transaction>> {
        let writer = self.writer().await?;
        writer
            .execute("BEGIN IMMEDIATE", ())
            .await
            .map_err(map_err)?;
        Ok(Box::new(TursoTransaction {
            writer: Some(writer),
        }))
    }
}

/// A transaction on the writer, which it holds until it ends.
pub struct TursoTransaction {
    writer: Option<OwnedMutexGuard<Connection>>,
}

impl TursoTransaction {
    fn conn(&self) -> &Connection {
        self.writer.as_ref().expect("transaction in progress")
    }

    /// End the transaction. If that fails, the next use of the writer
    /// rolls it back.
    async fn finish(mut self, sql: &str) -> StorageResult<()> {
        let writer = self.writer.take().expect("transaction in progress");
        writer.execute(sql, ()).await.map(drop).map_err(map_err)
    }
}

#[async_trait]
impl Exec for TursoTransaction {
    async fn query(&self, sql: &str, values: &[Value]) -> StorageResult<Vec<Row>> {
        query(self.conn(), sql, values).await
    }

    async fn execute(&self, sql: &str, values: &[Value]) -> StorageResult<u64> {
        execute(self.conn(), sql, values).await
    }

    fn now_millis(&self) -> &'static str {
        NOW_MILLIS
    }

    fn for_update(&self) -> &'static str {
        ""
    }

    fn text_match(&self, columns: &[&str], param: &str) -> String {
        text_match(columns, param)
    }

    fn text_query(&self, text: &str) -> Option<String> {
        text_query(text)
    }
}

#[async_trait]
impl Transaction for TursoTransaction {
    async fn apply_ddl(&self, sql: &str) -> StorageResult<()> {
        self.conn().execute_batch(sql).await.map_err(map_err)
    }

    /// The transaction holds the only writer; Turso's file lock keeps other
    /// servers out.
    async fn lock_migrations(&self) -> StorageResult<()> {
        Ok(())
    }

    async fn commit(self: Box<Self>) -> StorageResult<()> {
        self.finish("COMMIT").await
    }

    async fn rollback(self: Box<Self>) -> StorageResult<()> {
        self.finish("ROLLBACK").await
    }
}

impl Drop for TursoTransaction {
    fn drop(&mut self) {
        // Roll back now rather than at the next write, when a runtime runs.
        if let Some(writer) = self.writer.take()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            runtime.spawn(async move {
                let _ = writer.execute("ROLLBACK", ()).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_outside_quotes() {
        assert_eq!(
            numbered("SELECT a FROM t WHERE b = $1 AND c = '$2' AND \"$3\" = $12"),
            "SELECT a FROM t WHERE b = ?1 AND c = '$2' AND \"$3\" = ?12"
        );
        assert!(matches!(numbered("SELECT 1"), Cow::Borrowed(_)));
    }

    #[test]
    fn text_queries_keep_words_only() {
        assert_eq!(
            text_query("Sales_ord (2024) a:b").as_deref(),
            Some("\"Sales_ord\" AND \"2024\"")
        );
        assert_eq!(text_query("\"-* a"), None);
    }

    /// The index finds a part of a word, ignores case, and follows writes;
    /// syntax of tantivy's queries is plain text.
    #[tokio::test]
    async fn full_text_index_finds_parts_of_words() {
        let db = TursoStorage::open(None).await.unwrap();
        let tx = db.begin().await.unwrap();
        tx.apply_ddl(
            "CREATE TABLE t (k TEXT PRIMARY KEY, name TEXT NOT NULL, comment TEXT);\n\
             INSERT INTO t VALUES ('a', 'sales_orders', NULL), ('b', 'words', 'Raw ORDER lines');\n\
             CREATE INDEX t_search ON t USING fts (name, comment) WITH (tokenizer = 'ngram');",
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let find = async |text: &str| {
            let sql = format!(
                "SELECT k FROM t WHERE {} ORDER BY k",
                db.text_match(&["name", "comment"], "$1")
            );
            let Some(query) = db.text_query(text) else {
                return vec![];
            };
            let rows = db.query(&sql, &[query.into()]).await.unwrap();
            rows.iter().map(|r| r.text(0).unwrap()).collect::<Vec<_>>()
        };
        assert_eq!(find("rders").await, ["a"]);
        assert_eq!(find("ORD").await, ["a", "b"]);
        assert_eq!(find("order lines").await, ["b"]);
        assert_eq!(find("sales lines").await, Vec::<String>::new());
        assert_eq!(find("(ord) -x:y").await, ["a", "b"]);

        db.execute("UPDATE t SET name = 'invoices' WHERE k = 'a'", &[])
            .await
            .unwrap();
        assert_eq!(find("rders").await, Vec::<String>::new());
        assert_eq!(find("voic").await, ["a"]);
    }

    #[tokio::test]
    async fn readers_see_commits_and_a_dropped_transaction_rolls_back() {
        let db = TursoStorage::open(None).await.unwrap();
        db.execute("CREATE TABLE t (k TEXT PRIMARY KEY, n BIGINT)", &[])
            .await
            .unwrap();
        let tx = db.begin().await.unwrap();
        tx.execute(
            "INSERT INTO t (k, n) VALUES ($1, $2)",
            &["a".into(), 1.into()],
        )
        .await
        .unwrap();
        assert!(db.query("SELECT k FROM t", &[]).await.unwrap().is_empty());
        tx.commit().await.unwrap();
        assert_eq!(db.query("SELECT k FROM t", &[]).await.unwrap().len(), 1);

        let tx = db.begin().await.unwrap();
        tx.execute("DELETE FROM t", &[]).await.unwrap();
        drop(tx);
        assert_eq!(
            db.execute("UPDATE t SET n = n + 1", &[]).await.unwrap(),
            1,
            "the dropped delete is gone"
        );
        let err = db
            .execute(
                "INSERT INTO t (k, n) VALUES ($1, $2)",
                &["a".into(), Value::Null],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StorageError::Unique(_)), "{err:?}");
    }
}
