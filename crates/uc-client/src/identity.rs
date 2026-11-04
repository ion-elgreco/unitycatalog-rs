//! Sign-in with the identity of the environment (the `identity` feature).
//!
//! The server takes the tokens of the identity providers of its `[[auth.issuers]]`, and says
//! which at `/.well-known/oauth-protected-resource` ([`metadata`]). A client that runs as an
//! Azure service principal or managed identity, a Google service account, an AWS role, or an
//! OAuth client of any provider finds its identity here, on olai-http's credential providers,
//! which keep and refresh their credentials. [`detect`] picks a source from the environment
//! and the server's issuers; [`azure`], [`google`], [`aws`], [`client_credentials`],
//! [`token_file`] and [`token_env`] make one of a kind. Each is a [`TokenSource`] for
//! [`Client::new_with_token_source`](crate::Client::new_with_token_source).

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use olai_http::ClientOptions;
use olai_http::bearer::{
    AzureManagedIdentity, BearerAuth, BearerCredentialProvider, ClientCredentials,
    GcpServiceAccount,
};
use serde::Deserialize;
use url::Url;

use crate::transport::{StaticToken, TokenError, TokenFuture, TokenSource};

/// The result of making a source.
pub type Result<T> = std::result::Result<T, TokenError>;

/// The variables of the environment.
pub type Env = HashMap<String, String>;

/// A source over an olai-http bearer provider, which keeps and refreshes its tokens.
pub struct BearerSource(pub BearerCredentialProvider);

impl fmt::Debug for BearerSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BearerSource").field(&self.0).finish()
    }
}

impl TokenSource for BearerSource {
    fn token(&self) -> TokenFuture<'_> {
        Box::pin(async move { Ok(self.0.get_credential().await?.token.clone()) })
    }
}

fn source(provider: BearerCredentialProvider) -> Arc<dyn TokenSource> {
    Arc::new(BearerSource(provider))
}

/// The scope of an Entra token for the audience `audience` of the server's issuer block:
/// `api://uc` is `api://uc/.default`.
pub fn azure_scope(audience: &str) -> String {
    match audience.ends_with("/.default") {
        true => audience.to_string(),
        false => format!("{}/.default", audience.trim_end_matches('/')),
    }
}

/// The resource of a scope: `api://uc/.default` is `api://uc`.
fn azure_resource(scope: &str) -> String {
    scope.trim_end_matches("/.default").to_string()
}

/// An Azure identity of the environment, for the scope `scope`: a service principal
/// (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`), a workload identity
/// (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_FEDERATED_TOKEN_FILE`), the sign-in of the
/// Azure CLI when `az` is on the path, else the managed identity of the machine or pod
/// (`AZURE_CLIENT_ID` for a user-assigned one). `AZURE_AUTHORITY_HOST` names another cloud.
pub fn azure(scope: &str) -> Result<Arc<dyn TokenSource>> {
    azure_in(&environment(), scope, on_path("az"))
}

fn azure_in(env: &Env, scope: &str, az: bool) -> Result<Arc<dyn TokenSource>> {
    let auth = BearerAuth::new();
    let authority = set(env, "AZURE_AUTHORITY_HOST").map(str::to_string);
    let tenant = set(env, "AZURE_TENANT_ID");
    let client = set(env, "AZURE_CLIENT_ID");
    let provider = if let (Some(tenant), Some(client), Some(secret)) =
        (tenant, client, set(env, "AZURE_CLIENT_SECRET"))
    {
        auth.azure_client_secret(tenant, client, secret, scope, authority)?
    } else if let (Some(tenant), Some(client), Some(file)) =
        (tenant, client, set(env, "AZURE_FEDERATED_TOKEN_FILE"))
    {
        auth.azure_workload_identity(tenant, client, file, scope, authority)?
    } else if az {
        auth.azure_cli(scope)
    } else {
        auth.azure_managed_identity(AzureManagedIdentity {
            resource: azure_resource(scope),
            client_id: client.map(str::to_string),
            endpoint: set(env, "IDENTITY_ENDPOINT")
                .or_else(|| set(env, "MSI_ENDPOINT"))
                .map(str::to_string),
            ..Default::default()
        })?
    };
    Ok(source(provider))
}

