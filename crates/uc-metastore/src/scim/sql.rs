use std::collections::BTreeSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    PermissionsChange, ScimEmail, ScimGroup, ScimGroupMember, ScimMeta, ScimName, ScimPhoto,
    ScimUser,
};
use uc_common::{Error, Result, SecurableType, now_epoch_millis, require_non_empty};
use uc_storage::{Exec, Value};

use crate::ScimOps;
use crate::permissions::sql::{apply_permission_change, forget_principal, rename_principal};
use crate::scim::{ScimKey, SeedReport, SeedUser};
use crate::sql::rows::{SCIM_GROUP, SCIM_GROUP_MEMBER, SCIM_USER};
use crate::sql::{
    SqlMetastore, by_id, create_err, db_err, insert, insert_all, placeholders, select, update,
};

use uc_api::scim::schema_uri;

/// A user: its SCIM attributes, its principal and times.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimUserRow {
    id_string: String,
    user_name: String,
    principal: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<ScimName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    emails: Option<Vec<ScimEmail>>,
    /// None: enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    photos: Option<Vec<ScimPhoto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_at: Option<i64>,
}

/// RFC 3339 in UTC with milliseconds, as SCIM `meta` dates.
fn rfc3339(millis: i64) -> Option<String> {
    let time = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis)?;
    Some(time.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn meta(resource_type: &str, created_at: Option<i64>, updated_at: Option<i64>) -> ScimMeta {
    ScimMeta {
        resource_type: resource_type.to_string(),
        created: created_at.and_then(rfc3339),
        last_modified: updated_at.or(created_at).and_then(rfc3339),
        location: None,
    }
}

impl From<ScimUserRow> for ScimUser {
    fn from(r: ScimUserRow) -> Self {
        Self {
            schemas: vec![schema_uri::USER.to_string()],
            id: Some(r.id_string),
            external_id: r.external_id,
            meta: Some(meta("User", r.created_at, r.updated_at)),
            user_name: r.user_name,
            name: r.name,
            display_name: r.display_name,
            active: r.active,
            emails: r.emails,
            photos: r.photos,
            roles: None,
        }
    }
}

/// `items`, or none when empty.
fn non_empty<T>(items: Option<Vec<T>>) -> Option<Vec<T>> {
    items.filter(|items| !items.is_empty())
}

/// The row of `user`, with its user name and emails trimmed.
fn user_to_row(
    mut user: ScimUser,
    id: String,
    created_at: Option<i64>,
    updated_at: Option<i64>,
) -> ScimUserRow {
    user.user_name = user.user_name.trim().to_string();
    for email in user.emails.iter_mut().flatten() {
        email.value = email.value.trim().to_string();
    }
    let principal = user.principal().to_string();
    let user_name = match user.user_name.is_empty() {
        true => principal.clone(),
        false => user.user_name,
    };
    ScimUserRow {
        id_string: id,
        user_name,
        principal,
        name: user.name.filter(|name| *name != ScimName::default()),
        display_name: user.display_name,
        external_id: user.external_id,
        emails: non_empty(user.emails),
        active: user.active,
        photos: non_empty(user.photos),
        created_at,
        updated_at,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScimGroupRow {
    id_string: String,
    display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    members: Option<Vec<ScimGroupMember>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_at: Option<i64>,
}

impl From<ScimGroupRow> for ScimGroup {
    fn from(r: ScimGroupRow) -> Self {
        Self {
            schemas: vec![schema_uri::GROUP.to_string()],
            id: Some(r.id_string),
            external_id: r.external_id,
            meta: Some(meta("Group", r.created_at, r.updated_at)),
            display_name: r.display_name,
            members: r.members,
        }
    }
}

fn group_to_row(
    group: ScimGroup,
    id: String,
    created_at: Option<i64>,
    updated_at: Option<i64>,
) -> ScimGroupRow {
    // One entry per user.
    let mut seen = BTreeSet::new();
    let members: Vec<ScimGroupMember> = group
        .members
        .into_iter()
        .flatten()
        .filter(|m| seen.insert(m.value.clone()))
        .collect();
    ScimGroupRow {
        id_string: id,
        display_name: group.display_name,
        external_id: group.external_id,
        members: non_empty(Some(members)),
        created_at,
        updated_at,
    }
}

/// A user in a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupMemberRow {
    group_id: String,
    user_id: String,
}

/// The most values in one `IN (…)` list.
const IN_LIST: usize = 500;

/// Make `scim_group_member` hold the members of `group`, in `tx`: the rows
/// of members that left go, and new members get one. Each new member must
/// be a user, which stays locked until `tx` ends: a delete of the user
/// waits, then removes it from the group.
async fn write_members<E: Exec + ?Sized>(tx: &E, group: &ScimGroupRow) -> Result<()> {
    let failed = db_err("write group members");
    let group_id = group.id_string.as_str();
    let mut old = BTreeSet::new();
    let rows = tx
        .query(
            "SELECT user_id FROM scim_group_member WHERE group_id = $1",
            &[group_id.into()],
        )
        .await
        .map_err(&failed)?;
    for row in rows {
        old.insert(row.text(0).map_err(&failed)?);
    }
    let new: BTreeSet<&str> = (group.members.iter().flatten())
        .map(|m| m.value.as_str())
        .collect();
    let gone: Vec<&str> = (old.iter().map(String::as_str))
        .filter(|id| !new.contains(id))
        .collect();
    let added: Vec<&str> = (new.iter().copied())
        .filter(|id| !old.contains(*id))
        .collect();
    for chunk in gone.chunks(IN_LIST) {
        let mut params: Vec<Value> = vec![group_id.into()];
        params.extend(chunk.iter().map(|id| Value::from(*id)));
        tx.execute(
            &format!(
                "DELETE FROM scim_group_member WHERE group_id = $1 AND user_id IN ({})",
                placeholders(2, chunk.len())
            ),
            &params,
        )
        .await
        .map_err(&failed)?;
    }
    for chunk in added.chunks(IN_LIST) {
        let params: Vec<Value> = chunk.iter().map(|id| Value::from(*id)).collect();
        let rows = tx
            .query(
                &format!(
                    "SELECT id_string FROM scim_user WHERE id_string IN ({}){}",
                    placeholders(1, chunk.len()),
                    tx.for_update()
                ),
                &params,
            )
            .await
            .map_err(&failed)?;
        let mut users = BTreeSet::new();
        for row in rows {
            users.insert(row.text(0).map_err(&failed)?);
        }
        if let Some(missing) = chunk.iter().find(|id| !users.contains(**id)) {
            return Err(Error::invalid_argument(format!(
                "No user has the id of the member {missing}"
            )));
        }
    }
    let rows: Vec<GroupMemberRow> = (added.iter())
        .map(|id| GroupMemberRow {
            group_id: group_id.to_string(),
            user_id: id.to_string(),
        })
        .collect();
    insert_all(tx, &SCIM_GROUP_MEMBER, &rows)
        .await
        .map_err(&failed)
}

/// Grants name users by principal and groups by name, so no user has a
/// group's name as principal and no group a user's principal.
fn name_taken(name: &str) -> Error {
    Error::already_exists(format!("Principal already exists: {name}"))
}

async fn require_no_group<E: Exec + ?Sized>(ex: &E, name: &str) -> Result<()> {
    let group: Vec<ScimGroupRow> =
        select(ex, &SCIM_GROUP, "WHERE display_name = $1", &[name.into()]).await?;
    match group.is_empty() {
        true => Ok(()),
        false => Err(name_taken(name)),
    }
}

async fn require_no_user<E: Exec + ?Sized>(ex: &E, name: &str) -> Result<()> {
    let user: Vec<ScimUserRow> =
        select(ex, &SCIM_USER, "WHERE principal = $1", &[name.into()]).await?;
    match user.is_empty() {
        true => Ok(()),
        false => Err(name_taken(name)),
    }
}

/// Conditions on a user, with `$1` the value: the same user name, in any
/// case; the same principal; the same external id. The indexes of user
/// names and external ids are on their lower case.
const SAME_USER_NAME: &str = "lower(user_name) = lower($1)";
const SAME_PRINCIPAL: &str = "principal = $1";
const SAME_EXTERNAL_ID: &str = "lower(external_id) = lower($1) AND external_id = $1";

/// The order of users in lists.
const USER_ORDER: &str = "ORDER BY COALESCE(display_name, ''), user_name, id_string";
const GROUP_ORDER: &str = "ORDER BY display_name, id_string";

/// The users that `same` picks with `value`, other than the user `id`.
async fn others<E: Exec + ?Sized>(
    ex: &E,
    same: &str,
    value: &str,
    id: &str,
) -> Result<Vec<ScimUserRow>> {
    select(
        ex,
        &SCIM_USER,
        &format!("WHERE {same} AND id_string <> $2"),
        &[value.into(), id.into()],
    )
    .await
}

/// Check `row` against the other users and the groups: its user name,
/// principal and external id belong to no other user (`was` is the row
/// before an update), and no group has the principal as name.
async fn check_user<E: Exec + ?Sized>(
    ex: &E,
    row: &ScimUserRow,
    was: Option<&ScimUserRow>,
) -> Result<()> {
    require_non_empty("principal", &row.principal)?;
    require_non_empty("userName", &row.user_name)?;
    let id = row.id_string.as_str();
    if was.is_none_or(|w| !w.user_name.eq_ignore_ascii_case(&row.user_name))
        && !others(ex, SAME_USER_NAME, &row.user_name, id)
            .await?
            .is_empty()
    {
        return Err(Error::already_exists(format!(
            "User already exists: {}",
            row.user_name
        )));
    }
    if was.is_none_or(|w| w.principal != row.principal) {
        if !others(ex, SAME_PRINCIPAL, &row.principal, id)
            .await?
            .is_empty()
        {
            return Err(Error::already_exists(format!(
                "User already exists: {}",
                row.principal
            )));
        }
        require_no_group(ex, &row.principal).await?;
    }
    if let Some(external_id) = row.external_id.as_deref()
        && was.is_none_or(|w| w.external_id.as_deref() != Some(external_id))
        && !others(ex, SAME_EXTERNAL_ID, external_id, id)
            .await?
            .is_empty()
    {
        return Err(Error::already_exists(format!(
            "User already exists with external id: {external_id}"
        )));
    }
    Ok(())
}

/// The row `id` of `table`, locked for the rest of `tx`.
async fn lock<R, E>(tx: &E, table: &crate::sql::rows::TableDef, id: &str) -> Result<Option<R>>
where
    R: serde::de::DeserializeOwned,
    E: Exec + ?Sized,
{
    let rows = select(
        tx,
        table,
        &format!("WHERE id_string = $1{}", tx.for_update()),
        &[id.into()],
    )
    .await?;
    Ok(rows.into_iter().next())
}

fn user_not_found(id: &str) -> Error {
    Error::not_found(format!("User not found: {id}"))
}

fn group_not_found(id: &str) -> Error {
    Error::not_found(format!("Group not found: {id}"))
}

/// Another request changed `what` between its read and this write.
fn changed(what: &str) -> Error {
    Error::aborted(format!("{what} changed during the request"))
}

/// The condition of `key` with `$1` the value, in any case. Ids are
/// lower-case UUIDs.
fn key_filter(key: ScimKey, name_column: &str) -> String {
    match key {
        ScimKey::Id => "WHERE id_string = lower($1)".to_string(),
        ScimKey::Name => format!("WHERE lower({name_column}) = lower($1)"),
        ScimKey::ExternalId => "WHERE lower(external_id) = lower($1)".to_string(),
    }
}

impl SqlMetastore {
    /// The users that `filter` picks, with `params`.
    async fn users_where(&self, filter: &str, params: &[Value]) -> Result<Vec<ScimUser>> {
        let rows: Vec<ScimUserRow> = select(self.db.as_ref(), &SCIM_USER, filter, params).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl ScimOps for SqlMetastore {
    async fn create_user(&self, mut user: ScimUser) -> Result<ScimUser> {
        user.active.get_or_insert(true);
        let row = user_to_row(
            user,
            Uuid::new_v4().to_string(),
            Some(now_epoch_millis()),
            None,
        );
        let failed = db_err("create user");
        let tx = self.db.begin().await.map_err(&failed)?;
        check_user(tx.as_ref(), &row, None).await?;
        insert(tx.as_ref(), &SCIM_USER, &row)
            .await
            .map_err(create_err)?;
        tx.commit().await.map_err(&failed)?;
        Ok(row.into())
    }

    async fn get_user(&self, id: &str) -> Result<ScimUser> {
        let row: Option<ScimUserRow> = by_id(self.db.as_ref(), &SCIM_USER, id).await?;
        row.map(Into::into).ok_or_else(|| user_not_found(id))
    }

    async fn list_users(&self) -> Result<Vec<ScimUser>> {
        // Users without a display name first.
        self.users_where(USER_ORDER, &[]).await
    }

    async fn users_with(&self, key: ScimKey, value: &str) -> Result<Vec<ScimUser>> {
        let filter = key_filter(key, "user_name");
        self.users_where(&format!("{filter} {USER_ORDER}"), &[value.into()])
            .await
    }

    async fn delete_user(&self, id: &str) -> Result<()> {
        let failed = db_err("delete user");
        let tx = self.db.begin().await.map_err(&failed)?;
        let user: ScimUserRow = lock(tx.as_ref(), &SCIM_USER, id)
            .await?
            .ok_or_else(|| user_not_found(id))?;
        let groups: Vec<ScimGroupRow> = select(
            tx.as_ref(),
            &SCIM_GROUP,
            &format!(
                "WHERE id_string IN (SELECT group_id FROM scim_group_member WHERE user_id = $1){}",
                tx.for_update()
            ),
            &[id.into()],
        )
        .await?;
        for mut group in groups {
            if let Some(members) = group.members.as_mut() {
                members.retain(|m| m.value != id);
            }
            group.members = non_empty(group.members);
            group.updated_at = Some(now_epoch_millis());
            update(tx.as_ref(), &SCIM_GROUP, &group)
                .await
                .map_err(&failed)?;
        }
        for sql in [
            "DELETE FROM scim_group_member WHERE user_id = $1",
            "DELETE FROM scim_user WHERE id_string = $1",
        ] {
            tx.execute(sql, &[id.into()]).await.map_err(&failed)?;
        }
        forget_principal(tx.as_ref(), &user.principal).await?;
        tx.commit().await.map_err(&failed)?;
        Ok(())
    }

    async fn get_user_by_name(&self, user_name: &str) -> Result<Option<ScimUser>> {
        let filter = format!("WHERE {SAME_USER_NAME}");
        let users = self.users_where(&filter, &[user_name.into()]).await?;
        Ok(users.into_iter().next())
    }

    async fn find_user_by_principal(&self, principal: &str) -> Result<Option<ScimUser>> {
        let filter = format!("WHERE {SAME_PRINCIPAL}");
        let users = self.users_where(&filter, &[principal.into()]).await?;
        Ok(users.into_iter().next())
    }

    async fn find_user_by_principal_any_case(&self, principal: &str) -> Result<Option<ScimUser>> {
        if let Some(user) = self.find_user_by_principal(principal).await? {
            return Ok(Some(user));
        }
        let mut any = self
            .users_where(
                "WHERE lower(principal) = lower($1) LIMIT 2",
                &[principal.into()],
            )
            .await?;
        Ok(match any.len() {
            1 => any.pop(),
            _ => None,
        })
    }

    async fn find_user_by_external_id(&self, external_id: &str) -> Result<Option<ScimUser>> {
        let filter = format!("WHERE {SAME_EXTERNAL_ID}");
        let users = self.users_where(&filter, &[external_id.into()]).await?;
        Ok(users.into_iter().next())
    }

    async fn update_user(
        &self,
        id: &str,
        user: ScimUser,
        was: Option<&ScimUser>,
    ) -> Result<ScimUser> {
        let failed = db_err("update user");
        let tx = self.db.begin().await.map_err(&failed)?;
        let stored: ScimUserRow = lock(tx.as_ref(), &SCIM_USER, id)
            .await?
            .ok_or_else(|| user_not_found(id))?;
        if was.is_some_and(|was| *was != ScimUser::from(stored.clone())) {
            return Err(changed(&format!("User {id}")));
        }
        let row = user_to_row(
            user,
            id.to_string(),
            stored.created_at,
            Some(now_epoch_millis()),
        );
        check_user(tx.as_ref(), &row, Some(&stored)).await?;
        update(tx.as_ref(), &SCIM_USER, &row)
            .await
            .map_err(create_err)?;
        if row.principal != stored.principal {
            rename_principal(tx.as_ref(), &stored.principal, &row.principal).await?;
        }
        tx.commit().await.map_err(&failed)?;
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
            let mut found = None;
            for same in [SAME_PRINCIPAL, SAME_USER_NAME] {
                let users: Vec<ScimUserRow> = select(
                    tx.as_ref(),
                    &SCIM_USER,
                    &format!("WHERE {same}"),
                    &[email.into()],
                )
                .await?;
                found = users.into_iter().next();
                if found.is_some() {
                    break;
                }
            }
            // The principal that gets the privileges, unless disabled.
            let principal = match found {
                Some(user) => {
                    if let Some(external_id) = seed.external_id.as_deref()
                        && user.external_id.as_deref() != Some(external_id)
                    {
                        return Err(Error::invalid_argument(format!(
                            "User {email} exists with an external id other than {external_id}"
                        )));
                    }
                    (user.active != Some(false)).then_some(user.principal)
                }
                None => {
                    if let Some(external_id) = seed.external_id.as_deref()
                        && let Some(holder) = others(tx.as_ref(), SAME_EXTERNAL_ID, external_id, "")
                            .await?
                            .into_iter()
                            .next()
                    {
                        return Err(Error::already_exists(format!(
                            "The external id {external_id} of {email} belongs to {}",
                            holder.principal
                        )));
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
                            display: None,
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
                    check_user(tx.as_ref(), &row, None).await?;
                    insert(tx.as_ref(), &SCIM_USER, &row)
                        .await
                        .map_err(create_err)?;
                    report.created += 1;
                    Some(row.principal)
                }
            };
            if let Some(principal) = principal
                && !seed.privileges.is_empty()
            {
                let change = PermissionsChange {
                    principal,
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
        let row = group_to_row(
            group,
            Uuid::new_v4().to_string(),
            Some(now_epoch_millis()),
            None,
        );
        let failed = db_err("create group");
        let tx = self.db.begin().await.map_err(&failed)?;
        require_no_user(tx.as_ref(), &row.display_name).await?;
        insert(tx.as_ref(), &SCIM_GROUP, &row)
            .await
            .map_err(create_err)?;
        write_members(tx.as_ref(), &row).await?;
        tx.commit().await.map_err(&failed)?;
        Ok(row.into())
    }

    async fn get_group(&self, id: &str) -> Result<ScimGroup> {
        let row: Option<ScimGroupRow> = by_id(self.db.as_ref(), &SCIM_GROUP, id).await?;
        row.map(Into::into).ok_or_else(|| group_not_found(id))
    }

    async fn list_groups(&self) -> Result<Vec<ScimGroup>> {
        let rows: Vec<ScimGroupRow> =
            select(self.db.as_ref(), &SCIM_GROUP, GROUP_ORDER, &[]).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn groups_with(&self, key: ScimKey, value: &str) -> Result<Vec<ScimGroup>> {
        let filter = key_filter(key, "display_name");
        let rows: Vec<ScimGroupRow> = select(
            self.db.as_ref(),
            &SCIM_GROUP,
            &format!("{filter} {GROUP_ORDER}"),
            &[value.into()],
        )
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_group(
        &self,
        id: &str,
        group: ScimGroup,
        was: Option<&ScimGroup>,
    ) -> Result<ScimGroup> {
        require_non_empty("displayName", &group.display_name)?;
        let failed = db_err("update group");
        let tx = self.db.begin().await.map_err(&failed)?;
        let stored: ScimGroupRow = lock(tx.as_ref(), &SCIM_GROUP, id)
            .await?
            .ok_or_else(|| group_not_found(id))?;
        if was.is_some_and(|was| *was != ScimGroup::from(stored.clone())) {
            return Err(changed(&format!("Group {id}")));
        }
        let row = group_to_row(
            group,
            id.to_string(),
            stored.created_at,
            Some(now_epoch_millis()),
        );
        let renamed = stored.display_name != row.display_name;
        if renamed {
            require_no_user(tx.as_ref(), &row.display_name).await?;
        }
        update(tx.as_ref(), &SCIM_GROUP, &row)
            .await
            .map_err(create_err)?;
        write_members(tx.as_ref(), &row).await?;
        if renamed {
            rename_principal(tx.as_ref(), &stored.display_name, &row.display_name).await?;
        }
        tx.commit().await.map_err(&failed)?;
        Ok(row.into())
    }

    async fn delete_group(&self, id: &str) -> Result<()> {
        let failed = db_err("delete group");
        let tx = self.db.begin().await.map_err(&failed)?;
        let was: ScimGroupRow = lock(tx.as_ref(), &SCIM_GROUP, id)
            .await?
            .ok_or_else(|| group_not_found(id))?;
        for sql in [
            "DELETE FROM scim_group_member WHERE group_id = $1",
            "DELETE FROM scim_group WHERE id_string = $1",
        ] {
            tx.execute(sql, &[id.into()]).await.map_err(&failed)?;
        }
        forget_principal(tx.as_ref(), &was.display_name).await?;
        tx.commit().await.map_err(&failed)?;
        Ok(())
    }
}
