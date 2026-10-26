//! [`crate::Metastore`] in SQL, on any [`Storage`]: Turso or PostgreSQL.
//!
//! [`SqlMetastore`] and the helpers its implementations share: error mapping,
//! enum round-trips, rows ([`rows`]) and pagination. Each resource keeps
//! its implementation next to its trait, in `<resource>/sql.rs`.

pub(crate) mod rows;

use std::sync::Arc;

use serde::{Serialize, de::DeserializeOwned};
use tracing::debug;

use uc_common::{Error, ErrorCode, ManagedResource, Pagination, Result, three_part_full_name};

use crate::config::{ManagedStorageConfig, MetastoreConfig, TableSettings};
use crate::migrations::run_migrations;
use crate::{
    AuditLogOps, BackupOps, CatalogOps, CleanupOps, CredentialOps, DeltaTableOps,
    ExternalLocationOps, FunctionOps, IcebergTableOps, Metastore, ModelOps, PermissionOps,
    SchemaOps, ScimOps, SearchOps, SharingOps, TableOps, TokenOps, VolumeOps,
};
use rows::TableDef;
use uc_storage::{Exec, Storage, StorageError, StorageResult, Value};

/// The catalog in SQL.
pub struct SqlMetastore {
    pub(crate) db: Arc<dyn Storage>,
    pub(crate) roots: ManagedStorageConfig,
    pub(crate) tables: TableSettings,
}

impl SqlMetastore {
    /// Open the database `cfg` names and bring its tables up to date.
    pub async fn connect(cfg: MetastoreConfig) -> Result<Arc<Self>> {
        let described = cfg.database.describe();
        debug!(database = %described, "opening the catalog database");
        let db = uc_storage::open(&cfg.database)
            .await
            .map_err(|e| Error::internal(format!("cannot open {described}: {e}")))?;
        Self::new(db, cfg.storage, cfg.tables).await
    }

    /// The catalog on `db`, any [`Storage`], with its tables brought up to
    /// date.
    pub async fn new(
        db: Arc<dyn Storage>,
        roots: ManagedStorageConfig,
        tables: TableSettings,
    ) -> Result<Arc<Self>> {
        run_migrations(db.as_ref()).await?;
        Ok(Arc::new(Self { db, roots, tables }))
    }

    /// Upstream `checkManagedTableEnabled`.
    pub(crate) fn check_managed_enabled(&self) -> Result<()> {
        if !self.tables.managed_tables_enabled {
            return Err(Error::invalid_argument(
                "MANAGED table is an is currently disabled. To enable it, set \
                 tables.managed = true (UC_TABLES_MANAGED)",
            ));
        }
        Ok(())
    }
}

impl Metastore for SqlMetastore {
    fn catalogs(&self) -> &dyn CatalogOps {
        self
    }
    fn schemas(&self) -> &dyn SchemaOps {
        self
    }
    fn tables(&self) -> &dyn TableOps {
        self
    }
    fn delta_tables(&self) -> &dyn DeltaTableOps {
        self
    }
    fn iceberg_tables(&self) -> &dyn IcebergTableOps {
        self
    }
    fn volumes(&self) -> &dyn VolumeOps {
        self
    }
    fn functions(&self) -> &dyn FunctionOps {
        self
    }
    fn models(&self) -> &dyn ModelOps {
        self
    }
    fn credentials(&self) -> &dyn CredentialOps {
        self
    }
    fn external_locations(&self) -> &dyn ExternalLocationOps {
        self
    }
    fn permissions(&self) -> &dyn PermissionOps {
        self
    }
    fn tokens(&self) -> &dyn TokenOps {
        self
    }
    fn scim(&self) -> &dyn ScimOps {
        self
    }
    fn search(&self) -> &dyn SearchOps {
        self
    }
    fn audit_log(&self) -> &dyn AuditLogOps {
        self
    }
    fn backups(&self) -> &dyn BackupOps {
        self
    }
    fn sharing(&self) -> &dyn SharingOps {
        self
    }
    fn cleanup(&self) -> &dyn CleanupOps {
        self
    }
}

pub(crate) fn db_err(context: &'static str) -> impl Fn(StorageError) -> Error {
    move |e| Error::internal(format!("{context}: {e}"))
}

/// A unique key that already holds the value is `ALREADY_EXISTS`.
pub(crate) fn create_err(e: StorageError) -> Error {
    match e {
        StorageError::Unique(m) => Error::already_exists(m),
        e => Error::internal(format!("create: {e}")),
    }
}

/// Like [`create_err`], but with the resource's upstream code (e.g.
/// `CATALOG_ALREADY_EXISTS`) and message.
pub(crate) fn create_err_as(code: ErrorCode, msg: String) -> impl FnOnce(StorageError) -> Error {
    move |e| match e {
        StorageError::Unique(_) => Error::new(code, msg),
        e => Error::internal(format!("create: {e}")),
    }
}

