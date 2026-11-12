use std::collections::BTreeSet;
use std::str::FromStr;

use async_trait::async_trait;
use cedar_policy::{
    Context, Entities, EntityId, EntityTypeName, EntityUid, PolicyId, PolicySet, Request, Schema,
    ValidationMode, Validator,
};
use serde_json::Value;

use super::wire::{AuthorizeRequest, AuthorizeResponse, Decision, EntityRef};
use super::{Authorizer, failed_closed};

/// Supplies the Cedar entities a [`CedarAuthorizer`] evaluates a request
/// against, implemented by the host against wherever its users, groups and
/// grants live.
#[async_trait]
pub trait EntityLoader: Send + Sync + 'static {
    /// The entities `request` needs, as a Cedar JSON entities array: such as
    /// the principal with its attributes and its ancestors (groups, roles),
    /// and the resource's ancestors. A caller-supplied
    /// [`resource_entity`](AuthorizeRequest::resource_entity) is added
    /// afterwards and replaces a loaded entity with the same uid.
    ///
    /// An error is a deny.
    async fn load(&self, request: &AuthorizeRequest) -> Result<Value, String>;
}

/// Loads nothing: every fact a policy needs is in the request's
/// [`resource_entity`](AuthorizeRequest::resource_entity) and context.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoEntities;

#[async_trait]
impl EntityLoader for NoEntities {
    async fn load(&self, _request: &AuthorizeRequest) -> Result<Value, String> {
        Ok(Value::Array(Vec::new()))
    }
}

/// Policies or a schema [`CedarAuthorizer`] refuses.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CedarConfigError {
    /// The schema doesn't parse.
    #[error("invalid Cedar schema: {0}")]
    Schema(String),
    /// The policies don't parse.
    #[error("invalid Cedar policies: {0}")]
    Policies(String),
    /// The policies contain templates, which are not supported.
    #[error("Cedar policy templates are not supported")]
    Templates,
    /// Two policies have the same `@id`.
    #[error("more than one Cedar policy with id {0}")]
    DuplicateId(String),
    /// The policies don't validate against the schema in strict mode.
    #[error("Cedar policies don't validate against the schema: {0}")]
    Validation(String),
}

/// An [`Authorizer`] that evaluates Cedar policies in-process.
///
/// With a schema ([`with_schema`](Self::with_schema)), the policies are
/// validated against it in strict mode once, and each request, its entities
/// and its context on every request. Without one, nothing is validated.
///
/// A policy's id is its `@id("…")` annotation, else Cedar's positional
/// `policyN`, so [`policies`](AuthorizeResponse::policies) names the
/// determining policies stably.
///
/// It fails closed: if the entities can't be loaded, the request, context
/// or entities are not valid, or a policy fails to evaluate, the answer is
/// a deny with the reason in [`errors`](AuthorizeResponse::errors).
pub struct CedarAuthorizer {
    policies: PolicySet,
    schema: Option<Schema>,
    loader: Box<dyn EntityLoader>,
    engine: cedar_policy::Authorizer,
}

impl CedarAuthorizer {
    /// Evaluates `policies` (Cedar policy syntax) without a schema, against
    /// the entities `loader` supplies.
    ///
    /// # Errors
    ///
    /// When the policies don't parse, or contain templates or duplicate `@id`s.
    pub fn new(policies: &str, loader: impl EntityLoader) -> Result<Self, CedarConfigError> {
        Ok(Self {
            policies: policy_set(policies)?,
            schema: None,
            loader: Box::new(loader),
            engine: cedar_policy::Authorizer::new(),
        })
    }

    /// The names that the policies hold: the id of each entity and each
    /// text, such as the `alice` of `User::"alice"` and of
    /// `resource.owner == "alice"`. A host uses them to tell what its
    /// policies still name after a rename.
    pub fn names(&self) -> BTreeSet<String> {
        fn collect(value: &Value, names: &mut BTreeSet<String>) {
            match value {
                Value::Object(object) => {
                    if let (Some(Value::String(_)), Some(Value::String(id))) =
                        (object.get("type"), object.get("id"))
                    {
                        names.insert(id.clone());
                    }
                    if let Some(Value::String(text)) = object.get("Value") {
                        names.insert(text.clone());
                    }
                    object.values().for_each(|value| collect(value, names));
                }
                Value::Array(items) => items.iter().for_each(|value| collect(value, names)),
                _ => {}
            }
        }
        let mut names = BTreeSet::new();
        for policy in self.policies.policies() {
            if let Ok(json) = policy.to_json() {
                collect(&json, &mut names);
            }
        }
        names
    }

