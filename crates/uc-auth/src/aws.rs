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
//! current [`REFUSED_TTL`], so the other clients have places. After
//! [`MAX_STS_REFUSALS`] refusals in a [`REFUSED_TTL`], no request of that
//! client is sent until the next one starts: it gets HTTP 503 for new values
//! then. After as many calls in a row without an answer of STS, one request
//! of that client is sent at a time until STS answers one. The other clients
//! are served, and so are the identities that are kept.

use std::collections::BTreeMap;
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
use crate::window::{Client, Windows};
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
/// regional ones. The `*` is a region only: other AWS services have hosts of
/// this shape (`sts.s3.amazonaws.com` is a bucket).
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
/// The most calls to STS at a time: the others wait up to [`CALL_WAIT`], as
/// those that run may all be refused.
pub const MAX_STS_CALLS: usize = 20;
/// The most calls to STS at a time for the clients that had a call without
/// a caller in the current [`REFUSED_TTL`]. The other places are for the
/// clients that had none, so many clients with made-up signatures do not
/// keep them waiting.
pub const MAX_LIMITED_STS_CALLS: usize = MAX_STS_CALLS / 2;
/// How long a call to STS waits for a place.
pub const CALL_WAIT: Duration = Duration::from_secs(5);
/// The format of the `X-Amz-Date` header.
const AMZ_DATE: &str = "%Y%m%dT%H%M%SZ";
/// The header that each answer of AWS has, also one without an error code.
const REQUEST_ID_HEADER: &str = "x-amzn-requestid";
/// The signed headers that are not sent to STS as the client wrote them:
/// those of the connection, which the HTTP client writes.
const NOT_FORWARDED: [&str; 11] = [
    "host",
    "content-length",
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
        self.endpoints.is_allowed(origin)
            && !other_aws_service(origin)
            && self.server_ids.iter().any(|id| id == server_id)
    }
}

/// Whether `origin` is `https://sts.<label>.amazonaws.com` with a label
/// that is not a region, so the host of another AWS service.
fn other_aws_service(origin: &str) -> bool {
    (origin.strip_prefix("https://sts."))
        .and_then(|rest| rest.strip_suffix(".amazonaws.com"))
        .is_some_and(|label| !label.contains('.') && !is_region(label))
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
}

/// The places that a call to STS holds while it runs.
struct Place<'a> {
    _limited: Option<SemaphorePermit<'a>>,
    _call: SemaphorePermit<'a>,
    _probe: Option<Probe<'a>>,
}

/// The calls of a client that named no caller.
#[derive(Default)]
struct Refusals {
    /// Those that STS, or the check of its answer, refused.
    refused: u32,
    /// Those that STS did not answer, since its last answer.
    unanswered: u32,
    /// Whether a call of the client runs while it is over the limit of
    /// calls without an answer.
    probing: bool,
    /// Whether the log says that the client is over the limit.
    logged: bool,
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
    refusals: &'a Windows<Client, Refusals>,
    client: &'a Client,
}

