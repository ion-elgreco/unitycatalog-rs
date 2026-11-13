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
//! - An [`Authorizer`], such as Cedar's ([`cedar`]), can decide the
//!   privileges instead of the grants alone.
//!
//! With authorization off every check passes, as upstream's
//! `AllowingAuthorizer`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};

use uc_api::Privilege;
use uc_auth::authz::{AuthorizeRequest, Authorizer, EntityRef};
use uc_common::{Error, ErrorCode, Result, SecurableType};

pub use uc_metastore::{Lineage, Node, PathOwner};

use crate::{authn::principal::Principal, server::state::AppState};

pub mod cedar;

/// The entity type of principals.
pub const USER: &str = "User";
/// The entity type of actions: one per privilege, `Action::"SELECT"`.
pub const ACTION: &str = "Action";

/// The entity type of a securable: `Catalog`, `Schema`, `Table` and so on.
pub fn entity_type(kind: SecurableType) -> &'static str {
    match kind {
        SecurableType::Metastore => "Metastore",
        SecurableType::Catalog => "Catalog",
        SecurableType::Schema => "Schema",
        SecurableType::Table => "Table",
        SecurableType::Volume => "Volume",
        SecurableType::Function => "Function",
        SecurableType::RegisteredModel => "RegisteredModel",
        SecurableType::ExternalLocation => "ExternalLocation",
        SecurableType::Credential => "Credential",
    }
}

/// The action of `privilege`, by its wire name (`Action::"USE CATALOG"`).
pub fn action(privilege: Privilege) -> EntityRef {
    let name = serde_json::to_value(privilege)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{privilege:?}"));
    EntityRef::new(ACTION, name)
}

