//! Sign-in with the identity of the environment (the `identity` feature).
//!
//! The server takes the tokens of the identity providers of its `[[auth.issuers]]`, and says
//! which at `/.well-known/oauth-protected-resource` ([`metadata`]). A client that runs as an
//! Azure service principal or managed identity, a Google service account, or an OAuth client
//! of any provider finds its identity here, on olai-http's bearer providers, which keep and
//! refresh their tokens. [`detect`] picks a source from the environment and the server's
//! issuers; [`azure`], [`google`], [`client_credentials`], [`token_file`] and [`token_env`]
//! make one of a kind. Each is a [`TokenSource`] for
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
    /// The `iss` of its tokens; `*` stands for one DNS label.
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

    /// The provider, by its issuer.
    fn kind(&self) -> Kind {
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
///    on the path, or the machine is Google's).
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
}

/// Whether the environment has anything to sign in with.
fn hints(env: &Env, az: bool, gcloud: bool) -> bool {
    set(env, "UC_TOKEN").is_some()
        || set(env, "UC_TOKEN_FILE").is_some()
        || oauth_hints(env)
        || azure_hints(env, az)
        || google_hints(env, gcloud)
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
