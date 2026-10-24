//! `uc user`: create, list, get, update and delete users with the control
//! API's SCIM endpoints.

use anyhow::bail;
use clap::Subcommand;
use serde_json::{Value, json};
use uc_client::models::scim::v1::{ScimEmail, ScimUser};

use super::options::{ScimPaging, help};
use super::{Context, Result, upstream};
use crate::output::models::{self, shape};

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum UserOp {
    /// Create a user
    Create {
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::EMAIL)]
        email: String,
        #[arg(long, help = help::EXTERNAL_ID)]
        external_id: Option<String>,
    },
    /// List users
    List {
        #[command(flatten)]
        paging: ScimPaging,
    },
    /// Get a user
    Get {
        #[arg(long, help = help::USER_ID)]
        id: String,
    },
    /// Update a user
    Update {
        #[arg(long, help = help::USER_ID)]
        id: String,
        #[arg(long, help = help::NAME)]
        name: Option<String>,
        #[arg(long, help = help::EXTERNAL_ID)]
        external_id: Option<String>,
        /// Refused: a user's email cannot change. Upstream takes it, and its
        /// server keeps the email.
        #[arg(long, hide = true)]
        email: Option<String>,
    },
    /// Delete a user
    Delete {
        #[arg(long, help = help::USER_ID)]
        id: String,
    },
}

/// Upstream `UserCli.fromUserResource`: a SCIM user as the control API's
/// `User`.
fn user_from_scim(user: &ScimUser) -> Value {
    let millis = |time: Option<&String>| {
        time.and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.timestamp_millis())
    };
    let meta = user.meta.as_option();
    let user = json!({
        "id": (!user.id.is_empty()).then_some(&user.id),
        "name": user.display_name,
        "email": user.emails.first().map(|e| &e.value),
        "external_id": user.external_id,
        "state": if user.active != Some(false) { "ENABLED" } else { "DISABLED" },
        "created_at": millis(meta.and_then(|m| m.created.as_ref())),
        "updated_at": millis(meta.and_then(|m| m.last_modified.as_ref())),
    });
    shape(&models::USER, &user)
}

/// A user's one email, as primary.
fn primary_email(value: String) -> ScimEmail {
    ScimEmail {
        value,
        primary: Some(true),
        ..Default::default()
    }
}

pub async fn run(c: &Context<'_>, op: UserOp) -> Result<String> {
    match op {
        UserOp::Create {
            name,
            email,
            external_id,
        } => {
            let user = c
                .client
                .create_user()
                .with_display_name(name)
                .with_emails([primary_email(email)])
                .with_external_id(external_id)
                .await
                .map_err(upstream("createUser"))?;
            Ok(c.json(&user_from_scim(&user)))
        }
        UserOp::Update {
            id,
            name,
            external_id,
            email,
        } => {
            if email.is_some() {
                bail!("A user's email cannot change: it is the user's name in grants and owners");
            }
            let user = c
                .client
                .replace_user(id)
                .with_display_name(name)
                .with_external_id(external_id)
                .await
                .map_err(upstream("updateUser"))?;
            Ok(c.json(&user_from_scim(&user)))
        }
        UserOp::List { paging } => {
            let (start, count) = (paging.start_index, paging.count);
            let list = c
                .client
                .list_users()
                .with_filter(paging.filter)
                .with_start_index(start)
                .with_count(count)
                .await
                .map_err(upstream("listUsers"))?;
            c.start_index(start, count, list.resources.len());
            let users: Vec<Value> = list.resources.iter().map(user_from_scim).collect();
            Ok(c.json(&Value::Array(users)))
        }
        UserOp::Get { id } => {
            let user = c.client.get_user(id).await.map_err(upstream("getUser"))?;
            Ok(c.json(&user_from_scim(&user)))
        }
        UserOp::Delete { id } => {
            c.client
                .delete_user(id)
                .await
                .map_err(upstream("deleteUser"))?;
            Ok(String::new())
        }
    }
}
