//! Storage with credentials the server vends, as upstream's
//! `DeltaKernelUtils.getHDFSConfiguration`. Local paths need none; S3, GCS
//! and ADLS take the vended credentials over the environment's. S3 paths go
//! to the S3-compatible store of `aws.endpoint`, if any.

use std::sync::Arc;

use anyhow::Context as _;
use delta_kernel::object_store::aws::AmazonS3Builder;
use delta_kernel::object_store::azure::{AzureConfigKey, MicrosoftAzureBuilder};
use delta_kernel::object_store::gcp::{GcpCredential, GoogleCloudStorageBuilder};
use delta_kernel::object_store::local::LocalFileSystem;
use delta_kernel::object_store::{ObjectStore, StaticCredentialProvider};
use uc_client::models::credentials::v1::{
    PathOperation, TableOperation, TemporaryCredentials, VolumeOperation,
};
use uc_common::settings;
use url::Url;

use crate::commands::{Context, upstream};

/// An S3-compatible store, such as MinIO or RustFS, in place of AWS: the
/// server's `aws.endpoint` and `aws.region`, where its S3 credentials work.
#[derive(Debug, Clone)]
pub struct S3Store {
    endpoint: String,
    region: Option<String>,
}

impl S3Store {
    /// The store of `[aws]`; none without `endpoint`.
    pub fn from_settings(aws: &settings::Aws) -> Option<Self> {
        let endpoint = aws.endpoint.as_deref().map(str::trim)?;
        (!endpoint.is_empty()).then(|| Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            region: aws.region.clone(),
        })
    }
}

/// `POST /temporary-table-credentials` for `operation`.
pub async fn table_credentials(
    c: &Context<'_>,
    table_id: &str,
    operation: TableOperation,
) -> anyhow::Result<TemporaryCredentials> {
    c.client
        .generate_temporary_table_credentials(table_id, operation)
        .await
        .map_err(upstream("generateTemporaryTableCredentials"))
}

/// `POST /temporary-volume-credentials` for `operation`.
pub async fn volume_credentials(
    c: &Context<'_>,
    volume_id: &str,
    operation: VolumeOperation,
) -> anyhow::Result<TemporaryCredentials> {
    c.client
        .generate_temporary_volume_credentials(volume_id, operation)
        .await
        .map_err(upstream("generateTemporaryVolumeCredentials"))
}

/// `POST /temporary-path-credentials` for `operation`.
pub async fn path_credentials(
    c: &Context<'_>,
    url: &str,
    operation: PathOperation,
) -> anyhow::Result<TemporaryCredentials> {
    c.client
        .generate_temporary_path_credentials(url, operation)
        .await
        .map_err(upstream("generateTemporaryPathCredentials"))
}

/// A store that reaches `url`, with `credentials` where they apply; S3 paths
/// in `s3`, if any.
pub fn store_for(
    url: &Url,
    credentials: Option<&TemporaryCredentials>,
    s3: Option<&S3Store>,
) -> anyhow::Result<Arc<dyn ObjectStore>> {
    let store: Arc<dyn ObjectStore> = match url.scheme() {
        "file" => Arc::new(LocalFileSystem::new()),
        "s3" | "s3a" => {
            let mut builder = AmazonS3Builder::from_env().with_url(url.as_str());
            if let Some(s3) = s3 {
                builder = builder.with_endpoint(&s3.endpoint).with_allow_http(true);
                if let Some(region) = &s3.region {
                    builder = builder.with_region(region);
                }
            }
            if let Some(aws) = credentials.and_then(|c| c.aws_temp_credentials.as_option()) {
                if let Some(key) = given(&aws.access_key_id) {
                    builder = builder.with_access_key_id(key);
                }
                if let Some(secret) = given(&aws.secret_access_key) {
                    builder = builder.with_secret_access_key(secret);
                }
                if let Some(token) = given(&aws.session_token) {
                    builder = builder.with_token(token);
                }
            }
            Arc::new(builder.build().context("S3 storage")?)
        }
        "gs" => {
            let mut builder = GoogleCloudStorageBuilder::from_env().with_url(url.as_str());
            let token = credentials.and_then(|c| c.gcp_oauth_token.as_option());
            if let Some(bearer) = token.and_then(|t| given(&t.oauth_token)) {
                let provider = StaticCredentialProvider::new(GcpCredential {
                    bearer: bearer.to_string(),
                });
                builder = builder.with_credentials(Arc::new(provider));
            }
            Arc::new(builder.build().context("GCS storage")?)
        }
        "abfs" | "abfss" => {
            let mut builder = MicrosoftAzureBuilder::from_env().with_url(url.as_str());
            let sas = credentials.and_then(|c| c.azure_user_delegation_sas.as_option());
            if let Some(token) = sas.and_then(|s| given(&s.sas_token)) {
                builder = builder.with_config(AzureConfigKey::SasKey, token);
            }
            Arc::new(builder.build().context("Azure storage")?)
        }
        other => anyhow::bail!("Unsupported URI scheme: {other}"),
    };
    Ok(store)
}

/// A value of the vended credentials; none when the server left it out.
fn given(value: &str) -> Option<&str> {
    Some(value).filter(|value| !value.is_empty())
}

/// `location` as a directory URL (ending in `/`).
pub fn directory_url(location: &str) -> anyhow::Result<Url> {
    let mut text = location.to_string();
    if !text.ends_with('/') {
        text.push('/');
    }
    // `file:/x` as `file:///x`.
    if let Some(rest) = text.strip_prefix("file:/")
        && !rest.starts_with('/')
    {
        text = format!("file:///{rest}");
    }
    Url::parse(&text).with_context(|| format!("Invalid storage location: {location}"))
}
