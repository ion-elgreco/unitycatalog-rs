//! GCP credentials, after upstream `GcpCredentialVendor`.
//!
//! The token is the one of the storage credential's [`Identity`], of the
//! external location, else of the `[locations.<name>]`: of its service
//! account key, or of the server's own identity, which impersonates the
//! credential's service account when it has one. The server's own identity
//! is the key file of `[identity.gcp]`, else Application Default
//! Credentials: `GOOGLE_APPLICATION_CREDENTIALS`, gcloud's file or the
//! metadata server. Google STS exchanges the token for one downscoped to the
//! locations: `storage.objectViewer` for reads, and
//! `storage.legacyBucketWriter` too for writes.
//!
//! The server's token does not come from `object_store`: for a service
//! account key it is a self-signed JWT, not the OAuth access token that
//! STS exchanges, and `object_store` cannot impersonate.

use std::path::{Path, PathBuf};

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};

use uc_api::{GcpOauthToken, TemporaryCredentials};
use uc_common::{Error, Result, now_epoch_millis};

use super::{CredentialContext, StorageCredential};
use crate::http::outbound::SendOnIo;

const CLOUD_PLATFORM: &str = "https://www.googleapis.com/auth/cloud-platform";
const GOOGLE_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
/// Who gets the token that STS downscopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// The server's own identity, or the service account of this email that
    /// it impersonates.
    ServiceAccount(Option<String>),
    /// The JSON key of a service account.
    Key(String),
}

impl Identity {
    /// The identity of a storage credential.
    pub fn of(credential: &StorageCredential) -> Result<Self> {
        let info = &credential.info;
        if info.gcp_service_account_key.is_some() {
            return Ok(Self::Key(credential.secret.clone().unwrap_or_default()));
        }
        match &info.gcp_service_account {
            Some(account) => Ok(Self::ServiceAccount(account.email.clone())),
            None => Err(Error::failed_precondition(
                "The storage credential has no GCP identity.",
            )),
        }
    }
}

pub struct GcpVendor {
    /// The key file of the server's own identity, else Application Default
    /// Credentials.
    key_file: Option<PathBuf>,
    http: reqwest::Client,
    sts: String,
    metadata: String,
    /// Whether Application Default Credentials look for a file before the
    /// metadata server: not with a test's metadata server.
    credentials_files: bool,
    iam: String,
}

/// A token and when it expires (epoch milliseconds), if known.
struct Token {
    value: String,
    expires_at: Option<i64>,
}

impl GcpVendor {
    /// The vendor whose own identity is the key of `key_file`, else
    /// Application Default Credentials.
    pub fn new(key_file: Option<PathBuf>) -> Self {
        Self {
            key_file,
            http: reqwest::Client::new(),
            sts: "https://sts.googleapis.com/v1/token".to_string(),
            metadata: "http://metadata.google.internal".to_string(),
            credentials_files: true,
            iam: "https://iamcredentials.googleapis.com".to_string(),
        }
    }

    /// STS, metadata and IAM Credentials endpoints, for tests. A metadata
    /// server gives the server's own identity, whatever the files of
    /// Application Default Credentials.
    pub fn with_endpoints(
        mut self,
        sts: Option<String>,
        metadata: Option<String>,
        iam: Option<String>,
    ) -> Self {
        if let Some(sts) = sts {
            self.sts = sts;
        }
        if let Some(metadata) = metadata {
            self.metadata = metadata;
            self.credentials_files = false;
        }
        if let Some(iam) = iam {
            self.iam = iam;
        }
        self
    }

    /// The token of `identity`, downscoped to `ctx`.
    pub async fn vend(
        &self,
        ctx: &CredentialContext,
        identity: &Identity,
    ) -> Result<TemporaryCredentials> {
        let source = self.identity_token(identity).await?;
        let token = self.downscope(&source, ctx).await?;
        Ok(TemporaryCredentials {
            gcp_oauth_token: Some(GcpOauthToken {
                oauth_token: Some(token.value),
            }),
            expiration_time: token.expires_at,
            ..Default::default()
        })
    }

    /// The token of `identity`.
    async fn identity_token(&self, identity: &Identity) -> Result<Token> {
        let email = match identity {
            Identity::Key(json) => {
                let key: Value = serde_json::from_str(json)
                    .map_err(|e| Error::failed_precondition(format!("service account key: {e}")))?;
                return self.service_account_token(&key).await;
            }
            Identity::ServiceAccount(email) => email,
        };
        let own = self
            .source_token(self.key_file.as_deref())
            .await
            .map_err(|e| match &self.key_file {
                Some(_) => Error::failed_precondition(format!(
                    "The server's GCP identity, identity.gcp.json_key_file: {}",
                    e.message()
                )),
                None => Error::failed_precondition(format!(
                    "The server could not get its GCP identity: set [identity.gcp] \
                     (UC_IDENTITY_GCP_JSON_KEY_FILE), or run it with an identity in its \
                     environment (GOOGLE_APPLICATION_CREDENTIALS, gcloud's application default \
                     credentials, GKE Workload Identity, the service account of the VM). \
                     Cause: {}",
                    e.message()
                )),
            })?;
        match email {
            Some(email) => self.impersonate(&own, email).await,
            None => Ok(own),
        }
    }

