//! The AWS region of an S3 bucket, which Iceberg clients need with vended
//! credentials. Upstream `FileOperationsImpl`: a configured region, else
//! the bucket's answer to an anonymous HeadBucket, cached per bucket.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use tracing::warn;

use uc_common::{Error, ErrorCode, NormalizedUrl, Result};

/// The header S3 names a bucket's region in, also on redirects and denials.
const BUCKET_REGION_HEADER: &str = "x-amz-bucket-region";

pub struct S3Regions {
    known: RwLock<HashMap<NormalizedUrl, String>>,
    client: reqwest::Client,
}

impl Default for S3Regions {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            known: RwLock::new(HashMap::new()),
            client,
        }
    }
}

impl S3Regions {
    /// Configure the region of the bucket `storage_base` (`s3://bucket`).
    pub fn configure(&self, storage_base: &NormalizedUrl, region: impl Into<String>) {
        if let Ok(mut known) = self.known.write() {
            known.insert(storage_base.storage_base(), region.into());
        }
    }

    /// The region of the bucket of `location`.
    pub async fn region(&self, location: &NormalizedUrl) -> Result<String> {
        let base = location.storage_base();
        if let Some(region) = self
            .known
            .read()
            .ok()
            .and_then(|known| known.get(&base).cloned())
        {
            return Ok(region);
        }
        // Discovery runs outside the lock; a bucket whose discovery fails
        // is not cached and is probed again next time.
        let region = self.discover(&base).await?;
        if let Ok(mut known) = self.known.write() {
            known.entry(base).or_insert_with(|| region.clone());
        }
        Ok(region)
    }

    /// Upstream `discoverRegion`: an anonymous HeadBucket. S3 names the
    /// region even when it redirects or denies the request.
    async fn discover(&self, base: &NormalizedUrl) -> Result<String> {
        let bucket = base.as_str().strip_prefix("s3://").unwrap_or_default();
        let response = match self
            .client
            .head(format!("https://s3.amazonaws.com/{bucket}"))
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => return Err(transient(base, &e.to_string())),
        };
        if let Some(region) = response
            .headers()
            .get(BUCKET_REGION_HEADER)
            .and_then(|v| v.to_str().ok())
        {
            return Ok(region.to_string());
        }
        let status = response.status();
        // Server errors, timeouts and throttling may pass; any other answer
        // without a region (no such bucket) will not.
        if status.is_server_error() || status.as_u16() == 408 || status.as_u16() == 429 {
            return Err(transient(base, &format!("HeadBucket answered {status}")));
        }
        warn!(bucket = %base, %status, "could not resolve the AWS region");
        Err(Error::new(
            ErrorCode::FailedPrecondition,
            format!(
                "Could not resolve the AWS region for S3 bucket {base}; set the region of its \
                 [s3.<name>] settings."
            ),
        ))
    }
}

fn transient(base: &NormalizedUrl, reason: &str) -> Error {
    warn!(bucket = %base, reason, "transient failure resolving the AWS region");
    Error::internal(format!(
        "Could not resolve the AWS region for S3 bucket {base}; retry the request."
    ))
}
