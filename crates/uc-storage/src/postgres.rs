//! [`Storage`] on PostgreSQL: a pool of connections, each with its prepared
//! statements.
//!
//! The URL is libpq's (`postgresql://user:password@host:port/db?…`) or
//! pgJDBC's (`jdbc:postgresql://host:port/db?user=…`), as upstream's
//! `hibernate.connection.url`. `sslmode` takes libpq's values; certificates
//! are checked against `sslrootcert` (a PEM file) or the system's roots.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use bytes::BytesMut;
use deadpool::managed::{self, Metrics, Object, Pool, RecycleError, RecycleResult};
use sqlparser::ast::{
    self, AlterTableOperation, ColumnDef, ColumnOption, ColumnOptionDef, DataType, Ident,
    ObjectName,
};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use tokio_postgres::error::SqlState;
use tokio_postgres::types::{IsNull, ToSql, Type, to_sql_checked};
use tokio_postgres::{Client, NoTls, Statement};
use tracing::warn;

use super::tls::{MakeRustlsConnect, SslMode, TlsSettings};
use super::{Exec, Row, Storage, StorageError, StorageResult, Transaction, Value};

/// Upstream's pool size (`hibernate.hikari.maximumPoolSize`).
const DEFAULT_POOL_SIZE: usize = 20;
/// How long a request waits for a free connection, and a connection for the
/// server when the URL sets no `connect_timeout`.
const WAIT_TIMEOUT: Duration = Duration::from_secs(30);

const NOW_MILLIS: &str = "CAST(EXTRACT(EPOCH FROM clock_timestamp()) * 1000 AS BIGINT)";

/// A PostgreSQL server and how to sign in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresConfig {
    /// `postgres://`, `postgresql://` or `jdbc:postgresql://`.
    pub url: String,
    /// Override the URL's user and password (upstream
    /// `hibernate.connection.user` and `hibernate.connection.password`).
    pub user: Option<String>,
    pub password: Option<String>,
    /// Most connections at once; 20 by default.
    pub pool_size: Option<usize>,
    /// The schema of the catalog's tables, made when missing (pgJDBC's
    /// `currentSchema`). Without it, PostgreSQL's `search_path` decides.
    pub schema: Option<String>,
}

impl PostgresConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            user: None,
            password: None,
            pool_size: None,
            schema: None,
        }
    }

    /// Hosts, ports and database, without credentials.
    pub fn describe(&self) -> String {
        let parsed = match parse(self) {
            Ok(parsed) => parsed,
            Err(e) => return format!("(invalid URL: {e})"),
        };
        let hosts: Vec<String> = parsed
            .config
            .get_hosts()
            .iter()
            .map(|h| match h {
                tokio_postgres::config::Host::Tcp(h) => h.clone(),
                #[cfg(unix)]
                tokio_postgres::config::Host::Unix(p) => p.display().to_string(),
            })
            .collect();
        let ports: Vec<String> = match parsed.config.get_ports() {
            [] => vec!["5432".to_string()],
            ports => ports.iter().map(u16::to_string).collect(),
        };
        format!(
            "{}:{}/{}",
            hosts.join(","),
            ports.join(","),
            parsed.config.get_dbname().unwrap_or_default()
        )
    }
}

pub(super) fn is_url(url: &str) -> bool {
    ["postgres://", "postgresql://", "jdbc:postgresql://"]
        .iter()
        .any(|p| url.starts_with(p))
}

/// A parsed URL: the client settings, TLS, and the tables' schema.
struct Parsed {
    config: tokio_postgres::Config,
    tls: TlsSettings,
    schema: Option<String>,
}

fn decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

