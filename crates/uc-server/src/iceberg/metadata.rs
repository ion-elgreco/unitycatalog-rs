//! Iceberg metadata files, after upstream `MetadataService`. A file is read
//! or written only inside the table location UC persisted, and the table
//! location it records must be that location: client-written metadata must
//! not widen what the catalog serves or vends credentials for.
//!
//! Local tables are read and written with the file system directly, after
//! upstream `SimpleLocalFileIO`: clients that share the file system could
//! link a table's directories elsewhere, so a path through a symbolic link
//! at or below the table root is refused.

use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use iceberg::spec::TableMetadata;
use object_store::{PutMode, PutOptions, PutPayload};
use tracing::warn;
use uuid::Uuid;

use uc_common::{Error, ErrorCode, NormalizedUrl, Result, UriScheme};

use crate::{storage::ServerStorage, storage::vendor::VendOp};

/// Iceberg's `write.metadata.path` table property.
pub const WRITE_METADATA_LOCATION: &str = "write.metadata.path";

/// Upstream `validateMetadataLocation`: a metadata file or directory is
/// strictly under the table root.
pub fn validate_metadata_location(metadata: &NormalizedUrl, table: &NormalizedUrl) -> Result<()> {
    if !metadata.as_str().starts_with(&format!("{table}/")) {
        return Err(Error::invalid_argument(format!(
            "Iceberg metadata location ('{metadata}') must be a subpath of the persisted table \
             location ('{table}')."
        )));
    }
    Ok(())
}

/// Upstream `validateTableMetadataLocation`: the metadata's table root is
/// the persisted one, and its metadata directory, if set, is inside it.
pub fn validate_table_location(metadata: &TableMetadata, table: &NormalizedUrl) -> Result<()> {
    let recorded = NormalizedUrl::parse(metadata.location())?;
    if &recorded != table {
        return Err(Error::invalid_argument(format!(
            "Iceberg table location ('{recorded}') must match the persisted table location \
             ('{table}')."
        )));
    }
    if let Some(directory) = metadata.properties().get(WRITE_METADATA_LOCATION) {
        validate_metadata_location(&NormalizedUrl::parse(directory)?, table)?;
    }
    Ok(())
}

/// Upstream `toIcebergMetadataLocation`: a local file by its path, as
/// Iceberg's local file IO names it; anything else by its URL.
pub fn client_location(location: &NormalizedUrl) -> String {
    match location.scheme() {
        UriScheme::File => location.path(),
        _ => location.to_string(),
    }
}

/// Upstream `parseMetadataVersion`: the version in a metadata file name
/// (`00001-<uuid>.metadata.json`, `v1.metadata.json`), or -1.
pub fn parse_metadata_version(location: &NormalizedUrl) -> i32 {
    let name = location.as_str().rsplit('/').next().unwrap_or_default();
    let name = name.strip_prefix('v').unwrap_or(name);
    let digits = name.len() - name.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let rest = &name[digits..];
    let named = rest.ends_with(".metadata.json") || rest.ends_with(".metadata.json.gz");
    match (digits > 0 && named, name[..digits].parse::<i32>()) {
        (true, Ok(version)) => version,
        _ => -1,
    }
}

/// Upstream `newMetadataLocation`: where version `version` of `metadata`
/// goes, following Iceberg's naming, checked against the table root.
pub fn new_metadata_location(
    metadata: &TableMetadata,
    version: i32,
    table: &NormalizedUrl,
) -> Result<NormalizedUrl> {
    validate_table_location(metadata, table)?;
    let directory = match metadata.properties().get(WRITE_METADATA_LOCATION) {
        Some(directory) => directory.trim_end_matches('/').to_string(),
        None => format!("{}/metadata", metadata.location()),
    };
    let location = NormalizedUrl::parse(&format!(
        "{directory}/{version:05}-{}.metadata.json",
        Uuid::new_v4()
    ))?;
    validate_metadata_location(&location, table)?;
    Ok(location)
}

/// A table whose metadata cannot be read. The table exists, so this is not
/// a 404, and the error does not say where its files are.
fn unreadable(location: &NormalizedUrl, reason: impl std::fmt::Display) -> Error {
    warn!(%location, %reason, "cannot read Iceberg metadata");
    Error::internal("Could not read this table")
}

/// Upstream `SimpleLocalFileIO.existsRejectingLink`: whether something
/// exists at `path`, refusing a symbolic link there.
fn exists_rejecting_link(path: &Path, location: &str) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(Error::internal(format!(
            "Failed to read attributes of {}: {e}",
            path.display()
        ))),
        Ok(attributes) if attributes.file_type().is_symlink() => Err(Error::new(
            ErrorCode::PermissionDenied,
            format!(
                "Local path {location} goes through a symbolic link at {}, which the server does \
                 not follow",
                path.display()
            ),
        )),
        Ok(_) => Ok(true),
    }
}

/// Upstream `SimpleLocalFileIO.resolve`: the local path of `location`,
/// which is at or under `root`; the root and each existing component below
/// it must not be a symbolic link.
fn local_path(root: &NormalizedUrl, location: &NormalizedUrl) -> Result<PathBuf> {
    let root_path = PathBuf::from(root.path());
    let path = PathBuf::from(location.path());
    let below = path.strip_prefix(&root_path).map_err(|_| {
        Error::invalid_argument(format!(
            "Local path {} is not under {}",
            path.display(),
            root_path.display()
        ))
    })?;
    if !exists_rejecting_link(&root_path, location.as_str())? {
        return Ok(path);
    }
    let mut current = root_path;
    for component in below.components() {
        current.push(component);
        if !exists_rejecting_link(&current, location.as_str())? {
            break;
        }
    }
    Ok(path)
}

