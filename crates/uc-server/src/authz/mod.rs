//! Authorization, after upstream's jCasbin model (`jcasbin_auth_model.conf`),
//! `UnityAccessDecorator` and `AuthorizeExpressions`.
//!
//! - `OWNER` of a securable is its `owner` field (the creator, or as
//!   transferred). It is not inherited.
//! - The metastore `OWNER`, the admin, is the metastore owner (the bootstrap
//!   `admin`) or one of `auth.admin_principals`.
//! - Any other privilege counts when granted on the securable or on one of
//!   its parents: metastore, catalog, schema, object. Credentials and
//!   external locations have no parents.
//! - Each endpoint combines these as upstream's expression does, see
//!   [`rules`]. A denial is 403 "Access denied.".
//!
//! With authorization off every check passes, as upstream's
//! `AllowingAuthorizer`.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use uc_api::Privilege;
use uc_common::{Error, ErrorCode, Result, SecurableType};
use uc_metastore::DynMetastore;

pub use uc_metastore::{Lineage, Node, PathOwner};

use crate::{authn::principal::Principal, server::state::AppState};

pub mod cedar;

/// The grants of one principal.
pub trait Grants: Send + Sync {
    /// Whether `privilege` is granted on `chain[0]` or on one of its parents
    /// (the rest of `chain`, ending with the metastore).
    fn holds(&self, chain: &[&Node], privilege: Privilege) -> bool;
}

/// Where grants come from.
#[async_trait]
pub trait PolicyEngine: Send + Sync + 'static {
    async fn grants(&self, principal: &str) -> Result<Box<dyn Grants>>;
}

/// Every privilege is held: authorization off.
struct AllowAll;

impl Grants for AllowAll {
    fn holds(&self, _chain: &[&Node], _privilege: Privilege) -> bool {
        true
    }
}

/// The principal's grants in the `permission` table, read once per request.
pub struct PermissionEnforcer {
    metastore: DynMetastore,
}

impl PermissionEnforcer {
    pub fn new(metastore: DynMetastore) -> Self {
        Self { metastore }
    }
}

/// Grants by resource id.
pub struct TableGrants(pub HashMap<String, HashSet<Privilege>>);

impl Grants for TableGrants {
    fn holds(&self, chain: &[&Node], privilege: Privilege) -> bool {
        chain.iter().any(|node| {
            self.0
                .get(&node.id)
                .is_some_and(|held| held.contains(&privilege))
        })
    }
}

#[async_trait]
impl PolicyEngine for PermissionEnforcer {
    async fn grants(&self, principal: &str) -> Result<Box<dyn Grants>> {
        let grants = self.metastore.permissions().grants_of(principal).await?;
        Ok(Box::new(TableGrants(
            grants
                .into_iter()
                .map(|(id, p)| (id, p.into_iter().collect()))
                .collect(),
        )))
    }
}

/// Upstream's denial.
pub fn denied() -> Error {
    Error::new(ErrorCode::PermissionDenied, "Access denied.")
}

/// A level of a [`Lineage`] that a rule names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Metastore,
    Catalog,
    Schema,
    Object,
}

/// One caller's authorization for a request.
pub struct Access {
    enabled: bool,
    principal: String,
    admin: bool,
    metastore: Node,
    grants: Box<dyn Grants>,
}

