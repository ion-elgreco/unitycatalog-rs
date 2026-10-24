//! `uc credential`: create, list, get, update and delete storage
//! credentials.

use clap::Subcommand;
use clap::builder::BoolishValueParser;
use uc_client::models::credentials::v1::{AwsIamRoleRequest, CredentialPurpose};

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
    let role = |role_arn: String| AwsIamRoleRequest {
        role_arn,
        ..Default::default()
    };
    match op {
        CredentialOp::Create {
            name,
            comment,
            aws_iam_role_arn,
        } => {
            // The API takes a role: without `--aws_iam_role_arn`, its ARN is
            // empty.
            let info = c
                .client
                .create_credential(name, role(aws_iam_role_arn.unwrap_or_default()))
                .with_comment(comment)
                .with_purpose(CredentialPurpose::Storage)
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
        } => {
            let info = c
                .client
                .credential(name)
                .update()
                .with_comment(comment)
                .with_aws_iam_role(aws_iam_role_arn.map(role))
                .with_new_name(new_name)
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