/// The rows of `t` that `tail` (`WHERE …`, `ORDER BY …`, `LIMIT …`) picks.
pub(crate) async fn select<R, E>(
    ex: &E,
    t: &TableDef,
    tail: &str,
    params: &[Value],
) -> Result<Vec<R>>
where
    R: DeserializeOwned,
    E: Exec + ?Sized,
{
    let sql = format!("{} {tail}", t.select());
    let rows = ex.query(&sql, params).await.map_err(db_err(t.name))?;
    // Not `collect`: through `Result`, it does not know the length.
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(t.decode(r)?);
    }
    Ok(out)
}

/// The first row of [`select`].
pub(crate) async fn select_one<R, E>(
    ex: &E,
    t: &TableDef,
    tail: &str,
    params: &[Value],
) -> Result<Option<R>>
where
    R: DeserializeOwned,
    E: Exec + ?Sized,
{
    Ok(select(ex, t, tail, params).await?.into_iter().next())
}

/// The row of `t` with id `id`.
pub(crate) async fn by_id<R, E>(ex: &E, t: &TableDef, id: &str) -> Result<Option<R>>
where
    R: DeserializeOwned,
    E: Exec + ?Sized,
{
    select_one(ex, t, "WHERE id_string = $1", &[id.into()]).await
}

/// Insert `row` into `t`.
pub(crate) async fn insert<R, E>(ex: &E, t: &TableDef, row: &R) -> StorageResult<()>
where
    R: Serialize,
    E: Exec + ?Sized,
{
    let values = t
        .encode(row)
        .map_err(|e| StorageError::Other(e.message().to_string()))?;
    ex.execute(&t.insert(), &values).await.map(drop)
}

/// The most rows in one statement of [`insert_all`].
const INSERT_ALL_ROWS: usize = 128;

/// Insert `rows` into `t`, in statements of 128, 64, 32, … or 1 rows: a
/// connection prepares at most eight such statements. Each statement
/// commits on its own.
pub(crate) async fn insert_all<R, E>(ex: &E, t: &TableDef, rows: &[R]) -> StorageResult<()>
where
    R: Serialize,
    E: Exec + ?Sized,
{
    let mut rest = rows;
    while !rest.is_empty() {
        let n = 1 << rest.len().min(INSERT_ALL_ROWS).ilog2();
        let (chunk, tail) = rest.split_at(n);
        let mut values = Vec::with_capacity(n * t.columns.len());
        for row in chunk {
            values.extend(
                t.encode(row)
                    .map_err(|e| StorageError::Other(e.message().to_string()))?,
            );
        }
        ex.execute(&t.insert_rows(n), &values).await?;
        rest = tail;
    }
    Ok(())
}

/// Write every column of `row` to the row of `t` with its key; the number
/// of rows written (0 when it is gone).
pub(crate) async fn update<R, E>(ex: &E, t: &TableDef, row: &R) -> StorageResult<u64>
where
    R: Serialize,
    E: Exec + ?Sized,
{
    let values = t
        .encode(row)
        .map_err(|e| StorageError::Other(e.message().to_string()))?;
    ex.execute(&t.update(), &values).await
}

/// `v` as JSON text, for a JSON column.
pub(crate) fn json_text<T: Serialize + ?Sized>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|e| Error::internal(e.to_string()))
}

/// `$first, $first+1, …` for `n` values bound from position `first`.
pub(crate) fn placeholders(first: usize, n: usize) -> String {
    let all: Vec<String> = (first..first + n).map(|i| format!("${i}")).collect();
    all.join(", ")
}

/// SQL that holds when `column` starts with the text in parameter
/// `prefix`, whose length in characters is parameter `len`.
pub(crate) fn starts_with(column: &str, prefix: usize, len: usize) -> String {
    format!("substr({column}, 1, ${len}) = ${prefix}")
}

/// The length in characters of `s`, as [`starts_with`] wants it.
pub(crate) fn char_len(s: &str) -> Value {
    Value::Int(s.chars().count() as i64)
}

/// Java `String.split("\\.")`, which upstream uses on full names: split on
/// `.` and drop trailing empty parts.
pub(crate) fn split_dotted(s: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = s.split('.').collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

/// `<catalog>.<schema>`; anything else is upstream's `INVALID_ARGUMENT`.
pub(crate) fn split_schema_name(full: &str) -> Result<(&str, &str)> {
    match split_dotted(full).as_slice() {
        [c, s] => Ok((c, s)),
        _ => Err(Error::invalid_argument(format!(
            "Invalid schema full name: {full}"
        ))),
    }
}

/// `<catalog>.<schema>.<name>`; anything else is upstream's
/// `INVALID_ARGUMENT` "Invalid <what> name: <full>".
pub(crate) fn split_three_part<'a>(
    full: &'a str,
    what: &str,
) -> Result<(&'a str, &'a str, &'a str)> {
    match split_dotted(full).as_slice() {
        [c, s, n] => Ok((c, s, n)),
        _ => Err(Error::invalid_argument(format!(
            "Invalid {what} name: {full}"
        ))),
    }
}

