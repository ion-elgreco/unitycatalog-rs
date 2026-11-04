//! A fake AWS STS for tests: it answers `GetCallerIdentity` with the ARN of
//! the access key id in the request's `Authorization`, as STS does after it
//! checks the signature. It checks the request's shape, not its signature,
//! and counts the requests.

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use serde_json::json;

pub const GET_CALLER_IDENTITY: &str = "Action=GetCallerIdentity&Version=2011-06-15";

#[derive(Clone, Default)]
struct Served {
    /// The ARN of each access key id.
    arns: Arc<Mutex<HashMap<String, String>>>,
    calls: Arc<AtomicUsize>,
}

pub struct Sts {
    /// Where it listens, such as `http://127.0.0.1:4566`.
    pub endpoint: String,
    served: Served,
}

/// A signed request to make into a token: what tests change to tamper.
pub struct Request {
    pub url: String,
    pub key_id: String,
    pub server_id: String,
    /// Whether the server id header is in `SignedHeaders`.
    pub server_id_signed: bool,
    pub date: chrono::DateTime<Utc>,
    pub body: String,
}

impl Sts {
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let served = Served::default();
        let app = Router::new()
            .route("/", post(answer))
            .with_state(served.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { endpoint, served }
    }

    /// The access key `key_id` is `arn`.
    pub fn identity(&self, key_id: &str, arn: &str) {
        (self.served.arns.lock().unwrap()).insert(key_id.into(), arn.into());
    }

    /// How many requests it answered.
    pub fn calls(&self) -> usize {
        self.served.calls.load(Ordering::SeqCst)
    }

    /// A request of `key_id` for the server `server_id`, signed now.
    pub fn request(&self, key_id: &str, server_id: &str) -> Request {
        Request {
            url: format!("{}/", self.endpoint),
            key_id: key_id.into(),
            server_id: server_id.into(),
            server_id_signed: true,
            date: Utc::now(),
            body: GET_CALLER_IDENTITY.into(),
        }
    }

    /// The bearer value of a request of `key_id` for `server_id`.
    pub fn token(&self, key_id: &str, server_id: &str) -> String {
        self.request(key_id, server_id).token()
    }
}

impl Request {
    /// The bearer value: the request as base64url JSON after `uc-aws-v1.`.
    pub fn token(&self) -> String {
        let date = self.date.format("%Y%m%dT%H%M%SZ").to_string();
        let signed = match self.server_id_signed {
            true => "content-type;host;x-amz-date;x-uc-server-id",
            false => "content-type;host;x-amz-date",
        };
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}/us-east-1/sts/aws4_request, \
             SignedHeaders={signed}, Signature=00",
            self.key_id,
            &date[..8]
        );
        let request = json!({
            "method": "POST",
            "url": self.url,
            "headers": {
                "Authorization": authorization,
                "Content-Type": "application/x-www-form-urlencoded; charset=utf-8",
                "X-Amz-Date": date,
                "X-Uc-Server-Id": self.server_id,
            },
            "body": self.body,
        });
        format!("uc-aws-v1.{}", URL_SAFE_NO_PAD.encode(request.to_string()))
    }
}

async fn answer(
    State(served): State<Served>,
    headers: HeaderMap,
    body: String,
) -> (StatusCode, String) {
    served.calls.fetch_add(1, Ordering::SeqCst);
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
    };
    let key_id = header("authorization")
        .split_once("Credential=")
        .and_then(|(_, rest)| rest.split('/').next())
        .unwrap_or("");
    let arn = served.arns.lock().unwrap().get(key_id).cloned();
    let signed = header("authorization").contains("x-uc-server-id")
        && !header("x-uc-server-id").is_empty()
        && !header("x-amz-date").is_empty();
    match arn {
        Some(arn) if body == GET_CALLER_IDENTITY && signed => {
            let account = arn.split(':').nth(4).unwrap_or("").to_string();
            let xml = format!(
                "<GetCallerIdentityResponse xmlns=\"https://sts.amazonaws.com/doc/2011-06-15/\">\
                 <GetCallerIdentityResult><Arn>{arn}</Arn><UserId>AROAEXAMPLE:{key_id}</UserId>\
                 <Account>{account}</Account></GetCallerIdentityResult>\
                 </GetCallerIdentityResponse>"
            );
            (StatusCode::OK, xml)
        }
        _ => (
            StatusCode::FORBIDDEN,
            "<ErrorResponse><Error><Code>SignatureDoesNotMatch</Code></Error></ErrorResponse>"
                .into(),
        ),
    }
}
