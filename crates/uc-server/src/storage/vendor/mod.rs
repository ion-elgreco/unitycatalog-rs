//! Cloud credential vending, after upstream `StorageCredentialVendor` and
//! `CloudCredentialVendor`.
//!
//! Credentials for a path come from the identity of the storage credential
//! of the external location that covers it (an AWS role, an Azure service
//! principal, an Azure managed identity, a GCP service account or a GCP
//! service account key), else of the [`Location`] of the settings that
//! covers it. The server's own identity in the cloud (`[identity.<cloud>]`)
//! signs in for it. The vendor is chosen by the path's scheme: AWS STS for
//! `s3`, an Azure user delegation SAS for `abfs`/`abfss`, a downscoped GCP
//! token for `gs`, and nothing for local paths.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;

use uc_api::{AwsCredentials, CredentialInfo, TemporaryCredentials};
use uc_common::{Error, NormalizedUrl, Result, UriScheme, now_epoch_millis};
use uc_metastore::{DynMetastore, SecretValue};

use uc_common::settings::{self, Settings};

pub mod aws;
pub mod azure;
pub mod gcp;
mod location;

pub use location::{AdlsLocation, GcsLocation, Location, S3Location};

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
    pub secret: Option<SecretValue>,
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
    /// AWS (`identity.aws.endpoint`).
    fn s3_store(&self) -> Option<&aws::S3Store> {
        None
    }

    /// Whether a location of the settings covers `path`.
    fn covers(&self, _path: &NormalizedUrl) -> bool {
        false
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

/// The settings of credential vending: the server's identity in each cloud,
/// and the locations of the settings.
#[derive(Debug, Clone, Default)]
pub struct StorageConfig {
    /// `[identity.aws]`: the server's AWS identity, which assumes the roles
    /// of storage credentials.
    pub aws: aws::AwsIdentity,
    /// `identity.aws.endpoint`: an S3-compatible store in place of AWS.
    pub s3_store: Option<aws::S3Store>,
    /// `[identity.azure]`: the server's Azure identity; none for the
    /// identity of the environment.
    pub azure: Option<azure::Identity>,
    /// `identity.gcp.json_key_file`: the server's GCP identity; none for
    /// Application Default Credentials.
    pub gcp_key_file: Option<PathBuf>,
    /// `identity.gcp.endpoint`: a GCP emulator in place of Google's STS and
    /// IAM Credentials.
    pub gcp_endpoint: Option<String>,
    /// `[locations.<name>]`.
    pub locations: Vec<Location>,
    /// `secrets.cache_*`: the sign-ins of Azure identities kept in memory.
    pub sign_in_cache: Option<crate::storage::secrets::CacheConfig>,
}

impl StorageConfig {
    /// `[identity]` and `[locations]`. An incomplete identity is an error.
    pub fn from_settings(settings: &Settings) -> std::result::Result<Self, String> {
        let identity = &settings.identity;
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
        let aws = &identity.aws;
        if aws.access_key.is_some() != aws.secret_key.is_some() {
            return Err("[identity.aws] needs access_key and secret_key, or neither".into());
        }
        let s3_store =
            endpoint(&aws.endpoint, "identity.aws.endpoint")?.map(|endpoint| aws::S3Store {
                endpoint,
                region: aws.region.clone().unwrap_or_else(|| "us-east-1".into()),
            });
        let locations = settings
            .locations
            .iter()
            .map(|(name, entry)| Location::parse(name, entry, aws.arn.as_deref()))
            .collect::<std::result::Result<_, _>>()?;
        Ok(Self {
            aws: aws::AwsIdentity {
                access_key: aws.access_key.clone(),
                secret_key: aws.secret_key.clone(),
                region: aws.region.clone(),
            },
            s3_store,
            azure: azure_identity(&identity.azure)?,
            gcp_key_file: identity.gcp.json_key_file.clone(),
            gcp_endpoint: endpoint(&identity.gcp.endpoint, "identity.gcp.endpoint")?,
            locations,
            sign_in_cache: None,
        })
    }
}

/// `[identity.azure]`: a service principal, a user-assigned managed
/// identity, or none.
fn azure_identity(
    azure: &settings::AzureIdentity,
) -> std::result::Result<Option<azure::Identity>, String> {
    let principal = (
        &azure.directory_id,
        &azure.application_id,
        &azure.client_secret,
    );
    match (principal, &azure.managed_identity_id) {
        ((None, None, None), None) => Ok(None),
        ((None, None, None), Some(id)) => Ok(Some(azure::Identity::Managed(Some(id.clone())))),
        ((Some(tenant_id), Some(client_id), Some(client_secret)), None) => {
            Ok(Some(azure::Identity::ServicePrincipal {
                tenant_id: tenant_id.clone(),
                client_id: client_id.clone(),
                client_secret: client_secret.as_str().into(),
            }))
        }
        (_, Some(_)) => Err(
            "[identity.azure] holds a service principal or managed_identity_id, not both".into(),
        ),
        _ => Err("[identity.azure] needs directory_id, application_id and client_secret".into()),
    }
}

/// Upstream `CloudCredentialVendor`: the vendor of the path's scheme.
pub struct CloudVendor {
    aws: aws::AwsVendor,
    azure: azure::AzureVendor,
    gcp: gcp::GcpVendor,
    /// `[locations.<name>]`.
    locations: Vec<Location>,
}

impl CloudVendor {
    pub fn new(config: StorageConfig) -> Self {
        let gcp = gcp::GcpVendor::new(config.gcp_key_file);
        Self {
            aws: aws::AwsVendor::new(config.aws, config.s3_store),
            azure: azure::AzureVendor::new(config.azure, config.sign_in_cache),
            gcp: match config.gcp_endpoint {
                Some(url) => gcp.with_endpoints(Some(format!("{url}/v1/token")), None, Some(url)),
                None => gcp,
            },
            locations: config.locations,
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

    /// The longest location of the settings that covers `path`.
    fn covering(&self, path: &NormalizedUrl) -> Option<&Location> {
        self.locations
            .iter()
            .filter(|location| path.is_same_or_under(location.url()))
            .max_by_key(|location| location.url().as_str().len())
    }

    /// The longest location of the settings that covers the path of `ctx`.
    fn location(&self, ctx: &CredentialContext) -> Result<&Location> {
        let path = ctx
            .locations
            .first()
            .ok_or_else(|| Error::internal("credentials for no path"))?;
        self.covering(path).ok_or_else(|| {
            Error::failed_precondition(format!(
                "No external location and no [locations.<name>] of the settings covers \
                     {path}."
            ))
        })
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
        let credential = ctx.credential.as_ref();
        let other_cloud = || Error::internal("a location of the settings of another cloud");
        match ctx.scheme {
            UriScheme::S3 => {
                let role = match credential {
                    Some(credential) => aws::Role::of(credential)?,
                    None => match self.location(ctx)? {
                        Location::S3(location) => location.role.clone(),
                        _ => return Err(other_cloud()),
                    },
                };
                self.aws.vend(ctx, &role).await
            }
            UriScheme::Abfs | UriScheme::Abfss => {
                let identity = match credential {
                    Some(credential) => azure::Identity::of(credential)?,
                    None => match self.location(ctx)? {
                        Location::Adls(location) => location.identity.clone(),
                        _ => return Err(other_cloud()),
                    },
                };
                self.azure.vend(ctx, &identity).await
            }
            UriScheme::Gs => {
                let identity = match credential {
                    Some(credential) => gcp::Identity::of(credential)?,
                    None => match self.location(ctx)? {
                        Location::Gcs(location) => location.identity.clone(),
                        _ => return Err(other_cloud()),
                    },
                };
                self.gcp.vend(ctx, &identity).await
            }
            UriScheme::File => Ok(TemporaryCredentials::default()),
        }
    }

    fn s3_store(&self) -> Option<&aws::S3Store> {
        self.aws.store()
    }

    fn covers(&self, path: &NormalizedUrl) -> bool {
        self.covering(path).is_some()
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

    fn config(text: &str) -> std::result::Result<StorageConfig, String> {
        StorageConfig::from_settings(&Settings::from_toml(text)?)
    }

    #[test]
    fn the_server_identity_is_complete_or_absent() {
        assert_eq!(config("").unwrap().azure, None);
        assert_eq!(
            config("[identity.azure]\nmanaged_identity_id = '/subscriptions/s/uc'")
                .unwrap()
                .azure,
            Some(azure::Identity::Managed(Some("/subscriptions/s/uc".into())))
        );
        let principal = "[identity.azure]\ndirectory_id = 't'\napplication_id = 'c'\n\
                         client_secret = 's'";
        assert_eq!(
            config(principal).unwrap().azure,
            Some(azure::Identity::ServicePrincipal {
                tenant_id: "t".into(),
                client_id: "c".into(),
                client_secret: "s".into(),
            })
        );
        let error = |text: &str| config(text).unwrap_err();
        assert_eq!(
            error("[identity.azure]\ndirectory_id = 't'"),
            "[identity.azure] needs directory_id, application_id and client_secret"
        );
        assert!(error(&format!("{principal}\nmanaged_identity_id = 'x'")).contains("not both"),);
        assert_eq!(
            error("[identity.aws]\naccess_key = 'k'"),
            "[identity.aws] needs access_key and secret_key, or neither"
        );
    }
}
