//! The `[locations.<name>]` of the settings: a URL and the identity that
//! reaches the paths under it, for the paths that no external location
//! covers. The URL's scheme picks the cloud, and the entry holds exactly one
//! identity of that cloud.

use uc_common::settings;
use uc_common::{NormalizedUrl, UriScheme};

use super::{aws, azure, gcp};

/// A `[locations.<name>]` after parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    S3(S3Location),
    Adls(AdlsLocation),
    Gcs(GcsLocation),
}

/// An `s3://` location: the role that the server's AWS identity assumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Location {
    pub url: NormalizedUrl,
    /// The bucket's region, which Iceberg clients get.
    pub region: Option<String>,
    pub role: aws::Role,
}

/// An `abfs://` or `abfss://` location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdlsLocation {
    pub url: NormalizedUrl,
    pub identity: azure::Identity,
}

/// A `gs://` location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcsLocation {
    pub url: NormalizedUrl,
    pub identity: gcp::Identity,
}

/// The identity fields of each cloud, for messages.
const AWS: &[&str] = &["aws_iam_role_arn"];
const AZURE: &[&str] = &[
    "azure_directory_id",
    "azure_application_id",
    "azure_client_secret",
    "azure_managed_identity",
];
const GCP: &[&str] = &["gcp_service_account", "gcp_service_account_key"];

impl Location {
    pub fn url(&self) -> &NormalizedUrl {
        match self {
            Self::S3(location) => &location.url,
            Self::Adls(location) => &location.url,
            Self::Gcs(location) => &location.url,
        }
    }

    /// The entry `name`: its URL, and the identity of the URL's cloud. A
    /// field of another cloud, a missing identity or two identities are
    /// errors. A service account key is read from its file.
    pub fn parse(name: &str, entry: &settings::Location) -> Result<Self, String> {
        let key = |field: &str| format!("locations.{name}.{field}");
        let text = entry
            .url
            .as_deref()
            .ok_or_else(|| format!("{} is required", settings::name(&key("url"))))?;
        let url = NormalizedUrl::parse(text)
            .map_err(|e| settings::invalid(&key("url"), text, e.message()))?;
        let scheme = url.scheme();
        let given = [
            ("region", entry.region.is_some()),
            ("aws_iam_role_arn", entry.aws_iam_role_arn.is_some()),
            ("azure_directory_id", entry.azure_directory_id.is_some()),
            ("azure_application_id", entry.azure_application_id.is_some()),
            ("azure_client_secret", entry.azure_client_secret.is_some()),
            (
                "azure_managed_identity",
                entry.azure_managed_identity.is_some(),
            ),
            ("gcp_service_account", entry.gcp_service_account.is_some()),
            (
                "gcp_service_account_key",
                entry.gcp_service_account_key.is_some(),
            ),
        ];
        let allowed: &[&str] = match scheme {
            UriScheme::S3 => &["region", AWS[0]],
            UriScheme::Abfs | UriScheme::Abfss => AZURE,
            UriScheme::Gs => GCP,
            UriScheme::File => {
                return Err(format!(
                    "[locations.{name}]: a local path needs no identity"
                ));
            }
        };
        if let Some((field, _)) = given
            .iter()
            .find(|(field, set)| *set && !allowed.contains(field))
        {
            return Err(format!(
                "{} is not for {} URLs",
                settings::name(&key(field)),
                scheme.as_str()
            ));
        }
        match scheme {
            UriScheme::S3 => {
                let arn = entry.aws_iam_role_arn.clone().ok_or_else(|| {
                    format!(
                        "[locations.{name}] needs aws_iam_role_arn for {}",
                        url.as_str()
                    )
                })?;
                Ok(Self::S3(S3Location {
                    url,
                    region: entry.region.clone(),
                    role: aws::Role {
                        arn,
                        external_id: None,
                    },
                }))
            }
            UriScheme::Abfs | UriScheme::Abfss => {
                let identity = azure_identity(name, entry)?;
                Ok(Self::Adls(AdlsLocation { url, identity }))
            }
            _ => {
                let identity = gcp_identity(name, entry)?;
                Ok(Self::Gcs(GcsLocation { url, identity }))
            }
        }
    }
}

