//! `uc credential`: create, list, get, update and delete storage
//! credentials.

use clap::Subcommand;
use clap::builder::BoolishValueParser;
use uc_api::{
    AwsIamRoleRequest, CreateCredentialRequest, CredentialPurpose, UpdateCredentialRequest,
};

use super::options::{Paging, changes, help};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum CredentialOp {
    /// Create a storage credential
    Create {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
        #[arg(long, help = help::AWS_IAM_ROLE_ARN)]
        aws_iam_role_arn: Option<String>,
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
    /// Update a storage credential
    #[command(group = changes())]
    Update {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::NEW_NAME, group = "changes")]
        new_name: Option<String>,
        #[arg(long, help = help::COMMENT, group = "changes")]
        comment: Option<String>,
        #[arg(long, help = help::AWS_IAM_ROLE_ARN, group = "changes")]
        aws_iam_role_arn: Option<String>,
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

pub async fn run(c: &Context<'_>, op: CredentialOp) -> Result<String> {
    let role = |arn: Option<String>| arn.map(|role_arn| AwsIamRoleRequest { role_arn });
    match op {
        CredentialOp::Create {
            name,
            comment,
            aws_iam_role_arn,
        } => {
            let request = CreateCredentialRequest {
                name,
                comment,
                aws_iam_role: role(aws_iam_role_arn),
                purpose: Some(CredentialPurpose::Storage),
            };
            let info = c
                .client
                .create_credential(&request)
                .await
                .map_err(upstream("createCredential"))?;
            Ok(c.shaped(&models::CREDENTIAL, &serde_json::to_value(&info)?))
        }
        CredentialOp::List { paging } => {
            let list = c
                .client
                .list_credentials(&paging.into())
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
                .get_credential(&name)
                .await
                .map_err(upstream("getCredential"))?;
            Ok(c.shaped(&models::CREDENTIAL, &serde_json::to_value(&info)?))
        }
        CredentialOp::Update {
            name,
            new_name,
            comment,
            aws_iam_role_arn,
        } => {
            let request = UpdateCredentialRequest {
                comment,
                owner: None,
                aws_iam_role: role(aws_iam_role_arn),
                new_name,
            };
            let info = c
                .client
                .update_credential(&name, &request)
                .await
                .map_err(upstream("updateCredential"))?;
            Ok(c.shaped(&models::CREDENTIAL, &serde_json::to_value(&info)?))
        }
        CredentialOp::Delete { name, force } => {
            c.client
                .delete_credential(&name, force.unwrap_or(false))
                .await
                .map_err(upstream("deleteCredential"))?;
            Ok("{}".into())
        }
    }
}