/// Read the URL: the parameters tokio-postgres does not know (certificate
/// checks, pgJDBC's names) are taken out; the rest goes to tokio-postgres.
fn parse(pg: &PostgresConfig) -> Result<Parsed, String> {
    let invalid = |why: String| format!("Invalid PostgreSQL URL: {why}");
    let (url, jdbc) = match pg.url.strip_prefix("jdbc:") {
        Some(rest) => (rest, true),
        None => (pg.url.as_str(), false),
    };
    if !is_url(url) {
        return Err(invalid("expected postgresql://…".into()));
    }
    let (head, query) = url.split_once('?').unwrap_or((url, ""));
    let mut kept: Vec<(String, String)> = Vec::new();
    let mut tls = TlsSettings::default();
    let mut mode = None;
    let mut jdbc_ssl = None;
    let mut schema = pg.schema.clone();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, raw) = pair.split_once('=').unwrap_or((pair, ""));
        let value = decode(raw);
        match key {
            "sslmode" => mode = Some(SslMode::parse(&value).map_err(invalid)?),
            "sslrootcert" => tls.root_cert = Some(value),
            "sslcert" | "sslkey" | "sslpassword" => {
                return Err(invalid(format!(
                    "{key}: client certificates are not supported"
                )));
            }
            // pgJDBC's names.
            "ssl" if jdbc => jdbc_ssl = Some(value == "true"),
            "currentSchema" if jdbc => schema = schema.or(Some(value)),
            "ApplicationName" if jdbc => kept.push(("application_name".into(), raw.into())),
            "connectTimeout" if jdbc => kept.push(("connect_timeout".into(), raw.into())),
            "user" | "password" | "options" | "application_name" | "connect_timeout" => {
                kept.push((key.into(), raw.into()))
            }
            other if jdbc => warn!(parameter = other, "ignoring a JDBC URL parameter"),
            other => kept.push((other.into(), raw.into())),
        }
    }
    // pgJDBC: `ssl=true` alone checks the certificate and host name.
    tls.mode = mode.unwrap_or(match jdbc_ssl {
        Some(true) => SslMode::VerifyFull,
        _ => SslMode::Prefer,
    });
    let query: Vec<String> = kept.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let rebuilt = match query.is_empty() {
        true => head.to_string(),
        false => format!("{head}?{}", query.join("&")),
    };
    let mut config: tokio_postgres::Config = rebuilt
        .parse()
        .map_err(|e: tokio_postgres::Error| invalid(e.to_string()))?;
    if let Some(user) = &pg.user {
        config.user(user);
    }
    if let Some(password) = &pg.password {
        config.password(password);
    }
    if config.get_user().is_none() {
        // As libpq: the operating system user.
        if let Ok(user) = std::env::var("USER") {
            config.user(user);
        }
    }
    config.ssl_mode(tls.mode.client_mode());
    if config.get_connect_timeout().is_none() {
        config.connect_timeout(WAIT_TIMEOUT);
    }
    if let Some(schema) = &schema {
        if schema.is_empty()
            || !schema
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(invalid(format!(
                "schema '{schema}': use letters, digits and _"
            )));
        }
        let options = config.get_options().unwrap_or_default().to_string();
        config.options(format!("{options} -c search_path={schema}").trim());
    }
    Ok(Parsed {
        config,
        tls,
        schema,
    })
}

/// The catalog's DDL as PostgreSQL runs it: text columns sort and compare
/// by byte (`COLLATE "C"`), as on Turso, whatever the database's locale.
/// sqlparser reads the SQL and writes it again, without its comments.
pub fn ddl(sql: &str) -> StorageResult<String> {
    let mut statements = Parser::parse_sql(&PostgreSqlDialect {}, sql)
        .map_err(|e| StorageError::Other(format!("cannot read the SQL: {e}")))?;
    for statement in &mut statements {
        match statement {
            ast::Statement::CreateTable(table) => {
                table.columns.iter_mut().for_each(collate_by_byte);
            }
            ast::Statement::AlterTable(alter) => {
                for operation in &mut alter.operations {
                    if let AlterTableOperation::AddColumn { column_def, .. } = operation {
                        collate_by_byte(column_def);
                    }
                }
            }
            _ => {}
        }
    }
    let statements: Vec<String> = statements.iter().map(ToString::to_string).collect();
    Ok(statements.join(";\n"))
}