impl Access {
    pub async fn load(state: &AppState, principal: &str) -> Result<Self> {
        if !state.authorization_enabled {
            return Ok(Self {
                enabled: false,
                principal: principal.to_string(),
                admin: true,
                metastore: Node {
                    kind: SecurableType::Metastore,
                    id: String::new(),
                    owner: None,
                },
                grants: Box::new(AllowAll),
            });
        }
        let permissions = state.metastore.permissions();
        let (metastore_id, owner, grants) = tokio::try_join!(
            permissions.metastore_id(),
            permissions.metastore_owner(),
            state.policy.grants(principal),
        )?;
        Ok(Self {
            enabled: true,
            principal: principal.to_string(),
            admin: is_metastore_admin(state, principal, owner.as_deref()),
            metastore: Node {
                kind: SecurableType::Metastore,
                id: metastore_id,
                owner,
            },
            grants,
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn principal(&self) -> &str {
        &self.principal
    }

    /// `Ok` when authorization is off or `allowed`, else "Access denied.".
    pub fn check(&self, allowed: bool) -> Result<()> {
        match !self.enabled || allowed {
            true => Ok(()),
            false => Err(denied()),
        }
    }

    /// `#authorize(#principal, #metastore, OWNER)`.
    pub fn admin(&self) -> bool {
        self.admin
    }

    fn node<'a>(&'a self, lineage: &'a Lineage, level: Level) -> Option<&'a Node> {
        match level {
            Level::Metastore => Some(&self.metastore),
            Level::Catalog => lineage.catalog.as_ref(),
            Level::Schema => lineage.schema.as_ref(),
            Level::Object => lineage.object.as_ref(),
        }
    }

    /// `#authorize(#principal, #<level>, OWNER)`.
    pub fn owner(&self, lineage: &Lineage, level: Level) -> bool {
        if !self.enabled {
            return true;
        }
        if level == Level::Metastore {
            return self.admin;
        }
        self.node(lineage, level)
            .is_some_and(|n| n.owner.as_deref() == Some(self.principal.as_str()))
    }

    /// `#authorize(#principal, #<level>, privilege)`: granted there or on a
    /// parent.
    pub fn has(&self, lineage: &Lineage, level: Level, privilege: Privilege) -> bool {
        if !self.enabled {
            return true;
        }
        let Some(node) = self.node(lineage, level) else {
            return false;
        };
        let mut chain = vec![node];
        let parents: &[Level] = match level {
            Level::Metastore => &[],
            Level::Catalog => &[Level::Metastore],
            Level::Schema => &[Level::Catalog, Level::Metastore],
            Level::Object => &[Level::Schema, Level::Catalog, Level::Metastore],
        };
        for parent in parents {
            // Credentials and external locations are outside the
            // metastore's tree: upstream gives them no hierarchy parent.
            if *parent == Level::Metastore && lineage.catalog.is_none() {
                continue;
            }
            if let Some(parent) = self.node(lineage, *parent) {
                // A catalog or schema is its own object.
                if !chain.iter().any(|n| n.id == parent.id) {
                    chain.push(parent);
                }
            }
        }
        self.grants.holds(&chain, privilege)
    }

    /// `#authorizeAny(#principal, #<level>, OWNER, privileges…)`.
    pub fn owner_or_any(&self, lineage: &Lineage, level: Level, privileges: &[Privilege]) -> bool {
        self.owner(lineage, level) || privileges.iter().any(|p| self.has(lineage, level, *p))
    }

    /// `#authorizeAll(#principal, #<level>, privileges…)`.
    pub fn all(&self, lineage: &Lineage, level: Level, privileges: &[Privilege]) -> bool {
        privileges.iter().all(|p| self.has(lineage, level, *p))
    }
}

/// The metastore admin: one of `auth.admin_principals`, or the metastore
/// owner `owner`.
fn is_metastore_admin(state: &AppState, principal: &str, owner: Option<&str>) -> bool {
    state.admin_principals.contains(principal) || owner == Some(principal)
}

/// The lineage of `name` of `kind`, or the not-found error of the lookup:
/// upstream resolves keys before checking.
pub async fn lineage(state: &AppState, kind: SecurableType, name: &str) -> Result<Lineage> {
    state.metastore.permissions().lineage(kind, name).await
}

/// Check `rule` on the securable `name` of `kind`. Its lineage is resolved
/// first, so a missing securable is a 404, as upstream resolves keys before
/// checking. Nothing is looked up with authorization off.
pub async fn check<F>(
    state: &AppState,
    principal: &str,
    kind: SecurableType,
    name: &str,
    rule: F,
) -> Result<()>
where
    F: FnOnce(&Access, &Lineage) -> bool,
{
    if !state.authorization_enabled {
        return Ok(());
    }
    // The lineage error wins: a missing securable is a 404.
    let (lineage, access) =
        tokio::join!(lineage(state, kind, name), Access::load(state, principal));
    let lineage = lineage?;
    let access = access?;
    access.check(rule(&access, &lineage))
}

/// Check `rule` on creating a table or volume in the schema `schema`;
/// `external` is `Some(location)` for an EXTERNAL one, whose location's
/// owner the rule also gets. Nothing is looked up with authorization off.
pub async fn check_create(
    state: &AppState,
    principal: &str,
    schema: &str,
    external: Option<Option<&str>>,
    rule: fn(&Access, &Lineage, Option<&PathOwner>) -> bool,
) -> Result<()> {
    if !state.authorization_enabled {
        return Ok(());
    }
    let lineage = lineage(state, SecurableType::Schema, schema).await?;
    // With no location the create fails on its own.
    let location = external.flatten().filter(|l| !l.trim().is_empty());
    let owner = match location {
        Some(location) => Some(path_owner(state, location).await?),
        None => None,
    };
    let access = Access::load(state, principal).await?;
    match (rule(&access, &lineage, owner.as_ref()), location) {
        (true, _) => Ok(()),
        (false, Some(location)) if owner == Some(PathOwner::Unowned) => Err(uncovered(location)),
        (false, _) => Err(denied()),
    }
}

/// The error for a path that no external location covers.
pub fn uncovered(url: &str) -> Error {
    Error::new(
        ErrorCode::PermissionDenied,
        format!("No external location covers {url}."),
    )
}

/// Check `rule` on the table, or uncommitted staging table, with id
/// `table_id` (a UUID; upstream refuses others before checking).
pub async fn check_table_id<F>(
    state: &AppState,
    principal: &str,
    table_id: &str,
    rule: F,
) -> Result<()>
where
    F: FnOnce(&Access, &Lineage) -> bool,
{
    if !state.authorization_enabled {
        return Ok(());
    }
    let table_id = uc_delta::commits::parse_table_id(table_id)?;
    let permissions = state.metastore.permissions();
    let (lineage, access) = tokio::join!(
        permissions.table_lineage(&table_id),
        Access::load(state, principal)
    );
    let lineage = lineage?;
    let access = access?;
    access.check(rule(&access, &lineage))
}

/// Keep the schema children the caller may read: `rule` on each item's
/// lineage. `keys` gives an item's schema full name, id and owner. Schema
/// lineages are looked up once each.
pub async fn filter_children<T>(
    state: &AppState,
    access: &Access,
    kind: SecurableType,
    items: Vec<T>,
    keys: impl Fn(&T) -> (String, Option<String>, Option<String>),
    rule: fn(&Access, &Lineage) -> bool,
) -> Result<Vec<T>> {
    if !access.enabled() {
        return Ok(items);
    }
    let mut schemas: HashMap<String, Lineage> = HashMap::new();
    let mut kept = Vec::with_capacity(items.len());
    for item in items {
        let (schema, id, owner) = keys(&item);
        if !schemas.contains_key(&schema) {
            let lineage = lineage(state, SecurableType::Schema, &schema).await?;
            schemas.insert(schema.clone(), lineage);
        }
        let lineage = Lineage {
            object: node(kind, id.as_deref(), owner.as_deref()),
            ..schemas[&schema].clone()
        };
        if rule(access, &lineage) {
            kept.push(item);
        }
    }
    Ok(kept)
}

/// Upstream's storage-root tier of creating a catalog or schema: none, or
/// one with no data securable that the caller may put managed storage in.
pub async fn storage_root_allowed(
    state: &AppState,
    access: &Access,
    storage_root: Option<&str>,
) -> Result<Option<bool>> {
    let Some(root) = storage_root.filter(|_| access.enabled()) else {
        return Ok(None);
    };
    let owner = path_owner(state, root).await?;
    Ok(Some(rules::managed_storage(access, &owner)))
}

/// Upstream's `#external_location` and `#no_overlap_with_data_securable`
/// for a storage path: the data securable or external location that holds
/// it.
pub async fn path_owner(state: &AppState, url: &str) -> Result<PathOwner> {
    let url = uc_common::NormalizedUrl::parse(url)?;
    state.metastore.permissions().path_owner(&url).await
}

/// A node from an API object's id and owner.
pub fn node(kind: SecurableType, id: Option<&str>, owner: Option<&str>) -> Option<Node> {
    id.map(|id| Node {
        kind,
        id: id.to_string(),
        owner: owner.map(str::to_string),
    })
}

/// A schema of the catalog `catalog` as the lineage of its own checks.
pub fn schema_lineage(catalog: &Lineage, info: &uc_api::SchemaInfo) -> Lineage {
    let schema = node(
        SecurableType::Schema,
        info.schema_id.as_deref(),
        info.owner.as_deref(),
    );
    Lineage {
        catalog: catalog.catalog.clone(),
        schema: schema.clone(),
        object: schema,
    }
}

/// The lineage of a securable with no parents: an external location or a
/// credential.
pub fn object(kind: SecurableType, id: Option<&str>, owner: Option<&str>) -> Lineage {
    Lineage {
        object: node(kind, id, owner),
        ..Default::default()
    }
}

/// Metastore admin only (`#authorize(#principal, #metastore, OWNER)`):
/// SCIM and sharing endpoints.
pub async fn require_admin(state: &AppState, principal: &str) -> Result<()> {
    match is_admin(state, principal).await? {
        true => Ok(()),
        false => Err(denied()),
    }
}

/// Whether the checks let `principal` do what the metastore admin does:
/// the metastore owner and `auth.admin_principals`; with authorization
/// off, everyone.
pub async fn is_admin(state: &AppState, principal: &str) -> Result<bool> {
    if !state.authorization_enabled {
        return Ok(true);
    }
    let owner = state.metastore.permissions().metastore_owner().await?;
    Ok(is_metastore_admin(state, principal, owner.as_deref()))
}

/// Backups hold credentials, token hashes and users, and restores replace
/// everything: `/admin/*`, and revoking another user's token, take a
/// signed-in admin even with authorization off. The admin is then the
/// `admin` of `uc-server token`, the metastore owner or one of
/// `auth.admin_principals`.
pub async fn require_signed_in_admin(state: &AppState, principal: &Principal) -> Result<()> {
    let caller = principal.signed_in()?;
    let service = !state.authorization_enabled && caller == crate::authn::security::ADMIN;
    let owner = state.metastore.permissions().metastore_owner().await?;
    match service || is_metastore_admin(state, caller, owner.as_deref()) {
        true => Ok(()),
        false => Err(Error::permission_denied(format!(
            "principal '{caller}' is not an admin"
        ))),
    }
}

/// Upstream's expressions, one per kind of request. `l` is the lineage of
/// the object the request names; for creates, of its parent.
pub mod rules {
    use super::{Access, Level::*, Lineage, PathOwner};
    use uc_api::Privilege::*;
    use uc_common::SecurableType;

    /// `GET_CATALOG`: catalog reads and the Delta/Iceberg config.
    pub fn get_catalog(a: &Access, l: &Lineage) -> bool {
        a.admin() || a.owner_or_any(l, Catalog, &[UseCatalog])
    }

    /// `createCatalog`, given whether a storage root is set and, then,
    /// whether it is allowed (see [`managed_storage`]).
    pub fn create_catalog(a: &Access, storage_root_allowed: Option<bool>) -> bool {
        let empty = Lineage::default();
        (a.admin() || a.has(&empty, Metastore, CreateCatalog))
            && storage_root_allowed.unwrap_or(true)
    }

    /// `updateCatalog`.
    pub fn update_catalog(a: &Access, l: &Lineage) -> bool {
        a.owner(l, Catalog)
    }

    /// `deleteCatalog`.
    pub fn delete_catalog(a: &Access, l: &Lineage) -> bool {
        a.admin() || a.owner(l, Catalog)
    }

    /// A storage root under an external location that the caller owns or
    /// has `CREATE MANAGED STORAGE` on, with no data securable under it.
    pub fn managed_storage(a: &Access, owner: &PathOwner) -> bool {
        match owner {
            PathOwner::ExternalLocation(el) => a.owner_or_any(el, Object, &[CreateManagedStorage]),
            PathOwner::Data(..) | PathOwner::Unowned => false,
        }
    }

    /// `GET_SCHEMA`: schema reads and the schema/namespace list filter.
    pub fn get_schema(a: &Access, l: &Lineage) -> bool {
        a.admin()
            || a.owner(l, Catalog)
            || (a.has(l, Catalog, UseCatalog) && a.owner_or_any(l, Schema, &[UseSchema]))
    }

    /// `CREATE_SCHEMA` (catalog tier).
    pub fn create_schema(a: &Access, l: &Lineage) -> bool {
        a.owner(l, Catalog) || a.all(l, Catalog, &[UseCatalog, CreateSchema])
    }

    /// `UPDATE_SCHEMA` and `DELETE_SCHEMA`.
    pub fn update_schema(a: &Access, l: &Lineage) -> bool {
        a.owner(l, Catalog) || (a.has(l, Catalog, UseCatalog) && a.owner(l, Schema))
    }

    /// The catalog and schema tiers of creating a child: catalog `OWNER` or
    /// `USE_CATALOG`, and schema `OWNER` or `USE_SCHEMA` + one of `create`.
    pub fn create_in_schema(a: &Access, l: &Lineage, create: &[uc_api::Privilege]) -> bool {
        a.owner_or_any(l, Catalog, &[UseCatalog])
            && (a.owner(l, Schema) || create.iter().any(|c| a.all(l, Schema, &[UseSchema, *c])))
    }

    /// The external-location tier of an EXTERNAL table or volume: an
    /// external location covers the path, with no data securable at it, and
    /// the caller has `OWNER` or `create` on it. Unlike upstream, a path
    /// that no external location covers is refused, also for admins.
    pub fn external_path(a: &Access, owner: &PathOwner, create: uc_api::Privilege) -> bool {
        match owner {
            PathOwner::ExternalLocation(el) => a.owner_or_any(el, Object, &[create]),
            PathOwner::Data(..) | PathOwner::Unowned => false,
        }
    }

    /// Reads of a schema child: admin, catalog owner, schema owner with
    /// `USE_CATALOG`, or `USE_CATALOG` + `USE_SCHEMA` + one of `read` (or
    /// `OWNER`) on the child (`GET_TABLE` and the volume, function and model
    /// read rules).
    pub fn read_child(a: &Access, l: &Lineage, read: &[uc_api::Privilege]) -> bool {
        a.admin()
            || a.owner(l, Catalog)
            || (a.has(l, Catalog, UseCatalog) && a.owner(l, Schema))
            || (a.has(l, Catalog, UseCatalog)
                && a.has(l, Schema, UseSchema)
                && a.owner_or_any(l, Object, read))
    }

    /// Changes and deletes of a schema child (`DELETE_TABLE` and the volume,
    /// function and model rules): catalog owner, schema owner with
    /// `USE_CATALOG`, or `USE_CATALOG` + `USE_SCHEMA` + child owner.
    pub fn own_child(a: &Access, l: &Lineage) -> bool {
        a.owner(l, Catalog)
            || (a.has(l, Catalog, UseCatalog) && a.owner(l, Schema))
            || (a.has(l, Catalog, UseCatalog) && a.has(l, Schema, UseSchema) && a.owner(l, Object))
    }

    /// The catalog and schema tiers of using a child: `OWNER` or `USE_*` on
    /// each.
    pub fn use_parents(a: &Access, l: &Lineage) -> bool {
        a.owner_or_any(l, Catalog, &[UseCatalog]) && a.owner_or_any(l, Schema, &[UseSchema])
    }

    /// `GET_TABLE`.
    pub fn get_table(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[Select, Modify])
    }

    /// `CREATE_TABLE` on the schema `l`; for an EXTERNAL table, what holds
    /// its location.
    pub fn create_table(a: &Access, l: &Lineage, external: Option<&PathOwner>) -> bool {
        create_in_schema(a, l, &[CreateTable])
            && external.is_none_or(|owner| external_path(a, owner, CreateExternalTable))
    }

    /// `CREATE_STAGING_TABLE` on the schema `l`.
    pub fn create_staging_table(a: &Access, l: &Lineage) -> bool {
        create_in_schema(a, l, &[CreateTable])
    }

    /// Volume reads.
    pub fn get_volume(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[ReadVolume])
    }

    /// `createVolume` on the schema `l`; for an EXTERNAL volume, what holds
    /// its location.
    pub fn create_volume(a: &Access, l: &Lineage, external: Option<&PathOwner>) -> bool {
        create_in_schema(a, l, &[CreateVolume])
            && external.is_none_or(|owner| external_path(a, owner, CreateExternalVolume))
    }

    /// Function reads.
    pub fn get_function(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[Execute])
    }

