pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::{Privilege, ScimGroup, ScimUser};
use uc_common::Result;

/// A user that the server makes at start when it is missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedUser {
    /// The user's email: its name in grants and owners.
    pub email: String,
    pub display_name: Option<String>,
    /// The id of the user's OAuth client.
    pub external_id: Option<String>,
    /// Privileges on the metastore.
    pub privileges: Vec<Privilege>,
}

/// What [`ScimOps::seed_users`] changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeedReport {
    /// The users it made.
    pub created: usize,
    /// The privileges it granted.
    pub granted: usize,
}

/// An attribute that finds users or groups by index: the id, the name (a
/// user's `userName`, a group's `displayName`) or the external id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScimKey {
    Id,
    Name,
    ExternalId,
}

/// SCIM users and groups. A user's principal ([`ScimUser::principal`]) is
/// its name in grants and owners. User names compare without case.
#[async_trait]
pub trait ScimOps: Send + Sync {
    /// Make a user, enabled unless `active` is false. Its user name (the
    /// principal when empty), its principal and its external id belong to
    /// no other user, and no group has the principal as name.
    async fn create_user(&self, user: ScimUser) -> Result<ScimUser>;
    async fn get_user(&self, id: &str) -> Result<ScimUser>;
    /// Every user, by name.
    async fn list_users(&self) -> Result<Vec<ScimUser>>;
    /// The users whose `key` is `value` in any case, by name.
    async fn users_with(&self, key: ScimKey, value: &str) -> Result<Vec<ScimUser>>;
    /// Delete the user with its grants, its tokens and its group
    /// memberships. The objects that it owns lose their owner, so a later
    /// user with the same principal does not get them.
    async fn delete_user(&self, id: &str) -> Result<()>;
    /// The user with the user name `user_name`, in any case.
    async fn get_user_by_name(&self, user_name: &str) -> Result<Option<ScimUser>>;
    /// The user whose principal is `principal`.
    async fn find_user_by_principal(&self, principal: &str) -> Result<Option<ScimUser>>;
    /// The user whose principal is `principal` in any case: the one with it
    /// as it is written, else the only one with it in another case.
    async fn find_user_by_principal_any_case(&self, principal: &str) -> Result<Option<ScimUser>>;
    async fn find_user_by_external_id(&self, external_id: &str) -> Result<Option<ScimUser>>;
    /// Replace the user at `id` with `user`, under the rules of
    /// [`ScimOps::create_user`]. When the principal changes, the grants,
    /// owned objects and tokens of the old one move to the new one, in the
    /// same transaction. With `was`, the user that `user` was made from,
    /// the error is `Aborted` when another request changed the user since.
    async fn update_user(
        &self,
        id: &str,
        user: ScimUser,
        was: Option<&ScimUser>,
    ) -> Result<ScimUser>;
    /// Make the missing users of `users` and grant their missing metastore
    /// privileges, in one transaction. An entry names a user by principal,
    /// else by user name. A user that exists stays as it is, and a disabled
    /// one gets no privileges. An entry whose external id differs from its
    /// user's, or belongs to another user, is an error.
    async fn seed_users(&self, users: &[SeedUser]) -> Result<SeedReport>;

    /// A group's name is its name in grants, which its members get. No
    /// user can have it as principal. Members are users.
    async fn create_group(&self, group: ScimGroup) -> Result<ScimGroup>;
    async fn get_group(&self, id: &str) -> Result<ScimGroup>;
    /// Every group, by name.
    async fn list_groups(&self) -> Result<Vec<ScimGroup>>;
    /// The groups whose `key` is `value` in any case, by name.
    async fn groups_with(&self, key: ScimKey, value: &str) -> Result<Vec<ScimGroup>>;
    /// Replace the group at `id` with `group`. A renamed group keeps its
    /// grants and owned objects. With `was`, as for
    /// [`ScimOps::update_user`].
    async fn update_group(
        &self,
        id: &str,
        group: ScimGroup,
        was: Option<&ScimGroup>,
    ) -> Result<ScimGroup>;
    /// Delete the group and its grants. The objects that it owns lose
    /// their owner.
    async fn delete_group(&self, id: &str) -> Result<()>;
}
