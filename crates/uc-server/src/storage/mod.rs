//! Object storage with the server's own credentials. Upstream
//! `FileOperations`: local paths need none; cloud paths use credentials
//! vended for the operation (from the covering external location or
//! location of the settings), or else the server's environment credentials.
//! S3 paths go to the S3-compatible store of `identity.aws.endpoint`, if
//! any.

pub mod cleanup;
pub mod s3_region;
pub mod secrets;
pub mod vendor;

use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt, stream::BoxStream};
use object_store::{
    ObjectStore, StaticCredentialProvider,
    aws::AmazonS3Builder,
    azure::{AzureConfigKey, MicrosoftAzureBuilder},
    client::SpawnedReqwestConnector,
    gcp::{GcpCredential, GoogleCloudStorageBuilder},
    list::PaginatedListStore,
    local::LocalFileSystem,
    path::Path as ObjectPath,
};

use uc_api::TemporaryCredentials;
use uc_common::{NormalizedUrl, UriScheme, now_epoch_millis};
use uc_metastore::DynMetastore;
use uc_storage::runtime::io_handle;

use crate::storage::vendor::{
    CredentialContext, DynVendor, VendOp, aws::S3Store, credential_for_url,
};

/// Storage could not be reached or read.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StorageError(pub String);

impl From<object_store::Error> for StorageError {
    fn from(e: object_store::Error) -> Self {
        Self(e.to_string())
    }
}

impl From<uc_common::Error> for StorageError {
    fn from(e: uc_common::Error) -> Self {
        Self(e.message().to_string())
    }
}

/// Opens object stores for storage locations.
#[derive(Clone)]
pub struct ServerStorage {
    metastore: DynMetastore,
    vendor: DynVendor,
}

impl ServerStorage {
    pub fn new(metastore: DynMetastore, vendor: DynVendor) -> Self {
        Self { metastore, vendor }
    }

    /// A store that can reach `location`, and the location's path in it.
    pub async fn open(
        &self,
        location: &NormalizedUrl,
        op: VendOp,
    ) -> Result<(Arc<dyn ObjectStore>, ObjectPath), StorageError> {
        let opened = self.open_store(location, op).await?;
        Ok((opened.store, opened.root))
    }

    /// A store that can reach `location`, with its paged listing and the
    /// time at which its credentials expire.
    pub async fn open_store(
        &self,
        location: &NormalizedUrl,
        op: VendOp,
    ) -> Result<Opened, StorageError> {
        let root = ObjectPath::from(location.path().trim_matches('/'));
        if location.scheme() == UriScheme::File {
            return Ok(Opened {
                store: Arc::new(LocalFileSystem::new()),
                pages: None,
                root,
                expires_at: None,
            });
        }
        let credential = credential_for_url(&self.metastore, location.as_str()).await?;
        // Neither an external location nor a location of the settings: the
        // environment's credentials.
        if credential.is_none() && !self.vendor.covers(location) {
            let (store, pages) = build_store(location, None, self.vendor.s3_store())?;
            return Ok(Opened {
                store,
                pages,
                root,
                expires_at: None,
            });
        }
        let ctx = CredentialContext::new(location.clone(), op.privileges(), credential);
        let credentials = self.vendor.vend(&ctx).await?;
        let (store, pages) = build_store(location, Some(&credentials), self.vendor.s3_store())?;
        Ok(Opened {
            store,
            pages,
            root,
            expires_at: credentials.expiration_time,
        })
    }
}

/// A store opened for a location.
pub struct Opened {
    pub store: Arc<dyn ObjectStore>,
    /// The same store as a [`PaginatedListStore`]: S3, Azure and GCS list a
    /// page per request; local files have none.
    pub pages: Option<Arc<dyn PaginatedListStore>>,
    /// The location's path in the store.
    pub root: ObjectPath,
    /// When the vended credentials of the store expire, in epoch
    /// milliseconds; `None` without vended credentials.
    expires_at: Option<i64>,
}

/// Opened stores by location and operation, kept until shortly before
/// their credentials expire: one vend serves many requests, and requests
/// that come at the same time wait for one open.
#[derive(Clone)]
pub struct StoreCache(moka::future::Cache<(String, VendOp), Arc<Opened>>);

