//! `uc user`: create, list, get, update and delete users with the control
//! API's SCIM endpoints.

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
    /// Update a user. A new email moves the user's grants and owned
    /// objects to it.
    Update {
        #[arg(long, help = help::USER_ID)]
        id: String,
        #[arg(long, help = help::NAME)]
        name: Option<String>,
        #[arg(long, help = help::EXTERNAL_ID)]
        external_id: Option<String>,
        #[arg(long, help = help::EMAIL)]
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
        "email": user.principal(),
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

/// `emails` with `email` as the primary one: it takes the place of the
/// primary email, else of the first, with its type. The other emails stay,
/// but for one that was `email` already.
fn with_primary(emails: Vec<ScimEmail>, email: String) -> Vec<ScimEmail> {
    let main =
        (emails.iter().position(|e| e.primary == Some(true))).or((!emails.is_empty()).then_some(0));
    let mut out = Vec::with_capacity(emails.len() + 1);
    for (index, mut item) in emails.into_iter().enumerate() {
        if Some(index) == main {
            item.value = email.clone();
            item.primary = Some(true);
            out.push(item);
        } else if !item.value.eq_ignore_ascii_case(&email) {
            out.push(item);
        }
    }
    if main.is_none() {
        out.push(primary_email(email));
    }
    out
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
                .create_user(&email)
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
            // A replace keeps what it does not name, and needs the user
            // name.
            let current = c.client.get_user(&id).await.map_err(upstream("getUser"))?;
            let emails = email.map(|email| with_primary(current.emails, email));
            let mut replace = c
                .client
                .replace_user(id, current.user_name)
                .with_display_name(name)
                .with_external_id(external_id);
            if let Some(emails) = emails {
                replace = replace.with_emails(emails);
            }
            let user = replace.await.map_err(upstream("updateUser"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn email(value: &str, kind: &str, primary: Option<bool>) -> ScimEmail {
        ScimEmail {
            value: value.into(),
            email_type: Some(kind.into()),
            primary,
            ..Default::default()
        }
    }

    #[test]
    fn a_new_email_takes_the_place_of_the_primary_one() {
        let emails = vec![
            email("a@corp.com", "work", Some(true)),
            email("a@home.org", "home", None),
        ];
        assert_eq!(
            with_primary(emails.clone(), "b@corp.com".into()),
            [
                email("b@corp.com", "work", Some(true)),
                email("a@home.org", "home", None)
            ]
        );
        assert_eq!(
            with_primary(emails, "A@home.org".into()),
            [email("A@home.org", "work", Some(true))]
        );
        assert_eq!(
            with_primary(vec![email("a@corp.com", "work", None)], "b@corp.com".into()),
            [email("b@corp.com", "work", Some(true))]
        );
        assert_eq!(
            with_primary(Vec::new(), "b@corp.com".into()),
            [primary_email("b@corp.com".into())]
        );
    }
}