/// The entity of a securable.
fn entity(node: &Node) -> EntityRef {
    EntityRef::new(entity_type(node.kind), node.id.clone())
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

/// One caller's authorization for a request: who the caller is, its grants,
/// read once per request, and the authorizer that decides each privilege.
pub struct Access {
    enabled: bool,
    principal: String,
    admin: bool,
    metastore: Node,
    /// What is granted to the caller and its groups, by securable id.
    grants: HashMap<String, HashSet<Privilege>>,
    /// The authorizer; with none, the grants decide.
    authorizer: Option<Arc<dyn Authorizer>>,
    /// The authorizer's answers for this request, by the ids of the
    /// securable and its parents, and the privilege: the checks of a list
    /// ask for the same parents each time.
    decided: Mutex<HashMap<(Vec<String>, Privilege), bool>>,
    /// The context of the request ([`Principal::context`]).
    context: Map<String, Value>,
}

impl Access {
    /// The authorization of `principal`: its grants, and those of its
    /// groups, the server's and its token's.
    pub async fn load(state: &AppState, principal: &Principal) -> Result<Self> {
        let name = principal.username.as_str();
        if !state.authorization_enabled {
            return Ok(Self {
                enabled: false,
                principal: name.to_string(),
                admin: true,
                metastore: Node {
                    kind: SecurableType::Metastore,
                    id: String::new(),
                    owner: None,
                },
                grants: HashMap::new(),
                authorizer: None,
                decided: Mutex::default(),
                context: Map::new(),
            });
        }
        let permissions = state.metastore.permissions();
        let (metastore_id, owner, grants) = tokio::try_join!(
            permissions.metastore_id(),
            permissions.metastore_owner(),
            permissions.grants_of(name, &principal.groups),
        )?;
        Ok(Self {
            enabled: true,
            principal: name.to_string(),
            admin: is_metastore_admin(state, name, owner.as_deref()),
            metastore: Node {
                kind: SecurableType::Metastore,
                id: metastore_id,
                owner,
            },
            grants: (grants.into_iter())
                .map(|(id, privileges)| (id, privileges.into_iter().collect()))
                .collect(),
            authorizer: state.authorizer.clone(),
            decided: Mutex::default(),
            context: principal.context.as_map().clone(),
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

    /// What the caller reads of the object of `lineage`: all of it by
    /// `rule`, else, with `include_browse`, its discovery fields by
    /// `browse`; else "Access denied.".
    pub async fn read(
        &self,
        lineage: &Lineage,
        include_browse: bool,
        rule: impl AsyncFnOnce(&Access, &Lineage) -> bool,
        browse: impl AsyncFnOnce(&Access, &Lineage) -> bool,
    ) -> Result<Read> {
        if !self.enabled || rule(self, lineage).await {
            return Ok(Read::Full);
        }
        match include_browse && browse(self, lineage).await {
            true => Ok(Read::BrowseOnly),
            false => Err(denied()),
        }
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
    pub async fn owner(&self, lineage: &Lineage, level: Level) -> bool {
        if !self.enabled {
            return true;
        }
        if level == Level::Metastore {
            return self.admin;
        }
        self.node(lineage, level)
            .is_some_and(|n| n.owner.as_deref() == Some(self.principal.as_str()))
    }

    /// `#authorize(#principal, #<level>, privilege)`: what the authorizer
    /// says, else whether it is granted there or on a parent. An answer
    /// with errors is a denial.
    pub async fn has(&self, lineage: &Lineage, level: Level, privilege: Privilege) -> bool {
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
        let Some(authorizer) = &self.authorizer else {
            return (chain.iter())
                .any(|n| (self.grants.get(&n.id)).is_some_and(|held| held.contains(&privilege)));
        };
        let key = (chain.iter().map(|n| n.id.clone()).collect(), privilege);
        if let Some(allowed) = self.decided().get(&key) {
            return *allowed;
        }
        let answer = authorizer.authorize(&self.request(&chain, privilege)).await;
        if !answer.errors.is_empty() {
            tracing::warn!(errors = ?answer.errors, "authorization errors; denying");
        }
        let allowed = answer.is_allowed();
        self.decided().insert(key, allowed);
        allowed
    }

    fn decided(&self) -> std::sync::MutexGuard<'_, HashMap<(Vec<String>, Privilege), bool>> {
        self.decided.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The request for `privilege` on `chain[0]`, whose parents are the rest
    /// of `chain`. The resource entity has every parent as a parent, its
    /// owner, and in `granted_privileges` what is granted to the caller on
    /// it or a parent; the context is the request's.
    pub fn request(&self, chain: &[&Node], privilege: Privilege) -> AuthorizeRequest {
        let resource = entity(chain[0]);
        let mut granted: Vec<Value> = Vec::new();
        for node in chain {
            for held in self.grants.get(&node.id).into_iter().flatten() {
                let held = json!({ "__entity": action(*held) });
                if !granted.contains(&held) {
                    granted.push(held);
                }
            }
        }
        let mut attrs = Map::new();
        attrs.insert("granted_privileges".into(), Value::Array(granted));
        if let Some(owner) = &chain[0].owner {
            attrs.insert("owner".into(), Value::from(owner.clone()));
        }
        let parents: Vec<EntityRef> = chain[1..].iter().map(|node| entity(node)).collect();
        let resource_entity = json!({ "uid": resource, "attrs": attrs, "parents": parents });
        AuthorizeRequest::new(
            EntityRef::new(USER, self.principal.clone()),
            action(privilege),
            resource,
        )
        .with_context(self.context.clone())
        .with_resource_entity(resource_entity)
    }

    /// `#authorizeAny(#principal, #<level>, OWNER, privileges…)`.
    pub async fn owner_or_any(
        &self,
        lineage: &Lineage,
        level: Level,
        privileges: &[Privilege],
    ) -> bool {
        self.owner(lineage, level).await || self.any(lineage, level, privileges).await
    }

    /// `#authorizeAny(#principal, #<level>, privileges…)`, without `OWNER`.
    pub async fn any(&self, lineage: &Lineage, level: Level, privileges: &[Privilege]) -> bool {
        for privilege in privileges {
            if self.has(lineage, level, *privilege).await {
                return true;
            }
        }
        false
    }

    /// `#authorizeAll(#principal, #<level>, privileges…)`.
    pub async fn all(&self, lineage: &Lineage, level: Level, privileges: &[Privilege]) -> bool {
        for privilege in privileges {
            if !self.has(lineage, level, *privilege).await {
                return false;
            }
        }
        true
    }
}

/// The metastore admin: one of `auth.admin_principals`, or the metastore
/// owner `owner`.
fn is_metastore_admin(state: &AppState, principal: &str, owner: Option<&str>) -> bool {
    state.names_admin(principal) || owner == Some(principal)
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
    principal: &Principal,
    kind: SecurableType,
    name: &str,
    rule: F,
) -> Result<()>
where
    F: AsyncFnOnce(&Access, &Lineage) -> bool,
{
    if !state.authorization_enabled {
        return Ok(());
    }
    // The lineage error wins: a missing securable is a 404.
    let (lineage, access) =
        tokio::join!(lineage(state, kind, name), Access::load(state, principal));
    let lineage = lineage?;
    let access = access?;
    access.check(rule(&access, &lineage).await)
}

/// What a caller reads of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// All of it.
    Full,
    /// The discovery fields that `BROWSE` allows.
    BrowseOnly,
}

/// Check a read of the securable `name` of `kind`: `rule` for all of it,
/// else, when the caller asks with `include_browse`, `browse` for its
/// discovery fields. A missing securable is a 404.
pub async fn check_read(
    state: &AppState,
    principal: &Principal,
    kind: SecurableType,
    name: &str,
    include_browse: bool,
    rule: impl AsyncFnOnce(&Access, &Lineage) -> bool,
    browse: impl AsyncFnOnce(&Access, &Lineage) -> bool,
) -> Result<Read> {
    if !state.authorization_enabled {
        return Ok(Read::Full);
    }
    let (lineage, access) =
        tokio::join!(lineage(state, kind, name), Access::load(state, principal));
    let lineage = lineage?;
    let access = access?;
    access.read(&lineage, include_browse, rule, browse).await
}

/// Check `rule` on creating a table or volume in the schema `schema`;
/// `external` is `Some(location)` for an EXTERNAL one, whose location's
/// owner the rule also gets. Nothing is looked up with authorization off.
pub async fn check_create(
    state: &AppState,
    principal: &Principal,
    schema: &str,
    external: Option<Option<&str>>,
    rule: impl AsyncFnOnce(&Access, &Lineage, Option<&PathOwner>) -> bool,
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
    match (rule(&access, &lineage, owner.as_ref()).await, location) {
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
    principal: &Principal,
    table_id: &str,
    rule: F,
) -> Result<()>
where
    F: AsyncFnOnce(&Access, &Lineage) -> bool,
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
    access.check(rule(&access, &lineage).await)
}

/// Keep the schema children the caller may read: `rule` on each item's
/// lineage. `keys` gives an item's schema full name, id and owner. Schema
/// lineages are looked up once each. With `browse_only`, the request has
/// `include_browse`: an item of a catalog that the caller may browse
/// stays as what `browse_only` makes of it.
pub async fn filter_children<T>(
    state: &AppState,
    access: &Access,
    kind: SecurableType,
    items: Vec<T>,
    keys: impl Fn(&T) -> (String, Option<String>, Option<String>),
    rule: impl AsyncFn(&Access, &Lineage) -> bool,
    browse_only: Option<fn(T) -> T>,
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
        if rule(access, &lineage).await {
            kept.push(item);
        } else if let Some(browse_only) = browse_only
            && rules::browse_catalog(access, &lineage).await
        {
            kept.push(browse_only(item));
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
    Ok(Some(rules::managed_storage(access, &owner).await))
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
///
/// `MANAGE` stands in for `OWNER` in reads, changes, deletes and grant
/// changes, but not in data access. `READ METADATA` reads metadata and
/// grants, and no data. `BROWSE` shows the discovery fields of an object
/// to a caller who asks with `include_browse`.
pub mod rules {
    use super::{Access, Level::*, Lineage, PathOwner};
    use uc_api::Privilege::{self, *};
    use uc_common::SecurableType;

    /// The metastore tier of the read rules: the admin, or `READ METADATA`
    /// on the metastore.
    async fn reads_metastore(a: &Access) -> bool {
        a.admin() || a.has(&Lineage::default(), Metastore, ReadMetadata).await
    }

    /// `GET_CATALOG`: catalog reads and the Delta/Iceberg config.
    pub async fn get_catalog(a: &Access, l: &Lineage) -> bool {
        reads_metastore(a).await
            || a.owner_or_any(l, Catalog, &[Manage, UseCatalog, ReadMetadata])
                .await
    }

    /// `BROWSE_CATALOG`: the discovery fields of a catalog and of what it
    /// holds.
    pub async fn browse_catalog(a: &Access, l: &Lineage) -> bool {
        a.has(l, Catalog, Browse).await
    }

    /// `createCatalog`, given whether a storage root is set and, then,
    /// whether it is allowed (see [`managed_storage`]).
    pub async fn create_catalog(a: &Access, storage_root_allowed: Option<bool>) -> bool {
        let empty = Lineage::default();
        (a.admin() || a.has(&empty, Metastore, CreateCatalog).await)
            && storage_root_allowed.unwrap_or(true)
    }

    /// `updateCatalog`: a rename through `MANAGE` also takes what creating
    /// a catalog takes.
    pub async fn update_catalog(a: &Access, l: &Lineage, rename: bool) -> bool {
        a.owner_or_any(l, Catalog, &[Manage]).await
            && (!rename
                || a.owner(l, Catalog).await
                || a.admin()
                || a.has(&Lineage::default(), Metastore, CreateCatalog).await)
    }

    /// `deleteCatalog`.
    pub async fn delete_catalog(a: &Access, l: &Lineage) -> bool {
        a.admin() || a.owner_or_any(l, Catalog, &[Manage]).await
    }

    /// A storage root under an external location that the caller owns or
    /// has `CREATE MANAGED STORAGE` on, with no data securable under it.
    pub async fn managed_storage(a: &Access, owner: &PathOwner) -> bool {
        match owner {
            PathOwner::ExternalLocation(el) => {
                a.owner_or_any(el, Object, &[CreateManagedStorage]).await
            }
            PathOwner::Data(..) | PathOwner::Unowned => false,
        }
    }

    /// `GET_SCHEMA`: schema reads and the schema/namespace list filter.
    pub async fn get_schema(a: &Access, l: &Lineage) -> bool {
        reads_metastore(a).await
            || a.owner_or_any(l, Catalog, &[Manage, ReadMetadata]).await
            || (a.has(l, Catalog, UseCatalog).await
                && a.owner_or_any(l, Schema, &[Manage, UseSchema, ReadMetadata])
                    .await)
    }

    /// `CREATE_SCHEMA` (catalog tier).
    pub async fn create_schema(a: &Access, l: &Lineage) -> bool {
        a.owner(l, Catalog).await || a.all(l, Catalog, &[UseCatalog, CreateSchema]).await
    }

    /// `UPDATE_SCHEMA` and `DELETE_SCHEMA`; schema grant changes too.
    pub async fn update_schema(a: &Access, l: &Lineage) -> bool {
        a.owner_or_any(l, Catalog, &[Manage]).await
            || (a.has(l, Catalog, UseCatalog).await && a.owner_or_any(l, Schema, &[Manage]).await)
    }

    /// `updateSchema` with a new name: its owner with `USE CATALOG`, else
    /// what creating a schema takes.
    pub async fn rename_schema(a: &Access, l: &Lineage) -> bool {
        update_schema(a, l).await
            && ((a.has(l, Catalog, UseCatalog).await && a.owner(l, Schema).await)
                || create_schema(a, l).await)
    }

    /// The catalog and schema tiers of creating a child: catalog `OWNER` or
    /// `USE_CATALOG`, and schema `OWNER` or `USE_SCHEMA` + one of `create`.
    pub async fn create_in_schema(a: &Access, l: &Lineage, create: &[Privilege]) -> bool {
        a.owner_or_any(l, Catalog, &[UseCatalog]).await
            && (a.owner(l, Schema).await || any_create(a, l, create).await)
    }

    /// `USE_SCHEMA` and one of `create` on the schema.
    async fn any_create(a: &Access, l: &Lineage, create: &[Privilege]) -> bool {
        for c in create {
            if a.all(l, Schema, &[UseSchema, *c]).await {
                return true;
            }
        }
        false
    }

    /// The external-location tier of an EXTERNAL table or volume: an
    /// external location covers the path, with no data securable at it, and
    /// the caller has `OWNER` or `create` on it. Unlike upstream, a path
    /// that no external location covers is refused, also for admins.
    pub async fn external_path(a: &Access, owner: &PathOwner, create: Privilege) -> bool {
        match owner {
            PathOwner::ExternalLocation(el) => a.owner_or_any(el, Object, &[create]).await,
            PathOwner::Data(..) | PathOwner::Unowned => false,
        }
    }

    /// Reads of a schema child (`GET_TABLE`, `GET_FUNCTION` and the volume
    /// and model read rules). `READ METADATA` on the metastore or catalog
    /// needs no usage grants; on the schema, `USE CATALOG`; on the child,
    /// both, as do `OWNER`, `MANAGE` and `read` on the child.
    pub async fn read_child(a: &Access, l: &Lineage, read: &[Privilege]) -> bool {
        if reads_metastore(a).await || a.owner_or_any(l, Catalog, &[Manage, ReadMetadata]).await {
            return true;
        }
        if !a.has(l, Catalog, UseCatalog).await {
            return false;
        }
        if a.owner_or_any(l, Schema, &[Manage, ReadMetadata]).await {
            return true;
        }
        let mut on_child = vec![Manage, ReadMetadata];
        on_child.extend_from_slice(read);
        a.has(l, Schema, UseSchema).await && a.owner_or_any(l, Object, &on_child).await
    }

    /// Changes, deletes and grant changes of a schema child
    /// (`DELETE_TABLE`, `UPDATE_FUNCTION`, `UPDATE_VOLUME` and
    /// `UPDATE_REGISTERED_MODEL`): `OWNER` or `MANAGE` on the catalog, the
    /// schema or the child, with the usage grants above it. The admin
    /// alone is not enough.
    pub async fn manage_child(a: &Access, l: &Lineage) -> bool {
        if a.owner_or_any(l, Catalog, &[Manage]).await {
            return true;
        }
        a.has(l, Catalog, UseCatalog).await
            && (a.owner_or_any(l, Schema, &[Manage]).await
                || (a.has(l, Schema, UseSchema).await
                    && a.owner_or_any(l, Object, &[Manage]).await))
    }

    /// A new name for a schema child (`RENAME_TABLE` and the volume and
    /// model renames): [`manage_child`], and what creating one in its
    /// schema takes.
    pub async fn rename_child(a: &Access, l: &Lineage, create: &[Privilege]) -> bool {
        manage_child(a, l).await && create_in_schema(a, l, create).await
    }

    /// The catalog and schema tiers of using a child: `OWNER` or `USE_*` on
    /// each.
    pub async fn use_parents(a: &Access, l: &Lineage) -> bool {
        a.owner_or_any(l, Catalog, &[UseCatalog]).await
            && a.owner_or_any(l, Schema, &[UseSchema]).await
    }

    /// `GET_TABLE`. `MODIFY` alone does not read a table's metadata.
    pub async fn get_table(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[Select]).await
    }

    /// `CREATE_TABLE` on the schema `l`; for an EXTERNAL table, what holds
    /// its location.
    pub async fn create_table(a: &Access, l: &Lineage, external: Option<&PathOwner>) -> bool {
        create_in_schema(a, l, &[CreateTable]).await
            && match external {
                Some(owner) => external_path(a, owner, CreateExternalTable).await,
                None => true,
            }
    }

    /// `CREATE_STAGING_TABLE` on the schema `l`.
    pub async fn create_staging_table(a: &Access, l: &Lineage) -> bool {
        create_in_schema(a, l, &[CreateTable]).await
    }

    /// Volume reads.
    pub async fn get_volume(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[ReadVolume]).await
    }

    /// `createVolume` on the schema `l`; for an EXTERNAL volume, what holds
    /// its location.
    pub async fn create_volume(a: &Access, l: &Lineage, external: Option<&PathOwner>) -> bool {
        create_in_schema(a, l, &[CreateVolume]).await
            && match external {
                Some(owner) => external_path(a, owner, CreateExternalVolume).await,
                None => true,
            }
    }

    /// `updateVolume` with a new name.
    pub async fn rename_volume(a: &Access, l: &Lineage) -> bool {
        rename_child(a, l, &[CreateVolume]).await
    }

    /// `GET_FUNCTION`.
    pub async fn get_function(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[Execute]).await
    }

    /// `createFunction` on the schema `l`.
    pub async fn create_function(a: &Access, l: &Lineage) -> bool {
        create_in_schema(a, l, &[CreateFunction]).await
    }

    /// Registered model and model version reads.
    pub async fn get_model(a: &Access, l: &Lineage) -> bool {
        read_child(a, l, &[Execute]).await
    }

    /// `createRegisteredModel` on the schema `l`: `CREATE MODEL` or
    /// `CREATE FUNCTION`.
    pub async fn create_model(a: &Access, l: &Lineage) -> bool {
        create_in_schema(a, l, &[CreateModel, CreateFunction]).await
    }

    /// `updateRegisteredModel` with a new name.
    pub async fn rename_model(a: &Access, l: &Lineage) -> bool {
        rename_child(a, l, &[CreateModel, CreateFunction]).await
    }

    /// `CREATE_MODEL_VERSION`: creating and finalizing a version of the
    /// model `l`. Its owner, or `CREATE MODEL VERSION`.
    pub async fn create_model_version(a: &Access, l: &Lineage) -> bool {
        use_parents(a, l).await && a.owner_or_any(l, Object, &[CreateModelVersion]).await
    }

    /// `UPDATE_TABLE`: commits and Delta updates.
    pub async fn update_table(a: &Access, l: &Lineage) -> bool {
        use_parents(a, l).await
            && (a.owner(l, Object).await || a.all(l, Object, &[Select, Modify]).await)
    }

    /// `RENAME_TABLE`.
    pub async fn rename_table(a: &Access, l: &Lineage) -> bool {
        rename_child(a, l, &[CreateTable]).await
    }

    /// `VEND_TABLE_CREDENTIAL`: `write` for `READ_WRITE`. Iceberg
    /// credentials take the read rule. `MANAGE`, `READ METADATA` and the
    /// ownership of a parent give no credentials.
    pub async fn vend_table(a: &Access, l: &Lineage, write: bool) -> bool {
        use_parents(a, l).await
            && match write {
                false => a.owner_or_any(l, Object, &[Select]).await,
                true => a.owner(l, Object).await || a.all(l, Object, &[Select, Modify]).await,
            }
    }

    /// `getCommits`.
    pub async fn get_commits(a: &Access, l: &Lineage) -> bool {
        use_parents(a, l).await && a.owner_or_any(l, Object, &[Select]).await
    }

    /// `VEND_VOLUME_CREDENTIAL`: `READ VOLUME` to read, and `WRITE VOLUME`
    /// with it to write; else the owner.
    pub async fn vend_volume(a: &Access, l: &Lineage, write: bool) -> bool {
        use_parents(a, l).await
            && match write {
                false => a.owner_or_any(l, Object, &[ReadVolume]).await,
                true => {
                    a.owner(l, Object).await || a.all(l, Object, &[ReadVolume, WriteVolume]).await
                }
            }
    }

    /// Model version credentials: `EXECUTE` to read, `CREATE MODEL
    /// VERSION` to write; else the owner.
    pub async fn vend_model(a: &Access, l: &Lineage, write: bool) -> bool {
        use_parents(a, l).await
            && match write {
                false => a.owner_or_any(l, Object, &[Execute]).await,
                true => a.owner_or_any(l, Object, &[CreateModelVersion]).await,
            }
    }

    /// Credential reads.
    pub async fn get_credential(a: &Access, l: &Lineage) -> bool {
        reads_metastore(a).await
            || a.owner_or_any(l, Object, &[Manage, ReadMetadata, CreateExternalLocation])
                .await
    }

    /// Changes and deletes of a credential or an external location: the
    /// admin, its owner or `MANAGE`.
    pub async fn admin_or_manage(a: &Access, l: &Lineage) -> bool {
        a.admin() || a.owner_or_any(l, Object, &[Manage]).await
    }

    /// `createCredential`.
    pub async fn create_credential(a: &Access) -> bool {
        a.admin()
            || a.has(&Lineage::default(), Metastore, CreateStorageCredential)
                .await
    }

    /// External location reads.
    pub async fn get_external_location(a: &Access, l: &Lineage) -> bool {
        reads_metastore(a).await
            || a.owner_or_any(
                l,
                Object,
                &[
                    Manage,
                    ReadMetadata,
                    ReadFiles,
                    WriteFiles,
                    CreateExternalTable,
                    CreateExternalVolume,
                    CreateManagedStorage,
                ],
            )
            .await
    }

    /// The discovery fields of an external location.
    pub async fn browse_external_location(a: &Access, l: &Lineage) -> bool {
        a.has(l, Object, Browse).await
    }

    /// `createExternalLocation`, `credential` being the credential's lineage.
    pub async fn create_external_location(a: &Access, credential: &Lineage) -> bool {
        a.admin()
            || (a
                .has(&Lineage::default(), Metastore, CreateExternalLocation)
                .await
                && a.owner_or_any(credential, Object, &[CreateExternalLocation])
                    .await)
    }

    /// `updateExternalLocation`, `credential` being a new credential's
    /// lineage, if any.
    pub async fn update_external_location(
        a: &Access,
        l: &Lineage,
        credential: Option<&Lineage>,
    ) -> bool {
        a.admin()
            || (a.owner_or_any(l, Object, &[Manage]).await
                && match credential {
                    Some(c) => a.owner_or_any(c, Object, &[CreateExternalLocation]).await,
                    None => true,
                })
    }

    /// Who sees every grant on a securable of `kind`, after upstream
    /// `getAuthorization`: the admin; the owner of it, of its parent or of
    /// its grandparent; and who has `MANAGE` or `READ METADATA` on it or
    /// above it, with the usage grants above that. Others see their own
    /// grants only.
    pub async fn see_permissions(a: &Access, kind: SecurableType, l: &Lineage) -> bool {
        if a.admin()
            || a.owner(l, Object).await
            || a.owner(l, Schema).await
            || a.owner(l, Catalog).await
            || a.has(&Lineage::default(), Metastore, ReadMetadata).await
        {
            return true;
        }
        let reads = [Manage, ReadMetadata];
        match kind {
            SecurableType::Metastore => false,
            SecurableType::Catalog => a.any(l, Catalog, &reads).await,
            SecurableType::ExternalLocation | SecurableType::Credential => {
                a.any(l, Object, &reads).await
            }
            SecurableType::Schema => {
                a.any(l, Catalog, &reads).await
                    || (a.has(l, Catalog, UseCatalog).await && a.any(l, Schema, &reads).await)
            }
            SecurableType::Table
            | SecurableType::Volume
            | SecurableType::Function
            | SecurableType::RegisteredModel => {
                a.any(l, Catalog, &reads).await
                    || (a.has(l, Catalog, UseCatalog).await
                        && (a.any(l, Schema, &reads).await
                            || (a.has(l, Schema, UseSchema).await
                                && a.any(l, Object, &reads).await)))
            }
        }
    }

    /// `update<Securable>Authorization`.
    pub async fn update_permissions(a: &Access, kind: SecurableType, l: &Lineage) -> bool {
        a.admin()
            || match kind {
                SecurableType::Metastore => false,
                SecurableType::Catalog => a.owner_or_any(l, Catalog, &[Manage]).await,
                SecurableType::Schema => update_schema(a, l).await,
                SecurableType::ExternalLocation | SecurableType::Credential => {
                    a.owner_or_any(l, Object, &[Manage]).await
                }
                SecurableType::Table
                | SecurableType::Volume
                | SecurableType::Function
                | SecurableType::RegisteredModel => manage_child(a, l).await,
            }
    }

    /// The securable types that take a grant of `privilege`, after upstream
    /// `validateAddedPrivileges`.
    pub fn grant_targets(privilege: Privilege) -> &'static [SecurableType] {
        use SecurableType as T;
        match privilege {
            Browse => &[T::Catalog, T::ExternalLocation],
            Manage => &[
                T::Catalog,
                T::Schema,
                T::Table,
                T::Volume,
                T::Function,
                T::RegisteredModel,
                T::ExternalLocation,
                T::Credential,
            ],
            CreateCatalog | CreateStorageCredential => &[T::Metastore],
            UseCatalog | CreateSchema => &[T::Catalog],
            UseSchema | CreateTable | CreateFunction | CreateVolume | CreateModel => {
                &[T::Catalog, T::Schema]
            }
            Select | Modify => &[T::Catalog, T::Schema, T::Table],
            Execute => &[T::Catalog, T::Schema, T::Function, T::RegisteredModel],
            CreateModelVersion => &[T::Catalog, T::Schema, T::RegisteredModel],
            ReadVolume | WriteVolume => &[T::Catalog, T::Schema, T::Volume],
            CreateExternalLocation => &[T::Metastore, T::Credential],
            ReadFiles | WriteFiles | CreateExternalTable => &[T::ExternalLocation, T::Credential],
            CreateExternalVolume | CreateManagedStorage => &[T::ExternalLocation],
            ReadMetadata => &[
                T::Metastore,
                T::Catalog,
                T::Schema,
                T::Table,
                T::Function,
                T::Volume,
                T::RegisteredModel,
                T::ExternalLocation,
                T::Credential,
            ],
        }
    }
}