/// A resolved schema: the ids that rows link to, plus the current names
/// used to build full names in responses.
#[derive(Debug, Clone)]
pub(crate) struct SchemaRef {
    pub(crate) catalog_name: String,
    pub(crate) schema_id: String,
    pub(crate) schema_name: String,
    /// Managed storage location of the schema, then of the catalog.
    pub(crate) schema_storage_location: Option<String>,
    pub(crate) catalog_storage_location: Option<String>,
}

impl SchemaRef {
    /// Upstream `ExternalLocationUtils.getManagedStorageLocation`: where
    /// managed children of this schema go — the schema's storage location,
    /// else the catalog's, else `<root>/__unitystorage` with the server's
    /// root for the `child`'s kind: `storage.tables_root` for tables,
    /// `storage.models_root` for models, and none for volumes, as upstream.
    pub(crate) fn managed_parent_location(
        &self,
        roots: &ManagedStorageConfig,
        child: ManagedResource,
    ) -> Result<uc_common::NormalizedUrl> {
        if let Some(loc) = self.schema_storage_location.as_deref() {
            return uc_common::NormalizedUrl::parse(loc);
        }
        if let Some(loc) = self.catalog_storage_location.as_deref() {
            return uc_common::NormalizedUrl::parse(loc);
        }
        // Upstream's message names its `storage-root.tables` property for
        // every kind; this one names the setting that applies.
        let (root, setting) = match child {
            ManagedResource::Table => (roots.tables_root.as_ref(), "storage.tables_root"),
            ManagedResource::RegisteredModel | ManagedResource::ModelVersion => {
                (roots.models_root.as_ref(), "storage.models_root")
            }
            ManagedResource::Volume => {
                return Err(Error::failed_precondition(
                    "None of catalog or schema has managed location configured.",
                ));
            }
        };
        match root {
            Some(root) => root.managed_fallback_root(),
            None => Err(Error::failed_precondition(format!(
                "None of catalog, schema or {setting} server setting has managed location \
                 configured."
            ))),
        }
    }

    pub(crate) fn child_full_name(&self, name: &str) -> String {
        three_part_full_name(&self.catalog_name, &self.schema_name, name)
    }
}

/// Upstream `FileOperations.createStorageLocationDir`: create the directory
/// of a local (`file://`) storage location. Cloud locations are left alone
/// (upstream does not create cloud directories either).
pub(crate) fn create_storage_dir(url: &uc_common::NormalizedUrl) -> Result<()> {
    if url.scheme() != uc_common::UriScheme::File {
        return Ok(());
    }
    let path = url.local_path();
    if std::fs::symlink_metadata(&path).is_ok() {
        return Err(Error::already_exists(format!(
            "Directory already exists: {}",
            path.display()
        )));
    }
    std::fs::create_dir_all(&path).map_err(|e| {
        Error::internal(format!(
            "Failed to create directory: {}: {e}",
            path.display()
        ))
    })
}

/// Stamp the `(updated_at, updated_by)` pair on a row being persisted via
/// an update.
pub(crate) fn touch_audit(
    updated_at: &mut i64,
    updated_by: &mut Option<String>,
    actor: Option<&str>,
) {
    *updated_at = uc_common::now_epoch_millis();
    *updated_by = actor.map(str::to_string);
}

/// A 64-hex-character token: the bytes of two v4 UUIDs, from `getrandom`.
pub(crate) fn random_token() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    hex::encode(bytes)
}

pub(crate) fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(s.as_bytes()))
}

// Spec enums derive `Serialize` and `Deserialize` with a rename strategy, so
// serde_json gives their wire strings.

/// The wire string of a spec enum: its JSON string without the quotes.
pub(crate) fn enum_str<T: Serialize>(v: &T) -> String {
    let mut s = serde_json::to_string(v).expect("spec enum serializes as string");
    debug_assert!(s.starts_with('"') && s.ends_with('"'));
    s.truncate(s.len() - 1);
    s.drain(..1);
    s
}

pub(crate) fn parse_enum<T: DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str::<T>(&format!("\"{s}\""))
        .map_err(|e| Error::internal(format!("parse enum '{s}': {e}")))
}

