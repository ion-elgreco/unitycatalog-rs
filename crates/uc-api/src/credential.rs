use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CredentialPurpose {
    ServicePrincipal,
    Storage,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AwsIamRoleRequest {
    pub role_arn: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AwsIamRoleResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_arn: Option<String>,
    /// The server's own AWS identity, which assumes `role_arn`
    /// (`identity.aws.arn`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unity_catalog_iam_arn: Option<String>,
    /// Generated each time the role is set; the role's trust policy should
    /// require it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AzureServicePrincipalRequest {
    pub directory_id: String,
    pub application_id: String,
    pub client_secret: String,
}

/// An Azure service principal, without its client secret.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AzureServicePrincipalResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,
}

/// The server's own Azure identity, or the user-assigned managed identity
/// with the resource ID `managed_identity_id`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AzureManagedIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_identity_id: Option<String>,
}

/// The server's own GCP identity, or the service account `email` that the
/// server impersonates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct GcpServiceAccount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GcpServiceAccountKeyRequest {
    /// The JSON key file of the service account.
    pub json_key: String,
}

/// A GCP service account key, without the key.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GcpServiceAccountKeyResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AwsCredentials {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AzureUserDelegationSAS {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sas_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GcpOauthToken {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TemporaryCredentials {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_temp_credentials: Option<AwsCredentials>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_user_delegation_sas: Option<AzureUserDelegationSAS>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_oauth_token: Option<GcpOauthToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_time: Option<EpochMillis>,
    /// The storage path the credentials are for. Clients such as the
    /// Hadoop connector use it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CredentialInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_iam_role: Option<AwsIamRoleResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<CredentialPurpose>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_service_principal: Option<AzureServicePrincipalResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_managed_identity: Option<AzureManagedIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_service_account: Option<GcpServiceAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_service_account_key: Option<GcpServiceAccountKeyResponse>,
}

/// A credential has exactly one of `aws_iam_role`, `azure_service_principal`,
/// `azure_managed_identity`, `gcp_service_account` and
/// `gcp_service_account_key`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCredentialRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_iam_role: Option<AwsIamRoleRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<CredentialPurpose>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_service_principal: Option<AzureServicePrincipalRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_managed_identity: Option<AzureManagedIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_service_account: Option<GcpServiceAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_service_account_key: Option<GcpServiceAccountKeyRequest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateCredentialRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Accepted and ignored, as upstream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_iam_role: Option<AwsIamRoleRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
    /// With `aws_iam_role` and the other identities: at most one, which
    /// replaces the identity of the credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_service_principal: Option<AzureServicePrincipalRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_managed_identity: Option<AzureManagedIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_service_account: Option<GcpServiceAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_service_account_key: Option<GcpServiceAccountKeyRequest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListCredentialsResponse {
    pub credentials: Vec<CredentialInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TableOperation {
    UnknownTableOperation,
    Read,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VolumeOperation {
    UnknownVolumeOperation,
    ReadVolume,
    WriteVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelVersionOperation {
    UnknownModelVersionOperation,
    ReadModelVersion,
    ReadWriteModelVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PathOperation {
    UnknownPathOperation,
    PathRead,
    PathReadWrite,
    PathCreateTable,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateTemporaryTableCredential {
    pub table_id: String,
    pub operation: TableOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateTemporaryVolumeCredential {
    pub volume_id: String,
    pub operation: VolumeOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateTemporaryModelVersionCredential {
    pub catalog_name: String,
    pub schema_name: String,
    pub model_name: String,
    pub version: i64,
    pub operation: ModelVersionOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateTemporaryPathCredential {
    pub url: String,
    pub operation: PathOperation,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_wire_names;

    #[test]
    fn enums_match_upstream() {
        // Upstream has only `STORAGE`.
        assert_wire_names(&[
            (CredentialPurpose::ServicePrincipal, "SERVICE_PRINCIPAL"),
            (CredentialPurpose::Storage, "STORAGE"),
        ]);
        assert_wire_names(&[
            (
                TableOperation::UnknownTableOperation,
                "UNKNOWN_TABLE_OPERATION",
            ),
            (TableOperation::Read, "READ"),
            (TableOperation::ReadWrite, "READ_WRITE"),
        ]);
        assert_wire_names(&[
            (
                VolumeOperation::UnknownVolumeOperation,
                "UNKNOWN_VOLUME_OPERATION",
            ),
            (VolumeOperation::ReadVolume, "READ_VOLUME"),
            (VolumeOperation::WriteVolume, "WRITE_VOLUME"),
        ]);
        assert_wire_names(&[
            (
                ModelVersionOperation::UnknownModelVersionOperation,
                "UNKNOWN_MODEL_VERSION_OPERATION",
            ),
            (
                ModelVersionOperation::ReadModelVersion,
                "READ_MODEL_VERSION",
            ),
            (
                ModelVersionOperation::ReadWriteModelVersion,
                "READ_WRITE_MODEL_VERSION",
            ),
        ]);
        assert_wire_names(&[
            (
                PathOperation::UnknownPathOperation,
                "UNKNOWN_PATH_OPERATION",
            ),
            (PathOperation::PathRead, "PATH_READ"),
            (PathOperation::PathReadWrite, "PATH_READ_WRITE"),
            (PathOperation::PathCreateTable, "PATH_CREATE_TABLE"),
        ]);
    }
}
