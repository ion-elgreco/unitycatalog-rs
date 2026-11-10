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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedReport {
    /// The users it made.
    pub created: usize,
    /// The privileges it granted.
    pub granted: usize,
    /// The users it did not make, as they were deleted: only an admin makes
    /// a deleted user again.
    pub deleted: Vec<String>,
}

/// The server's own user, the first owner of the metastore, which
/// `uc-server token` signs in. Only [`ScimOps::create_admin`] makes it. It
/// keeps its name, stays active and is never deleted, and no other user or
/// group takes the name, in any case.
pub const ADMIN: &str = "admin";

/// An attribute that the database finds users or groups by: the id, the
/// name (a user's `userName`, a group's `displayName`), a user's display
/// name, or the external id. The external id compares with case, the
/// others without.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScimKey {
    Id,
    Name,
    DisplayName,
    ExternalId,
}

/// A page of a list: `limit` items after the first `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub offset: usize,
    pub limit: usize,
}

/// A stored user or group that a request changes. It is locked from
/// [`ScimOps::edit_user`] or [`ScimOps::edit_group`] until [`Edit::save`],
/// so no other request changes it in between. Dropped without a save, it
/// stays as it was.
#[async_trait]
pub trait Edit<T>: Send {
    /// The stored one.
    fn current(&self) -> &T;
    /// Replace the stored one with `new`.
    async fn save(self: Box<Self>, new: T) -> Result<T>;
}

/// SCIM users and groups. A user's principal ([`ScimUser::principal`]) is
/// its name in grants and owners. User names compare without case, external
/// ids with case.
#[async_trait]
pub trait ScimOps: Send + Sync {
    /// Make a user, enabled unless `active` is false. Its user name (the
    /// principal when empty), its principal and its external id belong to
    /// no other user, and no group has the principal as name. The name
    /// [`ADMIN`] is `ALREADY_EXISTS`.
    async fn create_user(&self, user: ScimUser) -> Result<ScimUser>;
    /// Make the user [`ADMIN`], unless it exists.
    async fn create_admin(&self) -> Result<()>;
    /// Make a user at its first sign-in, as [`ScimOps::create_user`] does.
    /// `None`, and no user, when a deleted user had its principal or user
    /// name: only [`ScimOps::create_user`] makes that user again.
    async fn create_user_at_sign_in(&self, user: ScimUser) -> Result<Option<ScimUser>>;
    async fn get_user(&self, id: &str) -> Result<ScimUser>;
    /// The users by name: those of `page`, else every user.
    async fn list_users(&self, page: Option<Page>) -> Result<Vec<ScimUser>>;
    async fn count_users(&self) -> Result<usize>;
    /// The users whose `key` is `value`, by name.
    async fn users_with(&self, key: ScimKey, value: &str) -> Result<Vec<ScimUser>>;
    /// Delete the user with its grants, its tokens and its group
    /// memberships. The objects that it owns lose their owner, so a later
    /// user with the same principal does not get them. The user [`ADMIN`]
    /// is `PERMISSION_DENIED`.
    async fn delete_user(&self, id: &str) -> Result<()>;
    /// The user with the user name `user_name`, in any case.
    async fn get_user_by_name(&self, user_name: &str) -> Result<Option<ScimUser>>;
    /// The user whose principal is `principal`.
    async fn find_user_by_principal(&self, principal: &str) -> Result<Option<ScimUser>>;
    /// The user whose principal is `principal` in any case: the one with it
    /// as it is written, else the only one with it in another case.
    async fn find_user_by_principal_any_case(&self, principal: &str) -> Result<Option<ScimUser>>;
    /// The user that `name` names, for sign-in, grants and the users of
    /// the settings: the one of [`ScimOps::find_user_by_principal_any_case`],
    /// else the one of [`ScimOps::get_user_by_name`].
    async fn find_user_named(&self, name: &str) -> Result<Option<ScimUser>>;
    /// The user with the external id `external_id`, as it is written.
    async fn find_user_by_external_id(&self, external_id: &str) -> Result<Option<ScimUser>>;
    /// The user at `id`, to change: [`Edit::save`] replaces it under the
    /// rules of [`ScimOps::create_user`]. When the principal changes, the
    /// grants, owned objects and tokens of the old one move to the new
    /// one, in the same transaction. The user [`ADMIN`] keeps its name and
    /// stays active.
    async fn edit_user(&self, id: &str) -> Result<Box<dyn Edit<ScimUser>>>;
    /// Make the missing users of `users` and grant their missing metastore
    /// privileges, in one transaction. An entry names a user as
    /// [`ScimOps::find_user_named`] does. A user that exists stays as it
    /// is, and a disabled one gets no privileges. An entry whose external
    /// id differs from its user's, or belongs to another user, is an error.
    async fn seed_users(&self, users: &[SeedUser]) -> Result<SeedReport>;

    /// A group's name is its name in grants, which its members get. No
    /// user can have it as principal, and it is not [`ADMIN`]. Members are
    /// users.
    async fn create_group(&self, group: ScimGroup) -> Result<ScimGroup>;
    async fn get_group(&self, id: &str) -> Result<ScimGroup>;
    /// The groups by name: those of `page`, else every group.
    async fn list_groups(&self, page: Option<Page>) -> Result<Vec<ScimGroup>>;
    async fn count_groups(&self) -> Result<usize>;
    /// The groups whose `key` is `value`, by name.
    async fn groups_with(&self, key: ScimKey, value: &str) -> Result<Vec<ScimGroup>>;
    /// The group at `id`, to change: [`Edit::save`] replaces it under the
    /// rules of [`ScimOps::create_group`]. A renamed group keeps its grants
    /// and owned objects.
    async fn edit_group(&self, id: &str) -> Result<Box<dyn Edit<ScimGroup>>>;
    /// Delete the group and its grants. The objects that it owns lose
    /// their owner.
    async fn delete_group(&self, id: &str) -> Result<()>;
}