/// `COLLATE "C"` for a text column, unless it names a collation.
fn collate_by_byte(column: &mut ColumnDef) {
    let text = matches!(
        column.data_type,
        DataType::Text
            | DataType::Varchar(_)
            | DataType::CharacterVarying(_)
            | DataType::CharVarying(_)
            | DataType::Char(_)
            | DataType::Character(_)
    );
    let collated = column
        .options
        .iter()
        .any(|o| matches!(o.option, ColumnOption::Collation(_)));
    if text && !collated {
        let c = ObjectName::from(vec![Ident::with_quote('"', "C")]);
        column.options.insert(
            0,
            ColumnOptionDef {
                name: None,
                option: ColumnOption::Collation(c),
            },
        );
    }
}

/// The key of the migration lock of the current schema: replicas that
/// share a schema migrate one at a time; other schemas do not wait.
fn migration_lock_key(schema: &str) -> i64 {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("unitycatalog-rs migrations {schema}").as_bytes());
    i64::from_be_bytes(digest[..8].try_into().expect("8 bytes"))
}

impl ToSql for Value {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match self {
            Value::Null => Ok(IsNull::Yes),
            Value::Int(n) => match *ty {
                Type::INT8 => n.to_sql(ty, out),
                Type::INT4 => i32::try_from(*n)?.to_sql(ty, out),
                Type::INT2 => i16::try_from(*n)?.to_sql(ty, out),
                _ => Err(format!("cannot bind an integer to a {ty} parameter").into()),
            },
            Value::Bool(b) => match *ty {
                Type::BOOL => b.to_sql(ty, out),
                _ => Err(format!("cannot bind a boolean to a {ty} parameter").into()),
            },
            Value::Text(s) => match *ty {
                Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::UNKNOWN => {
                    s.as_str().to_sql(ty, out)
                }
                _ => Err(format!("cannot bind text to a {ty} parameter").into()),
            },
        }
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    to_sql_checked!();
}

fn row(r: &tokio_postgres::Row) -> StorageResult<Row> {
    let get = |e: tokio_postgres::Error| StorageError::Other(e.to_string());
    let mut values = Vec::with_capacity(r.len());
    for (i, column) in r.columns().iter().enumerate() {
        let value = match *column.type_() {
            Type::BOOL => r.try_get::<_, Option<bool>>(i).map_err(get)?.into(),
            Type::INT2 => r
                .try_get::<_, Option<i16>>(i)
                .map_err(get)?
                .map(i64::from)
                .into(),
            Type::INT4 => r
                .try_get::<_, Option<i32>>(i)
                .map_err(get)?
                .map(i64::from)
                .into(),
            Type::INT8 => r.try_get::<_, Option<i64>>(i).map_err(get)?.into(),
            Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => {
                r.try_get::<_, Option<String>>(i).map_err(get)?.into()
            }
            ref other => {
                return Err(StorageError::Other(format!(
                    "column {} has the unsupported type {other}",
                    column.name()
                )));
            }
        };
        values.push(value);
    }
    Ok(Row(values))
}

fn map_err(e: tokio_postgres::Error) -> StorageError {
    let Some(db) = e.as_db_error() else {
        // The causes too: "error performing TLS handshake" alone says little.
        let mut message = e.to_string();
        let mut cause = std::error::Error::source(&e);
        while let Some(c) = cause {
            message = format!("{message}: {c}");
            cause = c.source();
        }
        return StorageError::Other(message);
    };
    let mut message = db.message().to_string();
    if let Some(detail) = db.detail() {
        message = format!("{message}: {detail}");
    }
    match *db.code() {
        SqlState::UNIQUE_VIOLATION => StorageError::Unique(message),
        SqlState::T_R_SERIALIZATION_FAILURE
        | SqlState::T_R_DEADLOCK_DETECTED
        | SqlState::LOCK_NOT_AVAILABLE => StorageError::Busy(message),
        _ => StorageError::Other(message),
    }
}