    /// `createFunction` on the schema `l`.
    pub fn create_function(a: &Access, l: &Lineage) -> bool {
        create_in_schema(a, l, &[CreateFunction])
    }

    /// Registered model and model version reads.
    pub fn get_model(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[Execute])
    }

    /// `createRegisteredModel` on the schema `l`: `CREATE MODEL` or
    /// `CREATE FUNCTION`.
    pub fn create_model(a: &Access, l: &Lineage) -> bool {
        create_in_schema(a, l, &[CreateModel, CreateFunction])
    }

    /// `UPDATE_TABLE`: commits and Delta updates.
    pub fn update_table(a: &Access, l: &Lineage) -> bool {
        use_parents(a, l) && (a.owner(l, Object) || a.all(l, Object, &[Select, Modify]))
    }

    /// `RENAME_TABLE`.
    pub fn rename_table(a: &Access, l: &Lineage) -> bool {
        own_child(a, l) && create_in_schema(a, l, &[CreateTable])
    }

    /// `VEND_TABLE_CREDENTIAL`: `write` for `READ_WRITE`.
    pub fn vend_table(a: &Access, l: &Lineage, write: bool) -> bool {
        use_parents(a, l)
            && match write {
                false => a.owner_or_any(l, Object, &[Select]),
                true => a.owner(l, Object) || a.all(l, Object, &[Select, Modify]),
            }
    }

    /// `getCommits`.
    pub fn get_commits(a: &Access, l: &Lineage) -> bool {
        use_parents(a, l) && a.owner_or_any(l, Object, &[Select])
    }

    /// Volume credentials: `READ_VOLUME`, else owner.
    pub fn vend_volume(a: &Access, l: &Lineage, write: bool) -> bool {
        use_parents(a, l)
            && match write {
                false => a.owner_or_any(l, Object, &[ReadVolume]),
                true => a.owner(l, Object),
            }
    }

    /// Model version credentials: `EXECUTE`, else owner.
    pub fn vend_model(a: &Access, l: &Lineage, write: bool) -> bool {
        use_parents(a, l)
            && match write {
                false => a.owner_or_any(l, Object, &[Execute]),
                true => a.owner(l, Object),
            }
    }

    /// Credential reads.
    pub fn get_credential(a: &Access, l: &Lineage) -> bool {
        a.admin() || a.owner_or_any(l, Object, &[CreateExternalLocation])
    }

    /// Credential changes and deletes; external location deletes.
    pub fn admin_or_owner(a: &Access, l: &Lineage) -> bool {
        a.admin() || a.owner(l, Object)
    }

    /// `createCredential`.
    pub fn create_credential(a: &Access) -> bool {
        a.admin() || a.has(&Lineage::default(), Metastore, CreateStorageCredential)
    }

    /// External location reads.
    pub fn get_external_location(a: &Access, l: &Lineage) -> bool {
        a.admin()
            || a.owner_or_any(
                l,
                Object,
                &[
                    ReadFiles,
                    WriteFiles,
                    CreateExternalTable,
                    CreateExternalVolume,
                    CreateManagedStorage,
                ],
            )
    }

    /// `createExternalLocation`, `credential` being the credential's lineage.
    pub fn create_external_location(a: &Access, credential: &Lineage) -> bool {
        a.admin()
            || (a.has(&Lineage::default(), Metastore, CreateExternalLocation)
                && a.owner_or_any(credential, Object, &[CreateExternalLocation]))
    }

    /// `updateExternalLocation`, `credential` being a new credential's
    /// lineage, if any.
    pub fn update_external_location(a: &Access, l: &Lineage, credential: Option<&Lineage>) -> bool {
        a.admin()
            || (a.owner(l, Object)
                && credential.is_none_or(|c| a.owner_or_any(c, Object, &[CreateExternalLocation])))
    }

    /// Who sees every grant on a securable, after upstream
    /// `getAuthorization`: the admin, or the owner of it, of its parent or
    /// of its grandparent. Others see their own grants only.
    pub fn see_permissions(a: &Access, l: &Lineage) -> bool {
        a.admin() || a.owner(l, Object) || a.owner(l, Schema) || a.owner(l, Catalog)
    }

    /// Reads of a catalog, schema, table, volume or registered model, such
    /// as of its protection.
    pub fn read_securable(a: &Access, kind: SecurableType, l: &Lineage) -> bool {
        match kind {
            SecurableType::Catalog => get_catalog(a, l),
            SecurableType::Schema => get_schema(a, l),
            SecurableType::Table => get_table(a, l),
            SecurableType::Volume => get_volume(a, l),
            SecurableType::RegisteredModel => get_model(a, l),
            _ => false,
        }
    }

    /// Setting and removing the protection of a securable: whoever can
    /// grant on it, which is whoever can delete it.
    pub fn protect(a: &Access, kind: SecurableType, l: &Lineage) -> bool {
        update_permissions(a, kind, l)
    }

    /// `update<Securable>Authorization`.
    pub fn update_permissions(a: &Access, kind: SecurableType, l: &Lineage) -> bool {
        a.admin()
            || match kind {
                SecurableType::Metastore => false,
                SecurableType::Catalog => a.owner(l, Catalog),
                SecurableType::Schema => update_schema(a, l),
                SecurableType::ExternalLocation | SecurableType::Credential => a.owner(l, Object),
                SecurableType::Table
                | SecurableType::Volume
                | SecurableType::Function
                | SecurableType::RegisteredModel => own_child(a, l),
            }
    }
}
