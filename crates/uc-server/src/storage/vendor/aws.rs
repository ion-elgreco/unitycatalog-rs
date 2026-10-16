//! AWS credentials, after upstream `AwsCredentialVendor`.
//!
//! - Under an external location, the server's master identity (`aws.*`)
//!   assumes the credential's `role_arn`, with its `external_id`.
//! - Otherwise the bucket's `s3.*.N` entry decides: its session token as
//!   is (testing only), or its keys assume its `awsRoleArn`.
//!
//! The session lasts an hour and its policy, from [`policy`], allows only
//! the requested locations and operations.
//!
//! With `aws.endpoint`, an S3-compatible store ([`S3Store`]) takes AWS's
//! place: STS calls go to it, and the policies have no KMS statement.

use std::collections::HashMap;

use aws_config::{BehaviorVersion, Region};
use aws_sdk_sts::Client as StsClient;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use uc_api::{AwsCredentials, TemporaryCredentials};
use uc_common::{Error, NormalizedUrl, Result};

use super::{CredentialContext, StoragePrivilege};

/// Upstream `S3StorageConfig`.
#[derive(Debug, Clone, Default)]
pub struct S3Config {
    pub region: Option<String>,
    pub aws_role_arn: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub session_token: Option<String>,
}

/// An S3-compatible store, such as MinIO or RustFS, in place of AWS
/// (`aws.endpoint`): its S3 and STS API, and the region that requests are
/// signed for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Store {
    pub endpoint: String,
    /// `aws.region`, else `us-east-1`.
    pub region: String,
}

pub struct AwsVendor {
    master: S3Config,
    buckets: HashMap<NormalizedUrl, S3Config>,
    store: Option<S3Store>,
    /// STS, when not AWS's: the store's, or a test's.
    endpoint: Option<String>,
    /// STS clients and their regions, by configuration (`master` or the
    /// bucket), made on first use.
    clients: Mutex<HashMap<String, (StsClient, String)>>,
}

impl AwsVendor {
    pub fn new(
        master: S3Config,
        buckets: HashMap<NormalizedUrl, S3Config>,
        store: Option<S3Store>,
    ) -> Self {
        Self {
            master,
            buckets,
            endpoint: store.as_ref().map(|s| s.endpoint.clone()),
            store,
            clients: Mutex::new(HashMap::new()),
        }
    }

    /// STS at `endpoint`, for tests.
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// The S3-compatible store in place of AWS, if any.
    pub fn store(&self) -> Option<&S3Store> {
        self.store.as_ref()
    }

    pub async fn vend(&self, ctx: &CredentialContext) -> Result<TemporaryCredentials> {
        let (key, config, role_arn, external_id) = match &ctx.credential {
            Some(credential) => {
                let role = credential.aws_iam_role.as_ref();
                let role_arn = role.and_then(|r| r.role_arn.clone()).ok_or_else(|| {
                    Error::failed_precondition("The storage credential has no AWS IAM role.")
                })?;
                (
                    "master".to_string(),
                    &self.master,
                    role_arn,
                    role.and_then(|r| r.external_id.clone()),
                )
            }
            None => {
                let config = self.buckets.get(&ctx.storage_base).ok_or_else(|| {
                    Error::failed_precondition("S3 bucket configuration not found.")
                })?;
                // Upstream `StaticAwsCredentialGenerator`: a preset session.
                if config.session_token.is_some() {
                    return Ok(TemporaryCredentials {
                        aws_temp_credentials: Some(AwsCredentials {
                            access_key_id: config.access_key.clone(),
                            secret_access_key: config.secret_key.clone(),
                            session_token: config.session_token.clone(),
                        }),
                        ..Default::default()
                    });
                }
                let role_arn = config.aws_role_arn.clone().ok_or_else(|| {
                    Error::failed_precondition("S3 bucket configuration has no role.")
                })?;
                (ctx.storage_base.to_string(), config, role_arn, None)
            }
        };
        let (client, region) = self.client(key, config).await;
        let session_policy = policy(
            &ctx.privileges,
            &ctx.locations,
            Some(&role_arn),
            Some(&region),
            self.store.is_none(),
        )?;
        let assumed = client
            .assume_role()
            .role_arn(&role_arn)
            .policy(session_policy)
            .role_session_name(format!("uc-{}", uuid::Uuid::new_v4()))
            .duration_seconds(3600)
            .set_external_id(external_id)
            .send()
            .await
            .map_err(|e| {
                Error::internal(format!(
                    "AWS AssumeRole of {role_arn} failed: {}",
                    error_chain(&e)
                ))
            })?;
        let credentials = assumed
            .credentials()
            .ok_or_else(|| Error::internal("AWS AssumeRole returned no credentials"))?;
        Ok(TemporaryCredentials {
            aws_temp_credentials: Some(AwsCredentials {
                access_key_id: Some(credentials.access_key_id().to_string()),
                secret_access_key: Some(credentials.secret_access_key().to_string()),
                session_token: Some(credentials.session_token().to_string()),
            }),
            expiration_time: credentials.expiration().to_millis().ok(),
            ..Default::default()
        })
    }