/// A Google identity of the environment, as an ID token for `audience`: the service-account
/// key file of `GOOGLE_APPLICATION_CREDENTIALS`, the sign-in of the Google Cloud CLI when
/// `gcloud` is on the path, else the metadata server of the machine or pod.
pub fn google(audience: &str) -> Result<Arc<dyn TokenSource>> {
    google_in(&environment(), audience, on_path("gcloud"))
}

fn google_in(env: &Env, audience: &str, gcloud: bool) -> Result<Arc<dyn TokenSource>> {
    let auth = BearerAuth::new();
    if let Some(path) = set(env, "GOOGLE_APPLICATION_CREDENTIALS") {
        let account = GcpServiceAccount::File(path.to_string());
        match auth.gcp_service_account_id_token(account, audience) {
            Ok(provider) => return Ok(source(provider)),
            // A person's own credentials (`gcloud auth application-default login`) cannot
            // make ID tokens for an audience; gcloud itself can.
            Err(e) if !gcloud => return Err(Box::new(e)),
            Err(_) => {}
        }
    }
    if gcloud {
        return Ok(Arc::new(Gcloud::new(audience)));
    }
    let endpoint = set(env, "GCE_METADATA_HOST").map(|host| {
        format!("http://{host}/computeMetadata/v1/instance/service-accounts/default/identity")
    });
    Ok(source(auth.gcp_id_token(audience, endpoint)?))
}

/// The start of an AWS bearer value, which the server reads as a signed request.
const AWS_PREFIX: &str = "uc-aws-v1.";
/// The signed header with the server id: the audience of the server's AWS issuer block.
const AWS_SERVER_ID_HEADER: &str = "x-uc-server-id";
/// The body of the signed request.
const GET_CALLER_IDENTITY: &str = "Action=GetCallerIdentity&Version=2011-06-15";
/// The global STS endpoint, which signs in `us-east-1`; the server's default.
const AWS_GLOBAL_STS: &str = "https://sts.amazonaws.com";
/// How long a signed request is reused. STS takes one for 15 minutes; the server keeps the
/// caller it names for 5.
const AWS_REUSE: Duration = Duration::from_secs(5 * 60);

/// An AWS identity of the environment, for the server whose AWS issuer block has the audience
/// `server_id`. AWS gives no token that the server can check, so each token is a signed
/// `sts:GetCallerIdentity` request, which the server sends to STS: the identity's credentials
/// never leave the client. The credentials are those of olai-http: `AWS_ACCESS_KEY_ID` and
/// `AWS_SECRET_ACCESS_KEY` (with `AWS_SESSION_TOKEN`), a web identity (`AWS_ROLE_ARN` and
/// `AWS_WEB_IDENTITY_TOKEN_FILE`, as on EKS), the container endpoint (ECS, EKS Pod Identity),
/// else the instance role of the machine. The request goes to the STS endpoint of
/// `AWS_REGION` (or `AWS_DEFAULT_REGION`), else the global one.
pub fn aws(server_id: &str) -> Result<Arc<dyn TokenSource>> {
    aws_in(&environment(), server_id, None)
}

/// [`aws`] with the variables `env`, and the STS endpoint `endpoint` instead of AWS's: one that
/// the server's AWS issuer block names, such as a VPC endpoint. A web identity and the
/// container endpoint are read from the process's variables, as olai-http does.
pub fn aws_in(env: &Env, server_id: &str, endpoint: Option<&str>) -> Result<Arc<dyn TokenSource>> {
    let region = set(env, "AWS_REGION")
        .or_else(|| set(env, "AWS_DEFAULT_REGION"))
        .map(str::to_string);
    let endpoint = match (endpoint, &region) {
        (Some(endpoint), _) => endpoint.to_string(),
        (None, Some(region)) => format!("https://sts.{region}.amazonaws.com"),
        (None, None) => AWS_GLOBAL_STS.to_string(),
    };
    let mut builder = olai_http::aws::AmazonBuilder::new();
    for (name, value) in env.iter().filter(|(name, _)| name.starts_with("AWS_")) {
        if let Ok(key) = name.to_ascii_lowercase().parse() {
            builder = builder.with_config(key, value);
        }
    }
    let credentials = builder.build(None)?.credentials;
    Ok(Arc::new(AwsSource {
        credentials,
        url: Url::parse(&endpoint)?.join("/")?,
        region: region.unwrap_or_else(|| "us-east-1".into()),
        server_id: server_id.to_string(),
        kept: Mutex::new(None),
    }))
}

