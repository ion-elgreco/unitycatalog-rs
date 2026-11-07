//! Sign-in with the identity of the environment (the `identity` feature).
//!
//! The server takes the tokens of the identity providers of its `[[auth.issuers]]`, and says
//! which at `/.well-known/oauth-protected-resource` ([`metadata`]). A client that runs as an
//! Azure service principal or managed identity, a Google service account, an AWS role, or an
//! OAuth client of any provider finds its identity here, on olai-http's credential providers,
//! which keep and refresh their credentials. [`detect`] picks a source from the environment
//! and the server's issuers; [`azure`], [`google`], [`aws`], [`client_credentials`],
//! [`token_file`] and [`token_env`] make one of a kind. Each is a bearer provider for
//! [`Client::new_with_bearer`](crate::Client::new_with_bearer); [`detect`] gives a signer for
//! [`Client::new_with_signer`](crate::Client::new_with_signer).

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use olai_http::bearer::{
    AwsSignedIdentity, AzureManagedIdentity, BearerAuth, BearerCredentialProvider,
    BearerProviderSigner, ClientCredentials, GcpServiceAccount,
};
use olai_http::reqwest::RequestBuilder;
use olai_http::{ClientOptions, RequestSigner};
use serde::Deserialize;
use url::Url;

/// The result of making a provider or a signer.
pub type Result<T> = olai_http::Result<T>;

/// The variables of the environment.
pub type Env = HashMap<String, String>;

/// An error with `message`.
fn failed(message: impl Into<String>) -> olai_http::Error {
    olai_http::Error::Generic {
        source: message.into().into(),
    }
}

/// The URL `url`.
fn parse(url: &str) -> Result<Url> {
    Url::parse(url).map_err(|e| failed(format!("invalid URL {url:?}: {e}")))
}

/// The signer of the bearer tokens of `provider`.
fn signer(provider: BearerCredentialProvider) -> Arc<dyn RequestSigner> {
    Arc::new(BearerProviderSigner(provider))
}

/// The scope of an Entra token for the audience `audience` of the server's issuer block:
/// `api://uc` is `api://uc/.default`.
pub fn azure_scope(audience: &str) -> String {
    match audience.ends_with("/.default") {
        true => audience.to_string(),
        false => format!("{}/.default", audience.trim_end_matches('/')),
    }
}

/// A character of an Entra scope: a letter, a digit, or one of `-._~:/`.
fn scope_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~:/".contains(c)
}

/// The resource of a scope: `api://uc/.default` is `api://uc`.
fn azure_resource(scope: &str) -> String {
    scope.trim_end_matches("/.default").to_string()
}

/// An Azure identity of the environment, for the scope `scope`: a service principal
/// (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`), a workload identity
/// (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_FEDERATED_TOKEN_FILE`), a managed identity
/// that the environment names (`AZURE_CLIENT_ID` for a user-assigned one, `IDENTITY_ENDPOINT`
/// or `MSI_ENDPOINT`), the sign-in of the Azure CLI when `az` is on the path, else the managed
/// identity of the machine or pod. `AZURE_AUTHORITY_HOST` names another cloud.
pub fn azure(scope: &str) -> Result<BearerCredentialProvider> {
    azure_in(&environment(), scope, on_path("az"))
}

fn azure_in(env: &Env, scope: &str, az: bool) -> Result<BearerCredentialProvider> {
    // The Azure CLI runs through `cmd /C` on Windows, which reads `&`, `|`, `%` and the like
    // in its arguments; an Entra scope is a URI without them.
    if scope.is_empty() || !scope.chars().all(scope_char) {
        return Err(failed(format!("not an Entra scope: {scope:?}")));
    }
    let auth = BearerAuth::new();
    let authority = set(env, "AZURE_AUTHORITY_HOST").map(str::to_string);
    let provider = match azure_identity(env, az) {
        AzureIdentity::ClientSecret {
            tenant,
            client,
            secret,
        } => auth.azure_client_secret(tenant, client, secret, scope, authority)?,
        AzureIdentity::Workload {
            tenant,
            client,
            file,
        } => auth.azure_workload_identity(tenant, client, file, scope, authority)?,
        AzureIdentity::Cli => auth.azure_cli(scope),
        AzureIdentity::Managed { client, endpoint } => {
            auth.azure_managed_identity(AzureManagedIdentity {
                resource: azure_resource(scope),
                client_id: client.map(str::to_string),
                endpoint: endpoint.map(str::to_string),
                ..Default::default()
            })?
        }
    };
    Ok(provider)
}

