use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Whether a securable is protected against deletes. A protected securable
/// cannot be deleted, also not with the delete of its catalog or schema,
/// until its protection is removed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Protection {
    pub protected: bool,
    /// When the protection was set, in epoch milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected_at: Option<i64>,
    /// Who set the protection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateProtection {
    /// `true` to protect the securable, `false` to remove its protection.
    pub protected: bool,
}