    /// The key file, else Application Default Credentials.
    async fn source_token(&self, key_file: Option<&Path>) -> Result<Token> {
        let file = key_file.map(Path::to_path_buf).or_else(|| {
            self.credentials_files
                .then(default_credentials_file)
                .flatten()
                .map(PathBuf::from)
        });
        let Some(file) = file else {
            return self.metadata_token().await;
        };
        let text = tokio::fs::read_to_string(&file)
            .await
            .map_err(|e| Error::failed_precondition(format!("{}: {e}", file.display())))?;
        let key: Value = serde_json::from_str(&text)
            .map_err(|e| Error::failed_precondition(format!("{}: {e}", file.display())))?;
        match key["type"].as_str() {
            Some("service_account") => self.service_account_token(&key).await,
            Some("authorized_user") => {
                let form = [
                    ("grant_type", "refresh_token"),
                    ("client_id", key["client_id"].as_str().unwrap_or_default()),
                    (
                        "client_secret",
                        key["client_secret"].as_str().unwrap_or_default(),
                    ),
                    (
                        "refresh_token",
                        key["refresh_token"].as_str().unwrap_or_default(),
                    ),
                ];
                self.token_request(GOOGLE_TOKEN_URI, &form).await
            }
            other => Err(Error::failed_precondition(format!(
                "unsupported credential type {other:?} in {}",
                file.display()
            ))),
        }
    }

    /// The OAuth JWT bearer grant of a service account key.
    async fn service_account_token(&self, key: &Value) -> Result<Token> {
        let field = |name: &str| {
            key[name].as_str().ok_or_else(|| {
                Error::failed_precondition(format!("service account key without {name}"))
            })
        };
        let token_uri = key["token_uri"].as_str().unwrap_or(GOOGLE_TOKEN_URI);
        let signing_key = EncodingKey::from_rsa_pem(field("private_key")?.as_bytes())
            .map_err(|e| Error::failed_precondition(format!("service account private key: {e}")))?;
        let now = now_epoch_millis() / 1000;
        let claims = json!({
            "iss": field("client_email")?,
            "scope": CLOUD_PLATFORM,
            "aud": token_uri,
            "iat": now,
            "exp": now + 3600,
        });
        let mut header = Header::new(Algorithm::RS256);
        header.kid = key["private_key_id"].as_str().map(str::to_string);
        let assertion = jsonwebtoken::encode(&header, &claims, &signing_key)
            .map_err(|e| Error::internal(format!("service account JWT: {e}")))?;
        let form = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", assertion.as_str()),
        ];
        self.token_request(token_uri, &form).await
    }

    /// The default service account's token from the metadata server.
    async fn metadata_token(&self) -> Result<Token> {
        let url = format!(
            "{}/computeMetadata/v1/instance/service-accounts/default/token",
            self.metadata
        );
        let response = self
            .http
            .get(url)
            .query(&[("scopes", CLOUD_PLATFORM)])
            .header("Metadata-Flavor", "Google")
            .send_on_io()
            .await
            .map_err(|e| Error::failed_precondition(format!("metadata server: {e}")))?;
        token_response(response).await
    }

    /// IAM Credentials `generateAccessToken`: a token of the service account
    /// `email`, which the identity of `source` impersonates. That identity
    /// needs the role Service Account Token Creator on the account.
    async fn impersonate(&self, source: &Token, email: &str) -> Result<Token> {
        let url = format!(
            "{}/v1/projects/-/serviceAccounts/{email}:generateAccessToken",
            self.iam
        );
        let failed = |reason: String| {
            Error::failed_precondition(format!("Impersonation of {email} failed: {reason}"))
        };
        let response = self
            .http
            .post(url)
            .bearer_auth(&source.value)
            .json(&json!({ "scope": [CLOUD_PLATFORM] }))
            .send_on_io()
            .await
            .map_err(|e| failed(e.to_string()))?;
        let status = response.status();
        let body: Value = response.json().await.map_err(|e| failed(e.to_string()))?;
        match body["accessToken"].as_str() {
            Some(value) if status.is_success() => Ok(Token {
                value: value.to_string(),
                expires_at: body["expireTime"]
                    .as_str()
                    .and_then(|time| chrono::DateTime::parse_from_rfc3339(time).ok())
                    .map(|time| time.timestamp_millis()),
            }),
            _ => Err(failed(format!("({status}) {body}"))),
        }
    }

    async fn token_request(&self, url: &str, form: &[(&str, &str)]) -> Result<Token> {
        let response = self
            .http
            .post(url)
            .form(form)
            .send_on_io()
            .await
            .map_err(|e| Error::failed_precondition(format!("{url}: {e}")))?;
        token_response(response).await
    }

    /// Upstream `downscopeGcpCreds`: Google STS exchanges the token for one
    /// limited by a credential access boundary.
    async fn downscope(&self, source: &Token, ctx: &CredentialContext) -> Result<Token> {
        // Fewer permissions than upstream's `storage.objectAdmin`: no object
        // metadata and ACL changes, which table clients do not need.
        let roles: &[&str] = match ctx.writes() {
            true => &[
                "inRole:roles/storage.objectViewer",
                "inRole:roles/storage.legacyBucketWriter",
            ],
            false => &["inRole:roles/storage.objectViewer"],
        };
        let rules: Vec<Value> = ctx
            .locations
            .iter()
            .map(|location| {
                let bucket = location.storage_base().as_str().trim_start_matches("gs://").to_string();
                let path = location.path().trim_start_matches('/').to_string();
                json!({
                    "availableResource": format!("//storage.googleapis.com/projects/_/buckets/{bucket}"),
                    "availablePermissions": roles,
                    "availabilityCondition": { "expression": within(&bucket, &path) },
                })
            })
            .collect();
        let options = json!({ "accessBoundary": { "accessBoundaryRules": rules } }).to_string();
        let form = [
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            (
                "subject_token_type",
                "urn:ietf:params:oauth:token-type:access_token",
            ),
            (
                "requested_token_type",
                "urn:ietf:params:oauth:token-type:access_token",
            ),
            ("subject_token", source.value.as_str()),
            ("options", options.as_str()),
        ];
        let response = self
            .http
            .post(&self.sts)
            .form(&form)
            .send_on_io()
            .await
            .map_err(|e| Error::internal(format!("GCP token downscoping failed: {e}")))?;
        let mut token = token_response(response).await.map_err(|e| {
            Error::internal(format!("GCP token downscoping failed: {}", e.message()))
        })?;
        token.expires_at = token.expires_at.or(source.expires_at);
        Ok(token)
    }
}

