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
//!   nothing to any other host, and follows no redirect;
//! - it is `POST /` with the [`GET_CALLER_IDENTITY`] body;
//! - it signs the host's server id header, whose value is one of the
//!   provider's server ids: a request signed for another service does not
//!   pass here;
//! - it signs no header of the connection but `host` and `content-length`:
//!   the HTTP client writes those, so STS would not find the signature
//!   right;
//! - it signs `x-amz-date`, and its access key id is no longer than
//!   [`MAX_ACCESS_KEY_LENGTH`];
//! - it was signed less than [`SIGNED_REQUEST_TTL`] ago, as STS also checks;
//! - STS answers with the ARN of one of the provider's `accounts`.
//!
//! The first provider that takes it all decides.
//!
//! The caller is named by the ARN, an assumed role by its role
//! (`arn:aws:iam::<account>:role/<name>`, without the role's path), so each
//! session is not a caller of its own. The identity is kept until the
//! signed request expires. What STS said of a request is kept for
//! [`REFUSED_TTL`]: a request that STS refused, or whose caller no provider
//! takes, is refused that long without a call, however the value writes it,
//! and requests with one value that arrive together make one call. An HTTP
//! 4xx answer without an STS error code and without the request id of AWS
//! is of a proxy on the way, and is no refusal.
//!
//! The signature is the caller's to choose, so each value could make a call
//! to STS. At most [`MAX_STS_CALLS`] run at a time, and at most
//! [`MAX_LIMITED_STS_CALLS`] of them for the clients
//! ([`RequestContext::client`]) that had a call without a caller in the
//! current [`REFUSED_TTL`], and for each call of a client next to another
//! of its calls, so the other clients have places. After
//! [`MAX_KEY_REFUSALS`] refusals of one access key of a client in a
//! [`REFUSED_TTL`], its next requests with that key are refused without a
//! call, so one job whose credentials are over does not use up the refusals
//! of all the callers behind its address. After [`MAX_STS_REFUSALS`]
//! refusals in a [`REFUSED_TTL`], no request of that client is sent until
//! the next one starts: it gets HTTP 503 for new values then. After as many
//! calls in a row without an answer of STS, one request of that client is
//! sent at a time until STS answers one, or until the client sent nothing
//! for a [`REFUSED_TTL`]. A call whose request went away before the answer
//! is one without an answer. The other clients are
//! served, and so are the identities that are kept.

use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, Utc};
use moka::future::Cache;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::{Semaphore, SemaphorePermit};

use crate::http::refuses;
use crate::window::{Client, Every, Windows, asked_once};
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
/// regional ones. A `*` that is the label before `amazonaws.com` is a region
/// only, in these and in a provider's own patterns: other AWS services have
/// hosts of this shape (`sts.s3.amazonaws.com` is a bucket).
pub const STS_ENDPOINTS: [&str; 2] = ["https://sts.amazonaws.com", "https://sts.*.amazonaws.com"];
/// How long a refused request is refused without a call.
pub const REFUSED_TTL: Duration = Duration::from_secs(60);
/// The most requests whose answer is remembered.
const MAX_ANSWERS: u64 = 10_000;
/// The most requests of one client that STS, or the check of its answer,
/// may refuse per [`REFUSED_TTL`], so values with made-up signatures do not
/// make a call each. It is also the most of its calls in a row that may end
/// without an answer of STS before one of its calls runs at a time, so
/// values for an endpoint that does not answer do not hold the places of
/// [`MAX_STS_CALLS`]. An answer of STS ends that: an STS that was down for a
/// short time serves the client again at once.
pub const MAX_STS_REFUSALS: u32 = 100;
/// The most requests of one client with one access key that STS, or the
/// check of its answer, may refuse per [`REFUSED_TTL`]. The next ones are
/// refused without a call, and do not count for [`MAX_STS_REFUSALS`], so one
/// job whose credentials are over does not stop the other callers behind
/// its address.
pub const MAX_KEY_REFUSALS: u32 = 10;
/// The most calls to STS at a time: the others wait up to [`CALL_WAIT`], as
/// those that run may all be refused.
pub const MAX_STS_CALLS: usize = 20;
/// The most calls to STS at a time for the clients that had a call without
/// a caller in the current [`REFUSED_TTL`], and for the calls of a client
/// that has another call waiting or running. The other places are for the
/// one call of each client that had none, so clients with made-up
/// signatures, or with many values for an endpoint that does not answer, do
/// not keep the others waiting.
pub const MAX_LIMITED_STS_CALLS: usize = MAX_STS_CALLS / 2;
/// How long a call to STS waits for a place.
pub const CALL_WAIT: Duration = Duration::from_secs(5);
/// The longest access key id of a signed request, which is the longest
/// that AWS makes.
pub const MAX_ACCESS_KEY_LENGTH: usize = 128;
/// The format of the `X-Amz-Date` header.
const AMZ_DATE: &str = "%Y%m%dT%H%M%SZ";
/// The header that each answer of AWS has, also one without an error code.
const REQUEST_ID_HEADER: &str = "x-amzn-requestid";
/// The signed headers that are not sent to STS as the client wrote them:
/// the HTTP client writes them.
const NOT_FORWARDED: [&str; 2] = ["host", "content-length"];
/// The headers that go to STS also when they are not signed. Any other
/// header that is not signed is not sent: STS does not read it, so each
/// value of it would be a request of its own with one signature.
const FORWARDED_UNSIGNED: [&str; 2] = ["content-type", "x-amz-security-token"];
/// The `content-type` that is sent when the request does not sign its own,
/// so that it is one request however it writes the header.
const FORM: &str = "application/x-www-form-urlencoded; charset=utf-8";
/// The headers of the connection, which a request to STS cannot carry as
/// the client wrote them: a request that signs one is refused.
const NOT_SIGNED: [&str; 9] = [
    "connection",
    "expect",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

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
        let endpoint = match self.endpoints.matching(origin) {
            None => false,
            Some(None) => true,
            Some(Some(pattern)) => !other_aws_service(pattern, origin),
        };
        endpoint && self.server_ids.iter().any(|id| id == server_id)
    }
}

/// Whether `origin`, which `pattern` matches, is the host of another AWS
/// service: the pattern has a `*` as the label before `amazonaws.com` or
/// `amazonaws.com.cn`, and the origin has no region there.
fn other_aws_service(pattern: &str, origin: &str) -> bool {
    fn labels(url: &str) -> Vec<&str> {
        let host = url.split_once("://").map_or(url, |(_, rest)| rest);
        let host = host.split([':', '/']).next().unwrap_or_default();
        host.split('.').collect()
    }
    let (names, origin) = (labels(pattern), labels(origin));
    let fixed = match names.as_slice() {
        [.., "amazonaws", "com", "cn"] => 3,
        [.., "amazonaws", "com"] => 2,
        _ => return false,
    };
    let Some(at) = names.len().checked_sub(fixed + 1) else {
        return false;
    };
    names[at] == "*" && !origin.get(at).is_some_and(|label| is_region(label))
}

