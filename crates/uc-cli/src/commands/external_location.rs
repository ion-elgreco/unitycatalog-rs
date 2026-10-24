//! `uc external_location`: create, list, get, update and delete external
//! locations.

use clap::Subcommand;
use clap::builder::BoolishValueParser;

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
            let info = c
                .client
                .create_external_location(name, url, credential_name)
                .with_comment(comment)
                .await
                .map_err(upstream("createExternalLocation"))?;
            Ok(c.shaped(&models::EXTERNAL_LOCATION, &serde_json::to_value(&info)?))
        }
        ExternalLocationOp::List { paging } => {
            let list = c
                .client
                .list_external_locations()
                .with_max_results(paging.max_results)
                .with_page_token(paging.page_token)
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
                .external_location(name)
                .get()
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
            let info = c
                .client
                .external_location(name)
                .update()
                .with_url(url)
                .with_new_name(new_name)
                .with_comment(comment)
                .with_credential_name(credential_name)
                .await
                .map_err(upstream("updateExternalLocation"))?;
            Ok(c.shaped(&models::EXTERNAL_LOCATION, &serde_json::to_value(&info)?))
        }
        ExternalLocationOp::Delete { name, force } => {
            c.client
                .external_location(name)
                .delete()
                .with_force(force.unwrap_or(false))
                .await
                .map_err(upstream("deleteExternalLocation"))?;
            Ok("{}".into())
        }
    }
}
