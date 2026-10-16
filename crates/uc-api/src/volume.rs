use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum VolumeType {
    Managed,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVolumeRequestContent {
    pub catalog_name: String,
    pub schema_name: String,
    pub name: String,
    pub volume_type: VolumeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Required for EXTERNAL volumes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateVolumeRequestContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct VolumeInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_type: Option<VolumeType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListVolumesResponseContent {
    pub volumes: Vec<VolumeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_wire_names;

    #[test]
    fn volume_types_match_upstream() {
        assert_wire_names(&[
            (VolumeType::Managed, "MANAGED"),
            (VolumeType::External, "EXTERNAL"),
        ]);
    }
}