/// A connection and its prepared statements.
pub(super) struct PgConn {
    client: Client,
    statements: Mutex<HashMap<String, Statement>>,
    /// A transaction is open: set at `BEGIN`, cleared when it ends.
    in_transaction: AtomicBool,
}

impl PgConn {
    async fn statement(&self, sql: &str) -> StorageResult<Statement> {
        if let Some(s) = self.statements.lock().expect("statements lock").get(sql) {
            return Ok(s.clone());
        }
        let statement = self.client.prepare(sql).await.map_err(map_err)?;
        self.statements
            .lock()
            .expect("statements lock")
            .insert(sql.to_string(), statement.clone());
        Ok(statement)
    }

    async fn query(&self, sql: &str, values: &[Value]) -> StorageResult<Vec<Row>> {
        let statement = self.statement(sql).await?;
        let params: Vec<&(dyn ToSql + Sync)> =
            values.iter().map(|v| v as &(dyn ToSql + Sync)).collect();
        let rows = self
            .client
            .query(&statement, &params)
            .await
            .map_err(map_err)?;
        rows.iter().map(row).collect()
    }

    async fn execute(&self, sql: &str, values: &[Value]) -> StorageResult<u64> {
        let statement = self.statement(sql).await?;
        let params: Vec<&(dyn ToSql + Sync)> =
            values.iter().map(|v| v as &(dyn ToSql + Sync)).collect();
        self.client
            .execute(&statement, &params)
            .await
            .map_err(map_err)
    }

    async fn batch(&self, sql: &str) -> StorageResult<()> {
        self.client.batch_execute(sql).await.map_err(map_err)
    }
}

pub(super) struct Manager {
    config: tokio_postgres::Config,
    tls: Option<MakeRustlsConnect>,
}

/// Connect, and drive the connection in a task of the current runtime.
async fn connect(
    config: tokio_postgres::Config,
    tls: Option<MakeRustlsConnect>,
) -> StorageResult<Client> {
    let client = match tls {
        None => {
            let (client, connection) = config.connect(NoTls).await.map_err(map_err)?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    warn!("PostgreSQL connection closed: {e}");
                }
            });
            client
        }
        Some(tls) => {
            let (client, connection) = config.connect(tls).await.map_err(map_err)?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    warn!("PostgreSQL connection closed: {e}");
                }
            });
            client
        }
    };
    Ok(client)
}

impl managed::Manager for Manager {
    type Type = PgConn;
    type Error = StorageError;

    /// A connection whose connection task runs on the current runtime. The
    /// connect future is boxed, as `Pool::get` would hold all of it on each
    /// call.
    async fn create(&self) -> StorageResult<PgConn> {
        let client = Box::pin(connect(self.config.clone(), self.tls.clone())).await?;
        Ok(PgConn {
            client,
            statements: Mutex::new(HashMap::new()),
            in_transaction: AtomicBool::new(false),
        })
    }

    async fn recycle(&self, conn: &mut PgConn, _: &Metrics) -> RecycleResult<StorageError> {
        if conn.client.is_closed() {
            return Err(RecycleError::message("connection closed"));
        }
        // A transaction whose `PostgresTransaction` was dropped part way.
        if conn.in_transaction.load(Ordering::Acquire) {
            conn.batch("ROLLBACK")
                .await
                .map_err(RecycleError::Backend)?;
            conn.in_transaction.store(false, Ordering::Release);
        }
        Ok(())
    }
}

/// PostgreSQL, through a pool of connections.
pub struct PostgresStorage {
    pool: Pool<Manager>,
}

impl PostgresStorage {
    pub async fn open(pg: &PostgresConfig) -> StorageResult<Self> {
        let parsed = parse(pg).map_err(StorageError::Other)?;
        let tls = super::tls::connector(&parsed.tls).map_err(StorageError::Other)?;
        let manager = Manager {
            config: parsed.config,
            tls,
        };
        let pool = Pool::builder(manager)
            .max_size(pg.pool_size.unwrap_or(DEFAULT_POOL_SIZE).max(1))
            .wait_timeout(Some(WAIT_TIMEOUT))
            .runtime(deadpool::Runtime::Tokio1)
            .build()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        let storage = Self { pool };
        // Connect once now, so that a wrong URL fails at start.
        let conn = storage.conn().await?;
        if let Some(schema) = &parsed.schema {
            conn.batch(&format!("CREATE SCHEMA IF NOT EXISTS {schema}"))
                .await?;
        }
        Ok(storage)
    }