/// Whether `label` has the shape of an AWS region: lowercase words and then
/// a number, with `-` between (`eu-west-1`, `us-gov-west-1`).
fn is_region(label: &str) -> bool {
    let mut parts: Vec<&str> = label.split('-').collect();
    let number = parts.pop().unwrap_or_default();
    parts.len() >= 2
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && (parts.iter()).all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase()))
}

/// Takes the signed requests of AWS identities: bearer values that start
/// with its prefix.
pub struct AwsResolver {
    prefix: String,
    server_id_header: String,
    providers: Vec<AwsProvider>,
    http: reqwest::Client,
    transport: Box<dyn Transport>,
    /// What STS said of a request, by the hash of what is sent to STS
    /// ([`Checked::key`]): its caller, or why it was refused.
    answers: Cache<[u8; 32], Result<Named, String>>,
    calls: StsCalls,
}

/// The limits on the calls to STS.
struct StsCalls {
    /// A place for each call that runs.
    all: Semaphore,
    /// A place for each call that runs for a client that had a call without
    /// a caller.
    limited: Semaphore,
    /// The calls of each client without a caller in the current
    /// [`REFUSED_TTL`].
    refusals: Windows<Client, Refusals>,
    /// The calls of each client in a row without an answer of STS. They
    /// are kept while the client's requests come, so a new [`REFUSED_TTL`]
    /// does not end the row: calls that hang end too slowly to reach the
    /// limit in one.
    unanswered: Windows<Client, Unanswered>,
    /// The refusals of each access key of a client in the current
    /// [`REFUSED_TTL`].
    keys: Windows<(Client, String), u32>,
    /// When the log last said that no place was free in time.
    busy: Every,
}

/// The places that a call to STS holds while it runs.
struct Place<'a> {
    _limited: Option<SemaphorePermit<'a>>,
    _call: SemaphorePermit<'a>,
    _probe: Option<Probe<'a>>,
    _entered: Entered<'a>,
}

/// The calls of a client that STS, or the check of its answer, refused.
#[derive(Default)]
struct Refusals {
    refused: u32,
    /// Whether the log says that the client is over a limit.
    logged: bool,
}

/// The calls of a client that STS did not answer, since its last answer,
/// and those that wait or run now.
#[derive(Default)]
struct Unanswered {
    calls: u32,
    /// The calls that wait for a place or run.
    entered: u32,
    /// Whether a call of the client runs while it is over the limit of
    /// calls without an answer.
    probing: bool,
}

/// What the current [`REFUSED_TTL`] says of a client.
enum Standing {
    /// Each of its calls named a caller.
    Clean,
    /// Some of its calls named no caller.
    WithoutCaller,
    /// STS answered none of its last [`MAX_STS_REFUSALS`] calls, and none
    /// of its calls runs now: one may.
    Probe,
    /// None of its requests goes to STS now.
    TooOften,
}

/// The one call of a client that is over the limit of calls without an
/// answer. The next one may run when this one ended.
struct Probe<'a> {
    unanswered: &'a Windows<Client, Unanswered>,
    client: &'a Client,
}

impl Drop for Probe<'_> {
    fn drop(&mut self) {
        (self.unanswered).with(self.client, |unanswered, _| unanswered.probing = false);
    }
}

/// A call of a client that waits for a place or runs.
struct Entered<'a> {
    unanswered: &'a Windows<Client, Unanswered>,
    client: &'a Client,
}

impl Drop for Entered<'_> {
    fn drop(&mut self) {
        (self.unanswered).with(self.client, |unanswered, _| {
            unanswered.entered = unanswered.entered.saturating_sub(1);
        });
    }
}

/// Why a request has no answer of STS to keep.
#[derive(Clone)]
enum NoAnswer {
    /// A limit of the client that asked. A request of another client with
    /// the same value asks for itself.
    Limit(AuthError),
    /// STS gave no answer, or one that may change.
    Sts(AuthError),
}

/// A call to STS that runs for an access key of a client. One that is
/// dropped before it ended is counted as a call without an answer: its
/// request went away, as after a timeout of the host.
struct Running<'a> {
    calls: &'a StsCalls,
    key: &'a (Client, String),
    ended: bool,
}

