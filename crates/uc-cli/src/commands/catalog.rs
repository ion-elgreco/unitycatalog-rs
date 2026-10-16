//! `uc catalog`: create, list, get, update and delete catalogs.

use clap::Subcommand;
use clap::builder::BoolishValueParser;
use uc_api::{CreateCatalog, UpdateCatalog};

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
            let request = CreateCatalog {
                name,
                comment,
                properties,
                storage_root,
            };
            let info = c
                .client
                .create_catalog(&request)
                .await
                .map_err(upstream("createCatalog"))?;
            Ok(c.shaped(&models::CATALOG, &serde_json::to_value(&info)?))
        }
        CatalogOp::List { paging } => {
            let list = c
                .client
                .list_catalogs(&paging.into())
                .await
                .map_err(upstream("listCatalogs"))?;
            Ok(c.shaped_list(&models::CATALOG, &serde_json::to_value(&list)?, "catalogs"))
        }
        CatalogOp::Get { name } => {
            let info = c
                .client
                .get_catalog(&name)
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
            let request = UpdateCatalog {
                new_name,
                comment,
                properties,
            };
            let info = c
                .client
                .update_catalog(&name, &request)
                .await
                .map_err(upstream("updateCatalog"))?;
            Ok(c.shaped(&models::CATALOG, &serde_json::to_value(&info)?))
        }
        CatalogOp::Delete { name, force } => {
            c.client
                .delete_catalog(&name, force.unwrap_or(false))
                .await
                .map_err(upstream("deleteCatalog"))?;
            Ok("{}".into())
        }
    }
}