/// How long a store is kept at most. Grants are checked on every request;
/// this bounds how long a changed storage credential takes to apply.
const STORE_TTL: Duration = Duration::from_secs(5 * 60);

/// How long before its credentials expire a store is dropped.
const EXPIRY_MARGIN: Duration = Duration::from_secs(2 * 60);

struct StoreExpiry;

impl moka::Expiry<(String, VendOp), Arc<Opened>> for StoreExpiry {
    fn expire_after_create(
        &self,
        _key: &(String, VendOp),
        opened: &Arc<Opened>,
        _created_at: Instant,
    ) -> Option<Duration> {
        let Some(expires_at) = opened.expires_at else {
            return Some(STORE_TTL);
        };
        let left = u64::try_from(expires_at - now_epoch_millis()).unwrap_or(0);
        Some(
            Duration::from_millis(left)
                .saturating_sub(EXPIRY_MARGIN)
                .min(STORE_TTL),
        )
    }
}

impl Default for StoreCache {
    fn default() -> Self {
        Self(
            moka::future::Cache::builder()
                .max_capacity(1_000)
                .expire_after(StoreExpiry)
                .build(),
        )
    }
}

impl StoreCache {
    /// [`ServerStorage::open_store`], or the store opened before for the
    /// same location and operation.
    pub async fn open(
        &self,
        metastore: &DynMetastore,
        vendor: &DynVendor,
        location: &NormalizedUrl,
        op: VendOp,
    ) -> Result<Arc<Opened>, StorageError> {
        let key = (location.as_str().to_string(), op);
        let open = async {
            let storage = ServerStorage::new(metastore.clone(), vendor.clone());
            storage.open_store(location, op).await.map(Arc::new)
        };
        self.0
            .try_get_with(key, open)
            .await
            .map_err(|e| StorageError(e.0.clone()))
    }
}

/// The store of `location` with `credentials`, and its paged listing.
type Built = (Arc<dyn ObjectStore>, Option<Arc<dyn PaginatedListStore>>);

fn build_store(
    location: &NormalizedUrl,
    credentials: Option<&TemporaryCredentials>,
    s3_store: Option<&S3Store>,
) -> Result<Built, StorageError> {
    let url = location.as_str();
    Ok(match location.scheme() {
        UriScheme::S3 => {
            let mut builder = AmazonS3Builder::from_env()
                .with_url(url)
                .with_http_connector(SpawnedReqwestConnector::new(io_handle()));
            if let Some(store) = s3_store {
                builder = builder
                    .with_endpoint(&store.endpoint)
                    .with_region(&store.region)
                    .with_allow_http(true);
            }
            if let Some(aws) = credentials.and_then(|c| c.aws_temp_credentials.as_ref()) {
                if let Some(key) = &aws.access_key_id {
                    builder = builder.with_access_key_id(key);
                }
                if let Some(secret) = &aws.secret_access_key {
                    builder = builder.with_secret_access_key(secret);
                }
                if let Some(token) = &aws.session_token {
                    builder = builder.with_token(token);
                }
            }
            let store = Arc::new(builder.build()?);
            (store.clone(), Some(store))
        }
        UriScheme::Gs => {
            let mut builder = GoogleCloudStorageBuilder::from_env()
                .with_url(url)
                .with_http_connector(SpawnedReqwestConnector::new(io_handle()));
            let token = credentials
                .and_then(|c| c.gcp_oauth_token.as_ref())
                .and_then(|t| t.oauth_token.clone());
            if let Some(bearer) = token {
                let provider = StaticCredentialProvider::new(GcpCredential { bearer });
                builder = builder.with_credentials(Arc::new(provider));
            }
            let store = Arc::new(builder.build()?);
            (store.clone(), Some(store))
        }
        UriScheme::Abfs | UriScheme::Abfss => {
            let mut builder = MicrosoftAzureBuilder::from_env()
                .with_url(url)
                .with_http_connector(SpawnedReqwestConnector::new(io_handle()));
            let sas = credentials
                .and_then(|c| c.azure_user_delegation_sas.as_ref())
                .and_then(|s| s.sas_token.clone());
            if let Some(sas) = sas {
                builder = builder.with_config(AzureConfigKey::SasKey, sas);
            }
            let store = Arc::new(builder.build()?);
            (store.clone(), Some(store))
        }
        UriScheme::File => (Arc::new(LocalFileSystem::new()), None),
    })
}

