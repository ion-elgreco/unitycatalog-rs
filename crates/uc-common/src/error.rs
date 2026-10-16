use http::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error codes of the upstream Unity Catalog server
/// (`io.unitycatalog.server.exception.ErrorCode`).
///
/// The wire name (`error_code` in the JSON body) and the HTTP status of each
/// code match upstream. The resource-specific `*_ALREADY_EXISTS` codes use
/// HTTP 400, and the generic `ALREADY_EXISTS` uses 409.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidArgument,
    UnsupportedTableFormat,
    NotFound,
    CatalogNotFound,
    SchemaNotFound,
    TableNotFound,
    AlreadyExists,
    PermissionDenied,
    Unauthenticated,
    ResourceExhausted,
    FailedPrecondition,
    Aborted,
    CommitVersionConflict,
    UpdateRequirementConflict,
    OutOfRange,
    Unimplemented,
    Internal,
    DataLoss,
    ResourceAlreadyExists,
    CatalogAlreadyExists,
    SchemaAlreadyExists,
    TableAlreadyExists,
    StorageCredentialAlreadyExists,
    ExternalLocationAlreadyExists,
    CommitStateUnknown,
    UnprocessableEntity,
    /// Local-only: a dependency (the database) is not reachable. Used by the
    /// readiness probe; upstream has no equivalent code.
    Unavailable,
}

impl ErrorCode {
    /// HTTP status of the UC REST API for this code.
    pub fn status(self) -> StatusCode {
        match self {
            Self::InvalidArgument
            | Self::UnsupportedTableFormat
            | Self::FailedPrecondition
            | Self::OutOfRange
            | Self::ResourceAlreadyExists
            | Self::CatalogAlreadyExists
            | Self::SchemaAlreadyExists
            | Self::TableAlreadyExists
            | Self::StorageCredentialAlreadyExists
            | Self::ExternalLocationAlreadyExists => StatusCode::BAD_REQUEST,
            Self::NotFound | Self::CatalogNotFound | Self::SchemaNotFound | Self::TableNotFound => {
                StatusCode::NOT_FOUND
            }
            Self::AlreadyExists
            | Self::Aborted
            | Self::CommitVersionConflict
            | Self::UpdateRequirementConflict => StatusCode::CONFLICT,
            Self::PermissionDenied => StatusCode::FORBIDDEN,
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
            Self::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            Self::Internal | Self::DataLoss | Self::CommitStateUnknown => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Upstream `deltaErrorType`: the error `type` of the UC Delta API.
    pub fn delta_error_type(self) -> &'static str {
        match self {
            Self::InvalidArgument | Self::FailedPrecondition | Self::UnprocessableEntity => {
                "InvalidParameterValueException"
            }
            Self::UnsupportedTableFormat => "UnsupportedTableFormatException",
            Self::NotFound => "NotFoundException",
            Self::CatalogNotFound => "NoSuchCatalogException",
            Self::SchemaNotFound => "NoSuchSchemaException",
            Self::TableNotFound => "NoSuchTableException",
            Self::PermissionDenied => "PermissionDeniedException",
            Self::Unauthenticated => "NotAuthorizedException",
            Self::ResourceExhausted => "ResourceExhaustedException",
            Self::Aborted | Self::CommitVersionConflict => "CommitVersionConflictException",
            Self::UpdateRequirementConflict => "UpdateRequirementConflictException",
            Self::OutOfRange => "BadRequestException",
            Self::Unimplemented => "NotImplementedException",
            Self::Internal | Self::DataLoss | Self::Unavailable => "InternalServerErrorException",
            Self::CommitStateUnknown => "CommitStateUnknownException",
            Self::AlreadyExists
            | Self::ResourceAlreadyExists
            | Self::CatalogAlreadyExists
            | Self::SchemaAlreadyExists
            | Self::TableAlreadyExists
            | Self::StorageCredentialAlreadyExists
            | Self::ExternalLocationAlreadyExists => "AlreadyExistsException",
        }
    }

    /// Upstream `deltaHttpStatus`: the UC Delta API status. "Already
    /// exists" is always 409 there, and an unprocessable entity 400.
    pub fn delta_status(self) -> StatusCode {
        if self.is_already_exists() {
            return StatusCode::CONFLICT;
        }
        match self {
            Self::UnprocessableEntity => StatusCode::BAD_REQUEST,
            other => other.status(),
        }
    }

    /// Upstream `IcebergRestExceptionHandler.icebergExceptionFor`: the
    /// Iceberg exception a client raises for this code.
    pub fn iceberg_error_type(self) -> &'static str {
        match self {
            Self::CatalogNotFound | Self::SchemaNotFound => "NoSuchNamespaceException",
            Self::TableNotFound => "NoSuchTableException",
            Self::NotFound => "NotFoundException",
            Self::AlreadyExists
            | Self::ResourceAlreadyExists
            | Self::CatalogAlreadyExists
            | Self::SchemaAlreadyExists
            | Self::TableAlreadyExists
            | Self::StorageCredentialAlreadyExists
            | Self::ExternalLocationAlreadyExists => "AlreadyExistsException",
            Self::CommitVersionConflict | Self::UpdateRequirementConflict => {
                "CommitFailedException"
            }
            Self::Aborted | Self::ResourceExhausted => "RESTException",
            Self::InvalidArgument
            | Self::UnsupportedTableFormat
            | Self::FailedPrecondition
            | Self::OutOfRange => "BadRequestException",
            Self::UnprocessableEntity => "UnprocessableEntityException",
            Self::Unauthenticated => "NotAuthorizedException",
            Self::PermissionDenied => "ForbiddenException",
            Self::Unimplemented => "UnsupportedOperationException",
            Self::Internal | Self::DataLoss | Self::CommitStateUnknown | Self::Unavailable => {
                "ServiceFailureException"
            }
        }
    }

