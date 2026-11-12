//! Storage access for shared objects: the server's own reads, the file URLs
//! of a query answer and the temporary credentials of the credential
//! endpoints. All of it is read-only.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use http::Method;
use object_store::{local::LocalFileSystem, path::Path as ObjectPath, signer::Signer};
use opensharing_api::{Error, Result};
use opensharing_common::models::opensharing::v1::{
    AwsCredentials, AzureUserDelegationSas, GcpOauthToken, TemporaryCredentials,
    temporary_credentials::Credentials,
};
use opensharing_engine::{
    StorageAccess,
    access::{FileSigner, SignedFile, TableStorage, UnsignedUrls},
    catalog::SharedAsset,
};
use uc_common::{NormalizedUrl, UriScheme, now_epoch_millis};
use uc_metastore::DynMetastore;
use url::Url;

use super::{Recipient, internal};
use crate::storage::{
    Expires, ServerStorage, azure, build_store, expiring_cache, s3,
    vendor::{DynVendor, VendOp, vend_credential},
};

/// The longest time a file URL of a query answer is valid.
const URL_LIFETIME: Duration = Duration::from_secs(60 * 60);

pub struct ServerAccess {
    metastore: DynMetastore,
    vendor: DynVendor,
    /// The storage of each cloud location that was read: one vend of
    /// credentials serves many requests.
    opened: moka::future::Cache<String, Arc<Opened>>,
}

/// The storage of a location and the end of its credentials.
struct Opened {
    storage: TableStorage,
    expires_at: Option<i64>,
}

impl Expires for Opened {
    fn expires_at(&self) -> Option<i64> {
        self.expires_at
    }
}

impl ServerAccess {
    pub fn new(metastore: DynMetastore, vendor: DynVendor) -> Self {
        Self {
            metastore,
            vendor,
            opened: expiring_cache(),
        }
    }

    /// The storage of the cloud location `location`, with new read-only
    /// credentials.
    async fn open(&self, location: &NormalizedUrl, root: Url) -> Result<Opened> {
        let storage = ServerStorage::new(self.metastore.clone(), self.vendor.clone());
        let credentials = (storage.credentials(location, VendOp::Read).await).map_err(internal)?;
        let credentials = credentials.as_ref();
        let expires_at = credentials.and_then(|c| c.expiration_time);
        let storage = match location.scheme() {
            UriScheme::S3 => {
                let store =
                    Arc::new(s3(location, credentials, storage.s3_store()).map_err(internal)?);
                TableStorage {
                    store: store.clone(),
                    root,
                    signer: Arc::new(Presigned { store, expires_at }),
                }
            }
            UriScheme::Abfs | UriScheme::Abfss => {
                let store = Arc::new(azure(location, credentials).map_err(internal)?);
                let sas = credentials
                    .and_then(|c| c.azure_user_delegation_sas.as_ref())
                    .and_then(|s| s.sas_token.clone());
                let signer: Arc<dyn FileSigner> = match sas {
                    Some(sas) => Arc::new(WithSas { sas, expires_at }),
                    None => Arc::new(Presigned {
                        store: store.clone(),
                        expires_at,
                    }),
                };
                TableStorage {
                    store,
                    root,
                    signer,
                }
            }
            UriScheme::Gs => {
                let (store, _) =
                    build_store(location, credentials, storage.s3_store()).map_err(internal)?;
                TableStorage {
                    store,
                    root,
                    signer: Arc::new(NoGcsUrls),
                }
            }
            UriScheme::File => {
                return Err(internal("a local location has no credentials"));
            }
        };
        Ok(Opened {
            storage,
            expires_at,
        })
    }
}

fn location(asset: &SharedAsset) -> Result<NormalizedUrl> {
    NormalizedUrl::parse(&asset.location.raw).map_err(internal)
}

#[async_trait]
impl StorageAccess<Recipient> for ServerAccess {
    async fn table_storage(&self, asset: &SharedAsset, _: &Recipient) -> Result<TableStorage> {
        let location = location(asset)?;
        let root = asset.location.as_directory();
        if location.scheme() == UriScheme::File {
            return Ok(TableStorage {
                store: Arc::new(LocalFileSystem::new()),
                root,
                signer: Arc::new(UnsignedUrls),
            });
        }
        let open = async { self.open(&location, root).await.map(Arc::new) };
        let opened = (self
            .opened
            .try_get_with(location.as_str().to_string(), open)
            .await)
            .map_err(|e: Arc<Error>| internal(&*e))?;
        Ok(opened.storage.clone())
    }