/// The tokens of an AWS identity: signed `GetCallerIdentity` requests, each reused for
/// [`AWS_REUSE`].
struct AwsSource {
    credentials: olai_http::aws::AwsCredentialProvider,
    url: Url,
    region: String,
    server_id: String,
    kept: Mutex<Option<(String, Instant)>>,
}

impl fmt::Debug for AwsSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AwsSource")
            .field("url", &self.url.as_str())
            .field("region", &self.region)
            .field("server_id", &self.server_id)
            .finish_non_exhaustive()
    }
}

impl AwsSource {
    /// A new signed request, as a bearer value.
    async fn sign(&self) -> Result<String> {
        use base64::Engine;
        let credential = self.credentials.get_credential().await?;
        let mut headers = std::collections::BTreeMap::from([
            (
                "content-type".to_string(),
                "application/x-www-form-urlencoded; charset=utf-8".to_string(),
            ),
            (AWS_SERVER_ID_HEADER.to_string(), self.server_id.clone()),
        ]);
        sigv4::sign(
            &sigv4::Signing {
                key_id: &credential.key_id,
                secret_key: &credential.secret_key,
                session_token: credential.token.as_deref(),
                region: &self.region,
                service: "sts",
                date: chrono::Utc::now(),
            },
            "POST",
            &self.url,
            &mut headers,
            GET_CALLER_IDENTITY.as_bytes(),
        );
        // The server's HTTP client sends the host of the URL itself.
        headers.remove("host");
        let signed = serde_json::json!({
            "method": "POST",
            "url": self.url.as_str(),
            "headers": headers,
            "body": GET_CALLER_IDENTITY,
        });
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signed.to_string());
        Ok(format!("{AWS_PREFIX}{encoded}"))
    }
}

/// AWS Signature Version 4, of a request with its body in hand. olai-http signs this way too,
/// but keeps its signer to itself.
mod sigv4 {
    use std::collections::BTreeMap;

    use chrono::{DateTime, Utc};
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use url::Url;

    /// Who signs, where, and when.
    pub struct Signing<'a> {
        pub key_id: &'a str,
        pub secret_key: &'a str,
        pub session_token: Option<&'a str>,
        pub region: &'a str,
        pub service: &'a str,
        pub date: DateTime<Utc>,
    }

    fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes any key");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// URI encoding: the unreserved characters stay, `/` too when `slash`.
    fn encode(text: &str, slash: bool) -> String {
        let mut out = String::with_capacity(text.len());
        for byte in text.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(byte as char)
                }
                b'/' if slash => out.push('/'),
                _ => out.push_str(&format!("%{byte:02X}")),
            }
        }
        out
    }

    /// Sign the request: add `host`, `x-amz-date`, `x-amz-security-token` with a session
    /// token, and `authorization` to `headers`, whose names are lowercase. Every header in
    /// `headers` is signed.
    pub fn sign(
        signing: &Signing<'_>,
        method: &str,
        url: &Url,
        headers: &mut BTreeMap<String, String>,
        body: &[u8],
    ) {
        let host = match url.port() {
            Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
            None => url.host_str().unwrap_or_default().to_string(),
        };
        let amz_date = signing.date.format("%Y%m%dT%H%M%SZ").to_string();
        headers.insert("host".into(), host);
        headers.insert("x-amz-date".into(), amz_date.clone());
        if let Some(token) = signing.session_token {
            headers.insert("x-amz-security-token".into(), token.into());
        }
        let mut query: Vec<(String, String)> = url
            .query_pairs()
            .map(|(key, value)| (encode(&key, false), encode(&value, false)))
            .collect();
        query.sort();
        let query = (query.iter())
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("&");
        let canonical_headers: String = (headers.iter())
            .map(|(name, value)| format!("{name}:{}\n", value.trim()))
            .collect();
        let signed_headers = headers.keys().cloned().collect::<Vec<_>>().join(";");
        let canonical_request = format!(
            "{method}\n{}\n{query}\n{canonical_headers}\n{signed_headers}\n{}",
            encode(url.path(), true),
            hex::encode(Sha256::digest(body)),
        );
        let day = signing.date.format("%Y%m%d").to_string();
        let scope = format!("{day}/{}/{}/aws4_request", signing.region, signing.service);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );
        let key = [signing.region, signing.service, "aws4_request"]
            .iter()
            .fold(
                hmac(
                    format!("AWS4{}", signing.secret_key).as_bytes(),
                    day.as_bytes(),
                ),
                |key, part| hmac(&key, part.as_bytes()),
            );
        let signature = hex::encode(hmac(&key, string_to_sign.as_bytes()));
        headers.insert(
            "authorization".into(),
            format!(
                "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, \
                 Signature={signature}",
                signing.key_id
            ),
        );
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::TimeZone;

        /// The example of AWS's documentation of Signature Version 4: IAM's `ListUsers`.
        #[test]
        fn the_example_of_aws() {
            let mut headers = BTreeMap::from([(
                "content-type".to_string(),
                "application/x-www-form-urlencoded; charset=utf-8".to_string(),
            )]);
            let url = Url::parse("https://iam.amazonaws.com/?Action=ListUsers&Version=2010-05-08")
                .unwrap();
            let signing = Signing {
                key_id: "AKIDEXAMPLE",
                secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
                session_token: None,
                region: "us-east-1",
                service: "iam",
                date: Utc.with_ymd_and_hms(2015, 8, 30, 12, 36, 0).unwrap(),
            };
            sign(&signing, "GET", &url, &mut headers, b"");
            assert_eq!(
                headers["authorization"],
                "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/iam/aws4_request, \
                 SignedHeaders=content-type;host;x-amz-date, \
                 Signature=5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
            );
            assert_eq!(headers["host"], "iam.amazonaws.com");
            assert_eq!(headers["x-amz-date"], "20150830T123600Z");
        }
    }
}

