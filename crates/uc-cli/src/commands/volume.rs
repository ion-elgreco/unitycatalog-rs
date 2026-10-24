//! `uc volume`: create, list, get, update and delete volumes; read and
//! write their files with [`crate::data::files`].

use clap::Subcommand;
use uc_client::models::volumes::v1::VolumeType;

use super::options::{
    Paging, TABLE_TYPES, ThreePartName, changes, help, one_of_wire, three_part_name,
};
use super::{Context, Result, upstream};
use crate::data::files;
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum VolumeOp {
    /// Create a volume
    Create {
        #[arg(long, help = help::FULL_NAME, value_parser = three_part_name)]
        full_name: ThreePartName,
        #[arg(long, help = help::STORAGE_LOCATION)]
        storage_location: Option<String>,
        /// The type of the volume
        #[arg(long, default_value = "EXTERNAL",
            value_parser = one_of_wire::<VolumeType>(TABLE_TYPES), ignore_case = true)]
        volume_type: VolumeType,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
    },
    /// List the volumes of a schema
    List {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[arg(long, help = help::SCHEMA)]
        schema: String,
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a volume
    Get {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
    /// Update a volume
    #[command(group = changes())]
    Update {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        #[arg(long, help = help::NEW_NAME, group = "changes")]
        new_name: Option<String>,
        #[arg(long, help = help::COMMENT, group = "changes")]
        comment: Option<String>,
    },
    /// Delete a volume
    Delete {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
    /// Print a file of a volume, or list a directory
    Read {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        /// Path inside the volume
        #[arg(long)]
        path: Option<String>,
    },
    /// Upload a file to a volume; without --source, a test file
    Write {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        /// The local file to upload; - for standard input
        #[arg(long, allow_hyphen_values = true)]
        source: Option<String>,
        /// Path inside the volume; by default, the name of the source file
        #[arg(long)]
        path: Option<String>,
    },
}

pub async fn run(c: &Context<'_>, op: VolumeOp) -> Result<String> {
    match op {
        VolumeOp::Create {
            full_name,
            storage_location,
            volume_type,
            comment,
        } => {
            let info = c
                .client
                .create_volume(
                    full_name.catalog,
                    full_name.schema,
                    full_name.name,
                    volume_type,
                )
                .with_comment(comment)
                .with_storage_location(storage_location)
                .await
                .map_err(upstream("createVolume"))?;
            Ok(c.shaped(&models::VOLUME, &serde_json::to_value(&info)?))
        }
        VolumeOp::List {
            catalog,
            schema,
            paging,
        } => {
            let list = c
                .client
                .list_volumes(catalog, schema)
                .with_max_results(paging.max_results)
                .with_page_token(paging.page_token)
                .await
                .map_err(upstream("listVolumes"))?;
            Ok(c.shaped_list(&models::VOLUME, &serde_json::to_value(&list)?, "volumes"))
        }
        VolumeOp::Get { full_name } => {
            let info = c
                .client
                .volume(full_name)
                .get()
                .await
                .map_err(upstream("getVolume"))?;
            Ok(c.shaped(&models::VOLUME, &serde_json::to_value(&info)?))
        }
        VolumeOp::Update {
            full_name,
            new_name,
            comment,
        } => {
            let info = c
                .client
                .volume(full_name)
                .update()
                .with_comment(comment)
                .with_new_name(new_name)
                .await
                .map_err(upstream("updateVolume"))?;
            Ok(c.shaped(&models::VOLUME, &serde_json::to_value(&info)?))
        }
        VolumeOp::Delete { full_name } => {
            c.client
                .volume(full_name)
                .delete()
                .await
                .map_err(upstream("deleteVolume"))?;
            Ok(String::new())
        }
        VolumeOp::Read { full_name, path } => {
            files::read_volume(c, &full_name, path.as_deref()).await
        }
        VolumeOp::Write {
            full_name,
            source,
            path,
        } => files::write_volume(c, &full_name, source.as_deref(), path.as_deref()).await,
    }
}