    async fn conn(&self) -> StorageResult<Object<Manager>> {
        self.pool.get().await.map_err(|e| match e {
            managed::PoolError::Backend(e) => e,
            e => StorageError::Other(format!("PostgreSQL pool: {e}")),
        })
    }
}

#[async_trait]
impl Exec for PostgresStorage {
    async fn query(&self, sql: &str, values: &[Value]) -> StorageResult<Vec<Row>> {
        self.conn().await?.query(sql, values).await
    }

    async fn execute(&self, sql: &str, values: &[Value]) -> StorageResult<u64> {
        self.conn().await?.execute(sql, values).await
    }

    fn now_millis(&self) -> &'static str {
        NOW_MILLIS
    }

    fn for_update(&self) -> &'static str {
        " FOR UPDATE"
    }
}

impl PostgresStorage {
    async fn begin_with(&self, sql: &str) -> StorageResult<Box<dyn Transaction>> {
        let conn = self.conn().await?;
        conn.in_transaction.store(true, Ordering::Release);
        conn.batch(sql).await?;
        Ok(Box::new(PostgresTransaction { conn: Some(conn) }))
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn begin(&self) -> StorageResult<Box<dyn Transaction>> {
        self.begin_with("BEGIN").await
    }

    /// Read committed (the default) takes a snapshot per statement.
    async fn begin_snapshot(&self) -> StorageResult<Box<dyn Transaction>> {
        self.begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .await
    }
}

/// A transaction on one pooled connection.
pub struct PostgresTransaction {
    conn: Option<Object<Manager>>,
}

impl PostgresTransaction {
    fn conn(&self) -> &PgConn {
        self.conn.as_ref().expect("transaction in progress")
    }

    /// End the transaction. PostgreSQL ends it even when `COMMIT` fails.
    async fn finish(mut self, sql: &str) -> StorageResult<()> {
        let conn = self.conn.take().expect("transaction in progress");
        let result = conn.batch(sql).await;
        conn.in_transaction.store(false, Ordering::Release);
        result
    }
}

#[async_trait]
impl Exec for PostgresTransaction {
    async fn query(&self, sql: &str, values: &[Value]) -> StorageResult<Vec<Row>> {
        self.conn().query(sql, values).await
    }

    async fn execute(&self, sql: &str, values: &[Value]) -> StorageResult<u64> {
        self.conn().execute(sql, values).await
    }

    fn now_millis(&self) -> &'static str {
        NOW_MILLIS
    }

    fn for_update(&self) -> &'static str {
        " FOR UPDATE"
    }
}

#[async_trait]
impl Transaction for PostgresTransaction {
    async fn apply_ddl(&self, sql: &str) -> StorageResult<()> {
        self.conn().batch(&ddl(sql)?).await
    }

    /// A transaction-level advisory lock, released when the transaction
    /// ends, or when the server dies.
    async fn lock_migrations(&self) -> StorageResult<()> {
        let rows = self.query("SELECT current_schema()", &[]).await?;
        let schema = rows
            .first()
            .map(|r| r.opt_text(0))
            .transpose()?
            .flatten()
            .ok_or_else(|| {
                StorageError::Other(
                    "no schema to create the catalog's tables in: check search_path".into(),
                )
            })?;
        self.execute(
            "SELECT pg_advisory_xact_lock($1)",
            &[Value::Int(migration_lock_key(&schema))],
        )
        .await
        .map(drop)
    }

    async fn commit(self: Box<Self>) -> StorageResult<()> {
        self.finish("COMMIT").await
    }

