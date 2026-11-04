//! AWS identities, which cannot make a token that a third party checks: the
//! bearer value is a signed `sts:GetCallerIdentity` request, which the server
//! sends to STS. STS checks the signature and answers with the caller's ARN.
//! Vault's and the EKS authenticator's AWS sign-in work this way.
//!
//! The value is [`PREFIX`] and the request as unpadded base64url JSON:
//! `{"method", "url", "headers", "body"}`. A block of `[[auth.issuers]]`
//! with `kind = "aws"` takes it when:
//!
//! - the request goes to one of its STS endpoints (`issuer`), so the server
//!   sends nothing to any other host;
//! - it is `POST /` with the `GetCallerIdentity` body;
//! - it signs the header [`SERVER_ID_HEADER`], whose value is one of the
//!   block's `audiences`: a request signed for another server does not pass
//!   here;
//! - it was signed less than [`SIGNED_REQUEST_TTL`] ago, as STS also checks;
//! - STS answers with the ARN of one of the block's `accounts`.
//!
//! The caller is named by the ARN, an assumed role by its role
//! (`arn:aws:iam::<account>:role/<name>`, without the role's path), so each
//! session is not a user of its own. The result is kept as a JWT's is
//! ([`idp::accept`]), until the signed request expires, at most
//! [`idp::ACCEPTED_TTL`].

use std::collections::BTreeMap;
use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, Utc};
use serde::Deserialize;
use serde_json::{Map, Value};

use uc_common::{Error, ErrorCode, Result};

use crate::{
    authn::idp::{self, Accepted, IssuerSettings, LEEWAY},
    http::outbound::SendOnIo,
    server::state::AppState,
};

/// The start of an AWS bearer value.
pub const PREFIX: &str = "uc-aws-v1.";
/// The signed header with the server id, one of the block's `audiences`.
pub const SERVER_ID_HEADER: &str = "x-uc-server-id";
/// The body of the signed request.
pub const GET_CALLER_IDENTITY: &str = "Action=GetCallerIdentity&Version=2011-06-15";
/// How long STS takes a signed request.
pub const SIGNED_REQUEST_TTL: Duration = Duration::from_secs(15 * 60);

/// The signed request in a bearer value.
#[derive(Debug, Deserialize)]
struct SignedRequest {
    method: String,
    url: String,
    headers: BTreeMap<String, String>,
    body: String,
}

/// What a bearer value of [`PREFIX`] carries, checked before STS is asked.
struct Checked<'a> {
    issuer: &'a IssuerSettings,
    url: reqwest::Url,
    /// The headers sent to STS, lowercase.
    headers: BTreeMap<String, String>,
    /// Until when STS takes it.
    left: Duration,
}

/// The caller that the AWS bearer value `token` names, and none of its
/// groups. A value that does not check out is "Invalid access token."
/// (401), with the reason in the log; a caller that names no active user is
/// "User not allowed" (403).
pub(crate) async fn accept(state: &AppState, token: &str) -> Result<Accepted> {
    let invalid = || Error::new(ErrorCode::Unauthenticated, "Invalid access token.");
    if let Some(accepted) = state.auth.accepted(token).await {
        return Ok(accepted);
    }
    let checked = check(state, token).map_err(|reason| {
        tracing::warn!(%reason, "AWS token refused");
        invalid()
    })?;
    let identity = caller_identity(state, &checked).await?;
    let issuer = checked.issuer;
    if !issuer.accounts.contains(&identity.account) {
        tracing::warn!(
            issuer = %issuer.name,
            account = %identity.account,
            "AWS token refused: account not allowed"
        );
        return Err(invalid());
    }
    let arn = principal_arn(&identity.arn).ok_or_else(|| {
        tracing::warn!(arn = %identity.arn, "AWS token refused: not an ARN");
        invalid()
    })?;
    let mut claims = Map::new();
    claims.insert("arn".into(), Value::from(arn));
    claims.insert("caller_arn".into(), Value::from(identity.arn));
    claims.insert("account".into(), Value::from(identity.account));
    claims.insert("user_id".into(), Value::from(identity.user_id));
    let accepted = Accepted {
        principal: idp::name_caller(state, issuer, &claims).await?,
        groups: Vec::new(),
    };
    state.auth.keep(token, &accepted, checked.left).await;
    Ok(accepted)
}

/// The checks that need no call: the shape of the request, its endpoint,
/// its server id and its age, and the block that takes it.
fn check<'a>(state: &'a AppState, token: &str) -> std::result::Result<Checked<'a>, String> {
    let encoded = token.strip_prefix(PREFIX).ok_or("not an AWS token")?;
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
    let server_id = headers
        .get(SERVER_ID_HEADER)
        .ok_or(format!("no {SERVER_ID_HEADER} header"))?;
    let authorization = headers
        .get("authorization")
        .ok_or("no authorization header")?;
    if !signed_headers(authorization).any(|h| h == SERVER_ID_HEADER) {
        return Err(format!("{SERVER_ID_HEADER} is not signed"));
    }
    let issuer = (state.auth.issuers.iter())
        .find(|i| i.sts_endpoint_allowed(&origin) && i.server_id_allowed(server_id))
        .ok_or_else(|| {
            format!("no auth.issuers block of kind aws takes {origin} and server id {server_id}")
        })?;
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
                || name == SERVER_ID_HEADER
                || name.starts_with("x-amz-")
        })
        .collect();
    Ok(Checked {
        issuer,
        url,
        headers,
        left,
    })
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

/// Send the signed request to STS. An answer of 4xx is "Invalid access
/// token."; no answer, or one of 5xx, is an internal error, as the caller
/// may well be signed in.
async fn caller_identity(state: &AppState, checked: &Checked<'_>) -> Result<CallerIdentity> {
    let mut request = state
        .auth
        .providers
        .http()
        .post(checked.url.clone())
        .body(GET_CALLER_IDENTITY);
    for (name, value) in &checked.headers {
        request = request.header(name, value);
    }
    let unreachable = |e: &dyn std::fmt::Display| {
        tracing::warn!(url = %checked.url, error = %e, "cannot reach STS");
        Error::internal("Could not check the AWS identity")
    };
    let response = request.send_on_io().await.map_err(|e| unreachable(&e))?;
    let status = response.status();
    let body = response.text().await.map_err(|e| unreachable(&e))?;
    if status.is_client_error() {
        tracing::warn!(
            status = status.as_u16(),
            answer = %body.chars().take(300).collect::<String>(),
            "AWS token refused by STS"
        );
        return Err(Error::new(
            ErrorCode::Unauthenticated,
            "Invalid access token.",
        ));
    }
    if !status.is_success() {
        return Err(unreachable(&format!("HTTP {}", status.as_u16())));
    }
    parse_caller_identity(&body).ok_or_else(|| unreachable(&"no Arn and Account in the answer"))
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
                             SignedHeaders=content-type;host;x-amz-date;x-uc-server-id, Signature=ab";
        assert_eq!(
            signed_headers(authorization).collect::<Vec<_>>(),
            ["content-type", "host", "x-amz-date", "x-uc-server-id"]
        );
    }
}
