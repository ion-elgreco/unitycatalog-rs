//! Cloud credential vending, after upstream `StorageCredentialVendor` and
//! `CloudCredentialVendor`.
//!
//! Credentials for a path come from the storage credential of the external
//! location that covers it (an AWS role, an Azure service principal, an
//! Azure managed identity, a GCP service account or a GCP service account
//! key) or, without one, from the per-bucket settings
//! (`[s3.<name>]`, `[adls.<name>]`, `[gcs.<name>]`). The
//! vendor is chosen by the path's scheme: AWS STS for `s3`, an Azure user
//! delegation SAS for `abfs`/`abfss`, a downscoped GCP token for `gs`, and
//! nothing for local paths.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;

use uc_api::{AwsCredentials, CredentialInfo, TemporaryCredentials};
use uc_common::{Error, NormalizedUrl, Result, UriScheme, now_epoch_millis};
use uc_metastore::DynMetastore;

use uc_common::settings::{self, Settings};

pub mod aws;
pub mod azure;
pub mod gcp;

/// What the server itself does with a location; see [`ServerStorage`].
///
/// [`ServerStorage`]: crate::storage::ServerStorage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendOp {
    Read,
    ReadWrite,
}

impl VendOp {
    pub fn privileges(self) -> &'static [StoragePrivilege] {
        match self {
            Self::Read => &[StoragePrivilege::Select],
            Self::ReadWrite => &[StoragePrivilege::Select, StoragePrivilege::Update],
        }
    }
}

/// Upstream `CredentialContext.Privilege`: what a vended credential allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoragePrivilege {
    Select,
    Update,
}

/// What a credential is for, upstream `CredentialContext`.
#[derive(Debug, Clone)]
pub struct CredentialContext {
    pub scheme: UriScheme,
    /// `scheme://authority`: the bucket, or the container and account.
    pub storage_base: NormalizedUrl,
    pub privileges: Vec<StoragePrivilege>,
    pub locations: Vec<NormalizedUrl>,
    /// The storage credential of the covering external location, if any.
    pub credential: Option<StorageCredential>,
}

/// A storage credential, with its secret, if any: the client secret of its
/// Azure service principal, or its GCP service account key.
#[derive(Debug, Clone)]
pub struct StorageCredential {
    pub info: CredentialInfo,
    pub secret: Option<String>,
}

impl CredentialContext {
    pub fn new(
        location: NormalizedUrl,
        privileges: &[StoragePrivilege],
        credential: Option<StorageCredential>,
    ) -> Self {
        Self {
            scheme: location.scheme(),
            storage_base: location.storage_base(),
            privileges: privileges.to_vec(),
            locations: vec![location],
            credential,
        }
    }

    /// Whether the credential may write.
    pub fn writes(&self) -> bool {
        self.privileges.contains(&StoragePrivilege::Update)
    }

    pub fn op(&self) -> VendOp {
        match self.writes() {
            true => VendOp::ReadWrite,
            false => VendOp::Read,
        }
    }
}

#[async_trait]
pub trait CredentialVendor: Send + Sync + 'static {
    /// Temporary credentials for `ctx`.
    async fn vend(&self, ctx: &CredentialContext) -> Result<TemporaryCredentials>;

    /// The S3-compatible store that vended S3 credentials are for, if not
    /// AWS (`aws.endpoint`).
    fn s3_store(&self) -> Option<&aws::S3Store> {
        None
    }
}

pub type DynVendor = Arc<dyn CredentialVendor>;

/// Upstream `StorageCredentialVendor.vendCredential`: credentials for
/// `path` that allow `privileges`. The response names the path in `url`.
pub async fn vend_credential(
    metastore: &DynMetastore,
    vendor: &DynVendor,
    path: &NormalizedUrl,
    privileges: &[StoragePrivilege],
) -> Result<TemporaryCredentials> {
    if path.is_cloud_storage_root() {
        return Err(Error::invalid_argument(format!(
            "Storage location must include a non-empty path prefix before credentials can be \
             vended: {path}"
        )));
    }
    if privileges.is_empty() {
        return Err(Error::invalid_argument(
            "Privileges cannot be null or empty.",
        ));
    }
    let credential = credential_for_url(metastore, path.as_str()).await?;
    let mut credentials = vendor
        .vend(&CredentialContext::new(
            path.clone(),
            privileges,
            credential,
        ))
        .await?;
    credentials.url = Some(path.to_string());
    Ok(credentials)
}

/// The text of the first `<tag>` element.
pub(crate) fn xml_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = xml[start..].find(&format!("</{tag}>"))? + start;
    Some(&xml[start..end])
}

/// The stored credential of the external location that covers `url`, if
/// any.
pub async fn credential_for_url(
    metastore: &DynMetastore,
    url: &str,
) -> Result<Option<StorageCredential>> {
    let Some(location) = metastore
        .external_locations()
        .find_by_url_prefix(url)
        .await?
    else {
        return Ok(None);
    };
    let name = location
        .credential_name
        .ok_or_else(|| Error::internal("external_location row missing credential_name"))?;
    let (info, secret) = metastore.credentials().get_with_secret(&name).await?;
    Ok(Some(StorageCredential { info, secret }))
}

