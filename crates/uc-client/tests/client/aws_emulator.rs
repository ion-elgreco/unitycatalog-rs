//! The AWS sign-in against AWS Emulator, a local STS that checks Signature
//! Version 4: the client signs `GetCallerIdentity`, and the resolver of the
//! server sends what it takes of the request to the emulator.
//!
//! The tests run when `UC_TEST_AWS_EMULATOR` names the folder that the
//! emulator writes at its start (`~/.aws/emulator/<port>`), with its
//! `config` and `emulator_credentials`: `just test-aws-emulator`.

use std::path::PathBuf;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use uc_auth::{
    AuthError, AwsProvider, AwsResolver, RequestContext, Resolution, Resolver, Transport,
    async_trait,
};
use uc_client::identity;

use crate::common::{sha256_hex, sigv4_signature};

/// The account of each identity of the emulator, and the region it is in.
const ACCOUNT: &str = "000000000000";
const REGION: &str = "us-east-1";
const SERVER_ID: &str = "https://uc.example.com";
const PREFIX: &str = "uc-aws-v1.";
const SERVER_ID_HEADER: &str = "x-uc-server-id";
/// What the resolver says of a request that STS refused.
const REFUSED_BY_STS: &str = "STS refused it: HTTP 403";

/// Sends each request without a check of the certificate: the emulator
/// serves HTTPS with a certificate of its own, new at each start.
struct Unchecked(reqwest::Client);

impl Unchecked {
    fn new() -> Self {
        let client = reqwest::Client::builder().danger_accept_invalid_certs(true);
        Self(client.build().unwrap())
    }
}

#[async_trait]
impl Transport for Unchecked {
    async fn send(&self, request: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
        self.0.execute(request.build()?).await
    }
}

/// A key pair that the emulator takes, with the session token of a
/// temporary one.
struct Keys {
    key_id: String,
    secret: String,
    session_token: Option<String>,
}

/// An emulator that runs: its endpoint, and the key pair of its user.
struct Emulator {
    endpoint: String,
    user: Keys,
}

impl Emulator {
    /// The emulator of `UC_TEST_AWS_EMULATOR`, `None` when the variable is
    /// not set.
    fn running() -> Option<Self> {
        let folder = PathBuf::from(std::env::var_os("UC_TEST_AWS_EMULATOR")?);
        let value = |file: &str, name: &str| {
            let text = std::fs::read_to_string(folder.join(file)).unwrap();
            let line = text.lines().find_map(|line| {
                let (key, value) = line.split_once('=')?;
                (key.trim() == name).then(|| value.trim().to_string())
            });
            line.unwrap_or_else(|| panic!("no {name} in {file}"))
        };
        Some(Self {
            endpoint: value("config", "endpoint_url"),
            user: Keys {
                key_id: value("emulator_credentials", "aws_access_key_id"),
                secret: value("emulator_credentials", "aws_secret_access_key"),
                session_token: None,
            },
        })
    }

    /// The bearer value that the client signs with `keys`.
    async fn token(&self, keys: &Keys) -> String {
        let mut env: identity::Env = [
            ("AWS_ACCESS_KEY_ID", keys.key_id.as_str()),
            ("AWS_SECRET_ACCESS_KEY", keys.secret.as_str()),
            ("AWS_REGION", REGION),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
        if let Some(session_token) = &keys.session_token {
            env.insert("AWS_SESSION_TOKEN".into(), session_token.clone());
        }
        let provider = identity::aws_in(&env, SERVER_ID, Some(&self.endpoint)).unwrap();
        provider.get_credential().await.unwrap().token.clone()
    }

    /// What the resolver of a server with the id [`SERVER_ID`], which takes
    /// the identities of [`ACCOUNT`] at this emulator, makes of `token`.
    async fn resolve(&self, token: String) -> Resolution {
        let provider =
            AwsProvider::new("aws", [SERVER_ID], [ACCOUNT]).with_endpoints(&[&self.endpoint]);
        let resolver = AwsResolver::new(PREFIX, SERVER_ID_HEADER, vec![provider])
            .with_transport(Unchecked::new());
        let (parts, ()) = axum::http::Request::new(()).into_parts();
        let cx = RequestContext::new(&parts, Some(token));
        Resolver::<()>::resolve(&resolver, &cx, &()).await
    }

    /// The ARN of the caller that `token` signs in.
    async fn caller(&self, token: String) -> String {
        match self.resolve(token).await {
            Resolution::Resolved(caller) => caller.subject,
            Resolution::Refused(error) => panic!("refused: {error:?}"),
            Resolution::Pass => panic!("not an AWS token"),
        }
    }

    /// Why the resolver refuses `token` as one that is not valid.
    async fn refused(&self, token: String) -> String {
        match self.resolve(token).await {
            Resolution::Refused(AuthError::Invalid(reason)) => reason,
            Resolution::Refused(error) => panic!("not refused as invalid: {error:?}"),
            _ => panic!("not refused"),
        }
    }

    /// A temporary key pair of the emulator's user, from `GetSessionToken`:
    /// each request that it signs has its session token.
    async fn session(&self) -> Keys {
        let body = "Action=GetSessionToken&Version=2011-06-15";
        let host = self.endpoint.split_once("://").unwrap().1;
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let scope = format!("{}/{REGION}/sts/aws4_request", now.format("%Y%m%d"));
        let canonical_request = format!(
            "POST\n/\n\nhost:{host}\nx-amz-date:{amz_date}\n\nhost;x-amz-date\n{}",
            sha256_hex(body.as_bytes())
        );
        let signature = sigv4_signature(&self.user.secret, &scope, &amz_date, &canonical_request);
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders=host;x-amz-date, \
             Signature={signature}",
            self.user.key_id
        );
        let request = (Unchecked::new().0.post(&self.endpoint))
            .header("x-amz-date", amz_date)
            .header("authorization", authorization)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body);
        let answer = request.send().await.unwrap().text().await.unwrap();
        let element = |name: &str| {
            let (_, rest) = (answer.split_once(&format!("<{name}>")))
                .unwrap_or_else(|| panic!("no {name} in {answer}"));
            rest.split_once("</").unwrap().0.to_string()
        };
        Keys {
            key_id: element("AccessKeyId"),
            secret: element("SecretAccessKey"),
            session_token: Some(element("SessionToken")),
        }
    }
}