    async fn vend_read_credential(
        &self,
        asset: &SharedAsset,
        _: &Recipient,
    ) -> Result<TemporaryCredentials> {
        let location = location(asset)?;
        let vended = vend_credential(
            &self.metastore,
            &self.vendor,
            &location,
            VendOp::Read.privileges(),
        )
        .await
        .map_err(internal)?;
        let credentials = if let Some(aws) = vended.aws_temp_credentials {
            Some(Credentials::AwsTempCredentials(Box::new(AwsCredentials {
                access_key_id: aws.access_key_id.unwrap_or_default(),
                secret_access_key: aws.secret_access_key.unwrap_or_default(),
                session_token: aws.session_token.unwrap_or_default(),
                ..Default::default()
            })))
        } else if let Some(azure) = vended.azure_user_delegation_sas {
            Some(Credentials::AzureUserDelegationSas(Box::new(
                AzureUserDelegationSas {
                    sas_token: azure.sas_token.unwrap_or_default(),
                    ..Default::default()
                },
            )))
        } else {
            vended.gcp_oauth_token.map(|gcp| {
                Credentials::GcpOauthToken(Box::new(GcpOauthToken {
                    oauth_token: gcp.oauth_token.unwrap_or_default(),
                    ..Default::default()
                }))
            })
        };
        Ok(TemporaryCredentials {
            expiration_time: vended.expiration_time.unwrap_or_default(),
            url: Some(location.into_string()),
            credentials,
            ..Default::default()
        })
    }
}

/// How long a URL signed now is valid: [`URL_LIFETIME`], but not past the
/// end of the credentials that sign it.
fn lifetime(expires_at: Option<i64>) -> Result<Duration> {
    let Some(expires_at) = expires_at else {
        return Ok(URL_LIFETIME);
    };
    let left = expires_at - now_epoch_millis();
    if left <= 0 {
        return Err(internal("the storage credentials have expired"));
    }
    Ok(URL_LIFETIME.min(Duration::from_millis(left as u64)))
}

fn expiration(lifetime: Duration) -> Option<i64> {
    Some(now_epoch_millis() + lifetime.as_millis() as i64)
}

/// URLs signed by the store's credentials: S3 presigned URLs, and Azure
/// SAS URLs from an account key or a user delegation key.
struct Presigned<S> {
    store: Arc<S>,
    /// When the credentials of the store expire, in epoch milliseconds.
    expires_at: Option<i64>,
}

