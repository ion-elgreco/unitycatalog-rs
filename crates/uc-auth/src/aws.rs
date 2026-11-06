//! AWS identities, which cannot make a token that a third party checks: the
//! bearer value is a signed `sts:GetCallerIdentity` request, which the
//! resolver sends to STS. STS checks the signature and answers with the
//! caller's ARN. Vault's and the EKS authenticator's AWS sign-in work this
//! way.
//!
//! The value is the host's prefix and the request as unpadded base64url
//! JSON: `{"method", "url", "headers", "body"}`. An [`AwsProvider`] takes it
//! when:
//!
//! - the request goes to one of its STS endpoints, so the resolver sends
//!   nothing to any other host;
//! - it is `POST /` with the [`GET_CALLER_IDENTITY`] body;
//! - it signs the host's server id header, whose value is one of the
//!   provider's server ids: a request signed for another service does not
//!   pass here;
//! - it was signed less than [`SIGNED_REQUEST_TTL`] ago, as STS also checks;
//! - STS answers with the ARN of one of the provider's `accounts`.
//!
//! The caller is named by the ARN, an assumed role by its role
//! (`arn:aws:iam::<account>:role/<name>`, without the role's path), so each
//! session is not a caller of its own. The identity is kept until the
//! signed request expires.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    Allowlist, AuthError, Direct, Identity, RequestContext, Resolution, Resolver, Transport,
};

/// The body of the signed request.
pub const GET_CALLER_IDENTITY: &str = "Action=GetCallerIdentity&Version=2011-06-15";
/// How long STS takes a signed request.
pub const SIGNED_REQUEST_TTL: Duration = Duration::from_secs(15 * 60);
/// The clock skew that a signed request may have.
pub const LEEWAY: Duration = Duration::from_secs(60);
/// The STS endpoints of a provider that names none: the global one and the
/// regional ones.
pub const STS_ENDPOINTS: [&str; 2] = ["https://sts.amazonaws.com", "https://sts.*.amazonaws.com"];

/// The accounts whose identities an [`AwsResolver`] takes, the STS
/// endpoints their requests may go to, and the server ids they must sign.
#[derive(Debug, Clone)]
pub struct AwsProvider {
    /// The name of the provider, the [`Identity::provider`] of its callers.
    pub name: String,
    endpoints: Allowlist,
    server_ids: Vec<String>,
    accounts: Vec<String>,
}

impl AwsProvider {
    /// The provider `name` of the identities of `accounts` that sign one of
    /// `server_ids`, at the [`STS_ENDPOINTS`].
    pub fn new(
        name: impl Into<String>,
        server_ids: impl IntoIterator<Item = impl Into<String>>,
        accounts: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            endpoints: Allowlist::new(&STS_ENDPOINTS),
            server_ids: server_ids.into_iter().map(Into::into).collect(),
            accounts: accounts.into_iter().map(Into::into).collect(),
        }
    }

    /// Signed requests go to `endpoints` instead (origins, with `*` for one
    /// DNS label), such as a VPC endpoint or a fake STS.
    pub fn with_endpoints(mut self, endpoints: &[impl AsRef<str>]) -> Self {
        self.endpoints = Allowlist::new(endpoints);
        self
    }

    fn takes(&self, origin: &str, server_id: &str) -> bool {
        self.endpoints.is_allowed(origin) && self.server_ids.iter().any(|id| id == server_id)
    }
}

/// Takes the signed requests of AWS identities: bearer values that start
/// with its prefix.
pub struct AwsResolver {
    prefix: String,
    server_id_header: String,
    providers: Vec<AwsProvider>,
    http: reqwest::Client,
    transport: Box<dyn Transport>,
}