impl Running<'_> {
    /// The call ended with `result`.
    fn end<T>(&mut self, result: &Result<T, StsError>) {
        self.ended = true;
        self.calls.ended(self.key, result);
    }
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        if !self.ended {
            let unanswered = StsError::Unavailable(String::new());
            self.calls.ended::<()>(self.key, &Err(unanswered));
        }
    }
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
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default();
        Self {
            prefix: prefix.into(),
            server_id_header: server_id_header.into().to_ascii_lowercase(),
            providers,
            http,
            transport: Box::new(Direct),
            answers: Cache::builder()
                .max_capacity(MAX_ANSWERS)
                .time_to_live(REFUSED_TTL)
                .build(),
            calls: StsCalls {
                all: Semaphore::new(MAX_STS_CALLS),
                limited: Semaphore::new(MAX_LIMITED_STS_CALLS),
                refusals: Windows::new(REFUSED_TTL),
                unanswered: Windows::since_last(REFUSED_TTL),
                keys: Windows::new(REFUSED_TTL),
                busy: Every::new(REFUSED_TTL),
            },
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
        let plain = url.username().is_empty() && url.password().is_none();
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() || !plain {
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
        let authorization =
            Authorization::parse(authorization).ok_or("not a Signature Version 4 authorization")?;
        let signed_names: HashSet<&str> = authorization.signed_headers.split(';').collect();
        let signed = |name: &str| signed_names.contains(name);
        if !signed(header) {
            return Err(format!("{header} is not signed"));
        }
        if !signed("x-amz-date") {
            return Err("x-amz-date is not signed".into());
        }
        let access_key = authorization.credential.split('/').next();
        let access_key = access_key.unwrap_or_default();
        if access_key.len() > MAX_ACCESS_KEY_LENGTH {
            return Err("the access key id is too long".into());
        }
        if let Some(name) = NOT_SIGNED.iter().find(|name| signed(name)) {
            return Err(format!(
                "{name} is signed, and is not sent to STS as it is written"
            ));
        }
        let providers: Vec<&AwsProvider> = (self.providers.iter())
            .filter(|p| p.takes(&origin, server_id))
            .collect();
        if providers.is_empty() {
            return Err(format!(
                "no AWS provider takes {origin} and server id {server_id}"
            ));
        }
        let date = headers.get("x-amz-date").ok_or("no x-amz-date header")?;
        let date = NaiveDateTime::parse_from_str(date, AMZ_DATE)
            .map_err(|e| format!("x-amz-date: {e}"))?
            .and_utc();
        let age = Utc::now().signed_duration_since(date);
        let ttl = chrono::Duration::from_std(SIGNED_REQUEST_TTL).unwrap_or_default();
        let leeway = chrono::Duration::from_std(LEEWAY).unwrap_or_default();
        if age > ttl || age < -leeway {
            return Err(format!("signed at {date}, too long ago or ahead"));
        }
        let left = (ttl - age).to_std().unwrap_or_default();
        // The headers that go to STS are the authorization, the signed ones
        // (but [`NOT_FORWARDED`]), and [`FORWARDED_UNSIGNED`]. They are
        // written in one way: the authorization from its parts, a
        // `content-type` that is not signed as [`FORM`], the others with
        // single spaces, as the signature reads them. The hash is of the
        // URL and these headers, in the order of their names.
        let forwarded = |name: &str| {
            name == "authorization"
                || FORWARDED_UNSIGNED.contains(&name)
                || (signed(name) && !NOT_FORWARDED.contains(&name))
        };
        let mut sent = HeaderMap::new();
        let mut hash = Sha256::new();
        hash.update(url.as_str());
        for (name, value) in &headers {
            if !forwarded(name) {
                continue;
            }
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| format!("not a header name: {name:?}"))?;
            let value = match name.as_str() {
                "authorization" => authorization.to_string(),
                "content-type" if !signed(name) => FORM.to_string(),
                _ => single_spaces(value),
            };
            let value = HeaderValue::from_str(&value)
                .map_err(|_| format!("not a value of header {name}"))?;
            for part in [name.as_bytes(), value.as_bytes()] {
                hash.update(b"\n");
                hash.update(part);
            }
            sent.insert(header_name, value);
        }
        Ok(Checked {
            providers,
            access_key: access_key.to_string(),
            url,
            headers: sent,
            key: hash.finalize().into(),
            left,
        })
    }

    /// Send the signed request to STS, which names the caller or refuses
    /// the request, in XML or, for a request that signs such an `accept`,
    /// in JSON. No answer, one of 5xx, STS's throttling (HTTP 400 or
    /// 429), or a proxy's own answer (HTTP 407 or 408, or another 4xx
    /// without an STS error code and without the request id of AWS) is
    /// [`StsError::Unavailable`], as the caller may well be signed in. A
    /// request that is too large (HTTP 413 or 431) is refused, whoever says
    /// so.
    async fn caller_identity(&self, checked: &Checked<'_>) -> Result<CallerIdentity, StsError> {
        let request = (self.http.post(checked.url.clone()))
            .headers(checked.headers.clone())
            .body(GET_CALLER_IDENTITY);
        let unreachable = |e: &dyn std::fmt::Display| {
            tracing::warn!(url = %checked.url, error = %e, "cannot reach STS");
            StsError::Unavailable("Could not check the AWS identity".into())
        };
        let response = (self.transport.send(request).await).map_err(|e| unreachable(&e))?;
        let status = response.status();
        let from_aws = response.headers().contains_key(REQUEST_ID_HEADER);
        let body = response.text().await.map_err(|e| unreachable(&e))?;
        if throttled(&body) {
            return Err(unreachable(&"STS throttles the requests"));
        }
        if refuses(status) {
            let code = element(&body, "Code");
            let too_large = status == reqwest::StatusCode::PAYLOAD_TOO_LARGE
                || status == reqwest::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE;
            if code.is_none() && !from_aws && !too_large {
                return Err(unreachable(&format!(
                    "HTTP {} without an STS error code",
                    status.as_u16()
                )));
            }
            let code = code.unwrap_or_default();
            tracing::warn!(
                status = status.as_u16(),
                %code,
                answer = %body.chars().take(300).collect::<String>(),
                "AWS token refused by STS"
            );
            let reason = format!("STS refused it: HTTP {}", status.as_u16());
            return Err(match code == NOT_YET_KNOWN {
                true => StsError::NotYet(reason),
                false => StsError::Refused(reason),
            });
        }
        if !status.is_success() {
            return Err(unreachable(&format!("HTTP {}", status.as_u16())));
        }
        parse_caller_identity(&body).ok_or_else(|| unreachable(&"no Arn and Account in the answer"))
    }

    /// Ask STS for the caller of a request of `client` that has no answer
    /// yet. `Ok` is an answer to keep: the caller, or why the request is
    /// refused.
    async fn ask(
        &self,
        checked: &Checked<'_>,
        client: &Client,
    ) -> Result<Result<Named, String>, NoAnswer> {
        let key = (client.clone(), checked.access_key.clone());
        let refused = || {
            NoAnswer::Limit(AuthError::Invalid(
                "STS refused the last requests of the access key, try again later".into(),
            ))
        };
        if self.calls.refuses(&key) {
            return Err(refused());
        }
        let _place = (self.calls.enter(client).await).map_err(NoAnswer::Limit)?;
        // The calls that this one waited for may have been the key's.
        if self.calls.refuses(&key) {
            return Err(refused());
        }
        let mut running = Running {
            calls: &self.calls,
            key: &key,
            ended: false,
        };
        let identity = self.caller_identity(checked).await;
        let named = identity.and_then(|identity| checked.named(identity));
        running.end(&named);
        match named {
            Ok(named) => Ok(Ok(named)),
            Err(StsError::Refused(reason)) => Ok(Err(reason)),
            // Not an answer to keep: STS may take the request later.
            Err(StsError::NotYet(reason)) => Err(NoAnswer::Sts(AuthError::Invalid(reason))),
            Err(StsError::Unavailable(reason)) => {
                Err(NoAnswer::Sts(AuthError::Unavailable(reason)))
            }
        }
    }
}

impl Checked<'_> {
    /// The caller that STS answered with, as the providers name it: of the
    /// providers of the endpoint and server id, the first that takes the
    /// account decides.
    fn named(&self, identity: CallerIdentity) -> Result<Named, StsError> {
        let refuse = |reason: String| {
            tracing::warn!(%reason, "AWS token refused");
            Err(StsError::Refused(reason))
        };
        let Some(provider) =
            (self.providers.iter()).find(|p| p.accounts.contains(&identity.account))
        else {
            let names: Vec<&str> = self.providers.iter().map(|p| p.name.as_str()).collect();
            return refuse(format!(
                "account {} not allowed by {}",
                identity.account,
                names.join(", ")
            ));
        };
        let Some(arn) = principal_arn(&identity.arn) else {
            return refuse(format!("not an ARN: {}", identity.arn));
        };
        Ok(Named {
            provider: provider.name.clone(),
            arn,
            identity,
        })
    }
}

