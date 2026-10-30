//! Cedar-based [`PolicyEngine`].
//!
//! Plugs in instead of [`crate::authz::PermissionEnforcer`] with
//! `auth.cedar` ([`AppState::with_cedar`]): Cedar policies then decide what
//! a caller may do on a resource.
//!
//! ## Default policy
//!
//! `default_policies()` ships a single permit policy: anyone who has been
//! granted a matching privilege in the `permission` table is allowed, as
//! with `PermissionEnforcer`. Operators add their own `.cedar` files with
//! `auth.cedar_policies`.
//!
//! [`AppState::with_cedar`]: crate::server::state::AppState::with_cedar
//!
//! ## Wire format
//!
//! Every UC privilege maps to a Cedar `Action::"<priv>"` (e.g. `Action::"MODIFY"`).
//! Principals are `User::"<username>"`. Resources are `Catalog::"<id>"`,
//! `Schema::"<id>"` and so on: one entity type per [`SecurableType`], named
//! by the securable's id. A resource's parents (schema, catalog, metastore)
//! are its Cedar parents, so policies can say `resource in Catalog::"<id>"`;
//! `granted_privileges` holds what is granted on it or a parent, and
//! `owner` its owner when it has one.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use cedar_policy::{
    Authorizer, Context, Decision, Entities, EntityId, EntityTypeName, EntityUid, PolicySet,
    Request, RestrictedExpression,
};

use uc_api::Privilege;
use uc_common::{Error, Result, SecurableType};
use uc_metastore::DynMetastore;

use crate::authz::{Grants, Node, PolicyEngine};

pub struct CedarEngine {
    authorizer: Arc<Authorizer>,
    policies: Arc<PolicySet>,
    /// Used to read the grants of a principal, which become attributes of
    /// the Cedar entities of a request.
    metastore: DynMetastore,
    /// The `Action::"<PRIV>"` UID of each [`Privilege`].
    action_uids: Arc<HashMap<Privilege, EntityUid>>,
}

impl CedarEngine {
    pub fn new(metastore: DynMetastore) -> Result<Self> {
        let policies = default_policies()?;
        let action_uids = build_action_uid_table()?;
        Ok(Self {
            authorizer: Arc::new(Authorizer::new()),
            policies: Arc::new(policies),
            metastore,
            action_uids: Arc::new(action_uids),
        })
    }

    /// The engine with the policies of the `.cedar` files `extra_paths` on
    /// top of the defaults.
    pub fn with_policy_files(
        metastore: DynMetastore,
        extra_paths: &[std::path::PathBuf],
    ) -> Result<Self> {
        let mut engine = Self::new(metastore)?;
        for path in extra_paths {
            let src = std::fs::read_to_string(path).map_err(|e| {
                Error::invalid_argument(format!("read Cedar policy file '{}': {e}", path.display()))
            })?;
            engine.add_policies(&src).map_err(|e| {
                Error::invalid_argument(format!("policy file '{}': {e}", path.display()))
            })?;
        }
        Ok(engine)
    }

    /// Add the Cedar policies of `src` to the current ones. Each gets the id
    /// `extra_<n>`: the ids that `PolicySet::from_str` gives (`policy0`,
    /// `policy1`, …) clash with the default policy's and with other files'.
    pub fn add_policies(&mut self, src: &str) -> Result<()> {
        use cedar_policy::PolicyId;
        let extra = PolicySet::from_str(src)
            .map_err(|e| Error::invalid_argument(format!("invalid Cedar policy: {e}")))?;
        let mut merged = (*self.policies).clone();
        let base_seq = merged.policies().count();
        for (i, p) in extra.policies().enumerate() {
            let new_id: PolicyId = format!("extra_{}", base_seq + i)
                .parse()
                .map_err(|e| Error::internal(format!("synthetic policy id: {e}")))?;
            let renamed = p.clone().new_id(new_id);
            merged
                .add(renamed)
                .map_err(|e| Error::invalid_argument(format!("Cedar policy id collision: {e}")))?;
        }
        self.policies = Arc::new(merged);
        Ok(())
    }
}

/// The `Privilege` → `Action::"<wire name>"` UID table.
fn build_action_uid_table() -> Result<HashMap<Privilege, EntityUid>> {
    let mut out = HashMap::with_capacity(Privilege::ALL.len());
    for &p in Privilege::ALL {
        let wire = serde_json::to_value(p)
            .map_err(|e| Error::internal(format!("privilege wire repr: {e}")))?;
        out.insert(p, uid("Action", wire.as_str().unwrap_or_default())?);
    }
    Ok(out)
}