/// Delete every object under `prefix`, in batches where the store has
/// them (S3, Azure, GCS).
pub async fn delete_all(store: &dyn ObjectStore, prefix: &ObjectPath) -> object_store::Result<()> {
    let locations = store
        .list(Some(prefix))
        .map_ok(|meta| meta.location)
        .boxed();
    store
        .delete_stream(locations)
        .try_for_each(|_| futures::future::ready(Ok(())))
        .await
}

/// Whether two objects hold the same bytes. Both are read in step, so
/// memory stays bounded and neither is read past the other's length.
pub async fn same_content(
    store: &dyn ObjectStore,
    a: &ObjectPath,
    b: &ObjectPath,
) -> Result<bool, StorageError> {
    let mut a = Chunks::new(store.get(a).await?.into_stream());
    let mut b = Chunks::new(store.get(b).await?.into_stream());
    loop {
        let (x, y) = (a.peek().await?, b.peek().await?);
        if x.is_empty() || y.is_empty() {
            return Ok(x.is_empty() && y.is_empty());
        }
        let n = x.len().min(y.len());
        if x[..n] != y[..n] {
            return Ok(false);
        }
        a.consume(n);
        b.consume(n);
    }
}

struct Chunks {
    stream: BoxStream<'static, object_store::Result<Bytes>>,
    unread: Bytes,
}

impl Chunks {
    fn new(stream: BoxStream<'static, object_store::Result<Bytes>>) -> Self {
        Self {
            stream,
            unread: Bytes::new(),
        }
    }

    /// The unread part of the current chunk; empty at the end.
    async fn peek(&mut self) -> Result<Bytes, StorageError> {
        while self.unread.is_empty() {
            match self.stream.next().await {
                Some(chunk) => self.unread = chunk?,
                None => break,
            }
        }
        Ok(self.unread.clone())
    }

    fn consume(&mut self, n: usize) {
        self.unread = self.unread.slice(n..);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::{PutPayload, memory::InMemory};

    #[tokio::test]
    async fn compares_contents_in_step() {
        let store = InMemory::new();
        let put = |name: &str, body: &'static str| {
            let store = &store;
            let path = ObjectPath::from(name);
            async move {
                store
                    .put(&path, PutPayload::from_static(body.as_bytes()))
                    .await
                    .unwrap()
            }
        };
        put("a", "same bytes").await;
        put("b", "same bytes").await;
        put("c", "same bytes, longer").await;
        put("d", "diff bytes").await;
        let p = ObjectPath::from;
        assert!(same_content(&store, &p("a"), &p("b")).await.unwrap());
        assert!(!same_content(&store, &p("a"), &p("c")).await.unwrap());
        assert!(!same_content(&store, &p("a"), &p("d")).await.unwrap());
        assert!(same_content(&store, &p("a"), &p("missing")).await.is_err());
    }

    /// With `identity.aws.endpoint`, S3 requests go to the store, path-style.
    #[tokio::test]
    async fn s3_paths_go_to_the_store() {
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        let app = axum::Router::new().fallback(
            move |method: axum::http::Method, uri: axum::http::Uri| {
                let log = log.clone();
                async move {
                    log.lock().unwrap().push(format!("{method} {uri}"));
                    [("etag", "\"1\"")]
                }
            },
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let credentials = TemporaryCredentials {
            aws_temp_credentials: Some(uc_api::AwsCredentials {
                access_key_id: Some("key".into()),
                secret_access_key: Some("secret".into()),
                session_token: Some("token".into()),
            }),
            ..Default::default()
        };
        let store = S3Store {
            endpoint,
            region: "us-east-1".into(),
        };
        let location = NormalizedUrl::parse("s3://bucket/tables/t1").unwrap();
        let (s3, _) = build_store(&location, Some(&credentials), Some(&store)).unwrap();
        let path = ObjectPath::from("tables/t1/part-0");
        s3.put(&path, PutPayload::from_static(b"x")).await.unwrap();
        assert_eq!(*seen.lock().unwrap(), ["PUT /bucket/tables/t1/part-0"]);
    }
}