/// Upstream `readTableMetadata`: the metadata file at `metadata_location`
/// of the table at `table_location`.
pub async fn read(
    storage: &ServerStorage,
    metadata_location: &NormalizedUrl,
    table_location: &NormalizedUrl,
) -> Result<TableMetadata> {
    validate_metadata_location(metadata_location, table_location)?;
    let bytes: bytes::Bytes = match metadata_location.scheme() {
        UriScheme::File => {
            let path = local_path(table_location, metadata_location)?;
            tokio::fs::read(&path)
                .await
                .map_err(|e| unreadable(metadata_location, e))?
                .into()
        }
        _ => {
            let (store, path) = storage
                .open(metadata_location, VendOp::Read)
                .await
                .map_err(|e| unreadable(metadata_location, e))?;
            let object = store
                .get(&path)
                .await
                .map_err(|e| unreadable(metadata_location, e))?;
            object
                .bytes()
                .await
                .map_err(|e| unreadable(metadata_location, e))?
        }
    };
    let metadata = parse(&bytes).map_err(|e| unreadable(metadata_location, e))?;
    validate_table_location(&metadata, table_location)?;
    Ok(metadata)
}

/// Table metadata JSON, gzip-compressed or not.
fn parse(bytes: &[u8]) -> std::result::Result<TableMetadata, String> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut json = Vec::new();
        GzDecoder::new(bytes)
            .read_to_end(&mut json)
            .map_err(|e| e.to_string())?;
        return serde_json::from_slice(&json).map_err(|e| e.to_string());
    }
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}

/// Upstream `writeTableMetadata`: write a new metadata file, inside the
/// persisted table location only. An existing file is never replaced.
pub async fn write(
    storage: &ServerStorage,
    metadata: &TableMetadata,
    metadata_location: &NormalizedUrl,
    table_location: &NormalizedUrl,
) -> Result<()> {
    validate_table_location(metadata, table_location)?;
    validate_metadata_location(metadata_location, table_location)?;
    let json = serde_json::to_vec(metadata)
        .map_err(|e| Error::internal(format!("cannot serialize table metadata: {e}")))?;
    let failed = |e: &dyn std::fmt::Display| {
        Error::internal(format!(
            "Failed to write Iceberg metadata {metadata_location}: {e}"
        ))
    };
    match metadata_location.scheme() {
        UriScheme::File => {
            let path = local_path(table_location, metadata_location)?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| failed(&e))?;
            }
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
                .map_err(|e| failed(&e))?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &json)
                .await
                .map_err(|e| failed(&e))?;
            file.sync_all().await.map_err(|e| failed(&e))
        }
        _ => {
            let (store, path) = storage
                .open(metadata_location, VendOp::ReadWrite)
                .await
                .map_err(|e| failed(&e))?;
            let options = PutOptions {
                mode: PutMode::Create,
                ..Default::default()
            };
            store
                .put_opts(&path, PutPayload::from(json), options)
                .await
                .map_err(|e| failed(&e))?;
            Ok(())
        }
    }
}

/// Upstream `deleteTableMetadata`: best-effort removal of a metadata file
/// that lost a commit. An orphaned file is harmless; the failure is logged.
pub async fn delete(
    storage: &ServerStorage,
    metadata_location: &NormalizedUrl,
    table_location: &NormalizedUrl,
) {
    let result = async {
        validate_metadata_location(metadata_location, table_location)?;
        match metadata_location.scheme() {
            UriScheme::File => {
                let path = local_path(table_location, metadata_location)?;
                tokio::fs::remove_file(&path)
                    .await
                    .map_err(|e| Error::internal(e.to_string()))
            }
            _ => {
                let (store, path) = storage
                    .open(metadata_location, VendOp::ReadWrite)
                    .await
                    .map_err(|e| Error::internal(e.0))?;
                store
                    .delete(&path)
                    .await
                    .map_err(|e| Error::internal(e.to_string()))
            }
        }
    }
    .await;
    if let Err(e) = result {
        warn!(location = %metadata_location, error = %e.message(), "cannot remove Iceberg metadata");
    }
}

/// Upstream `prepareTableLocation`: for a local table, create the table,
/// `metadata` and `data` directories, so that a staged create can write
/// files before its first commit. Object stores have no directories.
pub fn prepare_table_location(
    metadata: &TableMetadata,
    table_location: &NormalizedUrl,
) -> Result<()> {
    validate_table_location(metadata, table_location)?;
    if table_location.scheme() != UriScheme::File {
        return Ok(());
    }
    // The table directory first: a linked one fails before anything is
    // created through it.
    let root = PathBuf::from(table_location.path());
    for directory in [root.clone(), root.join("metadata"), root.join("data")] {
        if !exists_rejecting_link(&directory, table_location.as_str())? {
            std::fs::create_dir_all(&directory).map_err(|e| {
                Error::internal(format!(
                    "Failed to create directory: {}: {e}",
                    directory.display()
                ))
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_versions() {
        let version = |name: &str| {
            parse_metadata_version(
                &NormalizedUrl::parse(&format!("s3://b/t/metadata/{name}")).unwrap(),
            )
        };
        assert_eq!(version("00001-abc.metadata.json"), 1);
        assert_eq!(version("00012.metadata.json.gz"), 12);
        assert_eq!(version("v3.metadata.json"), 3);
        assert_eq!(version("v1abc.metadata.json"), 1);
        assert_eq!(version("iceberg.metadata.json"), -1);
        assert_eq!(version("00001-abc.json"), -1);
        assert_eq!(version("99999999999-x.metadata.json"), -1);
    }
}