/// `token` with its signed request changed by `change`.
fn changed(token: &str, change: impl FnOnce(&mut Value)) -> String {
    let json = URL_SAFE_NO_PAD.decode(token.strip_prefix(PREFIX).unwrap());
    let mut request: Value = serde_json::from_slice(&json.unwrap()).unwrap();
    change(&mut request);
    format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(request.to_string()))
}

#[tokio::test]
async fn the_emulator_takes_the_signature_of_the_client() {
    let Some(emulator) = Emulator::running() else {
        return;
    };
    let token = emulator.token(&emulator.user).await;
    let caller = emulator.caller(token).await;
    assert_eq!(caller, format!("arn:aws:iam::{ACCOUNT}:user/emulator"));
}

#[tokio::test]
async fn the_emulator_takes_the_signature_of_a_session() {
    let Some(emulator) = Emulator::running() else {
        return;
    };
    let session = emulator.session().await;
    let token = emulator.token(&session).await;
    let caller = emulator.caller(token.clone()).await;
    assert_eq!(caller, format!("arn:aws:iam::{ACCOUNT}:user/emulator"));
    // Not without its session token.
    let without = changed(&token, |request| {
        let headers = request["headers"].as_object_mut().unwrap();
        headers.remove("x-amz-security-token").unwrap();
    });
    let answer = emulator.resolve(without).await;
    assert!(matches!(answer, Resolution::Refused(_)));
}

#[tokio::test]
async fn the_emulator_refuses_a_signature_of_another_secret() {
    let Some(emulator) = Emulator::running() else {
        return;
    };
    let other = Keys {
        secret: "another-secret".into(),
        key_id: emulator.user.key_id.clone(),
        session_token: None,
    };
    let token = emulator.token(&other).await;
    assert_eq!(emulator.refused(token).await, REFUSED_BY_STS);
}

#[tokio::test]
async fn the_emulator_refuses_a_signed_header_that_was_changed() {
    let Some(emulator) = Emulator::running() else {
        return;
    };
    let token = emulator.token(&emulator.user).await;
    // The date one minute later: the signature is of another date.
    let later = (chrono::Utc::now() + chrono::Duration::minutes(1)).format("%Y%m%dT%H%M%SZ");
    let token = changed(&token, |request| {
        request["headers"]["x-amz-date"] = json!(later.to_string());
    });
    assert_eq!(emulator.refused(token).await, REFUSED_BY_STS);
}

/// The headers that the request does not sign do not reach the emulator: it
/// answers in XML, and the signature holds.
#[tokio::test]
async fn the_emulator_takes_a_request_with_headers_that_are_not_signed() {
    let Some(emulator) = Emulator::running() else {
        return;
    };
    let token = emulator.token(&emulator.user).await;
    let token = changed(&token, |request| {
        let headers = request["headers"].as_object_mut().unwrap();
        headers.insert("accept".into(), json!("application/json"));
        headers.insert("x-amz-n".into(), json!("1"));
    });
    let caller = emulator.caller(token).await;
    assert_eq!(caller, format!("arn:aws:iam::{ACCOUNT}:user/emulator"));
}
