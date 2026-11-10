use std::collections::BTreeSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    PermissionsChange, ScimEmail, ScimGroup, ScimGroupMember, ScimMeta, ScimName, ScimPhoto,
    ScimUser,
};
use uc_common::{Error, ErrorCode, Result, SecurableType, now_epoch_millis, require_non_empty};
use uc_storage::{Exec, Transaction, Value};

use crate::ScimOps;
use crate::permissions::sql::{apply_permission_change, forget_principal, rename_principal};
use crate::scim::{ADMIN, Edit, Page, ScimKey, SeedReport, SeedUser};
use crate::sql::rows::{SCIM_GROUP, SCIM_GROUP_MEMBER, SCIM_USER};
use crate::sql::{
    SqlMetastore, by_id, create_err_as, db_err, insert, insert_all, placeholders, select,
    select_one, update,
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

/// The member ids of `group`.
fn member_ids(group: &ScimGroupRow) -> BTreeSet<&str> {
    (group.members.iter().flatten())
        .map(|m| m.value.as_str())
        .collect()
}

/// Make `scim_group_member` hold the members of `group`, in `tx`, which
/// held those of `was`, the stored group: the rows of members that left
/// go, and new members get one. Each new member must be a user, which
/// stays locked until `tx` ends: a delete of the user waits, then removes
/// it from the group.
async fn write_members<E: Exec + ?Sized>(
    tx: &E,
    group: &ScimGroupRow,
    was: Option<&ScimGroupRow>,
) -> Result<()> {
    let failed = db_err("write group members");
    let group_id = group.id_string.as_str();
    let old = was.map(member_ids).unwrap_or_default();
    let new = member_ids(group);
    let gone: Vec<&str> = (old.iter().copied())
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
                    "SELECT id_string FROM scim_user WHERE id_string IN ({}) ORDER BY id_string{}",
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

/// Lock the names in grants for the rest of `tx`, before a principal is
/// checked against the group names or a group name against the
/// principals: requests that do so at once wait for each other. The lock
/// is the metastore row, which [`SqlMetastore::metastore_row`] makes.
async fn lock_names<E: Exec + ?Sized>(tx: &E) -> Result<()> {
    if !tx.for_update().is_empty() {
        let sql = format!("SELECT singleton FROM metastore{}", tx.for_update());
        tx.query(&sql, &[]).await.map_err(db_err("lock names"))?;
    }
    Ok(())
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
/// case; the same principal; the same principal, in any case; the same
/// external id. The indexes of user names and of principals in any case
/// are on their lower case.
const SAME_USER_NAME: &str = "lower(user_name) = lower($1)";
const SAME_PRINCIPAL: &str = "principal = $1";
const SAME_PRINCIPAL_ANY_CASE: &str = "lower(principal) = lower($1)";
const SAME_EXTERNAL_ID: &str = "external_id = $1";

/// Whether `name` is [`ADMIN`], in any case.
fn is_admin(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case(ADMIN)
}

fn admin_name_taken() -> Error {
    Error::already_exists(format!("The name {ADMIN} belongs to the server's own user"))
}

/// The rules of the user [`ADMIN`], for `row`: the row that replaces `was`,
/// or a new one. The user keeps its name and stays active, and no other
/// user takes the name.
fn check_admin(row: &ScimUserRow, was: Option<&ScimUserRow>) -> Result<()> {
    if was.is_some_and(|was| was.principal == ADMIN) {
        if row.user_name != ADMIN || row.principal != ADMIN {
            return Err(Error::invalid_argument(format!(
                "The server's own user keeps the name {ADMIN}"
            )));
        }
        if row.active == Some(false) {
            return Err(Error::invalid_argument(format!(
                "The server's own user {ADMIN} stays active"
            )));
        }
        return Ok(());
    }
    match is_admin(&row.user_name) || is_admin(&row.principal) {
        true => Err(admin_name_taken()),
        false => Ok(()),
    }
}

/// The names that find the user `row` at sign-in, in lower case: its
/// principal and its user name.
fn sign_in_names(row: &ScimUserRow) -> [Value; 2] {
    [
        row.principal.to_lowercase().into(),
        row.user_name.to_lowercase().into(),
    ]
}

/// Keep the names of the deleted user `row`, in `tx`: sign-in does not make
/// a user with one of them again.
async fn remember_deleted<E: Exec + ?Sized>(tx: &E, row: &ScimUserRow) -> Result<()> {
    forget_deleted(tx, row).await?;
    for name in sign_in_names(row) {
        // The principal can be the user name.
        tx.execute(
            "INSERT INTO scim_deleted_user (name, deleted_at) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
            &[name, now_epoch_millis().into()],
        )
        .await
        .map_err(db_err("keep the names of a deleted user"))?;
    }
    Ok(())
}

/// Drop the names of `row` from those of deleted users, in `tx`: an admin
/// made a user with them.
async fn forget_deleted<E: Exec + ?Sized>(tx: &E, row: &ScimUserRow) -> Result<()> {
    tx.execute(
        "DELETE FROM scim_deleted_user WHERE name IN ($1, $2)",
        &sign_in_names(row),
    )
    .await
    .map_err(db_err("drop the names of a deleted user"))?;
    Ok(())
}

/// Whether a deleted user had the principal or the user name of `row`.
async fn was_deleted<E: Exec + ?Sized>(ex: &E, row: &ScimUserRow) -> Result<bool> {
    let rows = ex
        .query(
            "SELECT 1 FROM scim_deleted_user WHERE name IN ($1, $2) LIMIT 1",
            &sign_in_names(row),
        )
        .await
        .map_err(db_err("names of deleted users"))?;
    Ok(!rows.is_empty())
}

/// The row of the user that `name` names ([`ScimOps::find_user_named`]).
/// With `lock`, in a transaction, the row stays locked until it ends.
async fn user_named<E: Exec + ?Sized>(
    ex: &E,
    name: &str,
    lock: bool,
) -> Result<Option<ScimUserRow>> {
    let lock = if lock { ex.for_update() } else { "" };
    let find = |same: &str, limit: &str| {
        let tail = format!("WHERE {same}{limit}{lock}");
        async move { select::<ScimUserRow, E>(ex, &SCIM_USER, &tail, &[name.into()]).await }
    };
    if let Some(user) = find(SAME_PRINCIPAL, "").await?.pop() {
        return Ok(Some(user));
    }
    let mut any_case = find(SAME_PRINCIPAL_ANY_CASE, " LIMIT 2").await?;
    if any_case.len() == 1 {
        return Ok(any_case.pop());
    }
    Ok(find(SAME_USER_NAME, "").await?.pop())
}

/// The name in grants that `name` stands for, in `tx`: a user's principal,
/// else a group's name, else the principal of the user that `name` names
/// in another case or by user name ([`ScimOps::find_user_named`]). The user
/// or group stays locked until `tx` ends: a delete or a rename of it waits,
/// then finds the grants that `tx` wrote.
pub(crate) async fn lock_grantee<E: Exec + ?Sized>(tx: &E, name: &str) -> Result<Option<String>> {
    let lock = tx.for_update();
    let user: Option<ScimUserRow> = select_one(
        tx,
        &SCIM_USER,
        &format!("WHERE {SAME_PRINCIPAL}{lock}"),
        &[name.into()],
    )
    .await?;
    if let Some(user) = user {
        return Ok(Some(user.principal));
    }
    let group: Option<ScimGroupRow> = select_one(
        tx,
        &SCIM_GROUP,
        &format!("WHERE display_name = $1{lock}"),
        &[name.into()],
    )
    .await?;
    if let Some(group) = group {
        return Ok(Some(group.display_name));
    }
    Ok(user_named(tx, name, true).await?.map(|user| user.principal))
}

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
/// before an update), and no group has the principal as name. A new
/// principal locks the names ([`lock_names`]).
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
        lock_names(ex).await?;
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

/// The error of a write that another request's user name, principal or
/// external id stopped, after the checks of [`check_user`].
fn user_exists(row: &ScimUserRow) -> impl FnOnce(uc_storage::StorageError) -> Error {
    create_err_as(
        ErrorCode::AlreadyExists,
        format!("User already exists: {}", row.user_name),
    )
}

/// The error of a write that another group's name stopped.
fn group_exists(row: &ScimGroupRow) -> impl FnOnce(uc_storage::StorageError) -> Error {
    create_err_as(
        ErrorCode::AlreadyExists,
        format!("Group already exists: {}", row.display_name),
    )
}

fn user_not_found(id: &str) -> Error {
    Error::not_found(format!("User not found: {id}"))
}

fn group_not_found(id: &str) -> Error {
    Error::not_found(format!("Group not found: {id}"))
}

/// The condition of `key` with `$1` the value: in any case, but for the
/// external id. Ids are lower-case UUIDs.
fn key_filter(key: ScimKey, name_column: &str) -> String {
    match key {
        ScimKey::Id => "WHERE id_string = lower($1)".to_string(),
        ScimKey::Name => format!("WHERE lower({name_column}) = lower($1)"),
        ScimKey::DisplayName => "WHERE lower(display_name) = lower($1)".to_string(),
        ScimKey::ExternalId => format!("WHERE {SAME_EXTERNAL_ID}"),
    }
}

/// `LIMIT … OFFSET …` for `page`; nothing for every row.
fn page_sql(page: Option<Page>) -> String {
    match page {
        Some(Page { offset, limit }) => format!(" LIMIT {limit} OFFSET {offset}"),
        None => String::new(),
    }
}

impl SqlMetastore {
    /// The users that `filter` picks, with `params`.
    async fn users_where(&self, filter: &str, params: &[Value]) -> Result<Vec<ScimUser>> {
        let rows: Vec<ScimUserRow> = select(self.db.as_ref(), &SCIM_USER, filter, params).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// The number of rows of `table`.
    async fn count(&self, table: &str) -> Result<usize> {
        let failed = db_err("count");
        let sql = format!("SELECT COUNT(*) FROM {table}");
        let rows = self.db.query(&sql, &[]).await.map_err(&failed)?;
        let count = match rows.first() {
            Some(row) => row.int(0).map_err(&failed)?,
            None => 0,
        };
        Ok(usize::try_from(count).unwrap_or_default())
    }
}

/// A user that a request changes ([`ScimOps::edit_user`]): its row is
/// locked in `tx`.
struct UserEdit {
    tx: Box<dyn Transaction>,
    stored: ScimUserRow,
    current: ScimUser,
}

#[async_trait]
impl Edit<ScimUser> for UserEdit {
    fn current(&self) -> &ScimUser {
        &self.current
    }

    async fn save(self: Box<Self>, new: ScimUser) -> Result<ScimUser> {
        let UserEdit { tx, stored, .. } = *self;
        let failed = db_err("update user");
        let row = user_to_row(
            new,
            stored.id_string.clone(),
            stored.created_at,
            Some(now_epoch_millis()),
        );
        check_admin(&row, Some(&stored))?;
        check_user(tx.as_ref(), &row, Some(&stored)).await?;
        update(tx.as_ref(), &SCIM_USER, &row)
            .await
            .map_err(user_exists(&row))?;
        if row.principal != stored.principal {
            rename_principal(tx.as_ref(), &stored.principal, &row.principal).await?;
        }
        forget_deleted(tx.as_ref(), &row).await?;
        tx.commit().await.map_err(&failed)?;
        Ok(row.into())
    }
}

/// A group that a request changes ([`ScimOps::edit_group`]): its row is
/// locked in `tx`.
struct GroupEdit {
    tx: Box<dyn Transaction>,
    stored: ScimGroupRow,
    current: ScimGroup,
}

#[async_trait]
impl Edit<ScimGroup> for GroupEdit {
    fn current(&self) -> &ScimGroup {
        &self.current
    }

    async fn save(self: Box<Self>, new: ScimGroup) -> Result<ScimGroup> {
        let GroupEdit { tx, stored, .. } = *self;
        let failed = db_err("update group");
        require_non_empty("displayName", &new.display_name)?;
        let row = group_to_row(
            new,
            stored.id_string.clone(),
            stored.created_at,
            Some(now_epoch_millis()),
        );
        let renamed = stored.display_name != row.display_name;
        if renamed && is_admin(&row.display_name) {
            return Err(admin_name_taken());
        }
        update(tx.as_ref(), &SCIM_GROUP, &row)
            .await
            .map_err(group_exists(&row))?;
        write_members(tx.as_ref(), &row, Some(&stored)).await?;
        if renamed {
            lock_names(tx.as_ref()).await?;
            require_no_user(tx.as_ref(), &row.display_name).await?;
            rename_principal(tx.as_ref(), &stored.display_name, &row.display_name).await?;
        }
        tx.commit().await.map_err(&failed)?;
        Ok(row.into())
    }
}

impl SqlMetastore {
    /// Write the new user `row`. At a sign-in, a user that had its names
    /// and was deleted is not made again: `None`.
    async fn insert_user(&self, row: ScimUserRow, at_sign_in: bool) -> Result<Option<ScimUser>> {
        self.metastore_row().await?;
        let failed = db_err("create user");
        let tx = self.db.begin().await.map_err(&failed)?;
        match at_sign_in {
            true if was_deleted(tx.as_ref(), &row).await? => return Ok(None),
            true => {}
            false => forget_deleted(tx.as_ref(), &row).await?,
        }
        check_user(tx.as_ref(), &row, None).await?;
        insert(tx.as_ref(), &SCIM_USER, &row)
            .await
            .map_err(user_exists(&row))?;
        tx.commit().await.map_err(&failed)?;
        Ok(Some(row.into()))
    }
}

/// The row of the new user `user`, enabled unless `active` is false.
fn new_user_row(mut user: ScimUser) -> ScimUserRow {
    user.active.get_or_insert(true);
    user_to_row(
        user,
        Uuid::new_v4().to_string(),
        Some(now_epoch_millis()),
        None,
    )
}

#[async_trait]
impl ScimOps for SqlMetastore {
    async fn create_user(&self, user: ScimUser) -> Result<ScimUser> {
        let row = new_user_row(user);
        check_admin(&row, None)?;
        let made = self.insert_user(row, false).await?;
        made.ok_or_else(|| Error::internal("no user made"))
    }

    async fn create_admin(&self) -> Result<()> {
        if self.find_user_by_principal(ADMIN).await?.is_some() {
            return Ok(());
        }
        // No email: the user name is the principal.
        let row = new_user_row(ScimUser {
            schemas: vec![schema_uri::USER.to_string()],
            user_name: ADMIN.to_string(),
            display_name: Some("Admin".to_string()),
            ..Default::default()
        });
        match self.insert_user(row, false).await {
            // Another replica made it at the same time.
            Err(e)
                if e.code().is_already_exists()
                    && self.find_user_by_principal(ADMIN).await?.is_some() =>
            {
                Ok(())
            }
            made => made.map(drop),
        }
    }

    async fn create_user_at_sign_in(&self, user: ScimUser) -> Result<Option<ScimUser>> {
        let row = new_user_row(user);
        check_admin(&row, None)?;
        self.insert_user(row, true).await
    }

    async fn get_user(&self, id: &str) -> Result<ScimUser> {
        let row: Option<ScimUserRow> = by_id(self.db.as_ref(), &SCIM_USER, id).await?;
        row.map(Into::into).ok_or_else(|| user_not_found(id))
    }

    async fn list_users(&self, page: Option<Page>) -> Result<Vec<ScimUser>> {
        // Users without a display name first.
        self.users_where(&format!("{USER_ORDER}{}", page_sql(page)), &[])
            .await
    }

    async fn count_users(&self) -> Result<usize> {
        self.count(SCIM_USER.name).await
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
        if user.principal == ADMIN {
            return Err(Error::new(
                ErrorCode::PermissionDenied,
                format!("The server's own user {ADMIN} cannot be deleted"),
            ));
        }
        // By id: deletes that run at once lock the groups in one order.
        let groups: Vec<ScimGroupRow> = select(
            tx.as_ref(),
            &SCIM_GROUP,
            &format!(
                "WHERE id_string IN (SELECT group_id FROM scim_group_member WHERE user_id = $1) \
                 ORDER BY id_string{}",
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
        remember_deleted(tx.as_ref(), &user).await?;
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
        let filter = format!("WHERE {SAME_PRINCIPAL_ANY_CASE} LIMIT 2");
        let mut any = self.users_where(&filter, &[principal.into()]).await?;
        Ok(match any.len() {
            1 => any.pop(),
            _ => None,
        })
    }

    async fn find_user_named(&self, name: &str) -> Result<Option<ScimUser>> {
        let user = user_named(self.db.as_ref(), name, false).await?;
        Ok(user.map(Into::into))
    }

    async fn find_user_by_external_id(&self, external_id: &str) -> Result<Option<ScimUser>> {
        let filter = format!("WHERE {SAME_EXTERNAL_ID}");
        let users = self.users_where(&filter, &[external_id.into()]).await?;
        Ok(users.into_iter().next())
    }

    async fn edit_user(&self, id: &str) -> Result<Box<dyn Edit<ScimUser>>> {
        self.metastore_row().await?;
        let tx = self.db.begin().await.map_err(db_err("update user"))?;
        let stored: ScimUserRow = lock(tx.as_ref(), &SCIM_USER, id)
            .await?
            .ok_or_else(|| user_not_found(id))?;
        Ok(Box::new(UserEdit {
            tx,
            current: stored.clone().into(),
            stored,
        }))
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
            let found = user_named(tx.as_ref(), email, true).await?;
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
                    let row = new_user_row(user);
                    check_admin(&row, None)?;
                    forget_deleted(tx.as_ref(), &row).await?;
                    check_user(tx.as_ref(), &row, None).await?;
                    insert(tx.as_ref(), &SCIM_USER, &row)
                        .await
                        .map_err(user_exists(&row))?;
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
        if is_admin(&group.display_name) {
            return Err(admin_name_taken());
        }
        let row = group_to_row(
            group,
            Uuid::new_v4().to_string(),
            Some(now_epoch_millis()),
            None,
        );
        self.metastore_row().await?;
        let failed = db_err("create group");
        let tx = self.db.begin().await.map_err(&failed)?;
        insert(tx.as_ref(), &SCIM_GROUP, &row)
            .await
            .map_err(group_exists(&row))?;
        write_members(tx.as_ref(), &row, None).await?;
        lock_names(tx.as_ref()).await?;
        require_no_user(tx.as_ref(), &row.display_name).await?;
        tx.commit().await.map_err(&failed)?;
        Ok(row.into())
    }

    async fn get_group(&self, id: &str) -> Result<ScimGroup> {
        let row: Option<ScimGroupRow> = by_id(self.db.as_ref(), &SCIM_GROUP, id).await?;
        row.map(Into::into).ok_or_else(|| group_not_found(id))
    }

    async fn list_groups(&self, page: Option<Page>) -> Result<Vec<ScimGroup>> {
        let tail = format!("{GROUP_ORDER}{}", page_sql(page));
        let rows: Vec<ScimGroupRow> = select(self.db.as_ref(), &SCIM_GROUP, &tail, &[]).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_groups(&self) -> Result<usize> {
        self.count(SCIM_GROUP.name).await
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

    async fn edit_group(&self, id: &str) -> Result<Box<dyn Edit<ScimGroup>>> {
        self.metastore_row().await?;
        let tx = self.db.begin().await.map_err(db_err("update group"))?;
        let stored: ScimGroupRow = lock(tx.as_ref(), &SCIM_GROUP, id)
            .await?
            .ok_or_else(|| group_not_found(id))?;
        Ok(Box::new(GroupEdit {
            tx,
            current: stored.clone().into(),
            stored,
        }))
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