    /// Upstream `StsAwsCredentialGenerator`: the configured region, else the
    /// store's, else the default region chain, else `us-east-1`; the
    /// configured keys, else the default credential chain.
    async fn client(&self, key: String, config: &S3Config) -> (StsClient, String) {
        let mut clients = self.clients.lock().await;
        if let Some((client, region)) = clients.get(&key) {
            return (client.clone(), region.clone());
        }
        let configured = config
            .region
            .clone()
            .or_else(|| self.store.as_ref().map(|s| s.region.clone()));
        let region = match configured {
            Some(region) => region,
            None => aws_config::default_provider::region::DefaultRegionChain::builder()
                .build()
                .region()
                .await
                .map(|r| r.to_string())
                .unwrap_or_else(|| "us-east-1".to_string()),
        };
        let mut loader =
            aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region.clone()));
        if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
            let static_keys = aws_sdk_sts::config::Credentials::new(
                access_key, secret_key, None, None, "uc.toml",
            );
            loader = loader.credentials_provider(static_keys);
        }
        if let Some(endpoint) = &self.endpoint {
            loader = loader.endpoint_url(endpoint);
        }
        let client = StsClient::new(&loader.load().await);
        clients.insert(key, (client.clone(), region.clone()));
        (client, region)
    }
}

/// The error and its sources, one line.
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(s) = source {
        message.push_str(": ");
        message.push_str(&s.to_string());
        source = s.source();
    }
    message
}

/// The IAM partition of a role ARN, else of a region, else `aws`.
fn partition(role_arn: Option<&str>, region: Option<&str>) -> String {
    let from_arn = role_arn
        .and_then(|arn| arn.strip_prefix("arn:"))
        .and_then(|rest| rest.split(':').next())
        .filter(|p| !p.is_empty());
    if let Some(partition) = from_arn {
        return partition.to_string();
    }
    let region = region.unwrap_or_default();
    let partition = if region.starts_with("cn-") {
        "aws-cn"
    } else if region.starts_with("us-gov-") {
        "aws-us-gov"
    } else if region.starts_with("us-isob-") {
        "aws-iso-b"
    } else if region.starts_with("us-isof-") {
        "aws-iso-f"
    } else if region.starts_with("us-iso-") {
        "aws-iso"
    } else if region.starts_with("eu-isoe-") {
        "aws-iso-e"
    } else {
        "aws"
    };
    partition.to_string()
}

/// The DNS suffix of a partition, for `kms:ViaService`.
fn dns_suffix(partition: &str) -> &'static str {
    match partition {
        "aws-cn" => "amazonaws.com.cn",
        "aws-iso" => "c2s.ic.gov",
        "aws-iso-b" => "sc2s.sgov.gov",
        "aws-iso-e" => "cloud.adc-e.uk",
        "aws-iso-f" => "csp.hci.ic.gov",
        _ => "amazonaws.com",
    }
}

/// S3 treats `*`, `?` and `$` as plain characters; IAM does not. `$`
/// first, as the other escapes contain one.
fn escape_iam(path: &str) -> String {
    path.replace('$', "${$}")
        .replace('*', "${*}")
        .replace('?', "${?}")
}

