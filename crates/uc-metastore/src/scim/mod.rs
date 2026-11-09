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

/// A page of a list: `limit` items after the first `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub offset: usize,
    pub limit: usize,
}

/// A change of a stored user or group: the new one from the stored one.
/// It runs while the stored one is locked, so no other request changes it
/// in between. An error leaves the stored one as it was.
pub type Change<'a, T> = &'a mut (dyn FnMut(&T) -> Result<T> + Send);

/// SCIM users and groups. A user's principal ([`ScimUser::principal`]) is
/// its name in grants and owners. User names compare without case.
#[async_trait]
pub trait ScimOps: Send + Sync {
    /// Make a user, enabled unless `active` is false. Its user name (the
    /// principal when empty), its principal and its external id belong to
    /// no other user, and no group has the principal as name.
    async fn create_user(&self, user: ScimUser) -> Result<ScimUser>;
    async fn get_user(&self, id: &str) -> Result<ScimUser>;
    /// The users by name: those of `page`, else every user.
    async fn list_users(&self, page: Option<Page>) -> Result<Vec<ScimUser>>;
    async fn count_users(&self) -> Result<usize>;
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
    /// Replace the user at `id` with the one that `change` makes, under
    /// the rules of [`ScimOps::create_user`]. When the principal changes,
    /// the grants, owned objects and tokens of the old one move to the new
    /// one, in the same transaction.
    async fn update_user(&self, id: &str, change: Change<'_, ScimUser>) -> Result<ScimUser>;
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
    /// The groups by name: those of `page`, else every group.
    async fn list_groups(&self, page: Option<Page>) -> Result<Vec<ScimGroup>>;
    async fn count_groups(&self) -> Result<usize>;
    /// The groups whose `key` is `value` in any case, by name.
    async fn groups_with(&self, key: ScimKey, value: &str) -> Result<Vec<ScimGroup>>;
    /// Replace the group at `id` with the one that `change` makes. A
    /// renamed group keeps its grants and owned objects.
    async fn update_group(&self, id: &str, change: Change<'_, ScimGroup>) -> Result<ScimGroup>;
    /// Delete the group and its grants. The objects that it owns lose
    /// their owner.
    async fn delete_group(&self, id: &str) -> Result<()>;
}
