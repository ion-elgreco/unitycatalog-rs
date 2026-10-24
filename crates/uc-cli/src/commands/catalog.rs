//! `uc catalog`: create, list, get, update and delete catalogs.

use clap::Subcommand;
use clap::builder::BoolishValueParser;

use super::options::{Paging, Properties, changes, help, properties};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum CatalogOp {
    /// Create a catalog
    Create {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
        #[arg(long, help = help::PROPERTIES, value_parser = properties)]
        properties: Option<Properties>,
        #[arg(long, help = help::STORAGE_ROOT)]
        storage_root: Option<String>,
    },
    /// List catalogs
    List {
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a catalog
    Get {
        #[arg(long, help = help::NAME)]
        name: String,
    },
    /// Update a catalog
    #[command(group = changes())]
    Update {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::NEW_NAME, group = "changes")]
        new_name: Option<String>,
        #[arg(long, help = help::COMMENT, group = "changes")]
        comment: Option<String>,
        #[arg(long, help = help::PROPERTIES, value_parser = properties, group = "changes")]
        properties: Option<Properties>,
    },
    /// Delete a catalog
    Delete {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::FORCE, value_name = "BOOL", num_args = 0..=1,
            default_missing_value = "true", value_parser = BoolishValueParser::new())]
        force: Option<bool>,
    },
}

pub async fn run(c: &Context<'_>, op: CatalogOp) -> Result<String> {
    match op {
        CatalogOp::Create {
            name,
            comment,
            properties,
            storage_root,
        } => {
            let info = c
                .client
                .create_catalog(name)
                .with_comment(comment)
                .with_properties(properties.unwrap_or_default())
                .with_storage_root(storage_root)
                .await
                .map_err(upstream("createCatalog"))?;
            Ok(c.shaped(&models::CATALOG, &serde_json::to_value(&info)?))
        }
        CatalogOp::List { paging } => {
            let list = c
                .client
                .list_catalogs()
                .with_max_results(paging.max_results)
                .with_page_token(paging.page_token)
                .await
                .map_err(upstream("listCatalogs"))?;
            Ok(c.shaped_list(&models::CATALOG, &serde_json::to_value(&list)?, "catalogs"))
        }
        CatalogOp::Get { name } => {
            let info = c
                .client
                .catalog(name)
                .get()
                .await
                .map_err(upstream("getCatalog"))?;
            Ok(c.shaped(&models::CATALOG, &serde_json::to_value(&info)?))
        }
        CatalogOp::Update {
            name,
            new_name,
            comment,
            properties,
        } => {
            let info = c
                .client
                .catalog(name)
                .update()
                .with_new_name(new_name)
                .with_comment(comment)
                .with_properties(properties.unwrap_or_default())
                .await
                .map_err(upstream("updateCatalog"))?;
            Ok(c.shaped(&models::CATALOG, &serde_json::to_value(&info)?))
        }
        CatalogOp::Delete { name, force } => {
            c.client
                .catalog(name)
                .delete()
                .with_force(force.unwrap_or(false))
                .await
                .map_err(upstream("deleteCatalog"))?;
            Ok("{}".into())
        }
    }
}