/// One permit policy: a principal may do an action that the resource's
/// `granted_privileges`, from the `permission` table, contain.
fn default_policies() -> Result<PolicySet> {
    let src = r#"
        permit (
            principal,
            action,
            resource
        )
        when {
            resource.granted_privileges.contains(action)
        };
    "#;
    PolicySet::from_str(src)
        .map_err(|e| Error::internal(format!("default Cedar policy failed to parse: {e}")))
}

#[async_trait]
impl PolicyEngine for CedarEngine {
    async fn grants(&self, principal: &str, groups: &[String]) -> Result<Box<dyn Grants>> {
        let grants = self
            .metastore
            .permissions()
            .grants_of(principal, groups)
            .await?;
        Ok(Box::new(CedarGrants {
            authorizer: self.authorizer.clone(),
            policies: self.policies.clone(),
            action_uids: self.action_uids.clone(),
            principal: uid("User", principal)?,
            grants,
        }))
    }
}

/// `<entity_type>::"<id>"`; any `id` is valid.
fn uid(entity_type: &str, id: &str) -> Result<EntityUid> {
    let name: EntityTypeName = entity_type
        .parse()
        .map_err(|e| Error::internal(format!("Cedar entity type: {e}")))?;
    Ok(EntityUid::from_type_name_and_id(name, EntityId::new(id)))
}

/// One principal's grants, evaluated with the Cedar policies.
struct CedarGrants {
    authorizer: Arc<Authorizer>,
    policies: Arc<PolicySet>,
    action_uids: Arc<HashMap<Privilege, EntityUid>>,
    principal: EntityUid,
    grants: HashMap<String, Vec<Privilege>>,
}

impl CedarGrants {
    fn decide(&self, chain: &[&Node], privilege: Privilege) -> Result<bool> {
        let action = self.action_uids.get(&privilege).cloned().ok_or_else(|| {
            Error::internal(format!("Cedar action UID missing for {privilege:?}"))
        })?;
        let uids: Vec<EntityUid> = chain
            .iter()
            .map(|n| uid(entity_type_for(n.kind), &n.id))
            .collect::<Result<_>>()?;
        let mut entities = Vec::with_capacity(chain.len());
        for (i, node) in chain.iter().enumerate() {
            // What is granted here or on a parent.
            let granted: Vec<RestrictedExpression> = chain[i..]
                .iter()
                .flat_map(|n| self.grants.get(&n.id).into_iter().flatten())
                .filter_map(|p| self.action_uids.get(p).cloned())
                .map(RestrictedExpression::new_entity_uid)
                .collect();
            let mut attrs = HashMap::from([(
                "granted_privileges".to_string(),
                RestrictedExpression::new_set(granted),
            )]);
            if let Some(owner) = &node.owner {
                attrs.insert(
                    "owner".to_string(),
                    RestrictedExpression::new_string(owner.clone()),
                );
            }
            let parents = uids.get(i + 1).cloned().into_iter().collect();
            let entity = cedar_policy::Entity::new(uids[i].clone(), attrs, parents)
                .map_err(|e| Error::internal(format!("Cedar entity: {e}")))?;
            entities.push(entity);
        }
        let entities = Entities::from_entities(entities, None)
            .map_err(|e| Error::internal(format!("Cedar entities: {e}")))?;
        let request = Request::new(
            self.principal.clone(),
            action,
            uids[0].clone(),
            Context::empty(),
            None,
        )
        .map_err(|e| Error::internal(format!("Cedar request: {e}")))?;
        let response = self
            .authorizer
            .is_authorized(&request, &self.policies, &entities);
        Ok(matches!(response.decision(), Decision::Allow))
    }
}

impl Grants for CedarGrants {
    fn holds(&self, chain: &[&Node], privilege: Privilege) -> bool {
        if chain.is_empty() {
            return false;
        }
        self.decide(chain, privilege).unwrap_or_else(|e| {
            tracing::warn!(error = %e.message(), "Cedar evaluation failed; denying");
            false
        })
    }
}

fn entity_type_for(t: SecurableType) -> &'static str {
    match t {
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