/// The Azure identity of the environment that [`azure`] takes.
#[derive(Debug, PartialEq, Eq)]
enum AzureIdentity<'a> {
    ClientSecret {
        tenant: &'a str,
        client: &'a str,
        secret: &'a str,
    },
    Workload {
        tenant: &'a str,
        client: &'a str,
        file: &'a str,
    },
    Cli,
    Managed {
        client: Option<&'a str>,
        endpoint: Option<&'a str>,
    },
}

fn azure_identity(env: &Env, az: bool) -> AzureIdentity<'_> {
    let tenant = set(env, "AZURE_TENANT_ID");
    let client = set(env, "AZURE_CLIENT_ID");
    let endpoint = set(env, "IDENTITY_ENDPOINT").or_else(|| set(env, "MSI_ENDPOINT"));
    if let (Some(tenant), Some(client), Some(secret)) =
        (tenant, client, set(env, "AZURE_CLIENT_SECRET"))
    {
        return AzureIdentity::ClientSecret {
            tenant,
            client,
            secret,
        };
    }
    if let (Some(tenant), Some(client), Some(file)) =
        (tenant, client, set(env, "AZURE_FEDERATED_TOKEN_FILE"))
    {
        return AzureIdentity::Workload {
            tenant,
            client,
            file,
        };
    }
    // A managed identity that the environment names comes before the CLI, which may be
    // installed but not signed in.
    match az && client.is_none() && endpoint.is_none() {
        true => AzureIdentity::Cli,
        false => AzureIdentity::Managed { client, endpoint },
    }
}

/// A Google identity of the environment, as an ID token for `audience`: the service-account
/// key file of `GOOGLE_APPLICATION_CREDENTIALS`, the sign-in of the Google Cloud CLI when
/// `gcloud` is on the path, else the metadata server of the machine or pod.
pub fn google(audience: &str) -> Result<BearerCredentialProvider> {
    google_in(&environment(), audience, on_path("gcloud"))
}

fn google_in(env: &Env, audience: &str, gcloud: bool) -> Result<BearerCredentialProvider> {
    let auth = BearerAuth::new();
    if let Some(path) = set(env, "GOOGLE_APPLICATION_CREDENTIALS") {
        let account = GcpServiceAccount::File(path.to_string());
        match auth.gcp_service_account_id_token(account, audience) {
            Ok(provider) => return Ok(provider),
            // A person's own credentials (`gcloud auth application-default login`) cannot
            // make ID tokens for an audience; gcloud itself can.
            Err(e) if !gcloud => return Err(e),
            Err(_) => {}
        }
    }
    if gcloud {
        return Ok(auth.gcloud_id_token(audience));
    }
    let endpoint = set(env, "GCE_METADATA_HOST").map(|host| {
        format!("http://{host}/computeMetadata/v1/instance/service-accounts/default/identity")
    });
    auth.gcp_id_token(audience, endpoint)
}

/// The start of an AWS bearer value, which the server reads as a signed request.
const AWS_PREFIX: &str = "uc-aws-v1.";
/// The signed header with the server id: the audience of the server's AWS issuer block.
const AWS_SERVER_ID_HEADER: &str = "x-uc-server-id";
/// The global STS endpoint, which signs in [`AWS_GLOBAL_REGION`].
const AWS_GLOBAL_STS: &str = "https://sts.amazonaws.com";
const AWS_GLOBAL_REGION: &str = "us-east-1";
/// The variables that [`aws_in`] gives olai-http. Not `AWS_ROLE_ARN`: it is the role of a web
/// identity, which olai-http reads itself, and as `role_arn` it would assume the role again.
const AWS_VARIABLES: [&str; 8] = [
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AWS_REGION",
    "AWS_DEFAULT_REGION",
    "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
    "AWS_METADATA_ENDPOINT",
    "AWS_IMDSV1_FALLBACK",
];