#[async_trait]
impl<S: Signer> FileSigner for Presigned<S> {
    async fn sign(&self, files: &[Url]) -> Result<Vec<SignedFile>> {
        let lifetime = lifetime(self.expires_at)?;
        let paths = (files.iter())
            .map(|file| ObjectPath::from_url_path(file.path()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(internal)?;
        let urls =
            (self.store.signed_urls(Method::GET, &paths, lifetime).await).map_err(internal)?;
        let expiration_timestamp = expiration(lifetime);
        Ok(urls
            .into_iter()
            .map(|url| SignedFile {
                url: url.into(),
                expiration_timestamp,
            })
            .collect())
    }
}

/// Azure URLs with the vended SAS token, which is valid for the location of
/// the shared object.
struct WithSas {
    sas: String,
    expires_at: Option<i64>,
}

#[async_trait]
impl FileSigner for WithSas {
    async fn sign(&self, files: &[Url]) -> Result<Vec<SignedFile>> {
        files
            .iter()
            .map(|file| {
                Ok(SignedFile {
                    url: blob_url(file, &self.sas)?,
                    expiration_timestamp: self.expires_at,
                })
            })
            .collect()
    }
}

/// `abfss://<container>@<account>.dfs.core.windows.net/<path>` as the URL
/// of the blob with `sas`.
fn blob_url(file: &Url, sas: &str) -> Result<String> {
    let (container, host) = (file.username(), file.host_str().unwrap_or_default());
    if container.is_empty() || host.is_empty() {
        return Err(internal(format!("not an Azure storage URL: {file}")));
    }
    let host = host.replacen(".dfs.", ".blob.", 1);
    let sas = sas.trim_start_matches('?');
    Ok(format!("https://{host}/{container}{}?{sas}", file.path()))
}

/// An OAuth token cannot sign a URL: recipients read Google Cloud Storage
/// with the temporary credentials of the Open Sharing protocol.
struct NoGcsUrls;

#[async_trait]
impl FileSigner for NoGcsUrls {
    async fn sign(&self, files: &[Url]) -> Result<Vec<SignedFile>> {
        if files.is_empty() {
            return Ok(Vec::new());
        }
        Err(Error::NotImplemented(
            "file URLs for Google Cloud Storage; use temporary-table-credentials",
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use opensharing_engine::catalog::{AssetRef, ResolvedLocation};
    use uc_metastore::{MetastoreConfig, SqlMetastore};

    use super::*;
    use crate::storage::vendor::{CredentialContext, CredentialVendor};

    /// Gives AWS session credentials for every location, and counts the
    /// vends.
    #[derive(Default)]
    struct CountingVendor(AtomicUsize);

    #[async_trait]
    impl CredentialVendor for CountingVendor {
        async fn vend(
            &self,
            ctx: &CredentialContext,
        ) -> uc_common::Result<uc_api::TemporaryCredentials> {
            assert!(!ctx.writes(), "recipients only read");
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(uc_api::TemporaryCredentials {
                aws_temp_credentials: Some(uc_api::AwsCredentials {
                    access_key_id: Some("AKIDEXAMPLE".into()),
                    secret_access_key: Some("secret".into()),
                    session_token: Some("session".into()),
                }),
                expiration_time: Some(now_epoch_millis() + 30 * 60 * 1000),
                ..Default::default()
            })
        }

        fn covers(&self, _: &NormalizedUrl) -> bool {
            true
        }
    }

    fn table(location: &str) -> SharedAsset {
        SharedAsset::delta_table(
            AssetRef::table("share", "schema", "table"),
            ResolvedLocation::parse(location).unwrap(),
        )
    }

    #[tokio::test]
    async fn one_vend_serves_the_requests_of_a_location() {
        let metastore = (SqlMetastore::connect(MetastoreConfig::for_tests()).await).unwrap();
        let vendor = Arc::new(CountingVendor::default());
        let access = ServerAccess::new(metastore, vendor.clone());
        let recipient = Recipient {
            name: "partner".into(),
            shares: Vec::new(),
        };

        let events = table("s3://lake/tables/events");
        let storage = access.table_storage(&events, &recipient).await.unwrap();
        access.table_storage(&events, &recipient).await.unwrap();
        assert_eq!(vendor.0.load(Ordering::SeqCst), 1);
        assert_eq!(storage.root.as_str(), "s3://lake/tables/events/");

        // A file URL is signed with the session credentials, and ends
        // before they do.
        let file = Url::parse("s3://lake/tables/events/part-0.parquet").unwrap();
        let [signed] = storage
            .signer
            .sign(&[file])
            .await
            .unwrap()
            .try_into()
            .unwrap();
        let url = Url::parse(&signed.url).unwrap();
        assert!(
            url.path().ends_with("/tables/events/part-0.parquet"),
            "{url}"
        );
        let query: Vec<_> = url.query_pairs().map(|(k, _)| k.into_owned()).collect();
        for key in ["X-Amz-Signature", "X-Amz-Security-Token", "X-Amz-Expires"] {
            assert!(query.iter().any(|k| k == key), "{key} in {url}");
        }
        let left = signed.expiration_timestamp.unwrap() - now_epoch_millis();
        assert!(left > 0 && left <= 30 * 60 * 1000, "{left}");

        // Another location has its own credentials.
        let orders = table("s3://lake/tables/orders");
        access.table_storage(&orders, &recipient).await.unwrap();
        assert_eq!(vendor.0.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn blob_url_of_an_abfss_file() {
        let file = Url::parse("abfss://data@acct.dfs.core.windows.net/t/part-0.parquet").unwrap();
        assert_eq!(
            blob_url(&file, "?sv=1&sig=x").unwrap(),
            "https://acct.blob.core.windows.net/data/t/part-0.parquet?sv=1&sig=x"
        );
    }

    #[test]
    fn url_lifetime_ends_with_the_credentials() {
        assert_eq!(lifetime(None).unwrap(), URL_LIFETIME);
        let soon = now_epoch_millis() + 60_000;
        assert!(lifetime(Some(soon)).unwrap() <= Duration::from_secs(60));
        assert!(lifetime(Some(now_epoch_millis() - 1)).is_err());
    }
}