impl StsCalls {
    /// Take the places of a call of `client`, after a wait of [`CALL_WAIT`]
    /// at most. An error when none of the client's requests goes to STS
    /// now, or when no place was free in time.
    async fn enter<'a>(&'a self, client: &'a Client) -> Result<Place<'a>, AuthError> {
        let too_often = || {
            AuthError::Unavailable("Too many AWS tokens without a caller, try again later".into())
        };
        // Counted before the wait, so the calls of one client that arrive
        // together do not all count as its first.
        let first = self.unanswered.with(client, |unanswered, _| {
            unanswered.entered += 1;
            unanswered.entered == 1
        });
        let entered = Entered {
            unanswered: &self.unanswered,
            client,
        };
        let limited = match self.standing(client, false) {
            Standing::TooOften => return Err(too_often()),
            Standing::Clean => !first,
            Standing::WithoutCaller | Standing::Probe => true,
        };
        let places = async {
            let limited = match limited {
                true => Some(self.limited.acquire().await?),
                false => None,
            };
            Ok::<_, tokio::sync::AcquireError>((limited, self.all.acquire().await?))
        };
        let Ok(Ok((limited, call))) = tokio::time::timeout(CALL_WAIT, places).await else {
            if self.busy.due() {
                tracing::warn!(
                    "no call to STS ended in time; AWS tokens are not checked while it is so"
                );
            }
            return Err(AuthError::Unavailable(
                "STS is busy, try again later".into(),
            ));
        };
        // The calls that this one waited for may have been the client's.
        let probe = match self.standing(client, true) {
            Standing::TooOften => return Err(too_often()),
            Standing::Probe => Some(Probe {
                unanswered: &self.unanswered,
                client,
            }),
            Standing::Clean | Standing::WithoutCaller => None,
        };
        Ok(Place {
            _limited: limited,
            _call: call,
            _probe: probe,
            _entered: entered,
        })
    }

    /// What the refusals of the current [`REFUSED_TTL`] and the calls
    /// without an answer say of `client`. With `claim`, a
    /// [`Standing::Probe`] is taken: the caller makes the one call, and
    /// holds a [`Probe`] for it. The log says once per client and
    /// [`REFUSED_TTL`] that the client is over a limit.
    fn standing(&self, client: &Client, claim: bool) -> Standing {
        let refused = (self.refusals).read(client, |refusals| refusals.refused);
        let refused = refused.unwrap_or(0);
        let standing = self.unanswered.with(client, |unanswered, _| {
            let over = unanswered.calls >= MAX_STS_REFUSALS;
            if refused >= MAX_STS_REFUSALS || (over && unanswered.probing) {
                Standing::TooOften
            } else if over {
                unanswered.probing = claim;
                Standing::Probe
            } else if refused > 0 || unanswered.calls > 0 {
                Standing::WithoutCaller
            } else {
                Standing::Clean
            }
        });
        let over = matches!(standing, Standing::TooOften | Standing::Probe);
        let log = over
            && (self.refusals).with(client, |refusals, _| {
                !std::mem::replace(&mut refusals.logged, true)
            });
        if log {
            tracing::warn!(
                ?client,
                "too many AWS tokens without a caller; the client's go to STS one at a time or \
                 not at all for now"
            );
        }
        standing
    }

    /// Whether STS refused [`MAX_KEY_REFUSALS`] requests of the client and
    /// access key `key` in the current [`REFUSED_TTL`].
    fn refuses(&self, key: &(Client, String)) -> bool {
        let refused = self.keys.read(key, |refused| *refused);
        refused.is_some_and(|refused| refused >= MAX_KEY_REFUSALS)
    }

    /// Count a call of the client and access key `key` that ended with
    /// `result`. An answer of STS lets the calls without an answer start
    /// anew.
    fn ended<T>(&self, key: &(Client, String), result: &Result<T, StsError>) {
        let refused = matches!(result, Err(StsError::Refused(_) | StsError::NotYet(_)));
        let over = refused
            && self.keys.with(key, |refused, _| {
                *refused += 1;
                *refused == MAX_KEY_REFUSALS
            });
        if over {
            tracing::warn!(
                client = ?key.0,
                access_key = %key.1,
                "STS refused too many requests of the access key; its next ones from the client \
                 are refused without a call for now"
            );
        }
        if refused {
            (self.refusals).with(&key.0, |refusals, _| refusals.refused += 1);
        }
        let answered = !matches!(result, Err(StsError::Unavailable(_)));
        self.unanswered.with(&key.0, |unanswered, _| {
            unanswered.calls = match answered {
                true => 0,
                false => unanswered.calls.saturating_add(1),
            };
        });
    }
}

#[async_trait]
impl<S: Send + Sync> Resolver<S> for AwsResolver {
    async fn resolve(&self, cx: &RequestContext<'_>, _state: &S) -> Resolution {
        let Some(token) = cx.token().filter(|t| t.starts_with(&self.prefix)) else {
            return Resolution::Pass;
        };
        let checked = match self.check(token) {
            Ok(checked) => checked,
            Err(reason) => {
                // The reason may hold text of the token, such as its server
                // id: the log has it with control characters escaped.
                tracing::warn!(?reason, "AWS token refused");
                return Resolution::Refused(AuthError::Invalid(reason));
            }
        };
        // One call for the requests with one value that arrive together.
        // A request that waited for the one of another client, which ended
        // at a limit of that client, asks again.
        let client = Client::of(cx.client());
        let ask = || self.ask(&checked, &client);
        let limit = |unanswered: &NoAnswer| matches!(unanswered, NoAnswer::Limit(_));
        let answer = match asked_once(&self.answers, &checked.key, ask, limit).await {
            Ok(answer) => answer,
            Err(unanswered) => {
                let (NoAnswer::Limit(e) | NoAnswer::Sts(e)) = &*unanswered;
                return Resolution::Refused(e.clone());
            }
        };
        let Named {
            provider,
            arn,
            identity,
        } = match answer {
            Ok(named) => named,
            Err(reason) => return Resolution::Refused(AuthError::Invalid(reason)),
        };
        let mut found = Identity::new(&provider, Identity::AWS_SIGV4, arn.clone());
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
    /// The providers of its endpoint and server id, one or more.
    providers: Vec<&'a AwsProvider>,
    /// The id of the access key that signed it, as the value says.
    access_key: String,
    url: reqwest::Url,
    /// The headers sent to STS.
    headers: HeaderMap,
    /// The hash of the URL and the headers sent to STS.
    key: [u8; 32],
    /// How long STS still takes it.
    left: Duration,
}

/// The caller of a request, as STS and the providers name it.
#[derive(Clone)]
struct Named {
    /// The name of the provider that takes the caller's account.
    provider: String,
    /// [`principal_arn`] of the caller.
    arn: String,
    identity: CallerIdentity,
}

/// Why a call to STS named no caller.
enum StsError {
    /// STS, or the check of its answer, refused the request, and will
    /// refuse it again.
    Refused(String),
    /// STS refused the request, and may take it later.
    NotYet(String),
    /// STS gave no answer on the request.
    Unavailable(String),
}

/// The parts of a SigV4 `Authorization` header.
struct Authorization<'a> {
    credential: &'a str,
    signed_headers: &'a str,
    signature: &'a str,
}