    /// The Iceberg REST status: the UC status, except that a schema or
    /// table that already exists is a 409 conflict there.
    pub fn iceberg_status(self) -> StatusCode {
        match self {
            Self::SchemaAlreadyExists | Self::TableAlreadyExists => StatusCode::CONFLICT,
            other => other.status(),
        }
    }

    /// The wire name, e.g. `CATALOG_NOT_FOUND`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::UnsupportedTableFormat => "UNSUPPORTED_TABLE_FORMAT",
            Self::NotFound => "NOT_FOUND",
            Self::CatalogNotFound => "CATALOG_NOT_FOUND",
            Self::SchemaNotFound => "SCHEMA_NOT_FOUND",
            Self::TableNotFound => "TABLE_NOT_FOUND",
            Self::AlreadyExists => "ALREADY_EXISTS",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::ResourceExhausted => "RESOURCE_EXHAUSTED",
            Self::FailedPrecondition => "FAILED_PRECONDITION",
            Self::Aborted => "ABORTED",
            Self::CommitVersionConflict => "COMMIT_VERSION_CONFLICT",
            Self::UpdateRequirementConflict => "UPDATE_REQUIREMENT_CONFLICT",
            Self::OutOfRange => "OUT_OF_RANGE",
            Self::Unimplemented => "UNIMPLEMENTED",
            Self::Internal => "INTERNAL",
            Self::DataLoss => "DATA_LOSS",
            Self::ResourceAlreadyExists => "RESOURCE_ALREADY_EXISTS",
            Self::CatalogAlreadyExists => "CATALOG_ALREADY_EXISTS",
            Self::SchemaAlreadyExists => "SCHEMA_ALREADY_EXISTS",
            Self::TableAlreadyExists => "TABLE_ALREADY_EXISTS",
            Self::StorageCredentialAlreadyExists => "STORAGE_CREDENTIAL_ALREADY_EXISTS",
            Self::ExternalLocationAlreadyExists => "EXTERNAL_LOCATION_ALREADY_EXISTS",
            Self::CommitStateUnknown => "COMMIT_STATE_UNKNOWN",
            Self::UnprocessableEntity => "UNPROCESSABLE_ENTITY",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    /// Parse a wire name. Returns `None` for unknown codes.
    pub fn from_wire(s: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }

