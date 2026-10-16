//! `table create`, `read` and `write` with the Delta kernel, as upstream's
//! `DeltaKernelUtils` with the Java kernel. MANAGED tables are
//! catalog-managed: their commits go through the server's Delta commits API.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anyhow::{Context as _, anyhow, bail};
use arrow::util::pretty::pretty_format_batches_with_schema;
use delta_kernel::arrow::array::{
    ArrayRef, BinaryArray, BooleanArray, Date32Array, Decimal128Array, Float32Array, Float64Array,
    Int8Array, Int16Array, Int32Array, Int64Array, RecordBatch, StringArray,
    TimestampMicrosecondArray,
};
use delta_kernel::arrow::datatypes::{DataType as ArrowType, Schema as ArrowSchema};
use delta_kernel::committer::{
    CommitMetadata, CommitResponse, CommitType, Committer, FileSystemCommitter, PublishMetadata,
};
use delta_kernel::engine::arrow_conversion::TryIntoArrow as _;
use delta_kernel::engine::arrow_data::ArrowEngineData;
use delta_kernel::schema::{DataType, PrimitiveType, StructField, StructType};
use delta_kernel::transaction::CommitResult;
use delta_kernel::transaction::create_table::create_table as kernel_create_table;
use delta_kernel::{
    DeltaResult, DeltaResultIterator, Engine, Error as KernelError, FileMeta, FilteredEngineData,
    LogPath, Snapshot, SnapshotRef,
};
use delta_kernel_default_engine::executor::tokio::TokioBackgroundExecutor;
use delta_kernel_default_engine::{DefaultEngine, DefaultEngineBuilder};
use rand::Rng as _;
use rand::rngs::ThreadRng;
use uc_api::{
    ColumnInfo, ColumnTypeName, CreateStagingTable, CreateTable, DataSourceFormat, DeltaCommit,
    DeltaCommitInfo, DeltaGetCommits, PathOperation, TableInfo, TableOperation, TableType,
    TemporaryCredentials,
};
use uc_client::Client;
use url::Url;

use super::storage::{S3Store, directory_url, path_credentials, store_for, table_credentials};
use crate::commands::columns::parse_columns;
use crate::commands::options::{ThreePartName, wire_name};
use crate::commands::table::TableCreate;
use crate::commands::{Context, upstream};
use crate::output::models::{self, shape};

const ENGINE_INFO: &str = "UnityCatalogCli";
const UC_TABLE_ID: &str = "io.unitycatalog.tableId";

type Kernel = DefaultEngine<TokioBackgroundExecutor>;

fn engine(
    url: &Url,
    credentials: Option<&TemporaryCredentials>,
    s3: Option<&S3Store>,
) -> anyhow::Result<Kernel> {
    Ok(DefaultEngineBuilder::new(store_for(url, credentials, s3)?).build())
}

/// Upstream `findBasicTypeFromString`: the Delta type of a column.
fn data_type(column: &ColumnInfo) -> anyhow::Result<DataType> {
    let name = column.name.as_deref().unwrap_or_default();
    let type_name = column
        .type_name
        .ok_or_else(|| anyhow!("Column type is missing: {name}"))?;
    Ok(match type_name {
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
        ColumnTypeName::Decimal => {
            let precision = column.type_precision.filter(|p| *p > 0);
            DataType::decimal(
                precision.map_or(10, |p| p as u8),
                column.type_scale.map_or(0, |s| s as u8),
            )?
        }
        other => bail!("Unsupported basic data type: {}", wire_name(&other)),
    })
}