impl<'a> Authorization<'a> {
    /// `None` unless the header is `AWS4-HMAC-SHA256` and one `Credential`,
    /// `SignedHeaders` and `Signature`, each once. A value with a space or
    /// a `=` could hold a second part that STS reads in another way.
    fn parse(authorization: &'a str) -> Option<Self> {
        let parts = authorization.strip_prefix("AWS4-HMAC-SHA256 ")?;
        let mut credential = None;
        let mut signed = None;
        let mut signature = None;
        for part in parts.split(',') {
            let (name, value) = part.trim().split_once('=')?;
            let slot = match name {
                "Credential" => &mut credential,
                "SignedHeaders" => &mut signed,
                "Signature" => &mut signature,
                _ => return None,
            };
            let plain =
                !value.is_empty() && !value.bytes().any(|b| b == b'=' || b.is_ascii_whitespace());
            if slot.replace(value).is_some() || !plain {
                return None;
            }
        }
        Some(Self {
            credential: credential?,
            signed_headers: signed?,
            signature: signature?,
        })
    }
}

/// The header as it is sent to STS, the same for each way to write it.
impl std::fmt::Display for Authorization<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AWS4-HMAC-SHA256 Credential={}, SignedHeaders={}, Signature={}",
            self.credential, self.signed_headers, self.signature
        )
    }
}

/// A header value without spaces at its ends and with one space for each
/// run of them, which is what its signature is of. The tabs at its ends go
/// too: HTTP does not carry them to STS.
fn single_spaces(value: &str) -> String {
    let value = value.trim_matches([' ', '\t']);
    let words: Vec<&str> = value.split(' ').filter(|w| !w.is_empty()).collect();
    words.join(" ")
}

/// Whether the answer of STS is its throttling (`Throttling: Rate
/// exceeded`), which comes as HTTP 400.
fn throttled(answer: &str) -> bool {
    matches!(
        element(answer, "Code").as_deref(),
        Some("Throttling" | "ThrottlingException" | "RequestLimitExceeded")
    )
}

/// The error code of STS for an access key that it does not know, which it
/// also says of a new key for some seconds.
const NOT_YET_KNOWN: &str = "InvalidClientTokenId";

/// The text of the first element `name` of an STS answer in XML, or of the
/// first member `name` of one in JSON.
fn element(answer: &str, name: &str) -> Option<String> {
    let text = match answer.trim_start().starts_with('{') {
        true => member(&serde_json::from_str(answer).ok()?, name)?,
        false => {
            let open = format!("<{name}>");
            let start = answer.find(&open)? + open.len();
            let end = start + answer[start..].find(&format!("</{name}>"))?;
            answer[start..end].to_string()
        }
    };
    Some(text.trim().to_string()).filter(|v| !v.is_empty())
}

/// The first text member `name` of a JSON value, at any depth.
fn member(json: &Value, name: &str) -> Option<String> {
    let object = json.as_object()?;
    match object.get(name).and_then(Value::as_str) {
        Some(text) => Some(text.to_string()),
        None => object.values().find_map(|value| member(value, name)),
    }
}

/// What STS says of the caller.
#[derive(Debug, Clone, PartialEq)]
struct CallerIdentity {
    arn: String,
    account: String,
    user_id: String,
}

