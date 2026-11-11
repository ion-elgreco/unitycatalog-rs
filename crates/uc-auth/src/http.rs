use async_trait::async_trait;

/// How the resolvers send their requests to identity providers (key sets,
/// STS). A host that runs its outbound connections on a runtime of their
/// own sends them there.
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    async fn send(&self, request: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response>;
}

/// Sends each request on the caller's runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct Direct;

#[async_trait]
impl Transport for Direct {
    async fn send(&self, request: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
        request.send().await
    }
}

/// Whether an HTTP 4xx `status` says nothing of the request: it is of a
/// proxy on the way (407, 408), or of a service that has too much to do
/// (429).
pub(crate) fn no_answer(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 407 | 408 | 429)
}