/// Upstream `AwsPolicyGenerator.generatePolicy`: the object operations on
/// each location, listing under it, and, with `kms`, KMS through S3. An
/// S3-compatible store has no AWS KMS, and RustFS refuses the statement.
pub fn policy(
    privileges: &[StoragePrivilege],
    locations: &[NormalizedUrl],
    role_arn: Option<&str>,
    region: Option<&str>,
    kms: bool,
) -> Result<String> {
    let partition = partition(role_arn, region);
    let (actions, kms_actions): (&[&str], &[&str]) = if privileges
        .contains(&StoragePrivilege::Update)
    {
        (
            &[
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject",
                "s3:AbortMultipartUpload",
                "s3:ListMultipartUploadParts",
            ],
            &["kms:Decrypt", "kms:GenerateDataKey*"],
        )
    } else if privileges.contains(&StoragePrivilege::Select) {
        (&["s3:GetObject"], &["kms:Decrypt"])
    } else {
        return Err(Error::permission_denied(format!(
            "Can't generate policy for unknown privileges '{privileges:?}' for locations: '{locations:?}'"
        )));
    };
    let arn = |resource: String| format!("arn:{partition}:s3:::{resource}");
    let mut buckets: Vec<(String, Vec<String>)> = Vec::new();
    for location in locations {
        let bucket = location
            .storage_base()
            .as_str()
            .split_once("://")
            .map(|(_, b)| b.to_string())
            .unwrap_or_default();
        let path = location.path();
        match buckets.iter_mut().find(|(b, _)| *b == bucket) {
            Some((_, paths)) => paths.push(path),
            None => buckets.push((bucket, vec![path])),
        }
    }
    let mut operation_resources = Vec::new();
    let mut statements = Vec::new();
    for (bucket, paths) in &buckets {
        let mut prefixes = Vec::new();
        for path in paths {
            let path = escape_iam(path.trim_start_matches('/'));
            if path.is_empty() {
                prefixes.push("*".to_string());
                operation_resources.push(arn(format!("{bucket}/*")));
            } else {
                prefixes.extend([path.clone(), format!("{path}/"), format!("{path}/*")]);
                operation_resources.push(arn(format!("{bucket}/{path}/*")));
                operation_resources.push(arn(format!("{bucket}/{path}")));
            }
        }
        statements.push(json!({
            "Effect": "Allow",
            "Action": ["s3:ListBucket"],
            "Resource": [arn(bucket.clone())],
            "Condition": { "StringLike": { "s3:prefix": prefixes } },
        }));
    }
    let operations =
        json!({ "Effect": "Allow", "Action": actions, "Resource": operation_resources });
    let mut all: Vec<Value> = vec![operations];
    all.extend(statements);
    if kms {
        all.push(json!({
            "Effect": "Allow",
            "Action": kms_actions,
            "Resource": ["*"],
            "Condition": { "StringLike": { "kms:ViaService": format!("s3.*.{}", dns_suffix(&partition)) } },
        }));
    }
    Ok(json!({ "Version": "2012-10-17", "Statement": all }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use StoragePrivilege::{Select, Update};

    fn parsed(privileges: &[StoragePrivilege], urls: &[&str]) -> Value {
        let locations: Vec<NormalizedUrl> = urls
            .iter()
            .map(|u| NormalizedUrl::parse(u).unwrap())
            .collect();
        serde_json::from_str(&policy(privileges, &locations, None, None, true).unwrap()).unwrap()
    }

    fn strings(v: &Value) -> Vec<&str> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect()
    }

    /// Upstream `AwsPolicyGeneratorTest`.
    #[test]
    fn scopes_operations_and_listing_to_the_path() {
        let p = parsed(&[Select], &["s3://my-bucket/path1/table1"]);
        assert_eq!(p["Version"], "2012-10-17");
        assert_eq!(strings(&p["Statement"][0]["Action"]), ["s3:GetObject"]);
        assert_eq!(
            strings(&p["Statement"][0]["Resource"]),
            [
                "arn:aws:s3:::my-bucket/path1/table1/*",
                "arn:aws:s3:::my-bucket/path1/table1"
            ]
        );
        assert_eq!(
            strings(&p["Statement"][1]["Resource"]),
            ["arn:aws:s3:::my-bucket"]
        );
        assert_eq!(
            strings(&p["Statement"][1]["Condition"]["StringLike"]["s3:prefix"]),
            ["path1/table1", "path1/table1/", "path1/table1/*"]
        );
        assert_eq!(strings(&p["Statement"][2]["Action"]), ["kms:Decrypt"]);
        assert_eq!(strings(&p["Statement"][2]["Resource"]), ["*"]);
        assert_eq!(
            p["Statement"][2]["Condition"]["StringLike"]["kms:ViaService"],
            "s3.*.amazonaws.com"
        );
    }

    #[test]
    fn writes_add_object_and_kms_actions() {
        let p = parsed(
            &[Select, Update],
            &["s3://my-bucket1/path1/table1", "s3://profile-bucket2/x"],
        );
        assert_eq!(
            strings(&p["Statement"][0]["Action"]),
            [
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject",
                "s3:AbortMultipartUpload",
                "s3:ListMultipartUploadParts"
            ]
        );
        assert_eq!(
            strings(&p["Statement"][3]["Action"]),
            ["kms:Decrypt", "kms:GenerateDataKey*"]
        );
        assert_eq!(
            strings(&p["Statement"][2]["Resource"]),
            ["arn:aws:s3:::profile-bucket2"]
        );
        assert!(!p.to_string().contains("kms:EncryptionContext"));
    }

    #[test]
    fn escapes_iam_special_characters() {
        for (encoded, expected) in [
            ("%2A", "${*}"),
            ("%3F", "${?}"),
            ("prefix*middle", "prefix${*}middle"),
            ("%24%7Baws:username%7D", "${$}{aws:username}"),
            ("$%7B*%7D", "${$}{${*}}"),
        ] {
            let p = parsed(&[Update], &[&format!("s3://victim-bucket/{encoded}")]);
            assert_eq!(
                strings(&p["Statement"][0]["Resource"]),
                [
                    format!("arn:aws:s3:::victim-bucket/{expected}/*"),
                    format!("arn:aws:s3:::victim-bucket/{expected}")
                ],
                "{encoded}"
            );
        }
    }

    #[test]
    fn bucket_root_is_the_whole_bucket() {
        let p = parsed(&[Select], &["s3://my-bucket"]);
        assert_eq!(
            strings(&p["Statement"][0]["Resource"]),
            ["arn:aws:s3:::my-bucket/*"]
        );
        assert_eq!(
            strings(&p["Statement"][1]["Condition"]["StringLike"]["s3:prefix"]),
            ["*"]
        );
    }

    #[test]
    fn partition_follows_the_role_then_the_region() {
        let location = [NormalizedUrl::parse("s3://b/p").unwrap()];
        let gov = policy(
            &[Select],
            &location,
            Some("arn:aws-us-gov:iam::1:role/r"),
            Some("us-east-1"),
            true,
        )
        .unwrap();
        assert!(gov.contains("arn:aws-us-gov:s3:::b/p") && gov.contains("s3.*.amazonaws.com"));
        let china = policy(&[Select], &location, None, Some("cn-north-1"), true).unwrap();
        assert!(china.contains("arn:aws-cn:s3:::b/p") && china.contains("s3.*.amazonaws.com.cn"));
        let invalid = policy(&[Select], &location, Some("not-an-arn"), None, true).unwrap();
        assert!(invalid.contains("arn:aws:s3:::b/p"));
    }

    #[test]
    fn needs_a_known_privilege() {
        let location = [NormalizedUrl::parse("s3://b/p").unwrap()];
        assert!(policy(&[], &location, None, None, true).is_err());
    }

    /// An S3-compatible store: the same S3 statements, without KMS.
    #[test]
    fn a_store_gets_no_kms_statement() {
        let location = [NormalizedUrl::parse("s3://b/p").unwrap()];
        let p: Value =
            serde_json::from_str(&policy(&[Select, Update], &location, None, None, false).unwrap())
                .unwrap();
        let statements = p["Statement"].as_array().unwrap();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0]["Action"][1], "s3:PutObject");
        assert_eq!(statements[1]["Action"], json!(["s3:ListBucket"]));
        assert!(!p.to_string().contains("kms:"));
    }
}