/// The `Arn`, `Account` and `UserId` of an STS `GetCallerIdentity` answer.
fn parse_caller_identity(answer: &str) -> Option<CallerIdentity> {
    Some(CallerIdentity {
        arn: element(answer, "Arn")?,
        account: element(answer, "Account")?,
        user_id: element(answer, "UserId").unwrap_or_default(),
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
    fn the_answer_of_sts_in_json() {
        let json = r#"{"GetCallerIdentityResponse": {"GetCallerIdentityResult": {
            "Account": "123456789012", "Arn": "arn:aws:iam::123456789012:user/alice",
            "UserId": "AIDAEXAMPLE"}, "ResponseMetadata": {"RequestId": "r1"}}}"#;
        let identity = parse_caller_identity(json).unwrap();
        assert_eq!(identity.arn, "arn:aws:iam::123456789012:user/alice");
        assert_eq!(identity.account, "123456789012");
        assert_eq!(identity.user_id, "AIDAEXAMPLE");
        let error = |code: &str| {
            format!(r#"{{"Error": {{"Code": "{code}", "Type": "Sender"}}, "RequestId": "r1"}}"#)
        };
        assert!(throttled(&error("Throttling")));
        let code = element(&error(NOT_YET_KNOWN), "Code");
        assert_eq!(code.as_deref(), Some(NOT_YET_KNOWN));
        assert_eq!(parse_caller_identity(r#"{"Arn": 1}"#), None);
        assert_eq!(parse_caller_identity("{"), None);
    }

    #[test]
    fn signed_header_names() {
        let authorization = "AWS4-HMAC-SHA256 Credential=AKID/20261104/us-east-1/sts/aws4_request, \
                             SignedHeaders=content-type;host;x-amz-date;x-server-id, Signature=ab";
        let signed = |a| Authorization::parse(a).map(|a| a.signed_headers);
        assert_eq!(
            signed(authorization),
            Some("content-type;host;x-amz-date;x-server-id")
        );
        // A second component, or one that STS does not read, is no
        // signature.
        for forged in [
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=host, Signature=ab, \
             SignedHeaders=x-server-id",
            "AWS4-HMAC-SHA256 SignedHeaders=x-server-id, Credential=A/x, SignedHeaders=host, \
             Signature=ab",
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=host, Signature=ab, Extra=x-server-id",
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=host",
            "Bearer SignedHeaders=x-server-id",
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=, Signature=ab",
            // A part inside the value of another.
            "AWS4-HMAC-SHA256 Credential=A/x SignedHeaders=host, SignedHeaders=x-server-id, \
             Signature=ab",
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=host;Signature=cd, Signature=ab",
        ] {
            assert_eq!(signed(forged), None, "{forged}");
        }
    }

    #[test]
    fn throttling_is_no_refusal() {
        let answer = |code: &str| {
            format!(
                "<ErrorResponse><Error><Type>Sender</Type><Code>{code}</Code>\
                 <Message>Rate exceeded</Message></Error></ErrorResponse>"
            )
        };
        assert!(throttled(&answer("Throttling")));
        assert!(!throttled(&answer("InvalidClientTokenId")));
        assert!(!throttled("not xml"));
    }

    /// A bearer value with the server id `api` in header `server_id`, signed
    /// now with the access key `A`.
    fn token(server_id: &str, extra: Option<&str>, signature: &str, padding: &str) -> String {
        token_of("A", server_id, extra, signature, padding)
    }

    /// As [`token`], signed with `access_key`.
    fn token_of(
        access_key: &str,
        server_id: &str,
        extra: Option<&str>,
        signature: &str,
        padding: &str,
    ) -> String {
        let date = Utc::now().format(AMZ_DATE).to_string();
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={access_key}/x, \
             SignedHeaders=host;x-amz-date;x-server-id, Signature={signature}"
        );
        let mut request = serde_json::json!({
            "method": "POST",
            "url": "https://sts.amazonaws.com/",
            "body": GET_CALLER_IDENTITY,
            "headers": { server_id: "api", "X-Amz-Date": date, "Authorization": authorization },
        });
        if let Some(extra) = extra {
            request["headers"]["Accept"] = extra.into();
        }
        let json = URL_SAFE_NO_PAD.encode(request.to_string());
        format!("aws:{json}{padding}")
    }

    /// Answers every request as it is set to, and counts them.
    #[derive(Clone, Default)]
    struct Answers {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        answer: std::sync::Arc<std::sync::Mutex<(u16, bool, &'static str)>>,
    }

    impl Answers {
        /// The next answers: the status, whether with the request id of
        /// AWS, and the body.
        fn set(&self, status: u16, from_aws: bool, body: &'static str) {
            *self.answer.lock().unwrap() = (status, from_aws, body);
        }

        fn calls(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Transport for Answers {
        async fn send(&self, _: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::task::yield_now().await;
            let (status, from_aws, body) = *self.answer.lock().unwrap();
            let mut response = axum::http::Response::builder().status(status);
            if from_aws {
                response = response.header(REQUEST_ID_HEADER, "r1");
            }
            Ok(response.body(body).unwrap().into())
        }
    }

    fn resolver(sts: &Answers) -> AwsResolver {
        let provider = AwsProvider::new("aws", ["api"], ["123456789012"]);
        AwsResolver::new("aws:", "X-Server-Id", vec![provider]).with_transport(sts.clone())
    }

    /// The HTTP status of the answer to a request with the value of
    /// `signature`: 200 for a caller, 401 for a refused one, 503 for one
    /// that was not checked.
    async fn status(resolver: &AwsResolver, signature: &str) -> u16 {
        status_of(resolver, signature, None).await
    }

    /// As [`status`], for a request from the address `peer`. Each value has
    /// an access key of its own.
    async fn status_of(resolver: &AwsResolver, signature: &str, peer: Option<&str>) -> u16 {
        status_with_key(resolver, &format!("K{signature}"), signature, peer).await
    }

    /// As [`status_of`], for a value signed with `access_key`.
    async fn status_with_key(
        resolver: &AwsResolver,
        access_key: &str,
        signature: &str,
        peer: Option<&str>,
    ) -> u16 {
        let (mut parts, ()) = axum::http::Request::new(()).into_parts();
        if let Some(peer) = peer {
            let peer: std::net::SocketAddr = format!("{peer}:1").parse().unwrap();
            parts.extensions.insert(axum::extract::ConnectInfo(peer));
        }
        let token = token_of(access_key, "x-server-id", None, signature, "");
        let cx = RequestContext::new(&parts, Some(token));
        match Resolver::<()>::resolve(resolver, &cx, &()).await {
            Resolution::Resolved(_) => 200,
            Resolution::Refused(AuthError::Invalid(_)) => 401,
            Resolution::Refused(AuthError::Unavailable(_)) => 503,
            _ => 0,
        }
    }

    const REFUSAL: &str =
        "<ErrorResponse><Error><Code>SignatureDoesNotMatch</Code></Error></ErrorResponse>";
    const CALLER: &str = "<Arn>arn:aws:iam::123456789012:user/alice</Arn>\
                          <Account>123456789012</Account>";

    #[tokio::test]
    async fn the_refusals_of_one_client_leave_the_others_served() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        sts.set(403, true, REFUSAL);
        for n in 0..MAX_STS_REFUSALS {
            let signature = format!("d{n}");
            assert_eq!(
                status_of(&resolver, &signature, Some("192.0.2.1")).await,
                401
            );
        }
        assert_eq!(status_of(&resolver, "e1", Some("192.0.2.1")).await, 503);
        sts.set(200, true, CALLER);
        assert_eq!(status_of(&resolver, "e1", Some("192.0.2.1")).await, 503);
        assert_eq!(status_of(&resolver, "e1", Some("192.0.2.2")).await, 200);
        assert_eq!(sts.calls(), 1 + MAX_STS_REFUSALS as usize);
    }

    /// Answers no request.
    struct Hangs;

    #[async_trait]
    impl Transport for Hangs {
        async fn send(&self, _: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
            std::future::pending().await
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_request_that_goes_away_is_a_call_without_an_answer() {
        let provider = AwsProvider::new("aws", ["api"], ["123456789012"]);
        let resolver =
            AwsResolver::new("aws:", "X-Server-Id", vec![provider]).with_transport(Hangs);
        let request = status(&resolver, "k1");
        assert!(
            tokio::time::timeout(Duration::from_secs(1), request)
                .await
                .is_err()
        );
        let unanswered = |resolver: &AwsResolver| {
            (resolver.calls.unanswered).with(&Client::of(None), |unanswered, _| unanswered.calls)
        };
        assert_eq!(unanswered(&resolver), 1);
        assert_eq!(resolver.calls.all.available_permits(), MAX_STS_CALLS);
    }

    #[tokio::test]
    async fn the_refusals_of_one_access_key_leave_the_others_of_the_client_served() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        let peer = Some("192.0.2.1");
        // A job with credentials that are over signs each request anew.
        sts.set(403, true, REFUSAL);
        for n in 0..2 * MAX_STS_REFUSALS {
            let signature = format!("l{n}");
            assert_eq!(
                status_with_key(&resolver, "OVER", &signature, peer).await,
                401
            );
        }
        assert_eq!(sts.calls(), MAX_KEY_REFUSALS as usize);
        // Another job behind the same address.
        sts.set(200, true, CALLER);
        assert_eq!(status_with_key(&resolver, "OTHER", "m1", peer).await, 200);
        assert_eq!(status_with_key(&resolver, "OVER", "m2", peer).await, 401);
        assert_eq!(sts.calls(), 1 + MAX_KEY_REFUSALS as usize);
    }

    #[test]
    fn a_signed_header_of_the_connection_is_refused() {
        let resolver = resolver(&Answers::default());
        for (name, value) in [
            ("TE", "trailers"),
            ("Connection", "close"),
            ("Expect", "100-continue"),
        ] {
            let signed = format!("host;{};x-amz-date;x-server-id", name.to_ascii_lowercase());
            let token = token_with(&signed, &[("Host", "sts.amazonaws.com"), (name, value)]);
            let refused = resolver.check(&token).map(|_| ()).unwrap_err();
            assert!(refused.contains("is not sent to STS"), "{name}: {refused}");
        }
    }

    #[tokio::test]
    async fn requests_with_one_value_make_one_call() {
        for (status, answer, expected) in [(200, CALLER, 200), (403, REFUSAL, 401)] {
            let sts = Answers::default();
            let resolver = resolver(&sts);
            sts.set(status, true, answer);
            // All are in flight before the first answer of STS.
            let (a, b, c, d) = tokio::join!(
                self::status(&resolver, "f1"),
                self::status(&resolver, "f1"),
                self::status(&resolver, "f1"),
                self::status(&resolver, "f1"),
            );
            assert_eq!([a, b, c, d], [expected; 4]);
            assert_eq!(sts.calls(), 1);
            let count = (resolver.calls.refusals).with(&Client::of(None), |calls, _| calls.refused);
            assert_eq!(count, u32::from(expected == 401));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_wait_for_a_call_that_ends_is_no_refusal() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        let _running = (resolver.calls.all)
            .acquire_many(MAX_STS_CALLS as u32)
            .await
            .unwrap();
        let (parts, ()) = axum::http::Request::new(()).into_parts();
        let cx = RequestContext::new(&parts, Some(token("x-server-id", None, "g1", "")));
        let answer = Resolver::<()>::resolve(&resolver, &cx, &()).await;
        let Resolution::Refused(AuthError::Unavailable(reason)) = answer else {
            panic!("not HTTP 503");
        };
        assert_eq!(reason, "STS is busy, try again later");
        assert_eq!(sts.calls(), 0);
    }

    #[tokio::test]
    async fn answers_without_an_error_code() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        // Of a proxy on the way: no refusal, and asked again.
        sts.set(403, false, "<html>Forbidden</html>");
        assert_eq!(status(&resolver, "a1").await, 503);
        assert_eq!(status(&resolver, "a1").await, 503);
        assert_eq!(sts.calls(), 2);
        // Of AWS, and of a hop that finds the request too large: refused,
        // and not asked again.
        sts.set(404, true, "<UnknownOperationException/>");
        assert_eq!(status(&resolver, "a2").await, 401);
        assert_eq!(status(&resolver, "a2").await, 401);
        sts.set(431, false, "");
        assert_eq!(status(&resolver, "a3").await, 401);
        assert_eq!(status(&resolver, "a3").await, 401);
        assert_eq!(sts.calls(), 4);
    }

    #[tokio::test]
    async fn refused_requests_make_a_limited_number_of_calls() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        sts.set(
            403,
            true,
            "<ErrorResponse><Error><Code>InvalidClientTokenId</Code></Error></ErrorResponse>",
        );
        for n in 0..MAX_STS_REFUSALS {
            assert_eq!(status(&resolver, &format!("b{n}")).await, 401, "{n}");
        }
        for n in 0..5 {
            assert_eq!(status(&resolver, &format!("c{n}")).await, 503, "{n}");
        }
        assert_eq!(sts.calls(), MAX_STS_REFUSALS as usize);
    }

    #[tokio::test]
    async fn calls_without_an_answer_are_limited_for_each_client() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        sts.set(500, false, "");
        for n in 0..MAX_STS_REFUSALS {
            let signature = format!("h{n}");
            assert_eq!(
                status_of(&resolver, &signature, Some("192.0.2.1")).await,
                503
            );
        }
        let calls = MAX_STS_REFUSALS as usize;
        assert_eq!(sts.calls(), calls);
        // One of its requests at a time goes to STS now.
        let (a, b, c) = tokio::join!(
            status_of(&resolver, "i1", Some("192.0.2.1")),
            status_of(&resolver, "i2", Some("192.0.2.1")),
            status_of(&resolver, "i3", Some("192.0.2.1")),
        );
        assert_eq!([a, b, c], [503; 3]);
        assert_eq!(sts.calls(), calls + 1);
        // The others are served, and so is the client once STS answers.
        sts.set(200, true, CALLER);
        assert_eq!(status_of(&resolver, "i4", Some("192.0.2.2")).await, 200);
        assert_eq!(status_of(&resolver, "i8", Some("192.0.2.1")).await, 200);
        let (a, b, c) = tokio::join!(
            status_of(&resolver, "i5", Some("192.0.2.1")),
            status_of(&resolver, "i6", Some("192.0.2.1")),
            status_of(&resolver, "i7", Some("192.0.2.1")),
        );
        assert_eq!([a, b, c], [200; 3]);
        assert_eq!(sts.calls(), calls + 6);
    }

    #[tokio::test(start_paused = true)]
    async fn clients_with_refusals_leave_places_to_the_others() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        sts.set(403, true, REFUSAL);
        assert_eq!(status_of(&resolver, "j1", Some("192.0.2.1")).await, 401);
        let _running = (resolver.calls.limited)
            .acquire_many(MAX_LIMITED_STS_CALLS as u32)
            .await
            .unwrap();
        sts.set(200, true, CALLER);
        assert_eq!(status_of(&resolver, "j2", Some("192.0.2.1")).await, 503);
        assert_eq!(status_of(&resolver, "j2", Some("192.0.2.2")).await, 200);
        assert_eq!(sts.calls(), 2);
    }

    #[test]
    fn the_signed_headers_go_to_sts() {
        let resolver = resolver(&Answers::default());
        let token = token_with(
            "amz-sdk-request;content-length;host;x-amz-date;x-server-id",
            &[
                ("Amz-Sdk-Request", "attempt=1"),
                ("Content-Length", "43"),
                ("Host", "sts.amazonaws.com"),
                ("TE", "trailers"),
                ("X-Amz-Security-Token", "t"),
                ("Accept", "*/*"),
            ],
        );
        let checked = resolver.check(&token).map_err(drop).unwrap();
        let mut sent: Vec<&str> = checked.headers.keys().map(HeaderName::as_str).collect();
        sent.sort_unstable();
        // Not those that the HTTP client writes, nor one that is not signed.
        let expected = [
            "amz-sdk-request",
            "authorization",
            "x-amz-date",
            "x-amz-security-token",
            "x-server-id",
        ];
        assert_eq!(sent, expected);
    }

    #[test]
    fn a_request_has_one_key_however_it_is_written() {
        let resolver = resolver(&Answers::default());
        let key = |token: String| resolver.check(&token).map(|c| c.key).unwrap();
        let one = key(token("x-server-id", None, "ab", ""));
        // Padding, the case of a header name, and a header that is not sent
        // to STS.
        assert_eq!(key(token("x-server-id", None, "ab", "==")), one);
        assert_eq!(key(token("X-Server-ID", None, "ab", "")), one);
        assert_eq!(key(token("x-server-id", Some("*/*"), "ab", "")), one);
        // Spaces that the signature does not read.
        assert_eq!(key(token("x-server-id", None, "ab ", "")), one);
        assert_eq!(single_spaces("  a   b "), "a b");
        assert_eq!(single_spaces("\t a b \t"), "a b");
        let written = "AWS4-HMAC-SHA256 Credential=A/x,SignedHeaders=host,  Signature=ab";
        assert_eq!(
            Authorization::parse(written).unwrap().to_string(),
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=host, Signature=ab"
        );
        assert_ne!(key(token("x-server-id", None, "cd", "")), one);
    }

    /// A bearer value with `headers` next to those that each request has,
    /// which signs `signed`.
    fn token_with(signed: &str, headers: &[(&str, &str)]) -> String {
        let date = Utc::now().format(AMZ_DATE).to_string();
        let authorization =
            format!("AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders={signed}, Signature=ab");
        let mut request = serde_json::json!({
            "method": "POST",
            "url": "https://sts.amazonaws.com/",
            "body": GET_CALLER_IDENTITY,
            "headers": { "X-Server-Id": "api", "X-Amz-Date": date, "Authorization": authorization },
        });
        for (name, value) in headers {
            request["headers"][name] = (*value).into();
        }
        format!("aws:{}", URL_SAFE_NO_PAD.encode(request.to_string()))
    }

    #[test]
    fn a_header_that_is_not_signed_makes_no_other_request() {
        let resolver = resolver(&Answers::default());
        let signed = "host;x-amz-date;x-server-id";
        let key = |headers: &[(&str, &str)]| {
            let checked = resolver.check(&token_with(signed, headers));
            checked.map(|c| c.key).map_err(drop).unwrap()
        };
        let one = key(&[("Content-Type", "application/x-www-form-urlencoded")]);
        assert_eq!(key(&[("Content-Type", "text/plain; n=1")]), one);
        assert_eq!(
            key(&[("Content-Type", "text/plain"), ("X-Amz-N", "1")]),
            one
        );
        assert_ne!(key(&[]), one);
        // A signed one is sent as it is written.
        let signed = "content-type;host;x-amz-date;x-server-id";
        let token = token_with(signed, &[("Content-Type", "text/plain")]);
        let checked = resolver.check(&token).map_err(drop).unwrap();
        assert_eq!(checked.headers["content-type"], "text/plain");
    }

    #[test]
    fn the_date_is_signed_and_the_access_key_is_short() {
        let resolver = resolver(&Answers::default());
        let refused = |token: String| resolver.check(&token).map(|_| ()).unwrap_err();
        assert_eq!(
            refused(token_with("host;x-server-id", &[])),
            "x-amz-date is not signed"
        );
        let long = "A".repeat(MAX_ACCESS_KEY_LENGTH + 1);
        let long = token_of(&long, "x-server-id", None, "ab", "");
        assert_eq!(refused(long), "the access key id is too long");
    }

    #[tokio::test(start_paused = true)]
    async fn the_calls_of_one_client_at_a_time_leave_places_to_the_others() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        sts.set(200, true, CALLER);
        let _running = (resolver.calls.limited)
            .acquire_many(MAX_LIMITED_STS_CALLS as u32)
            .await
            .unwrap();
        // The first call of the client has a place, its next one waits with
        // the clients that had a call without a caller.
        let (a, b) = tokio::join!(
            status_of(&resolver, "n1", Some("192.0.2.1")),
            status_of(&resolver, "n2", Some("192.0.2.1")),
        );
        assert_eq!([a, b], [200, 503]);
        assert_eq!(status_of(&resolver, "n3", Some("192.0.2.1")).await, 200);
        assert_eq!(sts.calls(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_limit_of_one_client_is_not_the_answer_to_another() {
        let sts = Answers::default();
        let resolver = resolver(&sts);
        sts.set(403, true, REFUSAL);
        assert_eq!(status_of(&resolver, "o1", Some("192.0.2.1")).await, 401);
        let _running = (resolver.calls.limited)
            .acquire_many(MAX_LIMITED_STS_CALLS as u32)
            .await
            .unwrap();
        sts.set(200, true, CALLER);
        // The first client finds no place. The other one waits for it, as
        // the value is the same, and then asks for itself.
        let (a, b) = tokio::join!(
            status_of(&resolver, "o2", Some("192.0.2.1")),
            status_of(&resolver, "o2", Some("192.0.2.2")),
        );
        assert_eq!([a, b], [503, 200]);
        assert_eq!(sts.calls(), 2);
    }

    #[test]
    fn a_url_with_a_user_is_no_endpoint() {
        let resolver = resolver(&Answers::default());
        let with_user = token("x-server-id", None, "ab", "");
        let json = URL_SAFE_NO_PAD
            .decode(with_user.strip_prefix("aws:").unwrap())
            .unwrap();
        let json = String::from_utf8(json)
            .unwrap()
            .replace("https://sts.", "https://a1@sts.");
        let with_user = format!("aws:{}", URL_SAFE_NO_PAD.encode(json));
        let refused = resolver.check(&with_user).map(|_| ()).unwrap_err();
        assert!(refused.starts_with("not an STS endpoint"), "{refused}");
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
        for other in ["s3", "s3-us-west-2", "s3-fips", "us-east", "1"] {
            let origin = format!("https://sts.{other}.amazonaws.com");
            assert!(
                !provider.takes(&origin, "https://api.example.com"),
                "{origin}"
            );
        }
        for region in ["us-gov-west-1", "ap-southeast-2", "eusc-de-east-1"] {
            let origin = format!("https://sts.{region}.amazonaws.com");
            assert!(
                provider.takes(&origin, "https://api.example.com"),
                "{origin}"
            );
        }
        // A `*` before `amazonaws.com` is a region in each pattern.
        let others = provider.clone().with_endpoints(&[
            "https://sts-fips.*.amazonaws.com",
            "https://sts.*.amazonaws.com.cn",
            "https://*.sts.eu-west-1.vpce.amazonaws.com",
        ]);
        for origin in [
            "https://sts-fips.us-east-1.amazonaws.com",
            "https://sts.cn-north-1.amazonaws.com.cn",
            "https://vpce-0a1b-2c3d.sts.eu-west-1.vpce.amazonaws.com",
        ] {
            assert!(others.takes(origin, "https://api.example.com"), "{origin}");
        }
        for origin in [
            "https://sts-fips.s3.amazonaws.com",
            "https://sts.s3.amazonaws.com.cn",
        ] {
            assert!(!others.takes(origin, "https://api.example.com"), "{origin}");
        }
        let local = provider.with_endpoints(&["http://127.0.0.1:9000"]);
        assert!(local.takes("http://127.0.0.1:9000", "https://api.example.com"));
        assert!(!local.takes("https://sts.amazonaws.com", "https://api.example.com"));
    }
}