/// An AWS identity of the environment, for the server whose AWS issuer block has the audience
/// `server_id`. AWS gives no token that the server can check, so each token is a signed
/// `sts:GetCallerIdentity` request, which the server sends to STS: the identity's credentials
/// never leave the client. The credentials are those of olai-http: `AWS_ACCESS_KEY_ID` and
/// `AWS_SECRET_ACCESS_KEY` (with `AWS_SESSION_TOKEN`), a web identity (`AWS_ROLE_ARN` and
/// `AWS_WEB_IDENTITY_TOKEN_FILE`, as on EKS), the container endpoint (ECS, EKS Pod Identity),
/// else the instance role of the machine. The request goes to the STS endpoint of
/// `AWS_REGION` (or `AWS_DEFAULT_REGION`), else the global one. A signed request is used for
/// 5 of its 15 minutes at STS, as long as the server keeps the caller it names.
pub fn aws(server_id: &str) -> Result<BearerCredentialProvider> {
    aws_in(&environment(), server_id, None)
}

/// [`aws`] with the variables `env`, and the STS endpoint `endpoint` instead of AWS's: one that
/// the server's AWS issuer block names, such as a regional or VPC endpoint, whose region signs
/// the request. A web identity and the container endpoint are read from the process's
/// variables, as olai-http does.
pub fn aws_in(
    env: &Env,
    server_id: &str,
    endpoint: Option<&str>,
) -> Result<BearerCredentialProvider> {
    let (url, region) = aws_endpoint(env, endpoint)?;
    let mut builder = olai_http::aws::AmazonBuilder::new();
    for name in AWS_VARIABLES {
        if let Some(value) = set(env, name)
            && let Ok(key) = name.to_ascii_lowercase().parse()
        {
            builder = builder.with_config(key, value);
        }
    }
    let credentials = builder.build(None)?.credentials;
    BearerAuth::new().aws_signed_identity(AwsSignedIdentity {
        credentials,
        endpoint: url.to_string(),
        region,
        headers: vec![(AWS_SERVER_ID_HEADER.into(), server_id.into())],
        prefix: AWS_PREFIX.into(),
    })
}

/// Where the signed request goes, and the region that signs it: the region of `endpoint`, an
/// AWS STS endpoint, else of the environment; without `endpoint`, the regional endpoint of the
/// environment's region, else the global one.
fn aws_endpoint(env: &Env, endpoint: Option<&str>) -> Result<(Url, String)> {
    let region = set(env, "AWS_REGION")
        .or_else(|| set(env, "AWS_DEFAULT_REGION"))
        .map(str::to_string);
    let (endpoint, region) = match (endpoint, region) {
        (Some(endpoint), region) => {
            let url = parse(endpoint)?;
            let region = (sts_region(&url).or(region)).unwrap_or_else(|| AWS_GLOBAL_REGION.into());
            (url, region)
        }
        (None, Some(region)) => (
            parse(&format!("https://sts.{region}.amazonaws.com"))?,
            region,
        ),
        (None, None) => (parse(AWS_GLOBAL_STS)?, AWS_GLOBAL_REGION.into()),
    };
    let root = endpoint.join("/").map_err(|e| failed(e.to_string()))?;
    Ok((root, region))
}

