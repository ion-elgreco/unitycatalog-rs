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
//! Without configured keys, `object_store` finds the identity in the
//! environment: the `AWS_*` variables, web identity (a Kubernetes service
//! account), the container endpoint (ECS, EKS Pod Identity) or the
//! instance role. `object_store` also signs the `AssumeRole` call.
//!
//! With `aws.endpoint`, an S3-compatible store ([`S3Store`]) takes AWS's
//! place: STS calls go to it, and the policies have no KMS statement.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use object_store::{
    StaticCredentialProvider,
    aws::{
        AmazonS3Builder, AmazonS3ConfigKey, AwsAuthorizer, AwsCredential, AwsCredentialProvider,
    },
    client::{HttpRequest, SpawnedReqwestConnector},
};
use serde_json::{Value, json};

use uc_api::{AwsCredentials, TemporaryCredentials};
use uc_common::{Error, NormalizedUrl, Result};
use uc_storage::runtime::io_handle;

use super::{CredentialContext, StoragePrivilege, xml_text};
use crate::http::outbound::SendOnIo;

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
    http: reqwest::Client,
    /// Identities and the regions of their STS calls, by configuration
    /// (`master` or the bucket), made on first use.
    identities: Mutex<HashMap<String, (AwsCredentialProvider, String)>>,
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
            http: reqwest::Client::new(),
            identities: Mutex::new(HashMap::new()),
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
        let (identity, region) = self.identity(key, config)?;
        let session_policy = policy(
            &ctx.privileges,
            &ctx.locations,
            Some(&role_arn),
            Some(&region),
            self.store.is_none(),
        )?;
        self.assume_role(&identity, &region, &role_arn, external_id, &session_policy)
            .await
            .map_err(|e| Error::internal(format!("AWS AssumeRole of {role_arn} failed: {e}")))
    }

    /// STS `AssumeRole` for an hour, signed by `object_store`.
    async fn assume_role(
        &self,
        identity: &AwsCredentialProvider,
        region: &str,
        role_arn: &str,
        external_id: Option<String>,
        session_policy: &str,
    ) -> std::result::Result<TemporaryCredentials, String> {
        let identity = identity.get_credential().await.map_err(|e| e.to_string())?;
        let endpoint = match &self.endpoint {
            Some(endpoint) => format!("{}/", endpoint.trim_end_matches('/')),
            None => {
                let suffix = dns_suffix(&partition(None, Some(region)));
                format!("https://sts.{region}.{suffix}/")
            }
        };
        let session_name = format!("uc-{}", uuid::Uuid::new_v4());
        let mut form = vec![
            ("Action", "AssumeRole"),
            ("Version", "2011-06-15"),
            ("RoleArn", role_arn),
            ("RoleSessionName", session_name.as_str()),
            ("DurationSeconds", "3600"),
            ("Policy", session_policy),
        ];
        if let Some(external_id) = &external_id {
            form.push(("ExternalId", external_id));
        }
        let body = serde_urlencoded::to_string(&form).map_err(|e| e.to_string())?;
        let mut request: HttpRequest = http::Request::post(&endpoint)
            .header(
                http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body.clone().into())
            .map_err(|e| e.to_string())?;
        AwsAuthorizer::new(&identity, "sts", region).authorize(&mut request, None);
        let response = self
            .http
            .post(&endpoint)
            .headers(request.headers().clone())
            .body(body)
            .send_on_io()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let xml = response.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            let message = xml_text(&xml, "Message").unwrap_or(&xml);
            return Err(format!("{status}: {message}"));
        }
        let field = |tag: &str| {
            xml_text(&xml, tag)
                .map(str::to_string)
                .ok_or_else(|| format!("no {tag} in the answer"))
        };
        let expiration = chrono::DateTime::parse_from_rfc3339(&field("Expiration")?)
            .map(|t| t.timestamp_millis())
            .ok();
        Ok(TemporaryCredentials {
            aws_temp_credentials: Some(AwsCredentials {
                access_key_id: Some(field("AccessKeyId")?),
                secret_access_key: Some(field("SecretAccessKey")?),
                session_token: Some(field("SessionToken")?),
            }),
            expiration_time: expiration,
            ..Default::default()
        })
    }

    /// Upstream `StsAwsCredentialGenerator`: the configured region, else the
    /// store's, else `AWS_REGION` or `AWS_DEFAULT_REGION`, else `us-east-1`;
    /// the configured keys, else `object_store`'s identity from the
    /// environment.
    fn identity(&self, key: String, config: &S3Config) -> Result<(AwsCredentialProvider, String)> {
        let mut identities = self.identities.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(found) = identities.get(&key) {
            return Ok(found.clone());
        }
        let env = AmazonS3Builder::from_env();
        let region = config
            .region
            .clone()
            .or_else(|| self.store.as_ref().map(|s| s.region.clone()))
            .or_else(|| env.get_config_value(&AmazonS3ConfigKey::Region))
            .unwrap_or_else(|| "us-east-1".to_string());
        let identity: AwsCredentialProvider = match (&config.access_key, &config.secret_key) {
            (Some(key_id), Some(secret_key)) => {
                Arc::new(StaticCredentialProvider::new(AwsCredential {
                    key_id: key_id.clone(),
                    secret_key: secret_key.clone(),
                    token: None,
                }))
            }
            // A store only for its credentials: it sends no requests.
            _ => env
                .with_bucket_name("uc-credentials")
                .with_region(&region)
                .with_http_connector(SpawnedReqwestConnector::new(io_handle()))
                .build()
                .map_err(|e| Error::internal(format!("The server's AWS identity: {e}")))?
                .credentials()
                .clone(),
        };
        identities.insert(key, (identity.clone(), region.clone()));
        Ok((identity, region))
    }
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