fn schema(columns: &[ColumnInfo]) -> anyhow::Result<Arc<StructType>> {
    let fields = columns
        .iter()
        .map(|c| {
            let name = c.name.as_deref().unwrap_or_default();
            Ok(StructField::nullable(name, data_type(c)?))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Arc::new(StructType::try_new(fields)?))
}

/// Upstream `FileOperations.toStandardizedURIString`: a URL as is, a local
/// path as an absolute `file://` URL.
fn standardized_location(input: &str) -> anyhow::Result<String> {
    if let Ok(url) = Url::parse(input) {
        // A one-letter scheme is a Windows drive, not a URL.
        if url.scheme().len() > 1 {
            return match url.scheme() {
                // `Url` writes `file:/x` as `file:///x`, as upstream's
                // `adjustLocalFileURI`.
                "file" => Ok(url.to_string()),
                "s3" | "gs" | "abfs" | "abfss" => Ok(input.to_string()),
                other => bail!("Unsupported URI scheme: {other}"),
            };
        }
    }
    let invalid =
        || format!("Storage location must be a valid URL or local filesystem path: {input}");
    let path = std::path::absolute(input).with_context(invalid)?;
    let url = Url::from_file_path(&path).map_err(|_| anyhow!(invalid()))?;
    Ok(url.to_string())
}

pub async fn create_table(c: &Context<'_>, create: TableCreate) -> anyhow::Result<String> {
    let TableCreate {
        full_name,
        columns,
        format,
        properties,
        storage_location,
        table_type,
    } = create;
    let ThreePartName {
        catalog,
        schema: schema_name,
        name,
    } = full_name;
    let columns = parse_columns(&columns)?;
    let mut properties: Option<HashMap<String, String>> =
        properties.map(|map| map.into_iter().collect());

    let (location, credentials, staging_id) = match table_type {
        TableType::External => {
            let Some(input) = storage_location else {
                bail!("Storage location is required for external tables");
            };
            let location = standardized_location(&input)?;
            // Upstream skips vending here, so only local paths work there.
            let credentials = if location.starts_with("file:") {
                None
            } else {
                Some(path_credentials(c, &location, PathOperation::PathCreateTable).await?)
            };
            (location, credentials, None)
        }
        TableType::Managed => {
            if storage_location.is_some() {
                bail!("Storage location is not allowed for managed tables");
            }
            if format != DataSourceFormat::Delta {
                bail!(
                    "Only Delta tables are supported for managed tables: {}",
                    wire_name(&format)
                );
            }
            let request = CreateStagingTable {
                name: name.clone(),
                catalog_name: catalog.clone(),
                schema_name: schema_name.clone(),
            };
            let staging = c
                .client
                .create_staging_table(&request)
                .await
                .map_err(upstream("createStagingTable"))?;
            let (Some(id), Some(location)) = (staging.id, staging.staging_location) else {
                bail!("Failed to create staging table");
            };
            let credentials = table_credentials(c, &id, TableOperation::ReadWrite).await?;
            (location, Some(credentials), Some(id))
        }
        other => unreachable!("--table_type {other:?}"),
    };

    if format == DataSourceFormat::Delta {
        let schema = schema(&columns)?;
        let managed = staging_id.map(|id| (id, properties.take().unwrap_or_default()));
        let client = c.client.clone();
        let target = location.clone();
        let s3 = c.s3.clone();
        let handle = tokio::runtime::Handle::current();
        let written = tokio::task::spawn_blocking(move || {
            create_delta_table(
                &target,
                schema,
                credentials.as_ref(),
                s3.as_ref(),
                managed,
                client,
                handle,
            )
        })
        .await?
        .map_err(|e| anyhow!("Failed to create Delta table at {location}: {e}"))?;
        // A catalog-managed table registers its Delta table's properties.
        if written.is_some() {
            properties = written;
        }
    }

    let request = CreateTable {
        name,
        catalog_name: catalog,
        schema_name,
        table_type,
        columns,
        data_source_format: Some(format),
        storage_location: Some(location),
        comment: None,
        properties: properties.map(|p| p.into_iter().collect()),
        view_definition: None,
        view_dependencies: None,
    };
    let info = c
        .client
        .create_table(&request)
        .await
        .map_err(upstream("createTable"))?;
    Ok(c.json(&shape(&models::TABLE, &serde_json::to_value(&info)?)))
}

/// Write version 0 of a Delta table at `location`. A catalog-managed table
/// (a staging table id, with the user's properties) gets the
/// `catalogManaged` feature and returns the properties to register.
fn create_delta_table(
    location: &str,
    schema: Arc<StructType>,
    credentials: Option<&TemporaryCredentials>,
    s3: Option<&S3Store>,
    managed: Option<(String, HashMap<String, String>)>,
    client: Client,
    handle: tokio::runtime::Handle,
) -> anyhow::Result<Option<HashMap<String, String>>> {
    let url = directory_url(location)?;
    let engine = engine(&url, credentials, s3)?;
    let Some((table_id, mut table_properties)) = managed else {
        // As upstream, an external table may go over a Delta table that is
        // already at its location, such as one that a failed attempt left
        // behind; but only one with the same columns (upstream's to-do).
        if !create_if_missing(&engine, &url, schema.clone())? {
            check_columns(&engine, &url, &schema)?;
        }
        return Ok(None);
    };
    make_local_directory(&url)?;
    table_properties.insert("delta.feature.catalogManaged".into(), "supported".into());
    table_properties.insert(
        "delta.feature.vacuumProtocolCheck".into(),
        "supported".into(),
    );
    table_properties.insert(UC_TABLE_ID.into(), table_id.clone());
    let committer = UcCommitter {
        client,
        table_id,
        table_uri: url.clone(),
        handle,
    };
    let result = kernel_create_table(url.as_str(), schema, ENGINE_INFO)
        .with_table_properties(table_properties)
        .build(&engine, Box::new(committer))?
        .commit(&engine)?;
    let CommitResult::Committed(committed) = result else {
        bail!("Table creation did not commit");
    };
    // Upstream `UnityCatalogUtils.getPropertiesForCreate`: the table's
    // configuration, its protocol as properties, and the commit's version
    // and time.
    let snapshot = committed
        .post_commit_snapshot()
        .context("The created table has no snapshot")?;
    let mut properties = snapshot.get_protocol_derived_properties();
    properties.extend(snapshot.metadata_configuration().clone());
    properties.insert(
        "delta.lastUpdateVersion".into(),
        snapshot.version().to_string(),
    );
    properties.insert(
        "delta.lastCommitTimestamp".into(),
        snapshot.get_timestamp(&engine)?.to_string(),
    );
    Ok(Some(properties))
}

/// Write version 0 of a Delta table at `url`, unless one is there; true
/// when this call wrote it.
fn create_if_missing(engine: &Kernel, url: &Url, schema: Arc<StructType>) -> anyhow::Result<bool> {
    if has_delta_log(engine, url)? {
        return Ok(false);
    }
    make_local_directory(url)?;
    let result = kernel_create_table(url.as_str(), schema, ENGINE_INFO)
        .build(engine, Box::new(FileSystemCommitter::new()))?
        .commit(engine)?;
    match result {
        CommitResult::Committed(_) => Ok(true),
        CommitResult::Retryable(retry) => Err(retry.error.into()),
        // Another writer made version 0 first.
        CommitResult::Conflicted(_) => Ok(false),
    }
}

/// Fail unless the Delta table at `url` has the columns of `schema`: the
/// same names and types, in the same order. Nullability does not count:
/// `--columns` cannot say NOT NULL.
fn check_columns(engine: &Kernel, url: &Url, schema: &StructType) -> anyhow::Result<()> {
    let there = load_snapshot(engine, url, None)?.schema();
    let columns = |schema: &StructType| -> Vec<(String, DataType)> {
        schema
            .fields()
            .map(|f| (f.name().clone(), f.data_type().clone()))
            .collect()
    };
    if columns(&there) != columns(schema) {
        let theirs: Vec<String> = there
            .fields()
            .map(|f| format!("{} {}", f.name(), type_name(f.data_type())))
            .collect();
        bail!(
            "the Delta table there has other columns: {}",
            theirs.join(", ")
        );
    }
    Ok(())
}

/// The Delta log tail the server keeps for a catalog-managed table: its
/// latest version and the commits not yet published.
async fn catalog_commits(
    c: &Context<'_>,
    table_id: &str,
    url: &Url,
) -> anyhow::Result<(u64, Vec<LogPath>)> {
    let mut start = 0i64;
    let mut paths = BTreeMap::new();
    loop {
        let request = DeltaGetCommits {
            table_id: Some(table_id.to_string()),
            table_uri: Some(url.to_string()),
            start_version: Some(start),
            end_version: None,
        };
        let page = c
            .client
            .get_delta_commits(&request)
            .await
            .map_err(upstream("getCommits"))?;
        let latest = page.latest_table_version;
        let commits = page.commits;
        let mut newest = start - 1;
        for commit in &commits {
            let version = commit.version.unwrap_or(-1);
            let file_name = commit.file_name.as_deref().unwrap_or_default();
            let modified = commit.file_modification_timestamp.unwrap_or(0);
            let size = commit.file_size.unwrap_or(0).max(0) as u64;
            let path = LogPath::staged_commit(url.clone(), file_name, modified, size)?;
            paths.entry(version).or_insert(path);
            newest = newest.max(version);
        }
        if commits.is_empty() || newest >= latest {
            return Ok((latest.max(0) as u64, paths.into_values().collect()));
        }
        start = newest + 1;
    }
}

/// The latest snapshot of a table. A catalog-managed table loads with the
/// commits the server holds for it.
fn load_snapshot(
    engine: &Kernel,
    url: &Url,
    catalog: Option<(u64, Vec<LogPath>)>,
) -> DeltaResult<SnapshotRef> {
    let mut builder = Snapshot::builder_for(url.as_str());
    if let Some((latest, tail)) = catalog {
        builder = builder.with_max_catalog_version(latest).with_log_tail(tail);
    }
    builder.build(engine)
}

/// Whether the table's properties mark it catalog-managed.
fn is_catalog_managed(info: &TableInfo) -> bool {
    let properties = info.properties.as_ref();
    properties
        .and_then(|p| p.get("delta.feature.catalogManaged"))
        .map(String::as_str)
        == Some("supported")
}

struct TableTarget {
    url: Url,
    table_id: String,
    columns: Vec<ColumnInfo>,
    catalog: Option<(u64, Vec<LogPath>)>,
    credentials: TemporaryCredentials,
    s3: Option<S3Store>,
}

async fn target(
    c: &Context<'_>,
    full_name: &str,
    operation: TableOperation,
    verb: &str,
) -> anyhow::Result<TableTarget> {
    // Upstream's CLI reads streaming tables and materialized views as
    // MANAGED tables.
    let info = c
        .client
        .get_table(full_name, true, true)
        .await
        .map_err(upstream("getTable"))?;
    if info.data_source_format != Some(DataSourceFormat::Delta) {
        bail!("Only Delta tables are supported for {verb} operations");
    }
    let table_id = info.table_id.clone().unwrap_or_default();
    let location = info
        .storage_location
        .as_deref()
        .ok_or_else(|| anyhow!("The table has no storage location"))?;
    let url = directory_url(location)?;
    let commits = async {
        if is_catalog_managed(&info) {
            catalog_commits(c, &table_id, &url).await.map(Some)
        } else {
            Ok(None)
        }
    };
    let (credentials, catalog) = tokio::join!(table_credentials(c, &table_id, operation), commits);
    let (credentials, catalog) = (credentials?, catalog?);
    Ok(TableTarget {
        url,
        table_id,
        columns: info.columns.unwrap_or_default(),
        catalog,
        credentials,
        s3: c.s3.clone(),
    })
}

pub async fn read_table(
    c: &Context<'_>,
    full_name: &str,
    max_results: usize,
) -> anyhow::Result<String> {
    let target = target(c, full_name, TableOperation::Read, "read").await?;
    let location = target.url.to_string();
    tokio::task::spawn_blocking(move || read_rows(target, max_results))
        .await?
        .map_err(|e| anyhow!("Failed to read Delta table {location}: {e}"))
}

/// The first `max_results` rows, printed by arrow, as DataFusion's `show`.
fn read_rows(target: TableTarget, max_results: usize) -> anyhow::Result<String> {
    let engine = Arc::new(engine(
        &target.url,
        Some(&target.credentials),
        target.s3.as_ref(),
    )?);
    let snapshot = load_snapshot(&engine, &target.url, target.catalog)?;
    // Headers as upstream's: `name(type)`.
    let arrow_schema: ArrowSchema = snapshot.schema().as_ref().try_into_arrow()?;
    let headers: Vec<_> = arrow_schema
        .fields()
        .iter()
        .zip(snapshot.schema().fields())
        .map(|(field, column)| {
            let header = format!("{}({})", column.name(), type_name(column.data_type()));
            field.as_ref().clone().with_name(header)
        })
        .collect();
    // The kernel's scan has no limit: stop before the next file is read.
    let mut scan = snapshot.scan_builder().build()?.execute(engine)?;
    let mut batches = Vec::new();
    let mut left = max_results;
    while left > 0
        && let Some(data) = scan.next()
    {
        let batch: RecordBatch = ArrowEngineData::try_from_engine_data(data?)?.into();
        let batch = batch.slice(0, batch.num_rows().min(left));
        left -= batch.num_rows();
        batches.push(batch);
    }
    let table = pretty_format_batches_with_schema(Arc::new(ArrowSchema::new(headers)), &batches)?;
    Ok(table.to_string())
}

/// The kernel's name of a type: `integer`, `decimal(10,2)`, ….
fn type_name(data_type: &DataType) -> String {
    match data_type {
        DataType::Primitive(p) => p.to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

pub async fn write_table(c: &Context<'_>, full_name: &str) -> anyhow::Result<String> {
    let target = target(c, full_name, TableOperation::ReadWrite, "write").await?;
    let location = target.url.to_string();
    let client = c.client.clone();
    let handle = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || write_rows(target, client, handle))
        .await?
        .map_err(|e| anyhow!("Failed to write sample data to Delta table {location}: {e}"))?;
    Ok(format!("Table written to successfully at: {location}"))
}

/// Upstream `writeSampleDataToDeltaTable`: 15 random rows, in one file
/// (upstream writes three files of five); an `id` INT column counts up
/// from 1.
fn write_rows(
    target: TableTarget,
    client: Client,
    handle: tokio::runtime::Handle,
) -> anyhow::Result<()> {
    let engine = engine(&target.url, Some(&target.credentials), target.s3.as_ref())?;
    let schema = schema(&target.columns)?;
    // No Delta table yet: create it from the table's columns, as upstream.
    if target.catalog.is_none() {
        create_if_missing(&engine, &target.url, schema)?;
    }
    let uc_committer = target.catalog.is_some().then(|| UcCommitter {
        client,
        table_id: target.table_id,
        table_uri: target.url.clone(),
        handle: handle.clone(),
    });
    let committer: Box<dyn Committer> = match &uc_committer {
        Some(committer) => Box::new(committer.clone()),
        None => Box::new(FileSystemCommitter::new()),
    };
    let snapshot = load_snapshot(&engine, &target.url, target.catalog)?;
    let table_schema = snapshot.schema();
    let mut txn = snapshot
        .transaction(committer, &engine)?
        .with_engine_info(ENGINE_INFO);
    let write_context = txn.write_state()?.write_context_builder().build()?;
    let arrow_schema = Arc::<ArrowSchema>::new(table_schema.as_ref().try_into_arrow()?);
    let mut rng = rand::thread_rng();
    let arrays = table_schema
        .fields()
        .map(|f| sample_column(f, 15, &mut rng))
        .collect::<anyhow::Result<Vec<ArrayRef>>>()?;
    let data = ArrowEngineData::new(RecordBatch::try_new(arrow_schema, arrays)?);
    txn.add_files(handle.block_on(engine.write_parquet(&data, &write_context))?);
    let CommitResult::Committed(committed) = txn.commit(&engine)? else {
        bail!("Table writing failed");
    };
    if let (Some(committer), Some(post)) = (&uc_committer, committed.post_commit_snapshot()) {
        // Publishing is best effort: the server keeps the commit.
        let _ = post.publish(&engine, committer);
    }
    Ok(())
}

/// A local table's directory: the kernel creates a table only in a local
/// directory that is there.
fn make_local_directory(url: &Url) -> anyhow::Result<()> {
    if let Ok(path) = url.to_file_path() {
        std::fs::create_dir_all(&path).with_context(|| format!("Creating {}", path.display()))?;
    }
    Ok(())
}

fn has_delta_log(engine: &dyn Engine, url: &Url) -> anyhow::Result<bool> {
    let log = url.join("_delta_log/")?;
    match engine.storage_handler().list_from(&log)?.next() {
        Some(Ok(_)) => Ok(true),
        None | Some(Err(KernelError::FileNotFound(_))) => Ok(false),
        Some(Err(e)) => Err(e.into()),
    }
}

/// Random values as upstream's `generateRandomValues`.
fn sample_column(
    field: &StructField,
    count: usize,
    rng: &mut ThreadRng,
) -> anyhow::Result<ArrayRef> {
    const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let is_id = field.name().eq_ignore_ascii_case("id");
    let ints =
        |rng: &mut ThreadRng| -> Vec<i64> { (0..count).map(|_| rng.gen_range(0..1000)).collect() };
    let text = |rng: &mut ThreadRng| -> String {
        (0..10)
            .map(|_| LETTERS[rng.gen_range(0..LETTERS.len())] as char)
            .collect()
    };
    let micros = |rng: &mut ThreadRng| {
        TimestampMicrosecondArray::from_iter_values(
            (0..count).map(|_| 1_600_000_000_000_000 + rng.gen_range(0..100_000_000_000_000)),
        )
    };
    Ok(match field.data_type() {
        DataType::Primitive(PrimitiveType::Integer) if is_id => {
            Arc::new(Int32Array::from_iter_values(1..=count as i32))
        }
        DataType::Primitive(PrimitiveType::Integer) => Arc::new(Int32Array::from_iter_values(
            ints(rng).into_iter().map(|v| v as i32),
        )),
        DataType::Primitive(PrimitiveType::String) => {
            Arc::new(StringArray::from_iter_values((0..count).map(|_| text(rng))))
        }
        DataType::Primitive(PrimitiveType::Double) => Arc::new(Float64Array::from_iter_values(
            (0..count).map(|_| rng.r#gen::<f64>() * 1000.0),
        )),
        DataType::Primitive(PrimitiveType::Byte) => Arc::new(Int8Array::from_iter_values(
            ints(rng).into_iter().map(|v| v as i8),
        )),
        DataType::Primitive(PrimitiveType::Long) => {
            Arc::new(Int64Array::from_iter_values(ints(rng)))
        }
        DataType::Primitive(PrimitiveType::Float) => Arc::new(Float32Array::from_iter_values(
            ints(rng).into_iter().map(|v| v as f32),
        )),
        DataType::Primitive(PrimitiveType::Short) => Arc::new(Int16Array::from_iter_values(
            ints(rng).into_iter().map(|v| v as i16),
        )),
        DataType::Primitive(PrimitiveType::Boolean) => Arc::new(BooleanArray::from_iter(
            (0..count).map(|_| Some(rng.r#gen::<bool>())),
        )),
        DataType::Primitive(PrimitiveType::Date) => Arc::new(Date32Array::from_iter_values(
            (0..count).map(|_| 18_000 + rng.gen_range(0..3_000)),
        )),
        DataType::Primitive(PrimitiveType::Timestamp) => Arc::new(micros(rng).with_timezone("UTC")),
        DataType::Primitive(PrimitiveType::TimestampNtz) => Arc::new(micros(rng)),
        DataType::Primitive(PrimitiveType::Binary) => Arc::new(BinaryArray::from_iter_values(
            (0..count).map(|_| text(rng).into_bytes()),
        )),
        // Upstream writes none. From 0 to 999.99…, as far as the precision
        // goes.
        DataType::Primitive(PrimitiveType::Decimal(decimal)) => {
            let (precision, scale) = (decimal.precision(), decimal.scale());
            let bound = 1000i128
                .saturating_mul(10i128.pow(scale.into()))
                .min(10i128.pow(precision.into()));
            let values = (0..count).map(|_| rng.gen_range(0..bound));
            Arc::new(
                Decimal128Array::from_iter_values(values)
                    .with_precision_and_scale(precision, scale as i8)?,
            )
        }
        other => {
            let arrow_type: ArrowType = other.try_into_arrow()?;
            delta_kernel::arrow::array::new_null_array(&arrow_type, count)
        }
    })
}

/// Commits to a catalog-managed table through the server, as Delta's
/// `UCCatalogManagedClient`: version 0 is written to the Delta log; later
/// versions are staged and then ratified with `POST /delta/preview/commits`.
#[derive(Clone)]
struct UcCommitter {
    client: Client,
    table_id: String,
    table_uri: Url,
    handle: tokio::runtime::Handle,
}

impl Committer for UcCommitter {
    fn commit(
        &self,
        engine: &dyn Engine,
        actions: DeltaResultIterator<'_, FilteredEngineData>,
        metadata: CommitMetadata,
    ) -> DeltaResult<CommitResponse> {
        // Version 0 is a plain write to the Delta log.
        if metadata.commit_type() == CommitType::CatalogManagedCreate {
            return FileSystemCommitter::new().commit(engine, actions, metadata);
        }
        let version = metadata.version();
        let timestamp = metadata.in_commit_timestamp();
        let staged = metadata.staged_commit_path()?;
        let size = engine
            .json_handler()
            .write_json_file(&staged, Box::new(actions), false)?;
        let file_name = staged
            .path_segments()
            .and_then(|mut s| s.next_back())
            .unwrap_or_default()
            .to_string();
        let commit = DeltaCommit {
            table_id: Some(self.table_id.clone()),
            table_uri: Some(self.table_uri.to_string()),
            commit_info: Some(DeltaCommitInfo {
                version: Some(version as i64),
                timestamp: Some(timestamp),
                file_name: Some(file_name),
                file_size: Some(size as i64),
                file_modification_timestamp: Some(timestamp),
            }),
            latest_backfilled_version: metadata.max_published_version().map(|v| v as i64),
            ..Default::default()
        };
        match self.handle.block_on(self.client.commit_delta(&commit)) {
            Ok(()) => Ok(CommitResponse::Committed {
                file_meta: FileMeta::new(staged, timestamp, size),
            }),
            Err(e) if e.status() == Some(409) => Ok(CommitResponse::Conflict { version }),
            Err(e) => Err(KernelError::generic(upstream("commit")(e).to_string())),
        }
    }

    fn is_catalog_committer(&self) -> bool {
        true
    }

    /// Copy ratified commits into the Delta log; the server's next commit
    /// call learns of them as `latest_backfilled_version`.
    fn publish(&self, engine: &dyn Engine, metadata: PublishMetadata) -> DeltaResult<()> {
        for commit in metadata.commits_to_publish() {
            copy_file(engine, commit.location(), commit.published_location())?;
        }
        Ok(())
    }
}

/// Copy a log file without overwriting an existing one.
fn copy_file(engine: &dyn Engine, from: &Url, to: &Url) -> DeltaResult<()> {
    match engine.storage_handler().copy_atomic(from, to) {
        Ok(()) | Err(KernelError::FileAlreadyExists(_)) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_standardize_as_upstream() {
        assert_eq!(standardized_location("s3://b/p").unwrap(), "s3://b/p");
        assert_eq!(
            standardized_location("file:/tmp/t").unwrap(),
            "file:///tmp/t"
        );
        assert_eq!(standardized_location("/tmp/t").unwrap(), "file:///tmp/t");
        assert!(standardized_location("ftp://x/y").is_err());
    }
}
