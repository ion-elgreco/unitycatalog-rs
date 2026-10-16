//! `uc registered_model`: create, list, get, update and delete registered
//! models.

use clap::Subcommand;
use clap::builder::BoolishValueParser;
use uc_api::{CreateRegisteredModel, UpdateRegisteredModel};

use super::options::{Paging, changes, help};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum RegisteredModelOp {
    /// Create a registered model
    Create {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[arg(long, help = help::SCHEMA)]
        schema: String,
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
    },
    /// List registered models
    List {
        #[arg(long, help = help::CATALOG)]
        catalog: Option<String>,
        #[arg(long, help = help::SCHEMA)]
        schema: Option<String>,
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a registered model
    Get {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
    /// Update a registered model
    #[command(group = changes())]
    Update {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::NEW_NAME, group = "changes")]
        new_name: Option<String>,
        #[arg(long, help = help::COMMENT, group = "changes")]
        comment: Option<String>,
    },
    /// Delete a registered model
    Delete {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::FORCE, value_name = "BOOL", num_args = 0..=1,
            default_missing_value = "true", value_parser = BoolishValueParser::new())]
        force: Option<bool>,
    },
}

pub async fn run(c: &Context<'_>, op: RegisteredModelOp) -> Result<String> {
    match op {
        RegisteredModelOp::Create {
            catalog,
            schema,
            name,
            comment,
        } => {
            let request = CreateRegisteredModel {
                name,
                catalog_name: catalog,
                schema_name: schema,
                comment,
            };
            let info = c
                .client
                .create_registered_model(&request)
                .await
                .map_err(upstream("createRegisteredModel"))?;
            Ok(c.shaped(&models::REGISTERED_MODEL, &serde_json::to_value(&info)?))
        }
        RegisteredModelOp::List {
            catalog,
            schema,
            paging,
        } => {
            let list = c
                .client
                .list_registered_models(catalog.as_deref(), schema.as_deref(), &paging.into())
                .await
                .map_err(upstream("listRegisteredModels"))?;
            Ok(c.shaped_list(
                &models::REGISTERED_MODEL,
                &serde_json::to_value(&list)?,
                "registered_models",
            ))
        }
        RegisteredModelOp::Get { full_name } => {
            let info = c
                .client
                .get_registered_model(&full_name)
                .await
                .map_err(upstream("getRegisteredModel"))?;
            Ok(c.shaped(&models::REGISTERED_MODEL, &serde_json::to_value(&info)?))
        }
        RegisteredModelOp::Update {
            full_name,
            new_name,
            comment,
        } => {
            let request = UpdateRegisteredModel { comment, new_name };
            let info = c
                .client
                .update_registered_model(&full_name, &request)
                .await
                .map_err(upstream("updateRegisteredModel"))?;
            Ok(c.shaped(&models::REGISTERED_MODEL, &serde_json::to_value(&info)?))
        }
        RegisteredModelOp::Delete { full_name, force } => {
            c.client
                .delete_registered_model(&full_name, force.unwrap_or(false))
                .await
                .map_err(upstream("deleteRegisteredModel"))?;
            Ok(String::new())
        }
    }
}
