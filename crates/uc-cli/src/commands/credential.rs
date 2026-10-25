//! `uc credential`: create, list, get, update and delete storage
//! credentials.

use clap::builder::BoolishValueParser;
use clap::{ArgGroup, Subcommand};
use uc_client::models::credentials::v1::{
    AwsIamRoleRequest, AzureManagedIdentity, AzureServicePrincipalRequest, CredentialPurpose,
};

use super::options::{Paging, changes, help};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum CredentialOp {
    /// Create a storage credential with one identity: an AWS IAM role, an
    /// Azure service principal or an Azure managed identity
    #[command(group = ArgGroup::new("identity").required(true))]
    Create {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
        #[arg(long, help = help::AWS_IAM_ROLE_ARN, group = "identity")]
        aws_iam_role_arn: Option<String>,
        #[arg(long, help = help::AZURE_DIRECTORY_ID, group = "identity",
            requires_all = ["azure_application_id", "azure_client_secret"])]
        azure_directory_id: Option<String>,
        #[arg(long, help = help::AZURE_APPLICATION_ID, requires = "azure_directory_id")]
        azure_application_id: Option<String>,
        #[arg(long, help = help::AZURE_CLIENT_SECRET, requires = "azure_directory_id")]
        azure_client_secret: Option<String>,
        #[arg(long, help = help::AZURE_MANAGED_IDENTITY, group = "identity",
            value_name = "RESOURCE_ID", num_args = 0..=1)]
        azure_managed_identity: Option<Option<String>>,
    },
    /// List storage credentials
    List {
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a storage credential
    Get {
        #[arg(long, help = help::NAME)]
        name: String,
    },
    /// Update a storage credential. A new identity replaces the old one
    #[command(group = changes(), group = ArgGroup::new("identity"))]
    Update {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::NEW_NAME, group = "changes")]
        new_name: Option<String>,
        #[arg(long, help = help::COMMENT, group = "changes")]
        comment: Option<String>,
        #[arg(long, help = help::AWS_IAM_ROLE_ARN, groups = ["changes", "identity"])]
        aws_iam_role_arn: Option<String>,
        #[arg(long, help = help::AZURE_DIRECTORY_ID, groups = ["changes", "identity"],
            requires_all = ["azure_application_id", "azure_client_secret"])]
        azure_directory_id: Option<String>,
        #[arg(long, help = help::AZURE_APPLICATION_ID, requires = "azure_directory_id")]
        azure_application_id: Option<String>,
        #[arg(long, help = help::AZURE_CLIENT_SECRET, requires = "azure_directory_id")]
        azure_client_secret: Option<String>,
        #[arg(long, help = help::AZURE_MANAGED_IDENTITY, groups = ["changes", "identity"],
            value_name = "RESOURCE_ID", num_args = 0..=1)]
        azure_managed_identity: Option<Option<String>>,
    },
    /// Delete a storage credential
    Delete {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::FORCE, value_name = "BOOL", num_args = 0..=1,
            default_missing_value = "true", value_parser = BoolishValueParser::new())]
        force: Option<bool>,
    },
}

/// The identity of a credential, as the options give it: at most one of
/// the three, which clap checks.
struct Identity {
    aws_iam_role: Option<AwsIamRoleRequest>,
    azure_service_principal: Option<AzureServicePrincipalRequest>,
    azure_managed_identity: Option<AzureManagedIdentity>,
}

impl Identity {
    fn new(
        aws_iam_role_arn: Option<String>,
        azure_directory_id: Option<String>,
        azure_application_id: Option<String>,
        azure_client_secret: Option<String>,
        azure_managed_identity: Option<Option<String>>,
    ) -> Self {
        Self {
            aws_iam_role: aws_iam_role_arn.map(|role_arn| AwsIamRoleRequest {
                role_arn,
                ..Default::default()
            }),
            azure_service_principal: azure_directory_id.map(|directory_id| {
                AzureServicePrincipalRequest {
                    directory_id,
                    application_id: azure_application_id.unwrap_or_default(),
                    client_secret: azure_client_secret.unwrap_or_default(),
                    ..Default::default()
                }
            }),
            azure_managed_identity: azure_managed_identity.map(|managed_identity_id| {
                AzureManagedIdentity {
                    managed_identity_id,
                    ..Default::default()
                }
            }),
        }
    }
}

pub async fn run(c: &Context<'_>, op: CredentialOp) -> Result<String> {
    match op {
        CredentialOp::Create {
            name,
            comment,
            aws_iam_role_arn,
            azure_directory_id,
            azure_application_id,
            azure_client_secret,
            azure_managed_identity,
        } => {
            let identity = Identity::new(
                aws_iam_role_arn,
                azure_directory_id,
                azure_application_id,
                azure_client_secret,
                azure_managed_identity,
            );
            let info = c
                .client
                .create_credential(name)
                .with_comment(comment)
                .with_purpose(CredentialPurpose::Storage)
                .with_aws_iam_role(identity.aws_iam_role)
                .with_azure_service_principal(identity.azure_service_principal)
                .with_azure_managed_identity(identity.azure_managed_identity)
                .await
                .map_err(upstream("createCredential"))?;
            Ok(c.shaped(&models::CREDENTIAL, &serde_json::to_value(&info)?))
        }
        CredentialOp::List { paging } => {
            let list = c
                .client
                .list_credentials()
                .with_max_results(paging.max_results)
                .with_page_token(paging.page_token)
                .await
                .map_err(upstream("listCredentials"))?;
            Ok(c.shaped_list(
                &models::CREDENTIAL,
                &serde_json::to_value(&list)?,
                "credentials",
            ))
        }
        CredentialOp::Get { name } => {
            let info = c
                .client
                .credential(name)
                .get()
                .await
                .map_err(upstream("getCredential"))?;
            Ok(c.shaped(&models::CREDENTIAL, &serde_json::to_value(&info)?))
        }
        CredentialOp::Update {
            name,
            new_name,
            comment,
            aws_iam_role_arn,
            azure_directory_id,
            azure_application_id,
            azure_client_secret,
            azure_managed_identity,
        } => {
            let identity = Identity::new(
                aws_iam_role_arn,
                azure_directory_id,
                azure_application_id,
                azure_client_secret,
                azure_managed_identity,
            );
            let info = c
                .client
                .credential(name)
                .update()
                .with_comment(comment)
                .with_aws_iam_role(identity.aws_iam_role)
                .with_new_name(new_name)
                .with_azure_service_principal(identity.azure_service_principal)
                .with_azure_managed_identity(identity.azure_managed_identity)
                .await
                .map_err(upstream("updateCredential"))?;
            Ok(c.shaped(&models::CREDENTIAL, &serde_json::to_value(&info)?))
        }
        CredentialOp::Delete { name, force } => {
            c.client
                .credential(name)
                .delete()
                .with_force(force.unwrap_or(false))
                .await
                .map_err(upstream("deleteCredential"))?;
            Ok("{}".into())
        }
    }
}