    /// `true` for every "not found" flavour.
    pub fn is_not_found(self) -> bool {
        matches!(
            self,
            Self::NotFound | Self::CatalogNotFound | Self::SchemaNotFound | Self::TableNotFound
        )
    }

    /// `true` for every "already exists" flavour.
    pub fn is_already_exists(self) -> bool {
        matches!(
            self,
            Self::AlreadyExists
                | Self::ResourceAlreadyExists
                | Self::CatalogAlreadyExists
                | Self::SchemaAlreadyExists
                | Self::TableAlreadyExists
                | Self::StorageCredentialAlreadyExists
                | Self::ExternalLocationAlreadyExists
        )
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct Error {
    code: ErrorCode,
    message: String,
}

impl Error {
    pub fn new(code: ErrorCode, msg: impl Into<String>) -> Self {
        Self {
            code,
            message: msg.into(),
        }
    }
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidArgument, msg)
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, msg)
    }
    pub fn catalog_not_found(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::CatalogNotFound, msg)
    }
    pub fn schema_not_found(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::SchemaNotFound, msg)
    }
    pub fn table_not_found(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::TableNotFound, msg)
    }
    pub fn already_exists(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::AlreadyExists, msg)
    }
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::PermissionDenied, msg)
    }
    pub fn unauthenticated(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthenticated, msg)
    }
    pub fn failed_precondition(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::FailedPrecondition, msg)
    }
    pub fn aborted(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Aborted, msg)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, msg)
    }
    pub fn unimplemented(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unimplemented, msg)
    }
    pub fn unavailable(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unavailable, msg)
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Same error with a different code. Used where a generic code from a
    /// shared helper must become the resource-specific upstream code.
    pub fn with_code(self, code: ErrorCode) -> Self {
        Self {
            code,
            message: self.message,
        }
    }
}

/// One `google.rpc.ErrorInfo` entry of the `details` array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetail {
    #[serde(rename = "@type")]
    pub type_url: String,
    pub reason: String,
}

/// The `@type` URL upstream puts in every error detail.
pub const ERROR_INFO_TYPE: &str = "type.googleapis.com/google.rpc.ErrorInfo";

/// JSON body returned to clients on error. Same shape as upstream's
/// `GlobalExceptionHandler`: `error_code`, `message` and a one-element
/// `details` array with a `google.rpc.ErrorInfo` whose `reason` repeats the
/// code.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error_code: ErrorCode,
    pub message: String,
    #[serde(default)]
    pub details: Vec<ErrorDetail>,
}

impl From<&Error> for ErrorResponse {
    fn from(e: &Error) -> Self {
        Self {
            error_code: e.code(),
            message: e.message().to_string(),
            details: vec![ErrorDetail {
                type_url: ERROR_INFO_TYPE.to_string(),
                reason: e.code().as_str().to_string(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_round_trip() {
        for code in [
            ErrorCode::InvalidArgument,
            ErrorCode::CatalogNotFound,
            ErrorCode::StorageCredentialAlreadyExists,
            ErrorCode::UpdateRequirementConflict,
            ErrorCode::UnprocessableEntity,
        ] {
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{}\"", code.as_str()));
            assert_eq!(ErrorCode::from_wire(code.as_str()), Some(code));
        }
    }

    #[test]
    fn statuses_match_upstream() {
        assert_eq!(ErrorCode::AlreadyExists.status(), StatusCode::CONFLICT);
        assert_eq!(
            ErrorCode::CatalogAlreadyExists.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorCode::FailedPrecondition.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(ErrorCode::Aborted.status(), StatusCode::CONFLICT);
        assert_eq!(
            ErrorCode::ResourceExhausted.status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(ErrorCode::TableNotFound.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn response_has_error_info_detail() {
        let body = ErrorResponse::from(&Error::catalog_not_found("Catalog not found: x"));
        let v = serde_json::to_value(&body).unwrap();
        assert_eq!(v["error_code"], "CATALOG_NOT_FOUND");
        assert_eq!(v["details"][0]["@type"], ERROR_INFO_TYPE);
        assert_eq!(v["details"][0]["reason"], "CATALOG_NOT_FOUND");
    }
}