impl Drop for Probe<'_> {
    fn drop(&mut self) {
        (self.refusals).with(self.client, |calls, _| calls.probing = false);
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
        let signed = |name: &str| authorization.signed_headers.split(';').any(|h| h == name);
        if !signed(header) {
            return Err(format!("{header} is not signed"));
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
        // (but [`NOT_FORWARDED`]), and those of AWS, which a client can add
        // after it signed. They are written in one way: the authorization
        // from its parts, the others with single spaces, as the signature
        // reads them. The hash is of the URL and these headers, in the
        // order of their names.
        let forwarded = |name: &str| {
            name == "authorization"
                || name == "content-type"
                || name.starts_with("x-amz-")
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
            let value = match name == "authorization" {
                true => authorization.to_string(),
                false => single_spaces(value),
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
            url,
            headers: sent,
            key: hash.finalize().into(),
            left,
        })
    }

    /// Send the signed request to STS, which names the caller or refuses
    /// the request. No answer, one of 5xx, STS's throttling (HTTP 400 or
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
    ) -> Result<Result<Named, String>, AuthError> {
        let _place = self.calls.enter(client).await?;
        let named = match self.caller_identity(checked).await {
            Ok(identity) => checked.named(identity),
            Err(e) => Err(e),
        };
        self.calls.ended(client, &named);
        match named {
            Ok(named) => Ok(Ok(named)),
            Err(StsError::Refused(reason)) => Ok(Err(reason)),
            // Not an answer to keep: STS may take the request later.
            Err(StsError::NotYet(reason)) => Err(AuthError::Invalid(reason)),
            Err(StsError::Unavailable(reason)) => Err(AuthError::Unavailable(reason)),
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
        let limited = match self.standing(client, false) {
            Standing::TooOften => return Err(too_often()),
            Standing::Clean => false,
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
            tracing::warn!("no call to STS ended in time; the AWS token is not checked");
            return Err(AuthError::Unavailable(
                "STS is busy, try again later".into(),
            ));
        };
        // The calls that this one waited for may have been the client's.
        let probe = match self.standing(client, true) {
            Standing::TooOften => return Err(too_often()),
            Standing::Probe => Some(Probe {
                refusals: &self.refusals,
                client,
            }),
            Standing::Clean | Standing::WithoutCaller => None,
        };
        Ok(Place {
            _limited: limited,
            _call: call,
            _probe: probe,
        })
    }

    /// What the current [`REFUSED_TTL`] says of `client`. With `claim`, a
    /// [`Standing::Probe`] is taken: the caller makes the one call, and
    /// holds a [`Probe`] for it. The log says once per client and
    /// [`REFUSED_TTL`] that the client is over a limit.
    fn standing(&self, client: &Client, claim: bool) -> Standing {
        let (standing, log) = self.refusals.with(client, |calls, _| {
            let unanswered = calls.unanswered >= MAX_STS_REFUSALS;
            let standing = if calls.refused >= MAX_STS_REFUSALS || (unanswered && calls.probing) {
                Standing::TooOften
            } else if unanswered {
                calls.probing = claim;
                Standing::Probe
            } else if calls.refused > 0 || calls.unanswered > 0 {
                Standing::WithoutCaller
            } else {
                Standing::Clean
            };
            let over = matches!(standing, Standing::TooOften | Standing::Probe);
            (
                standing,
                over && !std::mem::replace(&mut calls.logged, true),
            )
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

    /// Count a call of `client` that ended with `result`. An answer of STS
    /// lets the calls without an answer start anew.
    fn ended<T>(&self, client: &Client, result: &Result<T, StsError>) {
        self.refusals.with(client, |calls, _| match result {
            Ok(_) => calls.unanswered = 0,
            Err(StsError::Unavailable(_)) => {
                calls.unanswered = calls.unanswered.saturating_add(1);
            }
            Err(StsError::Refused(_) | StsError::NotYet(_)) => {
                calls.refused += 1;
                calls.unanswered = 0;
            }
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
                tracing::warn!(%reason, "AWS token refused");
                return Resolution::Refused(AuthError::Invalid(reason));
            }
        };
        // One call for the requests with one value that arrive together.
        let client = Client::of(cx.client());
        let answer = (self.answers)
            .try_get_with(checked.key, self.ask(&checked, &client))
            .await;
        let Named {
            provider,
            arn,
            identity,
        } = match answer {
            Ok(Ok(named)) => named,
            Ok(Err(reason)) => return Resolution::Refused(AuthError::Invalid(reason)),
            Err(unanswered) => return Resolution::Refused((*unanswered).clone()),
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
/// run of them, which is what its signature is of.
fn single_spaces(value: &str) -> String {
    let words: Vec<&str> = value.split(' ').filter(|w| !w.is_empty()).collect();
    words.join(" ")
}

/// Whether the answer of STS is its throttling (`Throttling: Rate
/// exceeded`), which comes as HTTP 400.
fn throttled(xml: &str) -> bool {
    matches!(
        element(xml, "Code").as_deref(),
        Some("Throttling" | "ThrottlingException" | "RequestLimitExceeded")
    )
}

/// The error code of STS for an access key that it does not know, which it
/// also says of a new key for some seconds.
const NOT_YET_KNOWN: &str = "InvalidClientTokenId";

/// The text of the first element `name` of an STS answer.
fn element(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let start = xml.find(&open)? + open.len();
    let end = start + xml[start..].find(&format!("</{name}>"))?;
    Some(xml[start..end].trim().to_string()).filter(|v| !v.is_empty())
}

/// What STS says of the caller.
#[derive(Debug, Clone, PartialEq)]
struct CallerIdentity {
    arn: String,
    account: String,
    user_id: String,
}

/// The `Arn`, `Account` and `UserId` of an STS `GetCallerIdentity` answer.
fn parse_caller_identity(xml: &str) -> Option<CallerIdentity> {
    Some(CallerIdentity {
        arn: element(xml, "Arn")?,
        account: element(xml, "Account")?,
        user_id: element(xml, "UserId").unwrap_or_default(),
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
    /// now.
    fn token(server_id: &str, extra: Option<&str>, signature: &str, padding: &str) -> String {
        let date = Utc::now().format(AMZ_DATE).to_string();
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=host;x-amz-date;x-server-id, \
             Signature={signature}"
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

    /// As [`status`], for a request from the address `peer`.
    async fn status_of(resolver: &AwsResolver, signature: &str, peer: Option<&str>) -> u16 {
        let (mut parts, ()) = axum::http::Request::new(()).into_parts();
        if let Some(peer) = peer {
            let peer: std::net::SocketAddr = format!("{peer}:1").parse().unwrap();
            parts.extensions.insert(axum::extract::ConnectInfo(peer));
        }
        let cx = RequestContext::new(&parts, Some(token("x-server-id", None, signature, "")));
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
        let date = Utc::now().format(AMZ_DATE).to_string();
        let request = serde_json::json!({
            "method": "POST",
            "url": "https://sts.amazonaws.com/",
            "body": GET_CALLER_IDENTITY,
            "headers": {
                "Authorization": "AWS4-HMAC-SHA256 Credential=A/x, \
                    SignedHeaders=amz-sdk-request;host;te;x-amz-date;x-server-id, Signature=ab",
                "Amz-Sdk-Request": "attempt=1",
                "Host": "sts.amazonaws.com",
                "TE": "trailers",
                "X-Amz-Date": date,
                "X-Amz-Security-Token": "t",
                "X-Server-Id": "api",
                "Accept": "*/*",
            },
        });
        let token = format!("aws:{}", URL_SAFE_NO_PAD.encode(request.to_string()));
        let checked = resolver.check(&token).map_err(drop).unwrap();
        let mut sent: Vec<&str> = checked.headers.keys().map(HeaderName::as_str).collect();
        sent.sort_unstable();
        // Not the headers of the connection, nor one that is not signed.
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
        let written = "AWS4-HMAC-SHA256 Credential=A/x,SignedHeaders=host,  Signature=ab";
        assert_eq!(
            Authorization::parse(written).unwrap().to_string(),
            "AWS4-HMAC-SHA256 Credential=A/x, SignedHeaders=host, Signature=ab"
        );
        assert_ne!(key(token("x-server-id", None, "cd", "")), one);
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
        let local = provider.with_endpoints(&["http://127.0.0.1:9000"]);
        assert!(local.takes("http://127.0.0.1:9000", "https://api.example.com"));
        assert!(!local.takes("https://sts.amazonaws.com", "https://api.example.com"));
    }
}
