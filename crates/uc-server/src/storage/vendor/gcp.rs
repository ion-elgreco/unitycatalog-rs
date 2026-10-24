//! GCP credentials, after upstream `GcpCredentialVendor`.
//!
//! Each bucket is configured by `gcs.*.N`. The server's credential (the
//! entry's service account key file, else Application Default Credentials:
//! `GOOGLE_APPLICATION_CREDENTIALS`, gcloud's file or the metadata server)
//! is exchanged at Google STS for a token downscoped to the locations:
//! `storage.objectViewer` for reads, `storage.objectAdmin` for writes.
//! Upstream's `TestingCredentialGenerator` gives a fixed token. External
//! locations are not supported for GCP yet, as upstream.
//!
//! The server's token does not come from `object_store`: for a service
//! account key it is a self-signed JWT, not the OAuth access token that
//! STS exchanges.

use std::collections::HashMap;

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};

use uc_api::{GcpOauthToken, TemporaryCredentials};
use uc_common::{Error, NormalizedUrl, Result, now_epoch_millis};

use super::CredentialContext;
use crate::http::outbound::SendOnIo;

const CLOUD_PLATFORM: &str = "https://www.googleapis.com/auth/cloud-platform";
const GOOGLE_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
/// Upstream's expiration of a testing token.
const TESTING_EXPIRATION: i64 = 253_370_790_000_000;

/// Upstream `GcsStorageConfig`.
#[derive(Debug, Clone)]
pub struct GcsConfig {
    pub bucket_path: String,
    pub json_key_file_path: Option<String>,
    /// Only upstream's `TestingCredentialGenerator` is known.
    pub credential_generator: Option<String>,
}

pub struct GcpVendor {
    buckets: HashMap<NormalizedUrl, GcsConfig>,
    http: reqwest::Client,
    sts: String,
    metadata: String,
}

/// A token and when it expires (epoch milliseconds), if known.
struct Token {
    value: String,
    expires_at: Option<i64>,
}

impl GcpVendor {
    pub fn new(buckets: HashMap<NormalizedUrl, GcsConfig>) -> Self {
        Self {
            buckets,
            http: reqwest::Client::new(),
            sts: "https://sts.googleapis.com/v1/token".to_string(),
            metadata: "http://metadata.google.internal".to_string(),
        }
    }

    /// STS and metadata endpoints, for tests.
    pub fn with_endpoints(mut self, sts: Option<String>, metadata: Option<String>) -> Self {
        if let Some(sts) = sts {
            self.sts = sts;
        }
        if let Some(metadata) = metadata {
            self.metadata = metadata;
        }
        self
    }

    pub async fn vend(&self, ctx: &CredentialContext) -> Result<TemporaryCredentials> {
        if ctx.credential.is_some() {
            return Err(Error::unimplemented(
                "Storage credential/external location for GCP is not supported yet.",
            ));
        }
        let config = self.buckets.get(&ctx.storage_base).ok_or_else(|| {
            Error::failed_precondition(format!(
                "Unknown GCS storage configuration for {}.",
                ctx.storage_base
            ))
        })?;
        let token = match config.credential_generator.as_deref() {
            Some(generator) if generator.ends_with("TestingCredentialGenerator") => Token {
                value: config
                    .json_key_file_path
                    .clone()
                    .unwrap_or_else(|| format!("testing://{}", config.bucket_path)),
                expires_at: Some(TESTING_EXPIRATION),
            },
            Some(generator) => {
                return Err(Error::internal(format!(
                    "Unable to instantiate GCS credentials generator {generator}"
                )));
            }
            None => {
                let source = self
                    .source_token(config.json_key_file_path.as_deref())
                    .await
                    .map_err(|_| Error::failed_precondition("GCS credentials not found."))?;
                self.downscope(&source, ctx).await?
            }
        };
        Ok(TemporaryCredentials {
            gcp_oauth_token: Some(GcpOauthToken {
                oauth_token: Some(token.value),
            }),
            expiration_time: token.expires_at,
            ..Default::default()
        })
    }

    /// The key file, else Application Default Credentials.
    async fn source_token(&self, key_file: Option<&str>) -> Result<Token> {
        let file = key_file
            .map(str::to_string)
            .or_else(default_credentials_file);
        let Some(file) = file else {
            return self.metadata_token().await;
        };
        let text = tokio::fs::read_to_string(&file)
            .await
            .map_err(|e| Error::failed_precondition(format!("{file}: {e}")))?;
        let key: Value = serde_json::from_str(&text)
            .map_err(|e| Error::failed_precondition(format!("{file}: {e}")))?;
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
                "unsupported credential type {other:?} in {file}"
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
        let role = match ctx.writes() {
            true => "inRole:roles/storage.objectAdmin",
            false => "inRole:roles/storage.objectViewer",
        };
        let rules: Vec<Value> = ctx
            .locations
            .iter()
            .map(|location| {
                let bucket = location.storage_base().as_str().trim_start_matches("gs://").to_string();
                let path = location.path().trim_start_matches('/').to_string();
                let object = cel_string(&format!("projects/_/buckets/{bucket}/objects/{path}"));
                let expression = format!(
                    "resource.name.startsWith({object}) || \
                     api.getAttribute('storage.googleapis.com/objectListPrefix', '').startsWith({})",
                    cel_string(&path)
                );
                json!({
                    "availableResource": format!("//storage.googleapis.com/projects/_/buckets/{bucket}"),
                    "availablePermissions": [role],
                    "availabilityCondition": { "expression": expression },
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
