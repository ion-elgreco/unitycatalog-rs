use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::EpochMillis;

/// One audit-log entry per mutating request. Recorded asynchronously by the
/// `audit_layer` tower middleware on the way out.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AuditEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Authenticated principal username (or `"anonymous"`).
    pub principal: String,
    /// HTTP method: `POST` / `PATCH` / `DELETE`.
    pub method: String,
    /// Request path (including query string).
    pub path: String,
    /// Final HTTP status code returned to the client.
    pub status: u16,
    /// Wall-clock time when the response was sent.
    pub timestamp: EpochMillis,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListAuditEventsResponse {
    pub events: Vec<AuditEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<uc_common::PageToken>,
}
