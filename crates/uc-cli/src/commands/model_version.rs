//! `uc model_version`: create, list, get, update, delete and finalize
//! model versions.

use clap::Subcommand;

use super::options::{Paging, help};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum ModelVersionOp {
    /// Create a model version
    Create {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[arg(long, help = help::SCHEMA)]
        schema: String,
        #[arg(long, help = help::MODEL_NAME)]
        name: String,
        /// The URI of the model version's source artifacts
        #[arg(long)]
        source: String,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
        /// The id of the run that made the model version
        #[arg(long)]
        run_id: Option<String>,
    },
    /// List the versions of a registered model
    List {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a model version
    Get {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::VERSION)]
        version: i64,
    },
    /// Update a model version
    Update {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::VERSION)]
        version: i64,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
    },
    /// Delete a model version
    Delete {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::VERSION)]
        version: i64,
    },
    /// Finalize a model version
    Finalize {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::VERSION)]
        version: i64,
    },
}

pub async fn run(c: &Context<'_>, op: ModelVersionOp) -> Result<String> {
    match op {
        ModelVersionOp::Create {
            catalog,
            schema,
            name,
            source,
            comment,
            run_id,
        } => {
            let info = c
                .client
                .create_model_version(name, catalog, schema, source)
                .with_run_id(run_id)
                .with_comment(comment)
                .await
                .map_err(upstream("createModelVersion"))?;
            Ok(c.shaped(&models::MODEL_VERSION, &serde_json::to_value(&info)?))
        }
        ModelVersionOp::List { full_name, paging } => {
            let list = c
                .client
                .list_model_versions(full_name)
                .with_max_results(paging.max_results)
                .with_page_token(paging.page_token)
                .await
                .map_err(upstream("listModelVersions"))?;
            Ok(c.shaped_list(
                &models::MODEL_VERSION,
                &serde_json::to_value(&list)?,
                "model_versions",
            ))
        }
        ModelVersionOp::Get { full_name, version } => {
            let info = c
                .client
                .get_model_version(full_name, version)
                .await
                .map_err(upstream("getModelVersion"))?;
            Ok(c.shaped(&models::MODEL_VERSION, &serde_json::to_value(&info)?))
        }
        ModelVersionOp::Update {
            full_name,
            version,
            comment,
        } => {
            // An empty comment is not sent: without `--comment`, the request
            // has none.
            let info = c
                .client
                .update_model_version(full_name, version, comment.unwrap_or_default())
                .await
                .map_err(upstream("updateModelVersion"))?;
            Ok(c.shaped(&models::MODEL_VERSION, &serde_json::to_value(&info)?))
        }
        ModelVersionOp::Delete { full_name, version } => {
            c.client
                .delete_model_version(full_name, version)
                .await
                .map_err(upstream("deleteModelVersion"))?;
            Ok(String::new())
        }
        ModelVersionOp::Finalize { full_name, version } => {
            let info = c
                .client
                .finalize_model_version(full_name, version)
                .await
                .map_err(upstream("finalizeModelVersion"))?;
            Ok(c.shaped(&models::MODEL_VERSION, &serde_json::to_value(&info)?))
        }
    }
}
