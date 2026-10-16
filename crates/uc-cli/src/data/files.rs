//! `volume read` and `volume write`, as upstream's `VolumeCli` with Hadoop
//! file systems: credentials the server vends for the volume.

use std::sync::Arc;

use anyhow::{Context as _, anyhow, bail};
use delta_kernel::object_store::buffered::BufWriter;
use delta_kernel::object_store::path::Path as ObjectPath;
use delta_kernel::object_store::{self, ObjectStore, ObjectStoreExt as _, PutPayload};
use tokio::io::{AsyncRead, AsyncWriteExt as _};
use uc_api::{VolumeInfo, VolumeOperation};
use url::Url;

use super::storage::{directory_url, store_for, volume_credentials};
use crate::commands::{Context, upstream};

/// `relative` in the volume, and a store with the credentials the server
/// vends for `operation`; also the volume's location.
async fn open(
    c: &Context<'_>,
    full_name: &str,
    operation: VolumeOperation,
    relative: &str,
) -> anyhow::Result<(String, Url, Arc<dyn ObjectStore>)> {
    let action = match operation {
        VolumeOperation::WriteVolume => "Write",
        _ => "Read",
    };
    let info: VolumeInfo = c
        .client
        .get_volume(full_name)
        .await
        .map_err(upstream("getVolume"))?;
    let location = info
        .storage_location
        .ok_or_else(|| anyhow!("The volume has no storage location"))?;
    let base = directory_url(&location)?;
    let target = resolve(&base, relative, action)?;
    let id = info.volume_id.unwrap_or_default();
    let credentials = volume_credentials(c, &id, operation).await?;
    Ok((
        location,
        target,
        store_for(&base, Some(&credentials), c.s3.as_ref())?,
    ))
}

/// Upstream `resolveURI`: `relative` under `base`, which it may not leave;
/// `action` (`Read`, `Write`) names the failure.
fn resolve(base: &Url, relative: &str, action: &str) -> anyhow::Result<Url> {
    let resolved = base.join(relative)?;
    if !resolved.path().starts_with(base.path()) {
        bail!(
            "Resolved URI is outside the base URI path. {action} Failed. {}",
            resolved.path()
        );
    }
    Ok(resolved)
}

/// A file's text, or a directory's entries (`<name> [file]` or
/// `<name> [directory]`, one per line).
pub async fn read_volume(
    c: &Context<'_>,
    full_name: &str,
    path: Option<&str>,
) -> anyhow::Result<String> {
    let relative = path.unwrap_or_default();
    let (_, target, store) = open(c, full_name, VolumeOperation::ReadVolume, relative).await?;
    read(store.as_ref(), &target).await
}

/// The text of the file at `target`, else the entries of the directory.
async fn read(store: &dyn ObjectStore, target: &Url) -> anyhow::Result<String> {
    let path = ObjectPath::from_url_path(target.path())?;
    if !target.path().ends_with('/') {
        match store.get(&path).await {
            Ok(object) => {
                let bytes = object.bytes().await?;
                return Ok(String::from_utf8_lossy(&bytes).into_owned());
            }
            // A directory: list it.
            Err(object_store::Error::NotFound { .. }) => {}
            Err(e) => bail!("Failed to read volume location {target}: {e}"),
        }
    }
    let listing = store
        .list_with_delimiter(Some(&path))
        .await
        .map_err(|e| anyhow!("Failed to read volume location {target}: {e}"))?;
    // The listing holds the directory's children only: their names are
    // their last segments.
    let name = |p: &ObjectPath| p.filename().unwrap_or_default().to_string();
    let mut entries: Vec<(String, &str)> = listing
        .common_prefixes
        .iter()
        .map(|p| (name(p), "[directory]"))
        .chain(
            listing
                .objects
                .iter()
                .map(|o| (name(&o.location), "[file]")),
        )
        .collect();
    if entries.is_empty() {
        bail!("Volume location does not exist {target}");
    }
    entries.sort();
    let lines: Vec<String> = entries
        .iter()
        .map(|(name, kind)| format!("{name} {kind}"))
        .collect();
    Ok(lines.join("\n"))
}

/// Upload `source` (a local file, `-` for standard input) to `path` in the
/// volume; `path` defaults to the source's file name. Without `source`,
/// upstream's `writeRandomFileInVolume`: a `<uuid>.txt` test file.
pub async fn write_volume(
    c: &Context<'_>,
    full_name: &str,
    source: Option<&str>,
    path: Option<&str>,
) -> anyhow::Result<String> {
    let name = match (path, source) {
        (Some(path), _) => path.to_string(),
        (None, None) => format!("{}.txt", uuid::Uuid::new_v4()),
        (None, Some(source)) => std::path::Path::new(source)
            .file_name()
            .filter(|_| source != "-")
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| anyhow!("--path is required with --source {source}"))?,
    };
    // Open the source before the calls to the server.
    let reader: Option<Box<dyn AsyncRead + Unpin + Send>> = match source {
        None => None,
        Some("-") => Some(Box::new(tokio::io::stdin())),
        Some(source) => Some(Box::new(
            tokio::fs::File::open(source)
                .await
                .with_context(|| format!("Failed to read {source}"))?,
        )),
    };
    let (location, file, store) = open(c, full_name, VolumeOperation::WriteVolume, &name).await?;
    if file.path().ends_with('/') {
        bail!("--path must name a file: {name}");
    }
    let object = ObjectPath::from_url_path(file.path())?;
    let failed = |e: &dyn std::fmt::Display| {
        anyhow!("Failed to write file to volume location {location}: {e}")
    };
    match reader {
        None => {
            let content = PutPayload::from("This is a test file with random content.");
            store.put(&object, content).await.map_err(|e| failed(&e))?;
        }
        // In parts, so a large file need not fit in memory.
        Some(mut reader) => {
            let mut writer = BufWriter::new(store, object);
            tokio::io::copy(&mut reader, &mut writer)
                .await
                .map_err(|e| failed(&e))?;
            writer.shutdown().await.map_err(|e| failed(&e))?;
        }
    }
    Ok(format!("File written to: {file}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_inside_the_volume() {
        let base = Url::parse("s3://b/vol/").unwrap();
        assert_eq!(
            resolve(&base, "a/b.txt", "Read").unwrap().as_str(),
            "s3://b/vol/a/b.txt"
        );
        assert_eq!(resolve(&base, "", "Read").unwrap().as_str(), "s3://b/vol/");
        assert!(resolve(&base, "../other", "Write").is_err());
    }

    /// A local volume reads through the same store as S3, GCS and ADLS.
    #[tokio::test]
    async fn local_files_and_directories_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b.txt"), "b").unwrap();
        let base = Url::from_directory_path(dir.path()).unwrap();
        let store = store_for(&base, None, None).unwrap();
        let read_at =
            async |relative: &str| read(store.as_ref(), &resolve(&base, relative, "Read")?).await;
        assert_eq!(read_at("").await.unwrap(), "a.txt [file]\nsub [directory]");
        assert_eq!(read_at("a.txt").await.unwrap(), "hello");
        assert_eq!(read_at("sub").await.unwrap(), "b.txt [file]");
        let missing = read_at("nope").await.unwrap_err().to_string();
        assert!(
            missing.starts_with("Volume location does not exist"),
            "{missing}"
        );
    }
}