/// The region of an AWS STS endpoint: `sts.<region>.amazonaws.com`, a VPC endpoint
/// `vpce-….sts.<region>.vpce.amazonaws.com`, or the global `sts.amazonaws.com`, which signs in
/// [`AWS_GLOBAL_REGION`]. `None` for any other host.
fn sts_region(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    if !host.ends_with(".amazonaws.com") && !host.ends_with(".amazonaws.com.cn") {
        return None;
    }
    let labels: Vec<&str> = host.split('.').collect();
    let sts = labels
        .iter()
        .position(|l| *l == "sts" || *l == "sts-fips")?;
    match *labels.get(sts + 1)? {
        "amazonaws" => Some(AWS_GLOBAL_REGION.into()),
        region => Some(region.to_string()),
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
) -> Result<BearerCredentialProvider> {
    // olai-http takes https only, unless told otherwise: a plain-http token endpoint is the
    // caller's choice, such as a provider on this machine.
    let options = ClientOptions::new().with_allow_http(token_url.starts_with("http://"));
    BearerAuth::new()
        .with_client_options(options)
        .client_credentials(ClientCredentials {
            token_url: token_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            scope: scope.map(str::to_string),
            audience: audience.map(str::to_string),
        })
}

/// The token in the file `path`, read again at each refresh: a projected Kubernetes
/// service-account token, for one.
pub fn token_file(path: &str) -> BearerCredentialProvider {
    BearerAuth::new().token_file(path)
}

/// The token in the variable `var`, read again at each refresh.
pub fn token_env(var: &str) -> BearerCredentialProvider {
    BearerAuth::new().token_env(var)
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
    /// The `iss` of its tokens; `*` stands for one DNS label. For AWS, the STS endpoint, empty
    /// for AWS's own.
    pub issuer: String,
    /// The `aud` its tokens must carry; `*` alone for any.
    #[serde(default)]
    pub audiences: Vec<String>,
    /// Its tokens are taken on every request, not only at the token exchange.
    #[serde(default)]
    pub direct: bool,
}

impl Issuer {
    /// The audience to ask the provider for, for the server at `server`: the block's audience
    /// that `UC_AUDIENCE` names, else the one that is the server's URL (its scheme, host and
    /// port). The server's metadata needs no sign-in, so the client does not take any other:
    /// a server could name the audience of another server or service, and have the identity
    /// of the environment sign for it.
    fn audience(&self, env: &Env, server: &Url) -> Option<&str> {
        let mut audiences = self.audiences.iter().map(String::as_str);
        match set(env, "UC_AUDIENCE") {
            Some(wanted) => audiences.find(|audience| *audience == wanted),
            None => audiences.find(|audience| {
                Url::parse(audience).is_ok_and(|url| url.origin() == server.origin())
            }),
        }
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

/// How long the server may take to give its metadata.
const METADATA_TIMEOUT: Duration = Duration::from_secs(10);

/// The server's metadata; `None` when the server has no such endpoint. `server` is its root,
/// such as `https://uc.example.com` or `https://example.com/uc`.
pub async fn metadata(server: &Url) -> Result<Option<ProtectedResource>> {
    let mut root = server.clone();
    if !root.path().ends_with('/') {
        root.set_path(&format!("{}/", root.path()));
    }
    let url =
        (root.join(".well-known/oauth-protected-resource")).map_err(|e| failed(e.to_string()))?;
    let client = reqwest::Client::builder()
        .timeout(METADATA_TIMEOUT)
        .build()?;
    let response = client.get(url).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    Ok(Some(response.error_for_status()?.json().await?))
}

/// The signer of the environment for the server at `server`, if it has one:
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
/// The audience of an issuer is the one that `UC_AUDIENCE` names, else the server's URL: a
/// block without either is skipped.
///
/// Without any of these, `None`, and the server is not asked anything. A server without the
/// metadata endpoint gives `None` too. A server that cannot say now gives a signer that asks it
/// again at each request until it answers ([`Later`]), sending the requests without a token
/// until then: a client made while the server is away signs in once it is back.
///
/// # Errors
///
/// When the signer cannot be made.
pub async fn detect(server: &Url) -> Result<Option<Arc<dyn RequestSigner>>> {
    detect_in(&environment(), server, on_path("az"), on_path("gcloud")).await
}

/// [`detect`] with the variables `env`, and whether the Azure and Google CLIs are on the path.
pub async fn detect_in(
    env: &Env,
    server: &Url,
    az: bool,
    gcloud: bool,
) -> Result<Option<Arc<dyn RequestSigner>>> {
    if !hints(env, az, gcloud) {
        return Ok(None);
    }
    let issuers = match choose(env, server, &[], az, gcloud) {
        // A token needs no issuer, nor an OAuth client with its own audience.
        Some(
            Choice::Token(_)
            | Choice::TokenFile(_)
            | Choice::ClientCredentials {
                audience: Some(_), ..
            },
        ) => Vec::new(),
        _ => match metadata(server).await {
            Ok(resource) => resource.map(|r| r.issuers).unwrap_or_default(),
            Err(_) => return Ok(Some(Arc::new(Later::new(env, server, az, gcloud)))),
        },
    };
    match choose(env, server, &issuers, az, gcloud) {
        Some(choice) => build(choice, env, az, gcloud).map(|p| Some(signer(p))),
        None => Ok(None),
    }
}

/// The signer of [`detect`] for a server that could not say what it takes: it asks the server
/// at each request until it answers, then keeps the provider of the environment that fits.
/// Until then, and when none fits, the requests go without a token, so a request to a server
/// that is away fails as it would without an identity.
struct Later {
    env: Env,
    server: Url,
    az: bool,
    gcloud: bool,
    chosen: tokio::sync::OnceCell<Option<BearerCredentialProvider>>,
}

impl Later {
    fn new(env: &Env, server: &Url, az: bool, gcloud: bool) -> Self {
        Self {
            env: env.clone(),
            server: server.clone(),
            az,
            gcloud,
            chosen: tokio::sync::OnceCell::new(),
        }
    }

    /// The provider that fits the server's issuers; `None` while the server cannot say.
    async fn chosen(&self) -> Result<Option<&Option<BearerCredentialProvider>>> {
        if let Some(chosen) = self.chosen.get() {
            return Ok(Some(chosen));
        }
        let Ok(resource) = metadata(&self.server).await else {
            return Ok(None);
        };
        let issuers = resource.map(|r| r.issuers).unwrap_or_default();
        let choice = choose(&self.env, &self.server, &issuers, self.az, self.gcloud);
        let provider = (choice.map(|c| build(c, &self.env, self.az, self.gcloud))).transpose()?;
        Ok(Some(self.chosen.get_or_init(|| async { provider }).await))
    }
}

impl fmt::Debug for Later {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Later")
            .field("server", &self.server.as_str())
            .finish_non_exhaustive()
    }
}

impl RequestSigner for Later {
    fn sign<'a>(&'a self, request: RequestBuilder) -> BoxFuture<'a, Result<RequestBuilder>> {
        Box::pin(async move {
            match self.chosen().await? {
                Some(Some(provider)) => {
                    let credential = provider.get_credential().await?;
                    Ok(request.bearer_auth(&credential.token))
                }
                _ => Ok(request),
            }
        })
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
        /// The STS endpoint that the block names, if it names one.
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

/// The choice of [`detect`] for the variables `env`, and the `issuers` of the server at
/// `server`.
fn choose(env: &Env, server: &Url, issuers: &[Issuer], az: bool, gcloud: bool) -> Option<Choice> {
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
                (direct.iter())
                    .filter(|issuer| issuer.kind() != Kind::Aws)
                    .find_map(|issuer| issuer.audience(env, server).map(str::to_string))
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
        let Some(audience) = issuer.audience(env, server) else {
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
                // The server names no endpoint for AWS's own.
                let endpoint = issuer.issuer.trim_end_matches('/');
                let own = !endpoint.is_empty() && !endpoint.contains('*');
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

fn build(choice: Choice, env: &Env, az: bool, gcloud: bool) -> Result<BearerCredentialProvider> {
    Ok(match choice {
        Choice::Token(token) => BearerAuth::static_token(token),
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

    /// The server of the tests.
    fn server() -> Url {
        Url::parse("https://uc.example.com/").unwrap()
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
    }

    #[test]
    fn the_audience_is_the_servers_url_or_the_one_named() {
        let block = Issuer {
            audiences: vec![
                "*".into(),
                "https://management.azure.com".into(),
                "1f0a6b3c".into(),
                "https://uc.example.com:443/api".into(),
            ],
            ..issuer("x", "https://a", "", true)
        };
        let none = env(&[]);
        assert_eq!(
            block.audience(&none, &server()),
            Some("https://uc.example.com:443/api")
        );
        let other = Url::parse("http://uc.example.com").unwrap();
        assert_eq!(block.audience(&none, &other), None);
        let named = env(&[("UC_AUDIENCE", "1f0a6b3c")]);
        assert_eq!(block.audience(&named, &server()), Some("1f0a6b3c"));
        // Only one of the block's.
        let unknown = env(&[("UC_AUDIENCE", "https://graph.microsoft.com")]);
        assert_eq!(block.audience(&unknown, &server()), None);
    }

    #[test]
    fn scopes_cannot_hold_commands() {
        for scope in ["api://x&calc&rem/.default", "a|b", "%PATH%", "a b", ""] {
            assert!(azure_in(&env(&[]), scope, true).is_err(), "{scope}");
        }
        assert!(azure_in(&env(&[]), "https://uc.example.com/.default", true).is_ok());
    }

    #[test]
    fn the_environment_chooses() {
        let server = server();
        let choose = |env: &Env, issuers: &[Issuer], az: bool, gcloud: bool| {
            super::choose(env, &server, issuers, az, gcloud)
        };
        let entra = issuer(
            "entra",
            "https://login.microsoftonline.com/t/v2.0",
            "https://uc.example.com",
            true,
        );
        let google = issuer(
            "google",
            "https://accounts.google.com",
            "https://uc.example.com",
            true,
        );
        let exchange_only = issuer(
            "ferriskey",
            "http://localhost:8090/realms/uc",
            "https://uc.example.com",
            false,
        );
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
        // An OAuth client takes the audience of the first direct issuer that is not AWS's.
        let oauth = env(&[
            ("UC_OAUTH_TOKEN_URL", "https://idp/token"),
            ("UC_OAUTH_CLIENT_ID", "etl"),
            ("UC_OAUTH_CLIENT_SECRET", "s"),
        ]);
        assert!(hints(&oauth, false, false));
        let aws = Issuer {
            kind: "aws".into(),
            ..issuer("aws", "", "https://uc.example.com", true)
        };
        assert_eq!(
            choose(&oauth, &[aws.clone(), entra.clone()], false, false),
            Some(Choice::ClientCredentials {
                token_url: "https://idp/token".into(),
                client_id: "etl".into(),
                client_secret: "s".into(),
                scope: None,
                audience: Some("https://uc.example.com".into()),
            })
        );
        assert!(matches!(
            choose(&oauth, std::slice::from_ref(&aws), false, false),
            Some(Choice::ClientCredentials { audience: None, .. })
        ));
        // The first direct issuer the environment can sign in to.
        let service_principal = env(&[
            ("AZURE_TENANT_ID", "t"),
            ("AZURE_CLIENT_ID", "c"),
            ("AZURE_CLIENT_SECRET", "s"),
        ]);
        let scope = Some(Choice::Azure {
            scope: "https://uc.example.com/.default".into(),
        });
        assert_eq!(choose(&service_principal, &issuers, false, false), scope);
        assert_eq!(choose(&env(&[]), &issuers, true, false), scope);
        let key_file = env(&[("GOOGLE_APPLICATION_CREDENTIALS", "/k.json")]);
        let audience = Some(Choice::Google {
            audience: "https://uc.example.com".into(),
        });
        assert_eq!(choose(&key_file, &issuers, false, false), audience);
        assert_eq!(choose(&env(&[]), &issuers, false, true), audience);
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
        // Not without an audience to ask for: one for another service, or any.
        for audiences in [
            vec!["*".to_string()],
            vec!["https://management.azure.com".into()],
        ] {
            let other = Issuer {
                audiences,
                ..entra.clone()
            };
            assert_eq!(choose(&service_principal, &[other], false, false), None);
        }
        // Unless `UC_AUDIENCE` names it.
        let client_id = Issuer {
            audiences: vec!["1f0a6b3c".into()],
            ..entra
        };
        let mut named = service_principal.clone();
        named.insert("UC_AUDIENCE".into(), "1f0a6b3c".into());
        assert_eq!(
            choose(&named, &[client_id], false, false),
            Some(Choice::Azure {
                scope: "1f0a6b3c/.default".into()
            })
        );

        // An AWS identity at an AWS block: AWS's endpoints, or the block's own.
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
        // Never the server id of another server.
        let elsewhere = Issuer {
            audiences: vec!["https://other.example.com".into()],
            ..aws.clone()
        };
        assert_eq!(choose(&keys, &[elsewhere], false, false), None);
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
        let global = Issuer {
            issuer: "https://sts.amazonaws.com".into(),
            ..aws.clone()
        };
        assert_eq!(
            choose(
                &env(&[("AWS_REGION", "eu-west-1")]),
                &[global],
                false,
                false
            ),
            Some(Choice::Aws {
                server_id: "https://uc.example.com".into(),
                endpoint: Some("https://sts.amazonaws.com".into()),
            })
        );
        assert_eq!(choose(&service_principal, &[aws], false, false), None);
    }

    #[tokio::test]
    async fn a_role_of_the_environment_is_not_assumed_again() {
        // `AWS_ROLE_ARN` is the role of a web identity: with keys, the keys sign, and no STS
        // is asked for the role.
        let vars = env(&[
            ("AWS_ACCESS_KEY_ID", "AKIAEXAMPLE"),
            ("AWS_SECRET_ACCESS_KEY", "s"),
            ("AWS_ROLE_ARN", "arn:aws:iam::123456789012:role/etl"),
        ]);
        let provider = aws_in(&vars, "https://uc.example.com", Some("http://127.0.0.1:1")).unwrap();
        let token = provider.get_credential().await.unwrap().token.clone();
        use base64::Engine;
        let signed = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(token.strip_prefix(AWS_PREFIX).unwrap())
            .unwrap();
        let signed: serde_json::Value = serde_json::from_slice(&signed).unwrap();
        let authorization = signed["headers"]["authorization"].as_str().unwrap();
        assert!(
            authorization.contains("Credential=AKIAEXAMPLE/"),
            "{authorization}"
        );
    }

    #[test]
    fn aws_requests_are_signed_for_their_endpoint() {
        let endpoint = |vars: &[(&str, &str)], endpoint: Option<&str>| {
            let (url, region) = aws_endpoint(&env(vars), endpoint).unwrap();
            (url.to_string(), region)
        };
        let pair = |url: &str, region: &str| (url.to_string(), region.to_string());
        // AWS's own endpoints: the environment's region, else the global one.
        assert_eq!(
            endpoint(&[], None),
            pair("https://sts.amazonaws.com/", "us-east-1")
        );
        assert_eq!(
            endpoint(&[("AWS_DEFAULT_REGION", "eu-west-1")], None),
            pair("https://sts.eu-west-1.amazonaws.com/", "eu-west-1")
        );
        // The block's endpoint, in its own region.
        let eu = [("AWS_REGION", "eu-west-1")];
        assert_eq!(
            endpoint(&eu, Some("https://sts.amazonaws.com")),
            pair("https://sts.amazonaws.com/", "us-east-1")
        );
        assert_eq!(
            endpoint(&[], Some("https://sts.ap-south-1.amazonaws.com")),
            pair("https://sts.ap-south-1.amazonaws.com/", "ap-south-1")
        );
        assert_eq!(
            endpoint(
                &eu,
                Some("https://vpce-1-ab.sts.us-west-2.vpce.amazonaws.com")
            ),
            pair(
                "https://vpce-1-ab.sts.us-west-2.vpce.amazonaws.com/",
                "us-west-2"
            )
        );
        assert_eq!(
            endpoint(&[], Some("https://sts.cn-north-1.amazonaws.com.cn")).1,
            "cn-north-1"
        );
        // Another host, such as a fake STS: the environment's region.
        assert_eq!(
            endpoint(&eu, Some("http://127.0.0.1:9000")),
            pair("http://127.0.0.1:9000/", "eu-west-1")
        );
        assert_eq!(endpoint(&[], Some("http://127.0.0.1:9000")).1, "us-east-1");
    }

    #[test]
    fn a_named_managed_identity_comes_before_the_azure_cli() {
        assert_eq!(azure_identity(&env(&[]), true), AzureIdentity::Cli);
        let user_assigned = env(&[("AZURE_CLIENT_ID", "c")]);
        assert_eq!(
            azure_identity(&user_assigned, true),
            AzureIdentity::Managed {
                client: Some("c"),
                endpoint: None
            }
        );
        let app_service = env(&[("IDENTITY_ENDPOINT", "http://localhost:42356/msi/token")]);
        assert_eq!(
            azure_identity(&app_service, true),
            AzureIdentity::Managed {
                client: None,
                endpoint: Some("http://localhost:42356/msi/token")
            }
        );
        assert_eq!(
            azure_identity(&env(&[]), false),
            AzureIdentity::Managed {
                client: None,
                endpoint: None
            }
        );
        let service_principal = env(&[
            ("AZURE_TENANT_ID", "t"),
            ("AZURE_CLIENT_ID", "c"),
            ("AZURE_CLIENT_SECRET", "s"),
        ]);
        assert!(matches!(
            azure_identity(&service_principal, true),
            AzureIdentity::ClientSecret { .. }
        ));
    }
}
