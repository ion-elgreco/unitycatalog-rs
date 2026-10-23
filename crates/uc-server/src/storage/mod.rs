//! Object storage with the server's own credentials. Upstream
//! `FileOperations`: local paths need none; cloud paths use credentials
//! vended for the operation (from the covering external location or the
//! bucket's configuration), or else the server's environment credentials.
//! S3 paths go to the S3-compatible store of `aws.endpoint`, if any.

pub mod cleanup;
pub mod s3_region;
pub mod vendor;

use std::sync::Arc;

use bytes::Bytes;
use futures::{StreamExt, stream::BoxStream};
use object_store::{
    ObjectStore, ObjectStoreExt, StaticCredentialProvider,
    aws::AmazonS3Builder,
    azure::{AzureConfigKey, MicrosoftAzureBuilder},
    client::SpawnedReqwestConnector,
    gcp::{GcpCredential, GoogleCloudStorageBuilder},
    local::LocalFileSystem,
    path::Path as ObjectPath,
};

use uc_api::TemporaryCredentials;
use uc_common::{ErrorCode, NormalizedUrl, UriScheme};
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
        let path = ObjectPath::from(location.path().trim_matches('/'));
        if location.scheme() == UriScheme::File {
            return Ok((Arc::new(LocalFileSystem::new()), path));
        }
        let credentials = self.credentials(location, op).await?;
        let store = build_store(location, credentials.as_ref(), self.vendor.s3_store())?;
        Ok((store, path))
    }

    /// Credentials for the cloud `location` that allow `op`: vended, or
    /// none for the environment's credentials.
    pub async fn credentials(
        &self,
        location: &NormalizedUrl,
        op: VendOp,
    ) -> Result<Option<TemporaryCredentials>, StorageError> {
        let credential = credential_for_url(&self.metastore, location.as_str()).await?;
        let under_location = credential.is_some();
        let ctx = CredentialContext::new(location.clone(), op.privileges(), credential);
        match self.vendor.vend(&ctx).await {
            Ok(credentials) => Ok(Some(credentials)),
            // Neither an external location nor a bucket configuration: the
            // environment's credentials.
            Err(e) if !under_location && e.code() == ErrorCode::FailedPrecondition => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

fn build_store(
    location: &NormalizedUrl,
    credentials: Option<&TemporaryCredentials>,
    s3_store: Option<&S3Store>,
) -> Result<Arc<dyn ObjectStore>, StorageError> {
    let url = location.as_str();
    Ok(match location.scheme() {
        UriScheme::S3 => Arc::new(s3_builder(url, credentials, s3_store).build()?),
        UriScheme::Gs => Arc::new(gcs_builder(url, credentials).build()?),
        UriScheme::Abfs | UriScheme::Abfss => Arc::new(azure_builder(url, credentials).build()?),
        UriScheme::File => Arc::new(LocalFileSystem::new()),
    })
}

/// A store for the S3 `url`, with the vended `credentials` if any, in the
/// S3-compatible `s3_store` if any.
pub(crate) fn s3_builder(
    url: &str,
    credentials: Option<&TemporaryCredentials>,
    s3_store: Option<&S3Store>,
) -> AmazonS3Builder {
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
    builder
}

/// A store for the GCS `url`, with the vended `credentials` if any.
pub(crate) fn gcs_builder(
    url: &str,
    credentials: Option<&TemporaryCredentials>,
) -> GoogleCloudStorageBuilder {
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
    builder
}

/// A store for the ADLS `url`, with the vended `credentials` if any.
pub(crate) fn azure_builder(
    url: &str,
    credentials: Option<&TemporaryCredentials>,
) -> MicrosoftAzureBuilder {
    let mut builder = MicrosoftAzureBuilder::from_env()
        .with_url(url)
        .with_http_connector(SpawnedReqwestConnector::new(io_handle()));
    let sas = credentials
        .and_then(|c| c.azure_user_delegation_sas.as_ref())
        .and_then(|s| s.sas_token.clone());
    if let Some(sas) = sas {
        builder = builder.with_config(AzureConfigKey::SasKey, sas);
    }
    builder
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

    /// With `aws.endpoint`, S3 requests go to the store, path-style.
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
        let s3 = build_store(&location, Some(&credentials), Some(&store)).unwrap();
        let path = ObjectPath::from("tables/t1/part-0");
        s3.put(&path, PutPayload::from_static(b"x")).await.unwrap();
        assert_eq!(*seen.lock().unwrap(), ["PUT /bucket/tables/t1/part-0"]);
    }
}
