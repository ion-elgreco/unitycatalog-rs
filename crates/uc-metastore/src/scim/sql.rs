use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    PermissionsChange, ScimEmail, ScimGroup, ScimGroupMember, ScimMeta, ScimPhoto, ScimUser,
};
use uc_common::{Error, Result, SecurableType, now_epoch_millis, require_non_empty};

use crate::ScimOps;
use crate::permissions::sql::apply_permission_change;
use crate::scim::{SeedReport, SeedUser};
use crate::sql::rows::{SCIM_GROUP, SCIM_USER};
use crate::sql::{SqlMetastore, by_id, create_err, db_err, insert, select, select_one, update};

use uc_api::scim::schema_uri;

/// A user, upstream's `user` row: the email (as `user_name` and the one
/// primary email), name, external id, state, picture and times.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimUserRow {
    id_string: String,
    user_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emails: Option<Vec<ScimEmail>>,
    active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    picture_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_at: Option<i64>,
}

/// RFC 3339 in UTC with milliseconds, as SCIM `meta` dates.
fn rfc3339(millis: i64) -> Option<String> {
    let time = jiff::Timestamp::from_millisecond(millis).ok()?;
    Some(format!("{time:.3}"))
}

/// Upstream `Scim2Utils.asUserResource`.
impl From<ScimUserRow> for ScimUser {
    fn from(r: ScimUserRow) -> Self {
        let emails = r.emails.unwrap_or_default();
        let email = emails
            .iter()
            .find(|e| e.primary == Some(true))
            .or(emails.first())
            .map(|e| e.value.clone())
            .unwrap_or(r.user_name);
        Self {
            schemas: vec![schema_uri::USER.to_string()],
            id: Some(r.id_string),
            external_id: r.external_id,
            meta: Some(ScimMeta {
                resource_type: "User".to_string(),
                created: r.created_at.and_then(rfc3339),
                last_modified: r.updated_at.or(r.created_at).and_then(rfc3339),
            }),
            user_name: email.clone(),
            display_name: r.display_name,
            active: Some(r.active),
            emails: Some(vec![ScimEmail {
                value: email,
                primary: Some(true),
                r#type: None,
            }]),
            photos: Some(vec![ScimPhoto {
                value: r.picture_url.unwrap_or_default(),
                r#type: None,
            }]),
            roles: None,
        }
    }
}

fn user_to_row(
    user: ScimUser,
    id: String,
    created_at: Option<i64>,
    updated_at: Option<i64>,
) -> ScimUserRow {
    let email = user.principal().to_string();
    ScimUserRow {
        id_string: id,
        user_name: email.clone(),
        display_name: user.display_name.clone(),
        external_id: user.external_id.clone(),
        emails: Some(vec![ScimEmail {
            value: email,
            primary: Some(true),
            r#type: None,
        }]),
        active: user.is_active(),
        picture_url: user
            .photos
            .into_iter()
            .flatten()
            .next()
            .map(|p| p.value)
            .filter(|v| !v.is_empty()),
        created_at,
        updated_at,
    }
}

