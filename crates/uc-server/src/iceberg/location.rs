//! Where the server may create the files of an external Iceberg table.
//! Upstream `LocalStorageLocationValidator`: the server writes local files
//! with its own identity, so a local location must be under an external
//! location (whose privilege authorization checked) or strictly under a
//! root in `storage.external_local_roots` (`UC_STORAGE_EXTERNAL_LOCAL_ROOTS`).
//! Cloud locations are reached only with credentials vended for them.

use std::path::{Path, PathBuf};

use uc_common::{Error, ErrorCode, NormalizedUrl, Pagination, Result, UriScheme};

use crate::server::state::AppState;

/// Upstream `validateLocalLocation`.
pub async fn validate_local_location(state: &AppState, location: &NormalizedUrl) -> Result<()> {
    if location.scheme() != UriScheme::File {
        return Ok(());
    }
    if !location.is_canonical_local() {
        return Err(Error::invalid_argument(format!(
            "Local location must not contain unneeded escapes: {location}"
        )));
    }
    let path = PathBuf::from(location.path());
    if under_external_location(state, location, &path).await?
        || under_local_root(&state.external_local_roots, &path)
    {
        return Ok(());
    }
    Err(Error::new(
        ErrorCode::PermissionDenied,
        format!(
            "Local location '{location}' must be under an external location or under a root \
             listed in storage.external_local_roots."
        ),
    ))
}

/// Whether a local external location covers `path`, comparing decoded
/// paths per segment. One spelled with an unneeded escape was never matched
/// by authorization, so a location under it is refused.
async fn under_external_location(
    state: &AppState,
    location: &NormalizedUrl,
    path: &Path,
) -> Result<bool> {
    let mut covered = false;
    let mut page = Pagination::default();
    loop {
        let (locations, next) = state.metastore.external_locations().list(&page).await?;
        for url in locations.into_iter().filter_map(|l| l.url) {
            let Ok(external) = NormalizedUrl::parse(&url) else {
                continue;
            };
            if external.scheme() != UriScheme::File || !path.starts_with(external.path()) {
                continue;
            }
            if !external.is_canonical_local() {
                return Err(Error::new(
                    ErrorCode::PermissionDenied,
                    format!(
                        "Local location '{location}' is under external location '{url}', whose \
                         URL has unneeded escapes."
                    ),
                ));
            }
            covered = true;
        }
        match next {
            Some(token) => page = Pagination::new(None, Some(token)),
            None => return Ok(covered),
        }
    }
}

/// Strictly under one of `roots` (the root itself is not allowed).
fn under_local_root(roots: &[PathBuf], path: &Path) -> bool {
    roots
        .iter()
        .any(|root| path.starts_with(root) && path != root)
}

/// The `storage.external_local_roots` entries as local paths.
pub fn parse_local_roots(entries: &[String]) -> Result<Vec<PathBuf>> {
    entries
        .iter()
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let url = NormalizedUrl::parse(entry)?;
            if url.scheme() != UriScheme::File {
                return Err(Error::invalid_argument(format!(
                    "only local paths are allowed: {entry}"
                )));
            }
            Ok(PathBuf::from(url.path()))
        })
        .collect()
}
