use axum::{
    Json,
    response::{IntoResponse, Response},
};
use tracing::error;

use uc_common::{Error, ErrorResponse};

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
