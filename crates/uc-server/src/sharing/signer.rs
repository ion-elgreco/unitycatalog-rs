//! URLs that let a recipient read one data file of a shared table for a
//! while, and the store the kernel reads the table from.
//!
//! - S3: presigned URLs, with credentials vended for reading the table (or
//!   the environment's), so a URL expires with them at the latest.
//! - ADLS: the blob URL with the SAS vended for the table, or a SAS from
//!   the environment's credentials.
//! - GCS: signed URLs from the environment's credentials, as a vended
//!   token cannot sign.
//! - Local files: URLs of this server,
//!   `/delta-sharing/files/<file>?sp=r&se=<expiry>&sig=<signature>`: the
//!   file URL in base64url, and [`SecurityContext::sign_file`]'s signature.
//!   The parameters are those of an Azure SAS: the Delta kernel reads an
//!   HTTP URL as presigned only when its query has such a signature.

use std::sync::Arc;
use std::time::Duration;

use axum::http::Method;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use object_store::{
    ObjectStore, azure::MicrosoftAzureBuilder, client::SpawnedReqwestConnector,
    gcp::GoogleCloudStorageBuilder, local::LocalFileSystem, path::Path as ObjectPath,
    signer::Signer,
};
use url::Url;

use uc_common::{Error, NormalizedUrl, Result, UriScheme, now_epoch_millis};
use uc_storage::runtime::io_handle;

use crate::{
    authn::security::SecurityContext,
    server::state::AppState,
    storage::{ServerStorage, azure_builder, gcs_builder, s3_builder, vendor::VendOp},
};

/// How long a file URL works.
pub const URL_TTL: Duration = Duration::from_secs(60 * 60);

/// A URL for reading a file, and when it expires (epoch milliseconds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedUrl {
    pub url: String,
    pub expires_at: i64,
}

pub enum FileSigner {
    /// The presigned URLs of an object store.
    Store {
        signer: Arc<dyn Signer>,
        /// When the store's credentials expire, if they do.
        not_after: Option<i64>,
    },
    /// An ADLS blob URL with a SAS for the table's location.
    Sas {
        /// `https://<account>.blob.core.windows.net/<container>/`.
        container: Url,
        sas: String,
        not_after: Option<i64>,
    },
    /// A URL of this server's files endpoint.
    Local {
        /// The endpoint, ending in `/`.
        endpoint: String,
        security: Arc<SecurityContext>,
    },
}

impl FileSigner {
    /// A URL for reading `file`, an absolute URL in the table's storage.
    pub async fn sign(&self, file: &Url) -> Result<SignedUrl> {
        let now = now_epoch_millis();
        let ttl_end = now + URL_TTL.as_millis() as i64;
        match self {
            Self::Store { signer, not_after } => {
                let expires_at = not_after.map_or(ttl_end, |end| end.min(ttl_end));
                let ttl = Duration::from_millis((expires_at - now).max(1000) as u64);
                let path = ObjectPath::from_url_path(file.path())
                    .map_err(|e| Error::internal(format!("file path {file}: {e}")))?;
                let url = signer
                    .signed_url(Method::GET, &path, ttl)
                    .await
                    .map_err(|e| Error::internal(format!("cannot sign a URL for {file}: {e}")))?;
                Ok(SignedUrl {
                    url: url.to_string(),
                    expires_at,
                })
            }
            Self::Sas {
                container,
                sas,
                not_after,
            } => {
                // `abfss://<container>@<account>/<path>`: the path follows
                // the container.
                let path = file.path().trim_start_matches('/');
                let mut url = container
                    .join(path)
                    .map_err(|e| Error::internal(format!("file path {file}: {e}")))?;
                url.set_query(Some(sas.trim_start_matches('?')));
                Ok(SignedUrl {
                    url: url.to_string(),
                    expires_at: not_after.unwrap_or(ttl_end),
                })
            }
            Self::Local { endpoint, security } => {
                let expires_at = ttl_end / 1000;
                let signature = security.sign_file(file.as_str(), expires_at);
                let file = URL_SAFE_NO_PAD.encode(file.as_str());
                Ok(SignedUrl {
                    url: format!("{endpoint}{file}?sp=r&se={expires_at}&sig={signature}"),
                    expires_at: expires_at * 1000,
                })
            }
        }
    }
}

