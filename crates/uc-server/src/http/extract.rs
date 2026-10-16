//! Request extractors that report bad input the way upstream does.
//!
//! axum's own `Json`, `Query` and `Path` reject bad input with plain-text
//! 400/415/422 responses. Upstream (Armeria + Jackson) turns every
//! unreadable body, query or path parameter into an `INVALID_ARGUMENT`
//! error with the usual JSON envelope and HTTP 400. These wrappers do the
//! same. `Json` also serves as the response type, so route modules import
//! it from here instead of from axum.

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Request},
    http::request::Parts,
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};

use uc_common::Error;

use crate::http::error::ApiError;

/// JSON request body (any content type) and JSON response.
#[derive(Debug, Clone, Copy, Default)]
pub struct Json<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| match e.status() {
                // Over the request size limit: the plain 413, as Armeria.
                axum::http::StatusCode::PAYLOAD_TOO_LARGE => e.into_response(),
                _ => ApiError(Error::invalid_argument(format!(
                    "Failed to read request body: {e}"
                )))
                .into_response(),
            })?;
        parse_json_body(&bytes, true)
            .map(Json)
            .map_err(|e| ApiError(e).into_response())
    }
}

/// Parse a request body as JSON, with upstream-style error messages;
/// `quote_parser` adds the parser's own message.
pub fn parse_json_body<T: DeserializeOwned>(bytes: &[u8], quote_parser: bool) -> Result<T, Error> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Err(Error::invalid_argument(
            "Malformed request body: no content",
        ));
    }
    serde_json::from_slice(bytes).map_err(|e| {
        let kind = if e.is_syntax() || e.is_eof() {
            "not valid JSON"
        } else {
            "not the structure this endpoint accepts"
        };
        match quote_parser {
            true => Error::invalid_argument(format!("Malformed request body: {kind}: {e}")),
            false => Error::invalid_argument(format!("Malformed request body: {kind}")),
        }
    })
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// Query string, deserialized with `serde_urlencoded`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(v)| Query(v))
            .map_err(|rejection| ApiError(Error::invalid_argument(rejection.body_text())))
    }
}

/// Path parameters.
#[derive(Debug, Clone, Copy, Default)]
pub struct Path<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Path(v)| Path(v))
            .map_err(|rejection| ApiError(Error::invalid_argument(rejection.body_text())))
    }
}
