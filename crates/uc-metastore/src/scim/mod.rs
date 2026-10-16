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

#[async_trait]
pub trait ScimOps: Send + Sync {
    async fn create_user(&self, user: ScimUser) -> Result<ScimUser>;
    async fn get_user(&self, id: &str) -> Result<ScimUser>;
    /// Every user, by name and email (upstream lists by name and filters in
    /// memory).
    async fn list_users(&self) -> Result<Vec<ScimUser>>;
    async fn delete_user(&self, id: &str) -> Result<()>;
    async fn get_user_by_name(&self, user_name: &str) -> Result<Option<ScimUser>>;
    /// The user with the email `email`, which is also its user name.
    async fn find_user_by_email(&self, email: &str) -> Result<Option<ScimUser>>;
    async fn find_user_by_external_id(&self, external_id: &str) -> Result<Option<ScimUser>>;
    /// Replace the user at `id` with `user`, which the caller has changed and
    /// validated.
    async fn update_user(&self, id: &str, user: ScimUser) -> Result<ScimUser>;
    /// Make the missing users of `users` and grant their missing metastore
    /// privileges, in one transaction. A user that exists stays as it is,
    /// and a disabled one gets no privileges. An entry whose external id
    /// differs from its user's, or belongs to another user, is an error.
    async fn seed_users(&self, users: &[SeedUser]) -> Result<SeedReport>;

    async fn create_group(&self, group: ScimGroup) -> Result<ScimGroup>;
    async fn get_group(&self, id: &str) -> Result<ScimGroup>;
    /// Every group, by name.
    async fn list_groups(&self) -> Result<Vec<ScimGroup>>;
    async fn update_group(&self, id: &str, group: ScimGroup) -> Result<ScimGroup>;
    async fn delete_group(&self, id: &str) -> Result<()>;
}