impl SqlMetastore {
    /// Upstream `validateNotOverlapWithManagedStorage`: an external location
    /// for data may not be inside `__unitystorage`, nor contain the managed
    /// storage of a catalog or schema.
    pub(crate) async fn validate_not_overlap_with_managed_storage(
        &self,
        url: &uc_common::NormalizedUrl,
    ) -> Result<()> {
        url.validate_not_under_managed_prefix()?;
        let prefix = format!("{url}/");
        // Staging tables can outlive a deleted catalog or schema, so
        // uncommitted ones count on their own.
        let sql = format!(
            "SELECT \
             CASE WHEN EXISTS (SELECT 1 FROM catalog WHERE {managed}) \
               OR EXISTS (SELECT 1 FROM schema WHERE {managed}) THEN 1 ELSE 0 END, \
             CASE WHEN EXISTS (SELECT 1 FROM staging_table \
               WHERE stage_committed = $3 AND {staging}) THEN 1 ELSE 0 END",
            managed = starts_with("storage_location", 1, 2),
            staging = starts_with("staging_location", 1, 2),
        );
        let params = [prefix.as_str().into(), char_len(&prefix), false.into()];
        let failed = db_err("storage overlap");
        let rows = self.db.query(&sql, &params).await.map_err(&failed)?;
        let row = rows
            .first()
            .ok_or_else(|| Error::internal("storage overlap: no row"))?;
        if row.int(0).map_err(&failed)? == 1 {
            return Err(Error::invalid_argument(format!(
                "Input path '{url}' overlaps with managed storage."
            )));
        }
        if row.int(1).map_err(&failed)? == 1 {
            return Err(Error::invalid_argument(format!(
                "Input path '{url}' overlaps with staging table."
            )));
        }
        Ok(())
    }

    /// Delete the row `id` of `table`, or fail with `not_found` when there
    /// is none.
    pub(crate) async fn delete_by_id(
        &self,
        table: &str,
        id: &str,
        not_found: impl FnOnce() -> Error,
    ) -> Result<()> {
        let removed = self
            .db
            .execute(
                &format!("DELETE FROM {table} WHERE id_string = $1"),
                &[id.into()],
            )
            .await
            .map_err(db_err("delete"))?;
        if removed == 0 {
            return Err(not_found());
        }
        Ok(())
    }

    /// Whether a row of `table` has `field = value`.
    pub(crate) async fn exists_where(&self, table: &str, field: &str, value: &str) -> Result<bool> {
        let sql = format!("SELECT 1 FROM {table} WHERE {field} = $1 LIMIT 1");
        let rows = self
            .db
            .query(&sql, &[value.into()])
            .await
            .map_err(db_err("exists"))?;
        Ok(!rows.is_empty())
    }

    /// Upstream `PagedListingHelper`: the rows of `t` that match `filters`
    /// (joined with `AND`, binding `params` from `$1`), by `sort_key`, one
    /// page, resuming after `page_token`. The next token is the key of the
    /// last row when the page is full.
    pub(crate) async fn paged_list<R, K>(
        &self,
        t: &TableDef,
        filters: &[&str],
        mut params: Vec<Value>,
        sort_key: &str,
        page: &Pagination,
        token_for: impl Fn(&R) -> K,
    ) -> Result<(Vec<R>, Option<String>)>
    where
        R: DeserializeOwned,
        K: ToString,
    {
        let limit = page.page_size()?;
        let mut filters: Vec<String> = filters.iter().map(|f| f.to_string()).collect();
        if let Some(token) = page.page_token.as_ref() {
            params.push(token.into());
            filters.push(format!("{sort_key} > ${}", params.len()));
        }
        let where_sql = match filters.is_empty() {
            true => String::new(),
            false => format!("WHERE {}", filters.join(" AND ")),
        };
        params.push(limit.into());
        let tail = format!("{where_sql} ORDER BY {sort_key} LIMIT ${}", params.len());
        let rows: Vec<R> = select(self.db.as_ref(), t, &tail, &params).await?;
        let next = page.next_token(&rows, token_for)?;
        Ok((rows, next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every column the metastore reads exists on both engines.
    #[tokio::test]
    async fn tables_have_their_columns() {
        let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
            .await
            .unwrap();
        for t in rows::ALL {
            let sql = format!("{} LIMIT 0", t.select());
            metastore
                .db
                .query(&sql, &[])
                .await
                .unwrap_or_else(|e| panic!("{}: {e}", t.name));
        }
    }

    #[test]
    fn helpers_number_parameters() {
        assert_eq!(placeholders(3, 3), "$3, $4, $5");
        assert_eq!(starts_with("url", 1, 2), "substr(url, 1, $2) = $1");
        assert_eq!(char_len("s3://é/"), Value::Int(7));
    }
}
