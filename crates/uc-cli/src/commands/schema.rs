//! `uc schema`: create, list, get, update and delete schemas.

use clap::Subcommand;
use clap::builder::BoolishValueParser;

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
            let info = c
                .client
                .create_schema(name, catalog)
                .with_comment(comment)
                .with_properties(properties.unwrap_or_default())
                .with_storage_root(storage_root)
                .await
                .map_err(upstream("createSchema"))?;
            Ok(c.shaped(&models::SCHEMA, &serde_json::to_value(&info)?))
        }
        SchemaOp::List { catalog, paging } => {
            let list = c
                .client
                .list_schemas(catalog)
                .with_max_results(paging.max_results)
                .with_page_token(paging.page_token)
                .await
                .map_err(upstream("listSchemas"))?;
            Ok(c.shaped_list(&models::SCHEMA, &serde_json::to_value(&list)?, "schemas"))
        }
        SchemaOp::Get { full_name } => {
            let info = c
                .client
                .schema(full_name)
                .get()
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
            let info = c
                .client
                .schema(full_name)
                .update()
                .with_new_name(new_name)
                .with_comment(comment)
                .with_properties(properties.unwrap_or_default())
                .await
                .map_err(upstream("updateSchema"))?;
            Ok(c.shaped(&models::SCHEMA, &serde_json::to_value(&info)?))
        }
        SchemaOp::Delete { full_name, force } => {
            c.client
                .schema(full_name)
                .delete()
                .with_force(force.unwrap_or(false))
                .await
                .map_err(upstream("deleteSchema"))?;
            Ok(String::new())
        }
    }
}
