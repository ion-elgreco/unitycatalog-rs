//! `uc schema`: create, list, get, update and delete schemas.

use clap::Subcommand;
use clap::builder::BoolishValueParser;
use uc_api::{CreateSchema, UpdateSchema};

use super::options::{Paging, Properties, changes, help, properties};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum SchemaOp {
    /// Create a schema
    Create {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
        #[arg(long, help = help::PROPERTIES, value_parser = properties)]
        properties: Option<Properties>,
        #[arg(long, help = help::STORAGE_ROOT)]
        storage_root: Option<String>,
    },
    /// List the schemas of a catalog
    List {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a schema
    Get {
        #[arg(long, help = help::SCHEMA_FULL_NAME)]
        full_name: String,
    },
    /// Update a schema
    #[command(group = changes())]
    Update {
        #[arg(long, help = help::SCHEMA_FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::NEW_NAME, group = "changes")]
        new_name: Option<String>,
        #[arg(long, help = help::COMMENT, group = "changes")]
        comment: Option<String>,
        #[arg(long, help = help::PROPERTIES, value_parser = properties, group = "changes")]
        properties: Option<Properties>,
    },
    /// Delete a schema
    Delete {
        #[arg(long, help = help::SCHEMA_FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::FORCE, value_name = "BOOL", num_args = 0..=1,
            default_missing_value = "true", value_parser = BoolishValueParser::new())]
        force: Option<bool>,
    },
}

pub async fn run(c: &Context<'_>, op: SchemaOp) -> Result<String> {
    match op {
        SchemaOp::Create {
            catalog,
            name,
            comment,
            properties,
            storage_root,
        } => {
            let request = CreateSchema {
                name,
                catalog_name: catalog,
                comment,
                properties,
                storage_root,
            };
            let info = c
                .client
                .create_schema(&request)
                .await
                .map_err(upstream("createSchema"))?;
            Ok(c.shaped(&models::SCHEMA, &serde_json::to_value(&info)?))
        }
        SchemaOp::List { catalog, paging } => {
            let list = c
                .client
                .list_schemas(&catalog, &paging.into())
                .await
                .map_err(upstream("listSchemas"))?;
            Ok(c.shaped_list(&models::SCHEMA, &serde_json::to_value(&list)?, "schemas"))
        }
        SchemaOp::Get { full_name } => {
            let info = c
                .client
                .get_schema(&full_name)
                .await
                .map_err(upstream("getSchema"))?;
            Ok(c.shaped(&models::SCHEMA, &serde_json::to_value(&info)?))
        }
        SchemaOp::Update {
            full_name,
            new_name,
            comment,
            properties,
        } => {
            let request = UpdateSchema {
                new_name,
                comment,
                properties,
            };
            let info = c
                .client
                .update_schema(&full_name, &request)
                .await
                .map_err(upstream("updateSchema"))?;
            Ok(c.shaped(&models::SCHEMA, &serde_json::to_value(&info)?))
        }
        SchemaOp::Delete { full_name, force } => {
            c.client
                .delete_schema(&full_name, force.unwrap_or(false))
                .await
                .map_err(upstream("deleteSchema"))?;
            Ok(String::new())
        }
    }
}