impl AwsResolver {
    /// Takes bearer values that start with `prefix`, whose signed request
    /// carries the server id in header `server_id_header` (lowercase).
    pub fn new(
        prefix: impl Into<String>,
        server_id_header: impl Into<String>,
        providers: Vec<AwsProvider>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            prefix: prefix.into(),
            server_id_header: server_id_header.into().to_ascii_lowercase(),
            providers,
            http,
            transport: Box::new(Direct),
        }
    }

    /// Sends the requests to STS through `transport`.
    pub fn with_transport(mut self, transport: impl Transport) -> Self {
        self.transport = Box::new(transport);
        self
    }

    /// The checks that need no call: the shape of the request, its
    /// endpoint, its server id and its age, and the provider that takes it.
    fn check(&self, token: &str) -> Result<Checked<'_>, String> {
        let encoded = token.strip_prefix(&self.prefix).ok_or("not an AWS token")?;
        let json = URL_SAFE_NO_PAD
            .decode(encoded.trim_end_matches('='))
            .map_err(|e| format!("not base64url: {e}"))?;
        let request: SignedRequest =
            serde_json::from_slice(&json).map_err(|e| format!("not a signed request: {e}"))?;
        if !request.method.eq_ignore_ascii_case("POST") || request.body != GET_CALLER_IDENTITY {
            return Err("not a GetCallerIdentity request".into());
        }
        let url: reqwest::Url = request.url.parse().map_err(|e| format!("not a URL: {e}"))?;
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
            return Err(format!("not an STS endpoint: {url}"));
        }
        let origin = url.origin().ascii_serialization();
        let headers: BTreeMap<String, String> = request
            .headers
            .into_iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value))
            .collect();
        let header = self.server_id_header.as_str();
        let server_id = headers.get(header).ok_or(format!("no {header} header"))?;
        let authorization = headers
            .get("authorization")
            .ok_or("no authorization header")?;
        if !signed_headers(authorization).any(|h| h == header) {
            return Err(format!("{header} is not signed"));
        }
        let provider = (self.providers.iter())
            .find(|p| p.takes(&origin, server_id))
            .ok_or_else(|| format!("no AWS provider takes {origin} and server id {server_id}"))?;
        let signed = headers.get("x-amz-date").ok_or("no x-amz-date header")?;
        let signed = NaiveDateTime::parse_from_str(signed, "%Y%m%dT%H%M%SZ")
            .map_err(|e| format!("x-amz-date: {e}"))?
            .and_utc();
        let age = Utc::now().signed_duration_since(signed);
        let ttl = chrono::Duration::from_std(SIGNED_REQUEST_TTL).unwrap_or_default();
        let leeway = chrono::Duration::from_std(LEEWAY).unwrap_or_default();
        if age > ttl || age < -leeway {
            return Err(format!("signed at {signed}, too long ago or ahead"));
        }
        let left = (ttl - age).to_std().unwrap_or_default();
        // Only the headers of the signature go to STS.
        let headers = headers
            .into_iter()
            .filter(|(name, _)| {
                name == "authorization"
                    || name == "content-type"
                    || name == header
                    || name.starts_with("x-amz-")
            })
            .collect();
        Ok(Checked {
            provider,
            url,
            headers,
            left,
        })
    }

    /// Send the signed request to STS. An answer of 4xx refuses the token;
    /// no answer, or one of 5xx, is [`AuthError::Unavailable`], as the
    /// caller may well be signed in.
    async fn caller_identity(&self, checked: &Checked<'_>) -> Result<CallerIdentity, AuthError> {
        let mut request = self
            .http
            .post(checked.url.clone())
            .body(GET_CALLER_IDENTITY);
        for (name, value) in &checked.headers {
            request = request.header(name, value);
        }
        let unreachable = |e: &dyn std::fmt::Display| {
            tracing::warn!(url = %checked.url, error = %e, "cannot reach STS");
            AuthError::Unavailable("Could not check the AWS identity".into())
        };
        let response = (self.transport.send(request).await).map_err(|e| unreachable(&e))?;
        let status = response.status();
        let body = response.text().await.map_err(|e| unreachable(&e))?;
        if status.is_client_error() {
            tracing::warn!(
                status = status.as_u16(),
                answer = %body.chars().take(300).collect::<String>(),
                "AWS token refused by STS"
            );
            return Err(AuthError::invalid(format!(
                "STS refused it: HTTP {}",
                status.as_u16()
            )));
        }
        if !status.is_success() {
            return Err(unreachable(&format!("HTTP {}", status.as_u16())));
        }
        parse_caller_identity(&body).ok_or_else(|| unreachable(&"no Arn and Account in the answer"))
    }
}

#[async_trait]
impl<S: Send + Sync> Resolver<S> for AwsResolver {
    async fn resolve(&self, cx: &RequestContext<'_>, _state: &S) -> Resolution {
        let Some(token) = cx.token().filter(|t| t.starts_with(&self.prefix)) else {
            return Resolution::Pass;
        };
        let refused = |reason: String| {
            tracing::warn!(%reason, "AWS token refused");
            Resolution::Refused(AuthError::Invalid(reason))
        };
        let checked = match self.check(token) {
            Ok(checked) => checked,
            Err(reason) => return refused(reason),
        };
        let identity = match self.caller_identity(&checked).await {
            Ok(identity) => identity,
            Err(e) => return Resolution::Refused(e),
        };
        let provider = checked.provider;
        if !provider.accounts.contains(&identity.account) {
            return refused(format!(
                "account {} not allowed by {}",
                identity.account, provider.name
            ));
        }
        let Some(arn) = principal_arn(&identity.arn) else {
            return refused(format!("not an ARN: {}", identity.arn));
        };
        let mut found = Identity::new(&provider.name, Identity::AWS_SIGV4, arn.clone());
        found.claims.insert("arn".into(), Value::from(arn));
        found
            .claims
            .insert("caller_arn".into(), Value::from(identity.arn));
        found
            .claims
            .insert("account".into(), Value::from(identity.account));
        found
            .claims
            .insert("user_id".into(), Value::from(identity.user_id));
        found.keep_until = Some(Instant::now() + checked.left);
        Resolution::Resolved(found)
    }

    fn keeps(&self) -> bool {
        true
    }
}

/// The signed request in a bearer value.
#[derive(Deserialize)]
struct SignedRequest {
    method: String,
    url: String,
    headers: BTreeMap<String, String>,
    body: String,
}