/// The store of the table at `location`, for the kernel, and the signer of
/// its files' URLs. `files_endpoint` is the URL of this server's files
/// endpoint, for local tables.
pub async fn open(
    state: &AppState,
    location: &NormalizedUrl,
    files_endpoint: &str,
) -> Result<(Arc<dyn ObjectStore>, FileSigner)> {
    let storage = || ServerStorage::new(state.metastore.clone(), state.vendor.clone());
    let credentials = async {
        storage()
            .credentials(location, VendOp::Read)
            .await
            .map_err(|e| Error::internal(e.0))
    };
    let url = location.as_str();
    let failed = |e: object_store::Error| Error::internal(format!("storage of {url}: {e}"));
    Ok(match location.scheme() {
        UriScheme::File => {
            let security = state.security.clone().ok_or_else(|| {
                Error::failed_precondition(
                    "Local tables need the server's signing key to be shared",
                )
            })?;
            let signer = FileSigner::Local {
                endpoint: files_endpoint.to_string(),
                security,
            };
            (Arc::new(LocalFileSystem::new()), signer)
        }
        UriScheme::S3 => {
            let credentials = credentials.await?;
            let s3_store = state.vendor.s3_store();
            let region = match s3_store {
                Some(store) => store.region.clone(),
                None => state.s3_regions.region(location).await?,
            };
            let s3 = s3_builder(url, credentials.as_ref(), s3_store)
                .with_region(region)
                .build()
                .map_err(failed)?;
            let s3 = Arc::new(s3);
            let signer = FileSigner::Store {
                signer: s3.clone(),
                not_after: credentials.and_then(|c| c.expiration_time),
            };
            (s3, signer)
        }
        UriScheme::Gs => {
            let credentials = credentials.await?;
            let store = gcs_builder(url, credentials.as_ref())
                .build()
                .map_err(failed)?;
            let signer = GoogleCloudStorageBuilder::from_env()
                .with_url(url)
                .with_http_connector(SpawnedReqwestConnector::new(io_handle()))
                .build()
                .map_err(failed)?;
            let signer = FileSigner::Store {
                signer: Arc::new(signer),
                not_after: None,
            };
            (Arc::new(store), signer)
        }
        UriScheme::Abfs | UriScheme::Abfss => {
            let credentials = credentials.await?;
            let store = azure_builder(url, credentials.as_ref())
                .build()
                .map_err(failed)?;
            let sas = credentials.as_ref().and_then(|c| {
                let sas = c.azure_user_delegation_sas.as_ref()?.sas_token.clone()?;
                Some((sas, c.expiration_time))
            });
            let signer = match sas {
                Some((sas, not_after)) => FileSigner::Sas {
                    container: blob_container_url(location)?,
                    sas,
                    not_after,
                },
                None => {
                    let signer = MicrosoftAzureBuilder::from_env()
                        .with_url(url)
                        .with_http_connector(SpawnedReqwestConnector::new(io_handle()))
                        .build()
                        .map_err(failed)?;
                    FileSigner::Store {
                        signer: Arc::new(signer),
                        not_after: None,
                    }
                }
            };
            (Arc::new(store), signer)
        }
    })
}

/// The blob endpoint URL of the container of the ADLS `location`:
/// `abfss://<container>@<account>.dfs.core.windows.net/...` is
/// `https://<account>.blob.core.windows.net/<container>/`.
fn blob_container_url(location: &NormalizedUrl) -> Result<Url> {
    let parts = crate::storage::vendor::azure::parse_location(location)?;
    let container = parts.container.ok_or_else(|| {
        Error::invalid_argument(format!("ADLS location without a container: {location}"))
    })?;
    let host = parts.account.replacen(".dfs.", ".blob.", 1);
    Url::parse(&format!("https://{host}/{container}/"))
        .map_err(|e| Error::invalid_argument(format!("ADLS location {location}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adls_files_are_read_from_the_blob_endpoint() {
        let location =
            NormalizedUrl::parse("abfss://data@acct.dfs.core.windows.net/tables/t").unwrap();
        assert_eq!(
            blob_container_url(&location).unwrap().as_str(),
            "https://acct.blob.core.windows.net/data/"
        );
    }

    #[tokio::test]
    async fn sas_urls_carry_the_sas_and_its_expiry() {
        let location =
            NormalizedUrl::parse("abfss://data@acct.dfs.core.windows.net/tables/t").unwrap();
        let signer = FileSigner::Sas {
            container: blob_container_url(&location).unwrap(),
            sas: "?sv=1&sig=x".into(),
            not_after: Some(42),
        };
        let file =
            Url::parse("abfss://data@acct.dfs.core.windows.net/tables/t/a%20b.parquet").unwrap();
        let signed = signer.sign(&file).await.unwrap();
        assert_eq!(
            signed.url,
            "https://acct.blob.core.windows.net/data/tables/t/a%20b.parquet?sv=1&sig=x"
        );
        assert_eq!(signed.expires_at, 42);
    }

    #[tokio::test]
    async fn s3_urls_expire_with_their_credentials() {
        let s3 = object_store::aws::AmazonS3Builder::new()
            .with_bucket_name("lake")
            .with_region("eu-west-1")
            .with_access_key_id("key")
            .with_secret_access_key("secret")
            .build()
            .unwrap();
        let soon = now_epoch_millis() + 60_000;
        let signer = FileSigner::Store {
            signer: Arc::new(s3),
            not_after: Some(soon),
        };
        let file = Url::parse("s3://lake/tables/t/part-0.parquet").unwrap();
        let signed = signer.sign(&file).await.unwrap();
        assert_eq!(signed.expires_at, soon);
        assert!(
            signed
                .url
                .starts_with("https://s3.eu-west-1.amazonaws.com/lake/tables/t/part-0.parquet?"),
            "{}",
            signed.url
        );
        let url = Url::parse(&signed.url).unwrap();
        let expires = url
            .query_pairs()
            .find(|(k, _)| k == "X-Amz-Expires")
            .unwrap()
            .1;
        assert!(expires.parse::<u64>().unwrap() <= 60, "{expires}");
    }
}