impl TokenSource for AwsSource {
    fn token(&self) -> TokenFuture<'_> {
        Box::pin(async move {
            if let Ok(kept) = self.kept.lock()
                && let Some((token, until)) = kept.as_ref()
                && Instant::now() < *until
            {
                return Ok(token.clone());
            }
            let token = self.sign().await?;
            if let Ok(mut kept) = self.kept.lock() {
                *kept = Some((token.clone(), Instant::now() + AWS_REUSE));
            }
            Ok(token)
        })
    }
}

/// An OAuth client of any provider, with the client-credentials grant at `token_url`:
/// FerrisKey, Keycloak, Okta, Entra with its own scope, and the like.
pub fn client_credentials(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scope: Option<&str>,
    audience: Option<&str>,
) -> Result<Arc<dyn TokenSource>> {
    // olai-http takes https only, unless told otherwise: a plain-http token endpoint is the
    // caller's choice, such as a provider on this machine.
    let options = ClientOptions::new().with_allow_http(token_url.starts_with("http://"));
    let provider = BearerAuth::new()
        .with_client_options(options)
        .client_credentials(ClientCredentials {
            token_url: token_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            scope: scope.map(str::to_string),
            audience: audience.map(str::to_string),
        })?;
    Ok(source(provider))
}

/// The token in the file `path`, read again at each refresh: a projected Kubernetes
/// service-account token, for one.
pub fn token_file(path: &str) -> Arc<dyn TokenSource> {
    source(BearerAuth::new().token_file(path))
}

/// The token in the variable `var`, read again at each refresh.
pub fn token_env(var: &str) -> Arc<dyn TokenSource> {
    source(BearerAuth::new().token_env(var))
}

/// What the server takes as a bearer token: its `/.well-known/oauth-protected-resource`
/// (RFC 9728), with the `[[auth.issuers]]` blocks as `issuers`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProtectedResource {
    /// The server's public URL, when it has one.
    #[serde(default)]
    pub resource: Option<String>,
    /// The exact issuers whose tokens are taken on every request.
    #[serde(default)]
    pub authorization_servers: Vec<String>,
    /// Every issuer block of the server.
    #[serde(default)]
    pub issuers: Vec<Issuer>,
}

/// An `[[auth.issuers]]` block of the server.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Issuer {
    pub name: String,
    /// `oidc`, or `aws` for AWS identities; empty from a server that does not say.
    #[serde(default)]
    pub kind: String,
    /// The `iss` of its tokens; `*` stands for one DNS label. For AWS, the STS endpoint.
    pub issuer: String,
    /// The `aud` its tokens must carry; `*` alone for any.
    #[serde(default)]
    pub audiences: Vec<String>,
    /// Its tokens are taken on every request, not only at the token exchange.
    #[serde(default)]
    pub direct: bool,
}

impl Issuer {
    /// The audience to ask the provider for: the first that is not `*`.
    fn audience(&self) -> Option<&str> {
        self.audiences
            .iter()
            .map(String::as_str)
            .find(|audience| *audience != "*")
    }