/// What a bearer value carries, checked before STS is asked.
struct Checked<'a> {
    provider: &'a AwsProvider,
    url: reqwest::Url,
    /// The headers sent to STS, lowercase.
    headers: BTreeMap<String, String>,
    /// Until when STS takes it.
    left: Duration,
}

/// The names in `SignedHeaders=` of a SigV4 `Authorization` header.
fn signed_headers(authorization: &str) -> impl Iterator<Item = &str> {
    authorization
        .split(',')
        .filter_map(|part| part.trim().strip_prefix("SignedHeaders="))
        .flat_map(|names| names.split(';'))
}

/// What STS says of the caller.
#[derive(Debug, PartialEq)]
struct CallerIdentity {
    arn: String,
    account: String,
    user_id: String,
}

/// The `Arn`, `Account` and `UserId` of an STS `GetCallerIdentity` answer.
fn parse_caller_identity(xml: &str) -> Option<CallerIdentity> {
    let element = |name: &str| -> Option<String> {
        let open = format!("<{name}>");
        let start = xml.find(&open)? + open.len();
        let end = start + xml[start..].find(&format!("</{name}>"))?;
        Some(xml[start..end].trim().to_string()).filter(|v| !v.is_empty())
    };
    Some(CallerIdentity {
        arn: element("Arn")?,
        account: element("Account")?,
        user_id: element("UserId").unwrap_or_default(),
    })
}

/// The ARN that names the caller: an assumed role's role
/// (`arn:aws:sts::1:assumed-role/etl/session` is `arn:aws:iam::1:role/etl`),
/// any other ARN as it is. `None` for what is not an ARN.
fn principal_arn(arn: &str) -> Option<String> {
    let mut parts = arn.splitn(6, ':');
    let (Some("arn"), Some(partition), Some(service), Some(_), Some(account), Some(resource)) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
        return None;
    };
    if partition.is_empty() || account.is_empty() || resource.is_empty() {
        return None;
    }
    match (service, resource.strip_prefix("assumed-role/")) {
        ("sts", Some(rest)) => {
            let role = rest.split('/').next().filter(|r| !r.is_empty())?;
            Some(format!("arn:{partition}:iam::{account}:role/{role}"))
        }
        _ => Some(arn.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assumed_roles_are_named_by_their_role() {
        assert_eq!(
            principal_arn("arn:aws:sts::123456789012:assumed-role/etl/i-0abc").as_deref(),
            Some("arn:aws:iam::123456789012:role/etl")
        );
        assert_eq!(
            principal_arn("arn:aws-cn:sts::123456789012:assumed-role/etl/s").as_deref(),
            Some("arn:aws-cn:iam::123456789012:role/etl")
        );
        assert_eq!(
            principal_arn("arn:aws:iam::123456789012:user/alice").as_deref(),
            Some("arn:aws:iam::123456789012:user/alice")
        );
        assert_eq!(
            principal_arn("arn:aws:sts::123456789012:assumed-role/"),
            None
        );
        assert_eq!(principal_arn("alice"), None);
    }

    #[test]
    fn the_answer_of_sts() {
        let xml = r#"<GetCallerIdentityResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
  <GetCallerIdentityResult>
    <Arn>arn:aws:iam::123456789012:user/alice</Arn>
    <UserId>AIDAEXAMPLE</UserId>
    <Account>123456789012</Account>
  </GetCallerIdentityResult>
</GetCallerIdentityResponse>"#;
        assert_eq!(
            parse_caller_identity(xml),
            Some(CallerIdentity {
                arn: "arn:aws:iam::123456789012:user/alice".into(),
                account: "123456789012".into(),
                user_id: "AIDAEXAMPLE".into(),
            })
        );
        assert_eq!(parse_caller_identity("<Arn>x</Arn>"), None);
    }

    #[test]
    fn signed_header_names() {
        let authorization = "AWS4-HMAC-SHA256 Credential=AKID/20261104/us-east-1/sts/aws4_request, \
                             SignedHeaders=content-type;host;x-amz-date;x-server-id, Signature=ab";
        assert_eq!(
            signed_headers(authorization).collect::<Vec<_>>(),
            ["content-type", "host", "x-amz-date", "x-server-id"]
        );
    }

    #[test]
    fn providers_take_their_endpoints_and_server_ids() {
        let provider = AwsProvider::new("aws", ["https://api.example.com"], ["123456789012"]);
        assert!(provider.takes("https://sts.amazonaws.com", "https://api.example.com"));
        assert!(provider.takes(
            "https://sts.eu-west-1.amazonaws.com",
            "https://api.example.com"
        ));
        assert!(!provider.takes("https://sts.amazonaws.com", "https://other.example.com"));
        assert!(!provider.takes("http://169.254.169.254", "https://api.example.com"));
        let local = provider.with_endpoints(&["http://127.0.0.1:9000"]);
        assert!(local.takes("http://127.0.0.1:9000", "https://api.example.com"));
        assert!(!local.takes("https://sts.amazonaws.com", "https://api.example.com"));
    }
}
