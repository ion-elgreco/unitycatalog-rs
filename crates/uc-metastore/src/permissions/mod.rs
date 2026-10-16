pub(crate) mod sql;

use std::collections::HashMap;

use async_trait::async_trait;

use uc_api::{PermissionsList, Privilege, UpdatePermissions};
use uc_common::{NormalizedUrl, Result, SecurableType};

/// A securable in an authorization check: its kind, id and owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: SecurableType,
    pub id: String,
    pub owner: Option<String>,
}

/// The securables an authorization rule names: upstream's `#catalog` and
/// `#schema`, and the object itself (a table, volume, function, model,
/// external location or credential; a catalog or schema is its own object).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lineage {
    pub catalog: Option<Node>,
    pub schema: Option<Node>,
    pub object: Option<Node>,
}

/// What owns a storage path, after upstream `getMapResourceIdsForPath`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathOwner {
    /// The table, volume or registered model whose location contains it.
    Data(SecurableType, Lineage),
    /// The external location that contains it.
    ExternalLocation(Lineage),
    Unowned,
}

#[async_trait]
pub trait PermissionOps: Send + Sync {
    async fn get(
        &self,
        securable_type: SecurableType,
        full_name: &str,
        principal_filter: Option<&str>,
    ) -> Result<PermissionsList>;

    async fn update(
        &self,
        securable_type: SecurableType,
        full_name: &str,
        req: UpdatePermissions,
    ) -> Result<PermissionsList>;

    /// Returns the metastore's persistent UUID, creating it on first call.
    async fn metastore_id(&self) -> Result<String>;

    /// The securable `name` of `kind` and its parents, or the not-found
    /// error of its kind.
    async fn lineage(&self, kind: SecurableType, name: &str) -> Result<Lineage>;

    /// The volume or registered model with id `id`, and its parents.
    async fn lineage_by_id(&self, kind: SecurableType, id: &str) -> Result<Lineage>;

    /// The table (or uncommitted staging table, owned by its creator) with
    /// id `table_id` and its parents; `TABLE_NOT_FOUND` otherwise.
    async fn table_lineage(&self, table_id: &str) -> Result<Lineage>;

    /// Every grant of `principal`, by resource id.
    async fn grants_of(&self, principal: &str) -> Result<HashMap<String, Vec<Privilege>>>;

    /// Remove every grant of `principal` (upstream
    /// `clearAuthorizationsForPrincipal`).
    async fn clear_principal(&self, principal: &str) -> Result<()>;

    /// The metastore's owner (upstream grants the bootstrap admin OWNER).
    async fn metastore_owner(&self) -> Result<Option<String>>;

    async fn set_metastore_owner(&self, owner: &str) -> Result<()>;

    /// Upstream `getMapResourceIdsForPath`: the data securable or external
    /// location that contains `url`. A path over a pending storage cleanup
    /// or above another entity is `PERMISSION_DENIED`; one inside two
    /// entities is `INVALID_ARGUMENT`.
    async fn path_owner(&self, url: &NormalizedUrl) -> Result<PathOwner>;
}

/// The id of the row of grants of `principal` on a securable: one row per
/// (securable, principal). The principal is hashed, so any string is safe
/// in the key and no two principals share one.
pub fn permission_key(resource_id: &str, principal: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(principal.as_bytes());
    format!("{resource_id}_{}", hex::encode(&digest[..16]))
}
