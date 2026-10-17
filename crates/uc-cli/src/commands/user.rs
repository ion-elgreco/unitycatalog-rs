//! `uc user`: create, list, get, update and delete users with the control
//! API's SCIM endpoints.

use anyhow::bail;
use clap::Subcommand;
use serde_json::{Value, json};
use uc_api::{ScimEmail, ScimUser, schema_uri};

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
        time.and_then(|t| t.parse::<jiff::Timestamp>().ok())
            .map(|t| t.as_millisecond())
    };
    let meta = user.meta.as_ref();
    let email = user.emails.as_deref().and_then(<[ScimEmail]>::first);
    let user = json!({
        "id": user.id,
        "name": user.display_name,
        "email": email.map(|e| &e.value),
        "external_id": user.external_id,
        "state": if user.is_active() { "ENABLED" } else { "DISABLED" },
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
        r#type: None,
    }
}

pub async fn run(c: &Context<'_>, op: UserOp) -> Result<String> {
    match op {
        UserOp::Create {
            name,
            email,
            external_id,
        } => {
            let request = ScimUser {
                schemas: vec![schema_uri::USER.to_string()],
                display_name: Some(name),
                emails: Some(vec![primary_email(email)]),
                external_id,
                ..Default::default()
            };
            let user = c
                .client
                .create_user(&request)
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
            let request = ScimUser {
                schemas: vec![schema_uri::USER.to_string()],
                id: Some(id.clone()),
                display_name: name,
                external_id,
                ..Default::default()
            };
            let user = c
                .client
                .update_user(&id, &request)
                .await
                .map_err(upstream("updateUser"))?;
            Ok(c.json(&user_from_scim(&user)))
        }
        UserOp::List { paging } => {
            let (start, count) = (paging.start_index, paging.count);
            let list = c
                .client
                .list_users(&paging.into())
                .await
                .map_err(upstream("listUsers"))?;
            c.start_index(start, count, list.resources.len());
            let users: Vec<Value> = list.resources.iter().map(user_from_scim).collect();
            Ok(c.json(&Value::Array(users)))
        }
        UserOp::Get { id } => {
            let user = c.client.get_user(&id).await.map_err(upstream("getUser"))?;
            Ok(c.json(&user_from_scim(&user)))
        }
        UserOp::Delete { id } => {
            c.client
                .delete_user(&id)
                .await
                .map_err(upstream("deleteUser"))?;
            Ok(String::new())
        }
    }
}
