// @generated — do not edit by hand.
use reqwest::Method;
use std::sync::Arc;
use url::Url;

/// The error of a [`TokenSource`].
pub type TokenError = Box<dyn ::std::error::Error + Send + Sync>;
/// The future of a token.
#[cfg(not(target_arch = "wasm32"))]
pub type TokenFuture<'a> =
    ::futures::future::BoxFuture<'a, ::std::result::Result<String, TokenError>>;
/// The future of a token.
#[cfg(target_arch = "wasm32")]
pub type TokenFuture<'a> =
    ::futures::future::LocalBoxFuture<'a, ::std::result::Result<String, TokenError>>;
/// Where the bearer token of each request comes from. The source is asked at every
/// request, so one that fetches tokens keeps them and refreshes them itself.
pub trait TokenSource: ::std::fmt::Debug + Send + Sync {
    /// The token to send now. [`NoToken`] sends the request without one.
    fn token(&self) -> TokenFuture<'_>;
}

/// The error of a source that has no token for a request: the request goes without
/// one.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoToken;
impl ::std::fmt::Display for NoToken {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str("no token for the request")
    }
}

impl ::std::error::Error for NoToken {}
/// A token that does not change.
pub struct StaticToken(pub String);
impl ::std::fmt::Debug for StaticToken {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.debug_tuple("StaticToken").field(&"<redacted>").finish()
    }
}

impl TokenSource for StaticToken {
    fn token(&self) -> TokenFuture<'_> {
        let token = self.0.clone();
        Box::pin(async move { Ok(token) })
    }
}

/// The HTTP transport of the generated clients: a `reqwest::Client`, plus a bearer token
/// source and a `User-Agent` that go in every request when set.
#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    token: Option<Arc<dyn TokenSource>>,
    user_agent: Option<String>,
}

impl Default for HttpClient {
    /// A transport with a default `reqwest::Client`. Outside the browser, which sends its
    /// own, its `User-Agent` is `uc-client/` and the version of this crate.
    fn default() -> Self {
        let client = Self::new(reqwest::Client::default());
        #[cfg(not(target_arch = "wasm32"))]
        let client = client.with_user_agent(concat!("uc-client/", env!("CARGO_PKG_VERSION")));
        client
    }
}

impl ::std::fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("user_agent", &self.user_agent)
            .finish_non_exhaustive()
    }
}

impl HttpClient {
    /// A transport over `client`, without a token or `User-Agent`.
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            token: None,
            user_agent: None,
        }
    }

    /// The same transport, sending `token` as the bearer token of every request.
    pub fn with_token(self, token: impl Into<String>) -> Self {
        self.with_token_source(Arc::new(StaticToken(token.into())))
    }

    /// The same transport, sending the token of `source` as the bearer token of every
    /// request. The source is asked at every request.
    pub fn with_token_source(mut self, source: Arc<dyn TokenSource>) -> Self {
        self.token = Some(source);
        self
    }

    /// The same transport, sending `user_agent` as the `User-Agent` of every request.
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
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
        let mut builder = self.client.request(method, url);
        if let Some(user_agent) = &self.user_agent {
            builder = builder.header(reqwest::header::USER_AGENT, user_agent);
        }
        RequestBuilder {
            builder,
            token: self.token.clone(),
        }
    }
}

/// A request being built: the methods of `reqwest::RequestBuilder` that the generated
/// clients use, and [`send`](Self::send), which gets the bearer token first.
pub struct RequestBuilder {
    builder: reqwest::RequestBuilder,
    token: Option<Arc<dyn TokenSource>>,
}

impl RequestBuilder {
    /// A JSON body.
    pub fn json<T: ::serde::Serialize + ?Sized>(mut self, json: &T) -> Self {
        self.builder = self.builder.json(json);
        self
    }

    /// A form-encoded body.
    pub fn form<T: ::serde::Serialize + ?Sized>(mut self, form: &T) -> Self {
        self.builder = self.builder.form(form);
        self
    }

    /// Query parameters, added to the URL.
    pub fn query<T: ::serde::Serialize + ?Sized>(mut self, query: &T) -> Self {
        self.builder = self.builder.query(query);
        self
    }

    /// A header. An invalid name or value fails the request at `send`.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.builder = self.builder.header(name, value);
        self
    }

    /// A raw body.
    pub fn body(mut self, body: impl Into<reqwest::Body>) -> Self {
        self.builder = self.builder.body(body);
        self
    }

    /// Change the request with `f`, for what the methods above do not cover.
    pub fn map(
        mut self,
        f: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    ) -> Self {
        self.builder = f(self.builder);
        self
    }

    /// Send the request. The bearer token comes from the token source first; a source
    /// that fails fails the request before it is sent, unless it has [`NoToken`].
    pub async fn send(self) -> ::std::result::Result<reqwest::Response, SendError> {
        let mut builder = self.builder;
        if let Some(source) = &self.token {
            match source.token().await {
                Ok(token) => builder = builder.bearer_auth(token),
                Err(e) if e.is::<NoToken>() => {}
                Err(e) => return Err(SendError::Token(e)),
            }
        }
        builder.send().await.map_err(SendError::Http)
    }
}

/// Why a request got no answer.
#[derive(Debug)]
pub enum SendError {
    /// The request failed.
    Http(reqwest::Error),
    /// The token source failed, so the request was not sent.
    Token(TokenError),
}

impl ::std::fmt::Display for SendError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            SendError::Http(e) => ::std::fmt::Display::fmt(e, f),
            SendError::Token(e) => write!(f, "no bearer token: {e}"),
        }
    }
}

impl ::std::error::Error for SendError {
    fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
        match self {
            SendError::Http(e) => Some(e),
            SendError::Token(e) => Some(e.as_ref()),
        }
    }
}