    /// The provider, by its kind and its issuer.
    fn kind(&self) -> Kind {
        if self.kind == "aws" {
            return Kind::Aws;
        }
        let host = Url::parse(&self.issuer)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_default();
        let is = |suffix: &str| host == suffix || host.ends_with(&format!(".{suffix}"));
        if is("microsoftonline.com")
            || is("windows.net")
            || is("microsoftonline.us")
            || is("chinacloudapi.cn")
        {
            Kind::Entra
        } else if is("accounts.google.com") {
            Kind::Google
        } else {
            Kind::Other
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Entra,
    Google,
    Aws,
    Other,
}

/// The server's metadata; `None` when the server has no such endpoint.
pub async fn metadata(server: &Url) -> Result<Option<ProtectedResource>> {
    let url = server.join(".well-known/oauth-protected-resource")?;
    let response = reqwest::Client::new().get(url).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    Ok(Some(response.error_for_status()?.json().await?))
}

/// The token source of the environment for the server at `server`, if it has one:
///
/// 1. a fixed token in `UC_TOKEN`, or a token file in `UC_TOKEN_FILE`;
/// 2. an OAuth client in `UC_OAUTH_TOKEN_URL`, `UC_OAUTH_CLIENT_ID` and `UC_OAUTH_CLIENT_SECRET`
///    (with `UC_OAUTH_SCOPE` and `UC_OAUTH_AUDIENCE`, else the audience of the server's first
///    issuer), with the client-credentials grant;
/// 3. else the identity of the environment at the first of the server's `direct` issuers that
///    it can sign in to: an Azure identity at an Entra issuer ([`azure`], when the Azure
///    variables are set, `az` is on the path, or a managed identity is announced), a Google
///    identity at Google ([`google`], when `GOOGLE_APPLICATION_CREDENTIALS` is set, `gcloud` is
///    on the path, or the machine is Google's), an AWS identity at an AWS block ([`aws`], when
///    the AWS variables of keys, a web identity, a container or a region are set).
///
/// Without any of these, `None`, and the server is not asked anything. A server that does not
/// answer, or has no metadata, gives `None` too: the call that follows says what is wrong.
pub async fn detect(server: &Url) -> Result<Option<Arc<dyn TokenSource>>> {
    detect_in(&environment(), server, on_path("az"), on_path("gcloud")).await
}

/// [`detect`] with the variables `env`, and whether the Azure and Google CLIs are on the path.
pub async fn detect_in(
    env: &Env,
    server: &Url,
    az: bool,
    gcloud: bool,
) -> Result<Option<Arc<dyn TokenSource>>> {
    if !hints(env, az, gcloud) {
        return Ok(None);
    }
    let issuers = match choose(env, &[], az, gcloud) {
        // A token needs no issuer; an OAuth client takes the audience from one.
        Some(Choice::Token(_) | Choice::TokenFile(_)) => Vec::new(),
        _ => metadata(server)
            .await
            .ok()
            .flatten()
            .map(|resource| resource.issuers)
            .unwrap_or_default(),
    };
    match choose(env, &issuers, az, gcloud) {
        Some(choice) => build(choice, env, az, gcloud).map(Some),
        None => Ok(None),
    }
}

/// What [`detect`] would make.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Choice {
    Token(String),
    TokenFile(String),
    ClientCredentials {
        token_url: String,
        client_id: String,
        client_secret: String,
        scope: Option<String>,
        audience: Option<String>,
    },
    Azure {
        scope: String,
    },
    Google {
        audience: String,
    },
    Aws {
        server_id: String,
        /// The block's STS endpoint, when it is not AWS's.
        endpoint: Option<String>,
    },
}

/// Whether the environment has anything to sign in with.
fn hints(env: &Env, az: bool, gcloud: bool) -> bool {
    set(env, "UC_TOKEN").is_some()
        || set(env, "UC_TOKEN_FILE").is_some()
        || oauth_hints(env)
        || azure_hints(env, az)
        || google_hints(env, gcloud)
        || aws_hints(env)
}

/// The variables of an AWS identity. On EC2 the instance role announces nothing: the region
/// is the hint there.
fn aws_hints(env: &Env) -> bool {
    [
        "AWS_ACCESS_KEY_ID",
        "AWS_WEB_IDENTITY_TOKEN_FILE",
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
        "AWS_REGION",
        "AWS_DEFAULT_REGION",
    ]
    .iter()
    .any(|var| set(env, var).is_some())
}

fn oauth_hints(env: &Env) -> bool {
    set(env, "UC_OAUTH_TOKEN_URL").is_some()
        && set(env, "UC_OAUTH_CLIENT_ID").is_some()
        && set(env, "UC_OAUTH_CLIENT_SECRET").is_some()
}

fn azure_hints(env: &Env, az: bool) -> bool {
    az || [
        "AZURE_CLIENT_SECRET",
        "AZURE_FEDERATED_TOKEN_FILE",
        "AZURE_CLIENT_ID",
        "IDENTITY_ENDPOINT",
        "MSI_ENDPOINT",
    ]
    .iter()
    .any(|var| set(env, var).is_some())
}

fn google_hints(env: &Env, gcloud: bool) -> bool {
    gcloud
        || [
            "GOOGLE_APPLICATION_CREDENTIALS",
            "GCE_METADATA_HOST",
            "K_SERVICE",
        ]
        .iter()
        .any(|var| set(env, var).is_some())
}

/// The choice of [`detect`] for the variables `env` and the server's `issuers`.
fn choose(env: &Env, issuers: &[Issuer], az: bool, gcloud: bool) -> Option<Choice> {
    if let Some(token) = set(env, "UC_TOKEN") {
        return Some(Choice::Token(token.to_string()));
    }
    if let Some(path) = set(env, "UC_TOKEN_FILE") {
        return Some(Choice::TokenFile(path.to_string()));
    }
    let direct: Vec<&Issuer> = issuers.iter().filter(|issuer| issuer.direct).collect();
    if let (Some(token_url), Some(client_id), Some(client_secret)) = (
        set(env, "UC_OAUTH_TOKEN_URL"),
        set(env, "UC_OAUTH_CLIENT_ID"),
        set(env, "UC_OAUTH_CLIENT_SECRET"),
    ) {
        let audience = set(env, "UC_OAUTH_AUDIENCE")
            .map(str::to_string)
            .or_else(|| {
                direct
                    .iter()
                    .find_map(|issuer| issuer.audience().map(str::to_string))
            });
        return Some(Choice::ClientCredentials {
            token_url: token_url.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            scope: set(env, "UC_OAUTH_SCOPE").map(str::to_string),
            audience,
        });
    }
    for issuer in direct {
        let Some(audience) = issuer.audience() else {
            continue;
        };
        match issuer.kind() {
            Kind::Entra if azure_hints(env, az) => {
                return Some(Choice::Azure {
                    scope: azure_scope(audience),
                });
            }
            Kind::Google if google_hints(env, gcloud) => {
                return Some(Choice::Google {
                    audience: audience.to_string(),
                });
            }
            Kind::Aws if aws_hints(env) => {
                let endpoint = issuer.issuer.trim_end_matches('/');
                let own =
                    !endpoint.is_empty() && endpoint != AWS_GLOBAL_STS && !endpoint.contains('*');
                return Some(Choice::Aws {
                    server_id: audience.to_string(),
                    endpoint: own.then(|| endpoint.to_string()),
                });
            }
            _ => {}
        }
    }
    None
}

fn build(choice: Choice, env: &Env, az: bool, gcloud: bool) -> Result<Arc<dyn TokenSource>> {
    Ok(match choice {
        Choice::Token(token) => Arc::new(StaticToken(token)),
        Choice::TokenFile(path) => token_file(&path),
        Choice::ClientCredentials {
            token_url,
            client_id,
            client_secret,
            scope,
            audience,
        } => client_credentials(
            &token_url,
            &client_id,
            &client_secret,
            scope.as_deref(),
            audience.as_deref(),
        )?,
        Choice::Azure { scope } => azure_in(env, &scope, az)?,
        Choice::Google { audience } => google_in(env, &audience, gcloud)?,
        Choice::Aws {
            server_id,
            endpoint,
        } => aws_in(env, &server_id, endpoint.as_deref())?,
    })
}

/// The ID tokens of the Google Cloud CLI's sign-in: `gcloud auth print-identity-token`, kept
/// until a minute before they expire. A person's sign-in cannot take an audience, so the
/// token is asked without one when it fails with one.
struct Gcloud {
    audience: String,
    kept: Mutex<Option<(String, Instant)>>,
}

impl Gcloud {
    fn new(audience: &str) -> Self {
        Self {
            audience: audience.to_string(),
            kept: Mutex::new(None),
        }
    }

