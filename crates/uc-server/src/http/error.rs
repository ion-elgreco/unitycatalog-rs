use axum::{
    Json,
    body::Body,
    http::{HeaderName, StatusCode, header},
    response::{IntoResponse, Response},
};
use tracing::error;

use uc_common::{Error, ErrorCode, ErrorResponse};

/// A `uc_common::Error` as a response: `IntoResponse` and the error are of
/// other crates, so the orphan rule needs a type of this crate.
pub struct ApiError(pub Error);

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    /// The UC error body. The response also carries the [`ErrorResponse`]
    /// as an extension, for the layers that answer in the Delta and Iceberg
    /// formats instead.
    fn into_response(self) -> Response {
        let status = self.0.code().status();
        let body = ErrorResponse::from(&self.0);
        if status.is_server_error() {
            error!(error = %self.0, "server error");
        }
        let mut response = (status, Json(body.clone())).into_response();
        response.extensions_mut().insert(body);
        response
    }
}

/// Result alias used by handlers.
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// Whether `path` is `mount` under `base`, or a path under it.
pub fn mounted(path: &str, base: &str, mount: &str) -> bool {
    path.strip_prefix(base)
        .and_then(|rest| rest.strip_prefix(mount))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// An error answer that is not in the format of the protocol of its
/// path: a UC error or a bare status, of sign-in, request parsing, an
/// unknown path or method, the request timeout or the rate limit.
#[derive(Debug)]
pub struct Stray {
    pub status: StatusCode,
    /// The code of a UC error.
    pub code: Option<ErrorCode>,
    /// The message of a UC error, else the text of the body, else the name
    /// of the status.
    pub message: String,
}

/// `resp`, or for an error answer the answer that `format` makes of it, in
/// the format of a protocol. An answer without a UC error whose body `own`
/// takes for the protocol's stays as it is. The new answer keeps the
/// headers of the old one, such as `Allow` and `Retry-After`, but for
/// those of the body.
pub async fn format_errors(
    resp: Response,
    own: impl FnOnce(&[u8]) -> bool,
    format: impl FnOnce(Stray) -> Response,
) -> Response {
    let status = resp.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return resp;
    }
    let (parts, body) = resp.into_parts();
    let stray = match parts.extensions.get::<ErrorResponse>() {
        Some(uc) => Stray {
            status,
            code: Some(uc.error_code),
            message: uc.message.clone(),
        },
        None => {
            let Ok(bytes) = axum::body::to_bytes(body, 1 << 20).await else {
                return Response::from_parts(parts, Body::empty());
            };
            if own(&bytes) {
                return Response::from_parts(parts, Body::from(bytes));
            }
            let text = String::from_utf8_lossy(&bytes).trim().to_string();
            let message = match text.is_empty() {
                true => status.canonical_reason().unwrap_or("Error").to_string(),
                false => text,
            };
            Stray {
                status,
                code: None,
                message,
            }
        }
    };
    let mut formatted = format(stray);
    let of_body = [
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CONTENT_ENCODING,
        header::TRANSFER_ENCODING,
    ];
    let own: Vec<HeaderName> = formatted.headers().keys().cloned().collect();
    for (name, value) in &parts.headers {
        if !of_body.contains(name) && !own.contains(name) {
            formatted.headers_mut().append(name, value.clone());
        }
    }
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_are_whole_path_segments() {
        assert!(mounted("/api/1.0/x/scim2", "/api/1.0/x", "/scim2"));
        assert!(mounted("/api/1.0/x/scim2/Users", "/api/1.0/x", "/scim2"));
        assert!(!mounted("/api/1.0/x/scim2x", "/api/1.0/x", "/scim2"));
        assert!(!mounted("/other/scim2", "/api/1.0/x", "/scim2"));
    }

    #[tokio::test]
    async fn formats_stray_errors_and_keeps_their_headers() {
        let text = |resp: Response| async move {
            let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
                .await
                .unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        };
        let format = |stray: Stray| {
            let code = stray
                .code
                .map(|code| format!("{code:?}"))
                .unwrap_or_default();
            (stray.status, format!("{}|{code}", stray.message)).into_response()
        };
        // A bare status with a body of text, and headers of its own.
        let mut bare = (
            StatusCode::TOO_MANY_REQUESTS,
            "Too Many Requests! Wait for 1s",
        )
            .into_response();
        bare.headers_mut()
            .insert(header::RETRY_AFTER, "1".parse().unwrap());
        for value in ["Origin", "Accept"] {
            bare.headers_mut()
                .append(header::VARY, value.parse().unwrap());
        }
        let formatted = format_errors(bare, |_| false, format).await;
        assert_eq!(formatted.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(formatted.headers()[header::RETRY_AFTER], "1");
        assert_eq!(formatted.headers().get_all(header::VARY).iter().count(), 2);
        assert_eq!(text(formatted).await, "Too Many Requests! Wait for 1s|");
        // A bare status without a body.
        let formatted = format_errors(
            StatusCode::METHOD_NOT_ALLOWED.into_response(),
            |_| false,
            format,
        )
        .await;
        assert_eq!(text(formatted).await, "Method Not Allowed|");
        // A UC error.
        let uc = ApiError(Error::not_found("no such thing")).into_response();
        let formatted = format_errors(uc, |_| true, format).await;
        assert_eq!(text(formatted).await, "no such thing|NotFound");
        // The protocol's own body, and an answer that is no error.
        let own = (StatusCode::BAD_REQUEST, "{}").into_response();
        assert_eq!(
            text(format_errors(own, |body| body == b"{}", format).await).await,
            "{}"
        );
        let fine = (StatusCode::OK, "fine").into_response();
        assert_eq!(
            text(format_errors(fine, |_| false, format).await).await,
            "fine"
        );
    }
}
