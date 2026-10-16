//! `uc external_location`: create, list, get, update and delete external
//! locations.

use clap::Subcommand;
use clap::builder::BoolishValueParser;
use uc_api::{CreateExternalLocation, UpdateExternalLocation};

use super::options::{Paging, changes, help};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum ExternalLocationOp {
    /// Create an external location
    Create {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::URL)]
        url: String,
        #[arg(long, help = help::CREDENTIAL_NAME)]
        credential_name: String,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
    },
    /// List external locations
    List {
        #[command(flatten)]
        paging: Paging,
    },
    /// Get an external location
    Get {
        #[arg(long, help = help::NAME)]
        name: String,
    },
    /// Update an external location
    #[command(group = changes())]
    Update {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::NEW_NAME, group = "changes")]
        new_name: Option<String>,
        #[arg(long, help = help::URL, group = "changes")]
        url: Option<String>,
        #[arg(long, help = help::CREDENTIAL_NAME, group = "changes")]
        credential_name: Option<String>,
        #[arg(long, help = help::COMMENT, group = "changes")]
        comment: Option<String>,
    },
    /// Delete an external location
    Delete {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::FORCE, value_name = "BOOL", num_args = 0..=1,
            default_missing_value = "true", value_parser = BoolishValueParser::new())]
        force: Option<bool>,
    },
}

pub async fn run(c: &Context<'_>, op: ExternalLocationOp) -> Result<String> {
    match op {
        ExternalLocationOp::Create {
            name,
            url,
            credential_name,
            comment,
        } => {
            let request = CreateExternalLocation {
                name,
                url,
                credential_name,
                comment,
            };
            let info = c
                .client
                .create_external_location(&request)
                .await
                .map_err(upstream("createExternalLocation"))?;
            Ok(c.shaped(&models::EXTERNAL_LOCATION, &serde_json::to_value(&info)?))
        }
        ExternalLocationOp::List { paging } => {
            let list = c
                .client
                .list_external_locations(&paging.into())
                .await
                .map_err(upstream("listExternalLocations"))?;
            Ok(c.shaped_list(
                &models::EXTERNAL_LOCATION,
                &serde_json::to_value(&list)?,
                "external_locations",
            ))
        }
        ExternalLocationOp::Get { name } => {
            let info = c
                .client
                .get_external_location(&name)
                .await
                .map_err(upstream("getExternalLocation"))?;
            Ok(c.shaped(&models::EXTERNAL_LOCATION, &serde_json::to_value(&info)?))
        }
        ExternalLocationOp::Update {
            name,
            new_name,
            url,
            credential_name,
            comment,
        } => {
            let request = UpdateExternalLocation {
                url,
                new_name,
                comment,
                credential_name,
                owner: None,
            };
            let info = c
                .client
                .update_external_location(&name, &request)
                .await
                .map_err(upstream("updateExternalLocation"))?;
            Ok(c.shaped(&models::EXTERNAL_LOCATION, &serde_json::to_value(&info)?))
        }
        ExternalLocationOp::Delete { name, force } => {
            c.client
                .delete_external_location(&name, force.unwrap_or(false))
                .await
                .map_err(upstream("deleteExternalLocation"))?;
            Ok("{}".into())
        }
    }
}