    async fn print_identity_token(&self, audience: bool) -> Result<String> {
        let mut command = tokio::process::Command::new("gcloud");
        command.args(["auth", "print-identity-token"]);
        if audience {
            command.arg("--audiences").arg(&self.audience);
        }
        let output = command
            .output()
            .await
            .map_err(|e| format!("cannot run gcloud: {e}"))?;
        if !output.status.success() {
            let reason = String::from_utf8_lossy(&output.stderr);
            return Err(format!("gcloud auth print-identity-token: {}", reason.trim()).into());
        }
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        match token.is_empty() {
            true => Err("gcloud printed no token".into()),
            false => Ok(token),
        }
    }
}

impl fmt::Debug for Gcloud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gcloud")
            .field("audience", &self.audience)
            .finish_non_exhaustive()
    }
}

impl TokenSource for Gcloud {
    fn token(&self) -> TokenFuture<'_> {
        Box::pin(async move {
            if let Ok(kept) = self.kept.lock()
                && let Some((token, until)) = kept.as_ref()
                && Instant::now() < *until
            {
                return Ok(token.clone());
            }
            let token = match self.print_identity_token(true).await {
                Ok(token) => token,
                Err(with_audience) => self
                    .print_identity_token(false)
                    .await
                    .map_err(|_| with_audience)?,
            };
            let until = jwt_expiry(&token)
                .map(|at| at.checked_sub(Duration::from_secs(60)).unwrap_or(at))
                .unwrap_or_else(|| Instant::now() + Duration::from_secs(5 * 60));
            if let Ok(mut kept) = self.kept.lock() {
                *kept = Some((token.clone(), until));
            }
            Ok(token)
        })
    }
}