/// Resolve the record id for a SCIM row: prefer the URL-bound override
/// (PUT/PATCH path), then the body's `id`, then a fresh UUID (POST).
fn pick_id(override_id: Option<String>, body_id: Option<String>) -> String {
    override_id
        .or(body_id)
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimGroupRow {
    id_string: String,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    members: Option<Vec<ScimGroupMember>>,
}

impl From<ScimGroupRow> for ScimGroup {
    fn from(r: ScimGroupRow) -> Self {
        Self {
            schemas: vec![schema_uri::GROUP.to_string()],
            id: Some(r.id_string),
            display_name: r.display_name,
            members: r.members,
        }
    }
}

fn group_to_row(group: ScimGroup, id_override: Option<String>) -> ScimGroupRow {
    ScimGroupRow {
        id_string: pick_id(id_override, group.id),
        display_name: group.display_name,
        members: group.members,
    }
}

#[async_trait]
impl ScimOps for SqlMetastore {
    async fn create_user(&self, user: ScimUser) -> Result<ScimUser> {
        let email = user.principal().to_string();
        require_non_empty("email", &email)?;
        // Upstream `createUser`: one user per email and per external id.
        if self.find_user_by_email(&email).await?.is_some() {
            return Err(Error::already_exists(format!(
                "User already exists: {email}"
            )));
        }
        if let Some(external_id) = user.external_id.as_deref()
            && self.find_user_by_external_id(external_id).await?.is_some()
        {
            return Err(Error::already_exists(format!(
                "User already exists with external id: {external_id}"
            )));
        }
        let row = user_to_row(
            user,
            Uuid::new_v4().to_string(),
            Some(now_epoch_millis()),
            None,
        );
        insert(self.db.as_ref(), &SCIM_USER, &row)
            .await
            .map_err(create_err)?;
        Ok(row.into())
    }

    async fn get_user(&self, id: &str) -> Result<ScimUser> {
        let row: Option<ScimUserRow> = by_id(self.db.as_ref(), &SCIM_USER, id).await?;
        row.map(Into::into)
            .ok_or_else(|| Error::not_found(format!("User not found: {id}")))
    }

    async fn list_users(&self) -> Result<Vec<ScimUser>> {
        // Users without a display name first.
        let rows: Vec<ScimUserRow> = select(
            self.db.as_ref(),
            &SCIM_USER,
            "ORDER BY COALESCE(display_name, ''), user_name, id_string",
            &[],
        )
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn delete_user(&self, id: &str) -> Result<()> {
        self.delete_by_id("scim_user", id, || {
            Error::not_found(format!("User not found: {id}"))
        })
        .await
    }

    async fn get_user_by_name(&self, user_name: &str) -> Result<Option<ScimUser>> {
        let row: Option<ScimUserRow> = select_one(
            self.db.as_ref(),
            &SCIM_USER,
            "WHERE user_name = $1",
            &[user_name.into()],
        )
        .await?;
        Ok(row.map(Into::into))
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<ScimUser>> {
        // A user's email is its user name.
        self.get_user_by_name(email).await
    }

    async fn find_user_by_external_id(&self, external_id: &str) -> Result<Option<ScimUser>> {
        let row: Option<ScimUserRow> = select_one(
            self.db.as_ref(),
            &SCIM_USER,
            "WHERE external_id = $1",
            &[external_id.into()],
        )
        .await?;
        Ok(row.map(Into::into))
    }

    async fn update_user(&self, id: &str, user: ScimUser) -> Result<ScimUser> {
        let not_found = || Error::not_found(format!("User not found: {id}"));
        let existing: ScimUserRow = by_id(self.db.as_ref(), &SCIM_USER, id)
            .await?
            .ok_or_else(not_found)?;
        if let Some(external_id) = user.external_id.as_deref() {
            let holder = self.find_user_by_external_id(external_id).await?;
            if holder.is_some_and(|other| other.id.as_deref() != Some(id)) {
                return Err(Error::already_exists(format!(
                    "User already exists with external id: {external_id}"
                )));
            }
        }
        let row = user_to_row(
            user,
            id.to_string(),
            existing.created_at,
            Some(now_epoch_millis()),
        );
        let written = update(self.db.as_ref(), &SCIM_USER, &row)
            .await
            .map_err(create_err)?;
        if written == 0 {
            return Err(not_found());
        }
        Ok(row.into())
    }

    async fn seed_users(&self, users: &[SeedUser]) -> Result<SeedReport> {
        let mut report = SeedReport::default();
        if users.is_empty() {
            return Ok(report);
        }
        let metastore_id = self.metastore_row().await?.metastore_id;
        let failed = db_err("seed users");
        let tx = self.db.begin().await.map_err(&failed)?;
        for seed in users {
            let email = seed.email.as_str();
            let found: Option<ScimUserRow> = select_one(
                tx.as_ref(),
                &SCIM_USER,
                "WHERE user_name = $1",
                &[email.into()],
            )
            .await?;
            let active = match found {
                Some(user) => {
                    if let Some(external_id) = seed.external_id.as_deref()
                        && user.external_id.as_deref() != Some(external_id)
                    {
                        return Err(Error::invalid_argument(format!(
                            "User {email} exists with an external id other than {external_id}"
                        )));
                    }
                    user.active
                }
                None => {
                    if let Some(external_id) = seed.external_id.as_deref() {
                        let holder: Option<ScimUserRow> = select_one(
                            tx.as_ref(),
                            &SCIM_USER,
                            "WHERE external_id = $1",
                            &[external_id.into()],
                        )
                        .await?;
                        if let Some(holder) = holder {
                            return Err(Error::already_exists(format!(
                                "The external id {external_id} of {email} belongs to {}",
                                holder.user_name
                            )));
                        }
                    }
                    let user = ScimUser {
                        schemas: vec![schema_uri::USER.to_string()],
                        user_name: email.to_string(),
                        display_name: seed.display_name.clone(),
                        external_id: seed.external_id.clone(),
                        emails: Some(vec![ScimEmail {
                            value: email.to_string(),
                            primary: Some(true),
                            r#type: None,
                        }]),
                        active: Some(true),
                        ..Default::default()
                    };
                    let row = user_to_row(
                        user,
                        Uuid::new_v4().to_string(),
                        Some(now_epoch_millis()),
                        None,
                    );
                    insert(tx.as_ref(), &SCIM_USER, &row)
                        .await
                        .map_err(create_err)?;
                    report.created += 1;
                    true
                }
            };
            if active && !seed.privileges.is_empty() {
                let change = PermissionsChange {
                    principal: email.to_string(),
                    add: seed.privileges.clone(),
                    remove: Vec::new(),
                };
                report.granted += apply_permission_change(
                    tx.as_ref(),
                    SecurableType::Metastore,
                    &metastore_id,
                    change,
                )
                .await?;
            }
        }
        tx.commit().await.map_err(&failed)?;
        Ok(report)
    }

    async fn create_group(&self, group: ScimGroup) -> Result<ScimGroup> {
        require_non_empty("displayName", &group.display_name)?;
        let row = group_to_row(group, None);
        insert(self.db.as_ref(), &SCIM_GROUP, &row)
            .await
            .map_err(create_err)?;
        Ok(row.into())
    }

    async fn get_group(&self, id: &str) -> Result<ScimGroup> {
        let row: Option<ScimGroupRow> = by_id(self.db.as_ref(), &SCIM_GROUP, id).await?;
        row.map(Into::into)
            .ok_or_else(|| Error::not_found(format!("scim group '{id}' not found")))
    }

    async fn list_groups(&self) -> Result<Vec<ScimGroup>> {
        let rows: Vec<ScimGroupRow> = select(
            self.db.as_ref(),
            &SCIM_GROUP,
            "ORDER BY display_name, id_string",
            &[],
        )
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_group(&self, id: &str, group: ScimGroup) -> Result<ScimGroup> {
        let row = group_to_row(group, Some(id.to_string()));
        let written = update(self.db.as_ref(), &SCIM_GROUP, &row)
            .await
            .map_err(create_err)?;
        if written == 0 {
            return Err(Error::not_found(format!("scim group '{id}' not found")));
        }
        Ok(row.into())
    }

    async fn delete_group(&self, id: &str) -> Result<()> {
        self.delete_by_id("scim_group", id, || {
            Error::not_found(format!("Group not found: {id}"))
        })
        .await
    }
}
