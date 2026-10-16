use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GetMetastoreSummaryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metastore_id: Option<String>,
}