/// When the JWT `token` expires, from its `exp`.
fn jwt_expiry(token: &str) -> Option<Instant> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let exp = claims.get("exp")?.as_i64()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    Some(Instant::now() + Duration::from_secs(exp.saturating_sub(now).max(0) as u64))
}

/// The variables of this process.
pub fn environment() -> Env {
    std::env::vars().collect()
}

/// The variable `name` of `env`, trimmed; none when empty.
fn set<'a>(env: &'a Env, name: &str) -> Option<&'a str> {
    env.get(name)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

/// Whether the program `name` is on the path.
pub fn on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        [
            name.to_string(),
            format!("{name}.cmd"),
            format!("{name}.exe"),
        ]
        .iter()
        .any(|file| dir.join(file).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(vars: &[(&str, &str)]) -> Env {
        vars.iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    fn issuer(name: &str, issuer: &str, audience: &str, direct: bool) -> Issuer {
        Issuer {
            name: name.into(),
            issuer: issuer.into(),
            audiences: vec![audience.into()],
            direct,
            ..Default::default()
        }
    }

    #[test]
    fn scopes_and_kinds() {
        assert_eq!(azure_scope("api://uc"), "api://uc/.default");
        assert_eq!(azure_scope("api://uc/.default"), "api://uc/.default");
        assert_eq!(azure_resource("api://uc/.default"), "api://uc");
        let kind = |iss: &str| issuer("x", iss, "a", true).kind();
        assert_eq!(
            kind("https://login.microsoftonline.com/t/v2.0"),
            Kind::Entra
        );
        assert_eq!(kind("https://sts.windows.net/t/"), Kind::Entra);
        assert_eq!(kind("https://accounts.google.com"), Kind::Google);
        assert_eq!(kind("http://localhost:8090/realms/uc"), Kind::Other);
        assert_eq!(kind("not a url"), Kind::Other);
        let any = Issuer {
            audiences: vec!["*".into()],
            ..issuer("x", "https://a", "", true)
        };
        assert_eq!(any.audience(), None);
    }

    #[test]
    fn the_environment_chooses() {
        let entra = issuer(
            "entra",
            "https://login.microsoftonline.com/t/v2.0",
            "api://uc",
            true,
        );
        let google = issuer("google", "https://accounts.google.com", "uc", true);
        let exchange_only = issuer("ferriskey", "http://localhost:8090/realms/uc", "uc", false);
        let issuers = [exchange_only.clone(), entra.clone(), google.clone()];

        // Nothing to sign in with.
        assert!(!hints(&env(&[]), false, false));
        assert_eq!(choose(&env(&[]), &issuers, false, false), None);
        // A token or a token file first.
        assert_eq!(
            choose(&env(&[("UC_TOKEN", " t ")]), &issuers, true, true),
            Some(Choice::Token("t".into()))
        );
        assert_eq!(
            choose(&env(&[("UC_TOKEN_FILE", "/run/token")]), &[], false, false),
            Some(Choice::TokenFile("/run/token".into()))
        );
        // An OAuth client takes the audience of the first direct issuer.
        let oauth = env(&[
            ("UC_OAUTH_TOKEN_URL", "https://idp/token"),
            ("UC_OAUTH_CLIENT_ID", "etl"),
            ("UC_OAUTH_CLIENT_SECRET", "s"),
        ]);
        assert!(hints(&oauth, false, false));
        assert_eq!(
            choose(&oauth, &issuers, false, false),
            Some(Choice::ClientCredentials {
                token_url: "https://idp/token".into(),
                client_id: "etl".into(),
                client_secret: "s".into(),
                scope: None,
                audience: Some("api://uc".into()),
            })
        );
        // The first direct issuer the environment can sign in to.
        let service_principal = env(&[
            ("AZURE_TENANT_ID", "t"),
            ("AZURE_CLIENT_ID", "c"),
            ("AZURE_CLIENT_SECRET", "s"),
        ]);
        assert_eq!(
            choose(&service_principal, &issuers, false, false),
            Some(Choice::Azure {
                scope: "api://uc/.default".into()
            })
        );
        assert_eq!(
            choose(&env(&[]), &issuers, true, false),
            Some(Choice::Azure {
                scope: "api://uc/.default".into()
            })
        );
        let key_file = env(&[("GOOGLE_APPLICATION_CREDENTIALS", "/k.json")]);
        assert_eq!(
            choose(&key_file, &issuers, false, false),
            Some(Choice::Google {
                audience: "uc".into()
            })
        );
        assert_eq!(
            choose(&env(&[]), &issuers, false, true),
            Some(Choice::Google {
                audience: "uc".into()
            })
        );
        // Not at an issuer the environment cannot sign in to, nor one taken at the exchange
        // only.
        assert_eq!(
            choose(&key_file, std::slice::from_ref(&entra), false, false),
            None
        );
        assert_eq!(
            choose(&service_principal, &[exchange_only], false, false),
            None
        );
        // Not without an audience to ask for.
        let any = Issuer {
            audiences: vec!["*".into()],
            ..entra
        };
        assert_eq!(choose(&service_principal, &[any], false, false), None);

        // An AWS identity at an AWS block: AWS's endpoint, or the block's own.
        let aws = Issuer {
            kind: "aws".into(),
            ..issuer(
                "aws",
                "https://sts.amazonaws.com",
                "https://uc.example.com",
                true,
            )
        };
        let keys = env(&[
            ("AWS_ACCESS_KEY_ID", "AKIA"),
            ("AWS_SECRET_ACCESS_KEY", "s"),
        ]);
        assert!(hints(&keys, false, false));
        assert_eq!(
            choose(&keys, &[google.clone(), aws.clone()], false, false),
            Some(Choice::Aws {
                server_id: "https://uc.example.com".into(),
                endpoint: None,
            })
        );
        let vpc = Issuer {
            issuer: "https://vpce-1.sts.eu-west-1.vpce.amazonaws.com/".into(),
            ..aws.clone()
        };
        assert_eq!(
            choose(&env(&[("AWS_REGION", "eu-west-1")]), &[vpc], false, false),
            Some(Choice::Aws {
                server_id: "https://uc.example.com".into(),
                endpoint: Some("https://vpce-1.sts.eu-west-1.vpce.amazonaws.com".into()),
            })
        );
        assert_eq!(choose(&service_principal, &[aws], false, false), None);
    }

    #[test]
    fn jwt_expiries() {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 600;
        let token = |claims: &str| {
            use base64::Engine;
            let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims);
            format!("eyJhbGciOiJub25lIn0.{payload}.sig")
        };
        let at = jwt_expiry(&token(&format!(r#"{{"exp":{exp}}}"#))).unwrap();
        let left = at.saturating_duration_since(Instant::now());
        assert!(left > Duration::from_secs(590) && left <= Duration::from_secs(600));
        assert!(jwt_expiry(&token("{}")).is_none());
        assert!(jwt_expiry("opaque").is_none());
    }
}