/// `access_token` and `expires_in` of an OAuth token response.
async fn token_response(response: reqwest::Response) -> Result<Token> {
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| Error::failed_precondition(format!("token response: {e}")))?;
    match body["access_token"].as_str() {
        Some(value) if status.is_success() => Ok(Token {
            value: value.to_string(),
            expires_at: body["expires_in"]
                .as_i64()
                .map(|s| now_epoch_millis() + s * 1000),
        }),
        _ => Err(Error::failed_precondition(format!(
            "token request failed ({status}): {body}"
        ))),
    }
}

/// `GOOGLE_APPLICATION_CREDENTIALS`, else gcloud's application default
/// credentials file, if present.
fn default_credentials_file() -> Option<String> {
    if let Ok(path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        && !path.is_empty()
    {
        return Some(path);
    }
    let config = match std::env::var("CLOUDSDK_CONFIG") {
        Ok(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
        _ if cfg!(windows) => {
            std::path::PathBuf::from(std::env::var("APPDATA").ok()?).join("gcloud")
        }
        _ => std::path::PathBuf::from(std::env::var("HOME").ok()?)
            .join(".config")
            .join("gcloud"),
    };
    let file = config.join("application_default_credentials.json");
    file.exists().then(|| file.to_string_lossy().into_owned())
}

/// The CEL condition of the objects at `path` of `bucket`: the object
/// `path` itself, which Hadoop's GCS connector reads to check a folder, the
/// objects under `path/`, and the lists of their names. Not a sibling with
/// the same start, such as `t10` for `t1`, as upstream's `startsWith(path)`
/// allows.
fn within(bucket: &str, path: &str) -> String {
    let object = format!("projects/_/buckets/{bucket}/objects/{path}");
    let list = "api.getAttribute('storage.googleapis.com/objectListPrefix', '')";
    format!(
        "resource.name == {} || resource.name.startsWith({}) || {list} == {} || \
         {list}.startsWith({})",
        cel_string(&object),
        cel_string(&format!("{object}/")),
        cel_string(path),
        cel_string(&format!("{path}/")),
    )
}

/// Upstream `celStringLiteral`: a double-quoted CEL string.
fn cel_string(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');
    for c in value.chars() {
        match c {
            '\u{7}' => literal.push_str("\\a"),
            '\u{8}' => literal.push_str("\\b"),
            '\u{c}' => literal.push_str("\\f"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\u{b}' => literal.push_str("\\v"),
            '\\' => literal.push_str("\\\\"),
            '"' => literal.push_str("\\\""),
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_cel_strings() {
        assert_eq!(cel_string("a/b"), "\"a/b\"");
        assert_eq!(cel_string("q\"x\\y\n"), "\"q\\\"x\\\\y\\n\"");
    }
}