    /// As [`new`](Self::new), validating everything against `schema`
    /// (Cedar schema syntax).
    ///
    /// # Errors
    ///
    /// As [`new`](Self::new), and when the schema doesn't parse or the
    /// policies don't validate against it.
    pub fn with_schema(
        schema: &str,
        policies: &str,
        loader: impl EntityLoader,
    ) -> Result<Self, CedarConfigError> {
        let (schema, warnings) = Schema::from_cedarschema_str(schema)
            .map_err(|e| CedarConfigError::Schema(e.to_string()))?;
        for warning in warnings {
            tracing::debug!(%warning, "Cedar schema warning");
        }
        let policies = policy_set(policies)?;
        let validation = Validator::new(schema.clone()).validate(&policies, ValidationMode::Strict);
        if !validation.validation_passed() {
            let errors: Vec<String> = validation
                .validation_errors()
                .map(ToString::to_string)
                .collect();
            return Err(CedarConfigError::Validation(errors.join("; ")));
        }
        Ok(Self {
            policies,
            schema: Some(schema),
            loader: Box::new(loader),
            engine: cedar_policy::Authorizer::new(),
        })
    }

    async fn decide(&self, request: &AuthorizeRequest) -> Result<AuthorizeResponse, String> {
        let principal = uid(&request.principal)?;
        let action = uid(&request.action)?;
        let resource = uid(&request.resource)?;

        let mut entities = match self.loader.load(request).await {
            Ok(Value::Array(entities)) => entities,
            Ok(_) => return Err("the entity loader did not return an array".into()),
            Err(e) => return Err(format!("loading entities: {e}")),
        };
        if let Some(entity) = &request.resource_entity {
            let kind = resource.type_name().to_string();
            let id: &str = resource.id().as_ref();
            // A uid in either of Cedar's JSON forms: `{type, id}`, or that
            // in `__entity`.
            let is_resource = |entity: &Value| {
                (entity.get("uid"))
                    .map(|uid| uid.get("__entity").unwrap_or(uid))
                    .is_some_and(|uid| {
                        uid.get("type").and_then(Value::as_str) == Some(kind.as_str())
                            && uid.get("id").and_then(Value::as_str) == Some(id)
                    })
            };
            if !is_resource(entity) {
                return Err("resource_entity is not the request's resource".into());
            }
            entities.retain(|loaded| !is_resource(loaded));
            entities.push(entity.clone());
        }
        let schema = self.schema.as_ref();
        let entities = Entities::from_json_value(Value::Array(entities), schema)
            .map_err(|e| format!("invalid entities: {e}"))?;
        let context = Context::from_json_value(
            Value::Object(request.context.clone()),
            schema.map(|schema| (schema, &action)),
        )
        .map_err(|e| format!("invalid context: {e}"))?;
        let cedar_request = Request::new(principal, action, resource, context, schema)
            .map_err(|e| format!("invalid request: {e}"))?;

        let response = self
            .engine
            .is_authorized(&cedar_request, &self.policies, &entities);
        let errors: Vec<String> = response
            .diagnostics()
            .errors()
            .map(ToString::to_string)
            .collect();
        // Cedar skips a policy that fails to evaluate, so a `forbid` that
        // errors would not stop an allow.
        if !errors.is_empty() {
            return Ok(AuthorizeResponse {
                decision: Decision::Deny,
                policies: Vec::new(),
                errors,
            });
        }
        let mut policies: Vec<String> = response
            .diagnostics()
            .reason()
            .map(ToString::to_string)
            .collect();
        policies.sort();
        Ok(AuthorizeResponse {
            decision: match response.decision() {
                cedar_policy::Decision::Allow => Decision::Allow,
                cedar_policy::Decision::Deny => Decision::Deny,
            },
            policies,
            errors,
        })
    }
}

/// The policies of `source`, each with its `@id`, else its position.
fn policy_set(source: &str) -> Result<PolicySet, CedarConfigError> {
    let parsed =
        PolicySet::from_str(source).map_err(|e| CedarConfigError::Policies(e.to_string()))?;
    if parsed.templates().next().is_some() {
        return Err(CedarConfigError::Templates);
    }
    let mut policies = PolicySet::new();
    for policy in parsed.policies() {
        let id = policy
            .annotation("id")
            .map_or_else(|| policy.id().to_string(), str::to_owned);
        policies
            .add(policy.new_id(PolicyId::new(&id)))
            .map_err(|_| CedarConfigError::DuplicateId(id))?;
    }
    Ok(policies)
}

fn uid(entity: &EntityRef) -> Result<EntityUid, String> {
    let entity_type = EntityTypeName::from_str(&entity.entity_type)
        .map_err(|e| format!("invalid entity type {:?}: {e}", entity.entity_type))?;
    Ok(EntityUid::from_type_name_and_id(
        entity_type,
        EntityId::new(&entity.id),
    ))
}

