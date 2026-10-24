// @generated — do not edit by hand.
use reqwest::{Method, RequestBuilder};
use url::Url;

/// The HTTP transport of the generated clients: a `reqwest::Client`, plus a bearer token
/// that goes in every request when set.
#[derive(Clone, Default)]
pub struct HttpClient {
    client: reqwest::Client,
    token: Option<String>,
}

impl ::std::fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .finish_non_exhaustive()
    }
}

impl HttpClient {
    /// A transport that sends its requests with `client`, without a token.
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            token: None,
        }
    }

    /// The same transport, sending `token` as the bearer token of every request.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Start a `GET` request.
    pub fn get(&self, url: Url) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    /// Start a `POST` request.
    pub fn post(&self, url: Url) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Start a `PUT` request.
    pub fn put(&self, url: Url) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    /// Start a `PATCH` request.
    pub fn patch(&self, url: Url) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// Start a `DELETE` request.
    pub fn delete(&self, url: Url) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Start a `HEAD` request.
    pub fn head(&self, url: Url) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Start a request with any method.
    pub fn request(&self, method: Method, url: Url) -> RequestBuilder {
        let builder = self.client.request(method, url);
        match &self.token {
            Some(token) => builder.bearer_auth(token),
            None => builder,
        }
    }
}