/// The per-bucket and master configuration of the settings.
#[derive(Debug, Clone, Default)]
pub struct StorageConfig {
    /// `[aws]`: the server's own AWS identity, which assumes the roles of
    /// storage credentials.
    pub aws_master: aws::S3Config,
    /// `aws.endpoint`: an S3-compatible store in place of AWS.
    pub s3_store: Option<aws::S3Store>,
    /// `[s3.<name>]`, by bucket (`s3://bucket`).
    pub s3: HashMap<NormalizedUrl, aws::S3Config>,
    /// `[adls.<name>]`, by storage account name.
    pub adls: HashMap<String, azure::AdlsConfig>,
    /// `gcp.endpoint`: a GCP emulator in place of Google's STS and IAM
    /// Credentials.
    pub gcp_endpoint: Option<String>,
    /// `[gcs.<name>]`, by bucket (`gs://bucket`).
    pub gcs: HashMap<NormalizedUrl, gcp::GcsConfig>,
}

impl StorageConfig {
    /// `[aws]`, and the entries of `[s3.<name>]`, `[adls.<name>]` and
    /// `[gcs.<name>]`. An incomplete entry is an error.
    pub fn from_settings(settings: &Settings) -> std::result::Result<Self, String> {
        let required = |value: &Option<String>, key: String| {
            value
                .clone()
                .ok_or_else(|| format!("{} is required", settings::name(&key)))
        };
        let bucket = |bucket: &str, key: String| {
            NormalizedUrl::parse(bucket)
                .map(|url| url.storage_base())
                .map_err(|e| settings::invalid(&key, bucket, e.message()))
        };
        let aws_master = aws::S3Config {
            region: settings.aws.region.clone(),
            access_key: settings.aws.access_key.clone(),
            secret_key: settings.aws.secret_key.clone(),
            ..Default::default()
        };
        let endpoint = |value: &Option<String>, key: &str| match value.as_deref().map(str::trim) {
            None | Some("") => Ok(None),
            Some(url) if url.starts_with("http://") || url.starts_with("https://") => {
                Ok(Some(url.trim_end_matches('/').to_string()))
            }
            Some(url) => Err(settings::invalid(
                key,
                url,
                "Expected an http or https URL.",
            )),
        };
        let gcp_endpoint = endpoint(&settings.gcp.endpoint, "gcp.endpoint")?;
        let s3_store = match endpoint(&settings.aws.endpoint, "aws.endpoint")? {
            None => None,
            Some(endpoint) => {
                let region = settings.aws.region.clone();
                Some(aws::S3Store {
                    endpoint,
                    region: region.unwrap_or_else(|| "us-east-1".into()),
                })
            }
        };
        let mut s3 = HashMap::new();
        for (name, entry) in &settings.s3 {
            let path = required(&entry.bucket, format!("s3.{name}.bucket"))?;
            let config = aws::S3Config {
                region: entry.region.clone(),
                aws_role_arn: entry.role_arn.clone(),
                access_key: entry.access_key.clone(),
                secret_key: entry.secret_key.clone(),
                session_token: entry.session_token.clone(),
            };
            let role = config.region.is_some() && config.aws_role_arn.is_some();
            let keys = config.access_key.is_some()
                && config.secret_key.is_some()
                && config.session_token.is_some();
            if !(role || keys) {
                return Err(format!(
                    "[s3.{name}] needs region and role_arn, or access_key, secret_key and \
                     session_token"
                ));
            }
            s3.insert(bucket(&path, format!("s3.{name}.bucket"))?, config);
        }
        let mut adls = HashMap::new();
        for (name, entry) in &settings.adls {
            let key = |field: &str| format!("adls.{name}.{field}");
            let identity = match entry.managed_identity {
                true => azure::Identity::Managed(entry.managed_identity_id.clone()),
                false => azure::Identity::ServicePrincipal {
                    tenant_id: required(&entry.tenant_id, key("tenant_id"))?,
                    client_id: required(&entry.client_id, key("client_id"))?,
                    client_secret: required(&entry.client_secret, key("client_secret"))?,
                },
            };
            adls.insert(
                required(&entry.account, key("account"))?,
                azure::AdlsConfig {
                    identity,
                    test_mode: entry.test_mode,
                },
            );
        }
        let mut gcs = HashMap::new();
        for (name, entry) in &settings.gcs {
            let path = required(&entry.bucket, format!("gcs.{name}.bucket"))?;
            let config = gcp::GcsConfig {
                bucket_path: path.clone(),
                json_key_file_path: entry.json_key_file.clone(),
                service_account: entry.service_account.clone(),
                credential_generator: entry.credential_generator.clone(),
            };
            gcs.insert(bucket(&path, format!("gcs.{name}.bucket"))?, config);
        }
        Ok(Self {
            aws_master,
            s3_store,
            s3,
            adls,
            gcp_endpoint,
            gcs,
        })
    }
}