/// A service principal, or a managed identity.
fn azure_identity(name: &str, entry: &settings::Location) -> Result<azure::Identity, String> {
    let principal = (
        &entry.azure_directory_id,
        &entry.azure_application_id,
        &entry.azure_client_secret,
    );
    match (principal, &entry.azure_managed_identity) {
        ((Some(tenant_id), Some(client_id), Some(client_secret)), None) => {
            Ok(azure::Identity::ServicePrincipal {
                tenant_id: tenant_id.clone(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
            })
        }
        ((None, None, None), Some(id)) => Ok(azure::Identity::Managed(id.clone())),
        ((None, None, None), None) => Err(format!(
            "[locations.{name}] needs azure_directory_id, azure_application_id and \
             azure_client_secret, or azure_managed_identity"
        )),
        (_, Some(_)) => Err(format!(
            "[locations.{name}] holds a service principal or azure_managed_identity, not both"
        )),
        _ => Err(format!(
            "[locations.{name}] needs azure_directory_id, azure_application_id and \
             azure_client_secret"
        )),
    }
}

/// A service account, or a service account key from its file.
fn gcp_identity(name: &str, entry: &settings::Location) -> Result<gcp::Identity, String> {
    match (&entry.gcp_service_account, &entry.gcp_service_account_key) {
        (Some(email), None) => Ok(gcp::Identity::ServiceAccount(email.clone())),
        (None, Some(file)) => {
            let failed = |e: String| {
                format!(
                    "{}: {}: {e}",
                    settings::name(&format!("locations.{name}.gcp_service_account_key")),
                    file.display()
                )
            };
            let json = std::fs::read_to_string(file).map_err(|e| failed(e.to_string()))?;
            serde_json::from_str::<serde_json::Value>(&json).map_err(|e| failed(e.to_string()))?;
            Ok(gcp::Identity::Key(json))
        }
        (None, None) => Err(format!(
            "[locations.{name}] needs gcp_service_account or gcp_service_account_key"
        )),
        (Some(_), Some(_)) => Err(format!(
            "[locations.{name}] holds gcp_service_account or gcp_service_account_key, not both"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The locations of the TOML text `text`, by name.
    fn parse(text: &str) -> Result<Vec<(String, Location)>, String> {
        settings::Settings::from_toml(text)?
            .locations
            .iter()
            .map(|(name, entry)| Location::parse(name, entry).map(|l| (name.clone(), l)))
            .collect()
    }

    fn url(text: &str) -> NormalizedUrl {
        NormalizedUrl::parse(text).unwrap()
    }

    #[test]
    fn the_url_picks_the_cloud() {
        let found = parse(
            r#"
            [locations.analytics]
            url = "s3://analytics"
            region = "eu-west-1"
            aws_iam_role_arn = "arn:aws:iam::1:role/uc"

            [locations.lake]
            url = "abfss://data@lake.dfs.core.windows.net"
            azure_directory_id = "t"
            azure_application_id = "c"
            azure_client_secret = "s"

            [locations.own]
            url = "abfss://data@own.dfs.core.windows.net/tables"
            azure_managed_identity = true

            [locations.models]
            url = "gs://models"
            gcp_service_account = "uc@p.iam.gserviceaccount.com"
            "#,
        )
        .unwrap();
        assert_eq!(
            found,
            [
                (
                    "analytics".into(),
                    Location::S3(S3Location {
                        url: url("s3://analytics"),
                        region: Some("eu-west-1".into()),
                        role: aws::Role {
                            arn: "arn:aws:iam::1:role/uc".into(),
                            external_id: None,
                        },
                    })
                ),
                (
                    "lake".into(),
                    Location::Adls(AdlsLocation {
                        url: url("abfss://data@lake.dfs.core.windows.net"),
                        identity: azure::Identity::ServicePrincipal {
                            tenant_id: "t".into(),
                            client_id: "c".into(),
                            client_secret: "s".into(),
                        },
                    })
                ),
                (
                    "models".into(),
                    Location::Gcs(GcsLocation {
                        url: url("gs://models"),
                        identity: gcp::Identity::ServiceAccount(Some(
                            "uc@p.iam.gserviceaccount.com".into()
                        )),
                    })
                ),
                (
                    "own".into(),
                    Location::Adls(AdlsLocation {
                        url: url("abfss://data@own.dfs.core.windows.net/tables"),
                        identity: azure::Identity::Managed(None),
                    })
                ),
            ]
        );
    }

    #[test]
    fn an_entry_holds_one_identity_of_its_cloud() {
        let error = |text: &str| parse(text).unwrap_err();
        assert_eq!(
            error("[locations.x]\nurl = 's3://b'\nazure_managed_identity = true"),
            "locations.x.azure_managed_identity (UC_LOCATIONS_X_AZURE_MANAGED_IDENTITY) is not \
             for s3 URLs"
        );
        assert_eq!(
            error("[locations.x]\nurl = 'gs://b'\nregion = 'us-east-1'"),
            "locations.x.region (UC_LOCATIONS_X_REGION) is not for gs URLs"
        );
        assert_eq!(
            error("[locations.x]\nurl = 's3://b'"),
            "[locations.x] needs aws_iam_role_arn for s3://b"
        );
        assert!(
            error(
                "[locations.x]\nurl = 'abfss://c@a.dfs.core.windows.net'\nazure_directory_id = 't'"
            )
            .contains("needs azure_directory_id, azure_application_id and azure_client_secret")
        );
        assert!(
            error(
                "[locations.x]\nurl = 'gs://b'\ngcp_service_account = true\n\
                 gcp_service_account_key = '/k.json'"
            )
            .contains("not both")
        );
        assert!(
            error("[locations.x]\nurl = 'gs://b'\ngcp_service_account_key = '/no/key.json'")
                .starts_with("locations.x.gcp_service_account_key")
        );
        assert!(error("[locations.x]\nregion = 'us-east-1'").contains("locations.x.url"));
    }
}
