//! `uc group`: create, list, get and delete groups with the control API's
//! SCIM endpoints.

use clap::Subcommand;
use uc_api::{ScimGroup, schema_uri};

use super::options::{ScimPaging, help};
use super::{Context, Result, upstream};

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum GroupOp {
    /// Create a group
    Create {
        #[arg(long, help = help::NAME)]
        name: String,
    },
    /// List groups
    List {
        #[command(flatten)]
        paging: ScimPaging,
    },
    /// Get a group
    Get {
        /// The id of the group
        #[arg(long)]
        id: String,
    },
    /// Delete a group
    Delete {
        /// The id of the group
        #[arg(long)]
        id: String,
    },
}

pub async fn run(c: &Context<'_>, op: GroupOp) -> Result<String> {
    match op {
        GroupOp::Create { name } => {
            let request = ScimGroup {
                schemas: vec![schema_uri::GROUP.to_string()],
                display_name: name,
                ..Default::default()
            };
            let group = c
                .client
                .create_group(&request)
                .await
                .map_err(upstream("createGroup"))?;
            Ok(c.json(&serde_json::to_value(&group)?))
        }
        GroupOp::List { paging } => {
            let (start, count) = (paging.start_index, paging.count);
            let list = c
                .client
                .list_groups(&paging.into())
                .await
                .map_err(upstream("listGroups"))?;
            c.start_index(start, count, list.resources.len());
            Ok(c.json(&serde_json::to_value(&list.resources)?))
        }
        GroupOp::Get { id } => {
            let group = c
                .client
                .get_group(&id)
                .await
                .map_err(upstream("getGroup"))?;
            Ok(c.json(&serde_json::to_value(&group)?))
        }
        GroupOp::Delete { id } => {
            c.client
                .delete_group(&id)
                .await
                .map_err(upstream("deleteGroup"))?;
            Ok(String::new())
        }
    }
}