#[async_trait]
impl Authorizer for CedarAuthorizer {
    async fn authorize(&self, request: &AuthorizeRequest) -> AuthorizeResponse {
        self.decide(request).await.unwrap_or_else(failed_closed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use serde_json::json;

    use super::*;
    use crate::authz::AuthContext;

    fn request(action: &str, context: AuthContext) -> AuthorizeRequest {
        AuthorizeRequest::new(
            EntityRef::new("User", "alice"),
            EntityRef::new("Action", action),
            EntityRef::new("Doc", "d1"),
        )
        .with_context(context)
        .with_resource_entity(json!({
            "uid": { "type": "Doc", "id": "d1" },
            "attrs": { "readers": ["alice"] },
            "parents": [{ "type": "Folder", "id": "f1" }],
        }))
    }

    #[tokio::test]
    async fn policies_see_the_resource_and_the_request_context() {
        let authorizer = CedarAuthorizer::new(
            r#"
            @id("readers")
            permit (principal, action == Action::"read", resource in Folder::"f1")
            when { resource.readers.contains("alice") };

            @id("office-network")
            forbid (principal, action, resource)
            unless { context has source_ip && context.source_ip.isInRange(ip("10.0.0.0/8")) };
            "#,
            NoEntities,
        )
        .unwrap();
        let office = AuthContext::new()
            .source_ip("10.1.2.3".parse().unwrap())
            .request_time(SystemTime::now());
        let answer = authorizer.authorize(&request("read", office.clone())).await;
        assert!(answer.is_allowed(), "{answer:?}");
        assert_eq!(answer.policies, ["readers"]);
        assert!(
            !authorizer
                .authorize(&request("write", office))
                .await
                .is_allowed()
        );
        let home = AuthContext::new().source_ip("192.0.2.1".parse().unwrap());
        let answer = authorizer.authorize(&request("read", home)).await;
        assert_eq!(answer.policies, ["office-network"]);
        assert!(!answer.is_allowed());
    }

    #[tokio::test]
    async fn bad_requests_fail_closed() {
        let authorizer =
            CedarAuthorizer::new("permit (principal, action, resource);", NoEntities).unwrap();
        let mut bad = request("read", AuthContext::new());
        bad.resource = EntityRef::new("not a type", "d1");
        let answer = authorizer.authorize(&bad).await;
        assert!(!answer.is_allowed() && !answer.errors.is_empty());
        let other = request("read", AuthContext::new())
            .with_resource_entity(json!({ "uid": { "type": "Doc", "id": "d2" } }));
        assert!(!authorizer.authorize(&other).await.is_allowed());
    }

    /// Loads the request's resource with its uid in the `__entity` form.
    struct ExplicitUid;

    #[async_trait]
    impl EntityLoader for ExplicitUid {
        async fn load(&self, _request: &AuthorizeRequest) -> Result<Value, String> {
            Ok(json!([{
                "uid": { "__entity": { "type": "Doc", "id": "d1" } },
                "attrs": {},
                "parents": [],
            }]))
        }
    }

    #[tokio::test]
    async fn the_resource_entity_replaces_the_loaded_one_in_either_uid_form() {
        let policy =
            "permit (principal, action, resource) when { resource.readers.contains(\"alice\") };";
        let authorizer = CedarAuthorizer::new(policy, ExplicitUid).unwrap();
        let implicit = request("read", AuthContext::new());
        let answer = authorizer.authorize(&implicit).await;
        assert!(answer.is_allowed(), "{answer:?}");
        let explicit = request("read", AuthContext::new()).with_resource_entity(json!({
            "uid": { "__entity": { "type": "Doc", "id": "d1" } },
            "attrs": { "readers": ["alice"] },
            "parents": [],
        }));
        let answer = authorizer.authorize(&explicit).await;
        assert!(answer.is_allowed(), "{answer:?}");
    }

    #[tokio::test]
    async fn a_policy_that_fails_to_evaluate_denies() {
        let authorizer = CedarAuthorizer::new(
            r#"
            permit (principal, action, resource);
            forbid (principal, action, resource) unless { resource.owner == "alice" };
            "#,
            NoEntities,
        )
        .unwrap();
        // The resource has no `owner`, so the `forbid` errors.
        let answer = authorizer
            .authorize(&request("read", AuthContext::new()))
            .await;
        assert!(
            !answer.is_allowed() && !answer.errors.is_empty(),
            "{answer:?}"
        );
        assert!(answer.policies.is_empty());
    }

    #[test]
    fn policies_and_schemas_are_checked() {
        assert!(matches!(
            CedarAuthorizer::new("permit (", NoEntities),
            Err(CedarConfigError::Policies(_))
        ));
        assert!(matches!(
            CedarAuthorizer::new(
                "@id(\"a\") permit (principal, action, resource);\n@id(\"a\") forbid (principal, action, resource);",
                NoEntities
            ),
            Err(CedarConfigError::DuplicateId(_))
        ));
        let schema =
            "entity User; entity Doc; action read appliesTo { principal: User, resource: Doc };";
        assert!(
            CedarAuthorizer::with_schema(
                schema,
                r#"permit (principal, action == Action::"read", resource);"#,
                NoEntities
            )
            .is_ok()
        );
        assert!(matches!(
            CedarAuthorizer::with_schema(
                schema,
                r#"permit (principal, action == Action::"write", resource);"#,
                NoEntities
            ),
            Err(CedarConfigError::Validation(_))
        ));
    }
}