    async fn rollback(self: Box<Self>) -> StorageResult<()> {
        self.finish("ROLLBACK").await
    }
}

impl Drop for PostgresTransaction {
    fn drop(&mut self) {
        // Roll back now rather than when the pool hands the connection out.
        if let Some(conn) = self.conn.take()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            runtime.spawn(async move {
                if conn.batch("ROLLBACK").await.is_ok() {
                    conn.in_transaction.store(false, Ordering::Release);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(url: &str) -> Parsed {
        parse(&PostgresConfig::new(url)).unwrap()
    }

    #[test]
    fn reads_libpq_urls() {
        let p = parsed(
            "postgresql://uc:secret@db.example.com:6543/ucdb?sslmode=verify-full\
             &sslrootcert=/etc/ca.pem&application_name=uc",
        );
        assert_eq!(p.config.get_user(), Some("uc"));
        assert_eq!(p.config.get_password(), Some(&b"secret"[..]));
        assert_eq!(p.config.get_dbname(), Some("ucdb"));
        assert_eq!(p.config.get_ports(), [6543]);
        assert_eq!(p.config.get_application_name(), Some("uc"));
        assert_eq!(p.tls.mode, SslMode::VerifyFull);
        assert_eq!(p.tls.root_cert.as_deref(), Some("/etc/ca.pem"));
        assert_eq!(parsed("postgres://h/db").tls.mode, SslMode::Prefer);
    }

    #[test]
    fn reads_jdbc_urls() {
        let mut pg = PostgresConfig::new(
            "jdbc:postgresql://db:5432/ucdb?sslmode=require&currentSchema=uc\
             &ApplicationName=x&sslfactory=y",
        );
        pg.user = Some("postgres".into());
        pg.password = Some("pw".into());
        let p = parse(&pg).unwrap();
        assert_eq!(p.config.get_user(), Some("postgres"));
        assert_eq!(p.config.get_password(), Some(&b"pw"[..]));
        assert_eq!(p.config.get_dbname(), Some("ucdb"));
        assert_eq!(p.config.get_application_name(), Some("x"));
        assert_eq!(p.config.get_options(), Some("-c search_path=uc"));
        assert_eq!(p.schema.as_deref(), Some("uc"));
        assert_eq!(p.tls.mode, SslMode::Require);
        assert_eq!(
            parsed("jdbc:postgresql://db/ucdb?ssl=true").tls.mode,
            SslMode::VerifyFull
        );
        assert!(parse(&PostgresConfig::new("jdbc:postgresql://db/x?sslcert=a")).is_err());
        assert!(
            parse(&PostgresConfig::new(
                "jdbc:postgresql://db/x?currentSchema=a%3Bdrop"
            ))
            .is_err(),
            "a schema is a plain name"
        );
    }

    #[test]
    fn describes_without_credentials() {
        let pg = PostgresConfig::new("postgresql://uc:secret@db:5432/ucdb");
        assert_eq!(pg.describe(), "db:5432/ucdb");
        assert_eq!(PostgresConfig::new("postgres://h/x").describe(), "h:5432/x");
    }

    #[test]
    fn ddl_collates_text_by_byte() {
        let sql = "CREATE TABLE t (a TEXT NOT NULL, b BIGINT, c varchar(3), \
                   d TEXT COLLATE \"en_US\");\n\
                   -- the metastore's TEXT\n\
                   ALTER TABLE t ADD COLUMN e text DEFAULT 'TEXT';\n\
                   INSERT INTO t (a) VALUES ('TEXT');";
        assert_eq!(
            ddl(sql).unwrap(),
            "CREATE TABLE t (a TEXT COLLATE \"C\" NOT NULL, b BIGINT, \
             c VARCHAR(3) COLLATE \"C\", d TEXT COLLATE \"en_US\");\n\
             ALTER TABLE t ADD COLUMN e TEXT COLLATE \"C\" DEFAULT 'TEXT';\n\
             INSERT INTO t (a) VALUES ('TEXT')"
        );
        assert!(ddl("CREATE TABLE (").is_err());
    }
}
