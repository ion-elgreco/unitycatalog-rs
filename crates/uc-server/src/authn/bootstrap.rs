//! The users of the settings: `[[auth.users]]` and `auth.admin_principals`.
//! At each start the server makes the ones that are missing, and grants
//! the missing metastore privileges of `[[auth.users]]`. It never changes
//! or deletes a user that exists, and a disabled user gets no privileges.

use std::collections::HashSet;

use uc_api::Privilege;
use uc_common::{Result, settings};
use uc_metastore::{DynMetastore, SeedReport, SeedUser};

use crate::authn::security::ADMIN;

/// The users of `auth`: those of `[[auth.users]]`, then the admins of
/// `auth.admin_principals` that the list does not have. An entry that the
/// API would refuse is an error, and so is the server's own `admin`.
pub fn seed_users(auth: &settings::Auth) -> std::result::Result<Vec<SeedUser>, String> {
    let mut emails = HashSet::new();
    let mut external_ids = HashSet::new();
    let mut users = Vec::with_capacity(auth.users.len());
    for (index, user) in auth.users.iter().enumerate() {
        let invalid = |key: &str, value: &str, reason: &str| {
            format!("Invalid value for auth.users[{index}].{key}: '{value}'. {reason}")
        };
        let email = user.email.trim();
        if email.is_empty() {
            return Err(invalid("email", email, "Expected an email."));
        }
        if email == ADMIN {
            return Err(invalid(
                "email",
                email,
                "This is the server's own user; `uc-server token` signs it in.",
            ));
        }
        if !emails.insert(email) {
            return Err(invalid("email", email, "Another entry has it too."));
        }
        let external_id = match user.external_id.as_deref().map(str::trim) {
            None => None,
            Some("") => return Err(invalid("external_id", "", "Expected an id.")),
            Some(id) if !external_ids.insert(id) => {
                return Err(invalid("external_id", id, "Another entry has it too."));
            }
            Some(id) => Some(id.to_string()),
        };
        let privileges = user
            .privileges
            .iter()
            .map(|text| {
                privilege(text).ok_or_else(|| {
                    invalid(
                        "privileges",
                        text,
                        "Expected a privilege, such as CREATE CATALOG.",
                    )
                })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        users.push(SeedUser {
            email: email.to_string(),
            display_name: user.name.clone().filter(|name| !name.trim().is_empty()),
            external_id,
            privileges,
        });
    }
    // The bootstrap admin exists already when authorization is on.
    for email in auth.admin_principals.iter().map(|e| e.trim()) {
        if !email.is_empty() && email != ADMIN && emails.insert(email) {
            users.push(SeedUser {
                email: email.to_string(),
                ..Default::default()
            });
        }
    }
    Ok(users)
}

/// A privilege by its name in the API, such as `CREATE CATALOG`.
fn privilege(text: &str) -> Option<Privilege> {
    serde_json::from_value(serde_json::Value::String(text.trim().to_string())).ok()
}

/// Make the missing users of `users` and grant their missing privileges,
/// in one transaction. Replicas that start together may make the same
/// rows at once: the second try finds them.
pub async fn seed(metastore: &DynMetastore, users: &[SeedUser]) -> Result<SeedReport> {
    match metastore.scim().seed_users(users).await {
        Err(e) if e.code().is_already_exists() => metastore.scim().seed_users(users).await,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uc_common::settings::{AuthUser, Settings};

    fn auth(text: &str) -> settings::Auth {
        Settings::from_toml(text).unwrap().auth
    }

    #[test]
    fn users_of_the_list_then_admins() {
        let users = seed_users(&auth(
            r#"
            [auth]
            admin_principals = ["alice@example.com", "carol@example.com", "admin"]

            [[auth.users]]
            email = "alice@example.com"
            name = "Alice"
            privileges = ["CREATE CATALOG", " USE CATALOG "]

            [[auth.users]]
            email = " etl@example.com "
            name = " "
            external_id = "etl-client"
            "#,
        ))
        .unwrap();
        assert_eq!(
            users,
            [
                SeedUser {
                    email: "alice@example.com".into(),
                    display_name: Some("Alice".into()),
                    external_id: None,
                    privileges: vec![Privilege::CreateCatalog, Privilege::UseCatalog],
                },
                SeedUser {
                    email: "etl@example.com".into(),
                    external_id: Some("etl-client".into()),
                    ..Default::default()
                },
                SeedUser {
                    email: "carol@example.com".into(),
                    ..Default::default()
                },
            ]
        );
        assert!(seed_users(&settings::Auth::default()).unwrap().is_empty());
    }

    #[test]
    fn refuses_what_the_api_refuses() {
        let error = |users: Vec<AuthUser>| {
            seed_users(&settings::Auth {
                users,
                ..Default::default()
            })
            .unwrap_err()
        };
        let user = |email: &str| AuthUser {
            email: email.into(),
            name: None,
            external_id: None,
            privileges: Vec::new(),
        };
        assert_eq!(
            error(vec![user(" ")]),
            "Invalid value for auth.users[0].email: ''. Expected an email."
        );
        assert!(error(vec![user("admin")]).contains("the server's own user"));
        assert_eq!(
            error(vec![user("a@x"), user("a@x")]),
            "Invalid value for auth.users[1].email: 'a@x'. Another entry has it too."
        );
        let with_id = |email: &str, id: &str| AuthUser {
            external_id: Some(id.into()),
            ..user(email)
        };
        assert!(error(vec![with_id("a@x", "c"), with_id("b@x", "c")]).contains("[1].external_id"));
        assert!(error(vec![with_id("a@x", " ")]).contains("Expected an id."));
        let error = error(vec![AuthUser {
            privileges: vec!["CREATE_CATALOG".into()],
            ..user("a@x")
        }]);
        assert_eq!(
            error,
            "Invalid value for auth.users[0].privileges: 'CREATE_CATALOG'. Expected a privilege, \
             such as CREATE CATALOG."
        );
    }
}
