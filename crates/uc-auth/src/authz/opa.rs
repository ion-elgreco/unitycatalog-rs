use std::fmt;
use std::time::Duration;

use reqwest::header::CONTENT_TYPE;
use reqwest::{Url, redirect};
use serde_json::{Value, json};

/// The largest answer that [`OpaClient`] reads.
const MAX_ANSWER: usize = 64 * 1024;

/// A client of an Open Policy Agent (OPA) server.
///
/// It posts `{"input": …}` to a decision URL of OPA's data API, such as
/// `http://opa:8181/v1/data/uc/authz`, and reads the boolean `result.allow`
/// of the answer.
///
/// It follows no redirects. The answer must come within the timeout and be
/// at most 64 KiB. Any other answer is an error, not a denial: the caller
/// decides what a failed decision means.
#[derive(Clone)]
pub struct OpaClient {
    client: reqwest::Client,
    url: Url,
    timeout: Duration,
}

impl fmt::Debug for OpaClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpaClient")
            .field("url", &self.url.as_str())
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl OpaClient {
    /// A client of the decision URL `url`: an absolute `http` or `https`
    /// URL with a host, and without credentials or a fragment. `timeout` is
    /// for each decision. An error does not show the URL, which can hold a
    /// secret in its query.
    pub fn new(url: &str, timeout: Duration) -> Result<Self, String> {
        let invalid = || {
            "must be an absolute http or https URL without credentials or a fragment".to_string()
        };
        let url = Url::parse(url).map_err(|_| invalid())?;
        let valid = matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some_and(|host| !host.is_empty())
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none();
        if !valid {
            return Err(invalid());
        }
        if timeout.is_zero() {
            return Err("the timeout must be more than zero".into());
        }
        let client = reqwest::Client::builder()
            .redirect(redirect::Policy::none())
            .build()
            .map_err(|e| format!("cannot make the HTTP client: {e}"))?;
        Ok(Self {
            client,
            url,
            timeout,
        })
    }

    /// The decision URL.
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    /// OPA's `result.allow` for `input`.
    pub async fn allow(&self, input: &Value) -> Result<bool, String> {
        let body = serde_json::to_vec(&json!({ "input": input }))
            .map_err(|e| format!("encoding the input: {e}"))?;
        let mut response = self
            .client
            .post(self.url.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| self.failed("OPA is unreachable", e))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("OPA answered HTTP {}", status.as_u16()));
        }
        let too_large = || format!("OPA's answer is larger than {MAX_ANSWER} bytes");
        if response
            .content_length()
            .is_some_and(|length| length > MAX_ANSWER as u64)
        {
            return Err(too_large());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) =
            (response.chunk().await).map_err(|e| self.failed("reading OPA's answer failed", e))?
        {
            if bytes.len() + chunk.len() > MAX_ANSWER {
                return Err(too_large());
            }
            bytes.extend_from_slice(&chunk);
        }
        let answer: Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("OPA's answer is not JSON: {e}"))?;
        answer
            .pointer("/result/allow")
            .and_then(Value::as_bool)
            .ok_or_else(|| "OPA's answer has no boolean result.allow".to_string())
    }

    /// The message of a failed call, with its causes and without the URL.
    fn failed(&self, what: &str, error: reqwest::Error) -> String {
        if error.is_timeout() {
            return format!("OPA did not answer within {:?}", self.timeout);
        }
        let error = error.without_url();
        let mut message = format!("{what}: {error}");
        let mut source = std::error::Error::source(&error);
        while let Some(cause) = source {
            message.push_str(&format!(": {cause}"));
            source = cause.source();
        }
        message
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        http::{StatusCode, header},
        response::IntoResponse,
        routing::post,
    };

    use super::*;

    /// An OPA whose answers depend on the path: `/v1/data/allow` allows
    /// the input whose `privilege` is `SELECT`, the others answer badly.
    async fn opa() -> String {
        let app = Router::new()
            .route(
                "/v1/data/allow",
                post(|Json(body): Json<Value>| async move {
                    let allow = body["input"]["privilege"] == "SELECT";
                    Json(json!({ "result": { "allow": allow } }))
                }),
            )
            .route("/v1/data/undefined", post(|| async { Json(json!({})) }))
            .route(
                "/v1/data/text",
                post(|| async { Json(json!({ "result": { "allow": "true" } })) }),
            )
            .route(
                "/v1/data/trailing",
                post(|| async { r#"{"result":{"allow":true}} {}"# }),
            )
            .route(
                "/v1/data/error",
                post(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
            )
            .route(
                "/v1/data/redirect",
                post(|| async {
                    (
                        StatusCode::TEMPORARY_REDIRECT,
                        [(header::LOCATION, "/v1/data/allow")],
                    )
                        .into_response()
                }),
            )
            .route(
                "/v1/data/large",
                post(|| async {
                    let padding = "x".repeat(MAX_ANSWER);
                    Json(json!({ "result": { "allow": true, "padding": padding } }))
                }),
            )
            .route(
                "/v1/data/slow",
                post(|| async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Json(json!({ "result": { "allow": true } }))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}/v1/data")
    }

    fn client(base: &str, path: &str) -> OpaClient {
        OpaClient::new(&format!("{base}/{path}"), Duration::from_secs(2)).unwrap()
    }

    #[test]
    fn takes_only_plain_http_urls() {
        let ok = |url: &str| OpaClient::new(url, Duration::from_secs(1)).is_ok();
        assert!(ok("http://opa:8181/v1/data/uc/authz"));
        assert!(ok("https://opa.example.com/v1/data/uc/authz?pretty"));
        for url in [
            "opa:8181/v1/data/uc/authz",
            "/v1/data/uc/authz",
            "ftp://opa/v1/data/uc/authz",
            "http://user:secret@opa/v1/data/uc/authz",
            "http://opa/v1/data/uc/authz#allow",
            "file:///v1/data/uc/authz",
        ] {
            let error = OpaClient::new(url, Duration::from_secs(1)).unwrap_err();
            assert!(
                !error.contains("secret") && !error.contains("opa/"),
                "{error}"
            );
        }
        assert!(OpaClient::new("http://opa/v1/data/uc/authz", Duration::ZERO).is_err());
    }

    #[tokio::test]
    async fn reads_the_decision() {
        let base = opa().await;
        let opa = client(&base, "allow");
        assert_eq!(opa.allow(&json!({ "privilege": "SELECT" })).await, Ok(true));
        assert_eq!(
            opa.allow(&json!({ "privilege": "MODIFY" })).await,
            Ok(false)
        );
    }

    #[tokio::test]
    async fn any_other_answer_is_an_error() {
        let base = opa().await;
        let input = json!({ "privilege": "SELECT" });
        for (path, expected) in [
            ("undefined", "no boolean result.allow"),
            ("text", "no boolean result.allow"),
            ("trailing", "not JSON"),
            ("error", "HTTP 500"),
            ("redirect", "HTTP 307"),
            ("large", "larger than"),
            ("missing", "HTTP 404"),
        ] {
            let error = client(&base, path).allow(&input).await.unwrap_err();
            assert!(error.contains(expected), "{path}: {error}");
        }
    }

    #[tokio::test]
    async fn fails_after_the_timeout() {
        let base = opa().await;
        let opa = OpaClient::new(&format!("{base}/slow"), Duration::from_millis(100)).unwrap();
        let error = opa.allow(&json!({})).await.unwrap_err();
        assert!(error.contains("did not answer within"), "{error}");
        let down = OpaClient::new("http://127.0.0.1:9/v1/data/uc", Duration::from_secs(1)).unwrap();
        let error = down.allow(&json!({})).await.unwrap_err();
        assert!(error.starts_with("OPA is unreachable"), "{error}");
        assert!(!error.contains("127.0.0.1:9"), "{error}");
    }
}