/// Upstream `CloudCredentialVendor`: the vendor of the path's scheme.
pub struct CloudVendor {
    aws: aws::AwsVendor,
    azure: azure::AzureVendor,
    gcp: gcp::GcpVendor,
}

impl CloudVendor {
    pub fn new(config: StorageConfig) -> Self {
        Self {
            aws: aws::AwsVendor::new(config.aws_master, config.s3, config.s3_store),
            azure: azure::AzureVendor::new(config.adls),
            gcp: match config.gcp_endpoint {
                Some(url) => gcp::GcpVendor::new(config.gcs).with_endpoints(
                    Some(format!("{url}/v1/token")),
                    None,
                    Some(url),
                ),
                None => gcp::GcpVendor::new(config.gcs),
            },
        }
    }

    /// The vendor with its cloud endpoints replaced, for tests.
    pub fn with_endpoints(mut self, endpoints: Endpoints) -> Self {
        if let Some(sts) = endpoints.aws_sts {
            self.aws = self.aws.with_endpoint(sts);
        }
        self.azure = self.azure.with_endpoints(
            endpoints.azure_login,
            endpoints.azure_blob,
            endpoints.azure_msi,
        );
        self.gcp =
            self.gcp
                .with_endpoints(endpoints.gcp_sts, endpoints.gcp_metadata, endpoints.gcp_iam);
        self
    }
}

/// Cloud endpoints to use instead of the real ones, for tests.
#[derive(Debug, Clone, Default)]
pub struct Endpoints {
    pub aws_sts: Option<String>,
    pub azure_login: Option<String>,
    /// Replaces `https://{account}.blob.core.windows.net`.
    pub azure_blob: Option<String>,
    /// Replaces the token endpoint of the instance metadata service.
    pub azure_msi: Option<String>,
    pub gcp_sts: Option<String>,
    pub gcp_metadata: Option<String>,
    /// Replaces `https://iamcredentials.googleapis.com`.
    pub gcp_iam: Option<String>,
}

#[async_trait]
impl CredentialVendor for CloudVendor {
    async fn vend(&self, ctx: &CredentialContext) -> Result<TemporaryCredentials> {
        match ctx.scheme {
            UriScheme::S3 => self.aws.vend(ctx).await,
            UriScheme::Abfs | UriScheme::Abfss => self.azure.vend(ctx).await,
            UriScheme::Gs => self.gcp.vend(ctx).await,
            UriScheme::File => Ok(TemporaryCredentials::default()),
        }
    }

    fn s3_store(&self) -> Option<&aws::S3Store> {
        self.aws.store()
    }
}

/// A vendor for tests and local development: for an S3 path under an
/// external location it returns the credential's role ARN as the session
/// token, without calling AWS. Other paths are vended as by an
/// unconfigured [`CloudVendor`].
pub struct PlaceholderVendor;

#[async_trait]
impl CredentialVendor for PlaceholderVendor {
    async fn vend(&self, ctx: &CredentialContext) -> Result<TemporaryCredentials> {
        match (&ctx.credential, ctx.scheme) {
            (Some(credential), UriScheme::S3) => Ok(TemporaryCredentials {
                aws_temp_credentials: Some(AwsCredentials {
                    access_key_id: None,
                    secret_access_key: None,
                    session_token: credential
                        .info
                        .aws_iam_role
                        .as_ref()
                        .and_then(|r| r.role_arn.clone()),
                }),
                expiration_time: Some(now_epoch_millis() + 60 * 60 * 1000),
                ..Default::default()
            }),
            _ => {
                static UNCONFIGURED: OnceLock<CloudVendor> = OnceLock::new();
                UNCONFIGURED
                    .get_or_init(|| CloudVendor::new(StorageConfig::default()))
                    .vend(ctx)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure::Identity;

    fn accounts(text: &str) -> std::result::Result<HashMap<String, azure::AdlsConfig>, String> {
        StorageConfig::from_settings(&Settings::from_toml(text)?).map(|c| c.adls)
    }

    #[test]
    fn adls_entries_hold_a_service_principal_or_a_managed_identity() {
        let found = accounts(
            r#"
            [adls.sp]
            account = "one"
            tenant_id = "t"
            client_id = "c"
            client_secret = "s"

            [adls.own]
            account = "two"
            managed_identity = true

            [adls.assigned]
            account = "three"
            managed_identity = true
            managed_identity_id = "/subscriptions/s/uc"
            "#,
        )
        .unwrap();
        let principal = Identity::ServicePrincipal {
            tenant_id: "t".into(),
            client_id: "c".into(),
            client_secret: "s".into(),
        };
        assert_eq!(found["one"].identity, principal);
        assert_eq!(found["two"].identity, Identity::Managed(None));
        assert_eq!(
            found["three"].identity,
            Identity::Managed(Some("/subscriptions/s/uc".into()))
        );
        // Without a managed identity, the service principal is required.
        let error = accounts("[adls.x]\naccount = 'a'\ntenant_id = 't'").unwrap_err();
        assert!(error.contains("adls.x.client_id"), "{error}");
    }
}
