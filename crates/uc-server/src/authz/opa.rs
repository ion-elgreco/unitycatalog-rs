//! Open Policy Agent as the engine of privileges (`auth.engine = "opa"`).
//!
//! The rules stay in the server ([`super::rules`]). Each privilege that a
//! rule checks, `OWNER` and the admin (`OWNER` on the metastore) included,
//! is one question to OPA, in the input of upstream's `OpaInputBuilder`:
//!
//! ```json
//! { "input": {
//!     "principal": { "id": "<user id>", "email": "alice@example.com",
//!                    "external_id": "…", "groups": ["entra:analysts"] },
//!     "privilege": "SELECT",
//!     "resource": { "id": "<table id>", "type": "TABLE", "name": "orders",
//!                   "owner": "bob@example.com",
//!                   "parents": [ { "id": "…", "type": "SCHEMA", "name": "sales" },
//!                                { "id": "…", "type": "CATALOG", "name": "main" } ] },
//!     "context": { "auth_method": "jwt", "source_ip": "10.1.2.3", … } } }
//! ```
//!
//! `groups`, `owner` and `context` are not in upstream's input. The grants
//! are not in it: they do not count. OPA answers `{"result": {"allow":
//! true}}`; any other answer, or none, fails the request with 503.

use std::time::Duration;

use serde_json::{Map, Value, json};

use uc_api::{Privilege, ScimUser};
use uc_auth::authz::OpaClient;
use uc_common::{Error, ErrorCode, SecurableType};
use uc_metastore::Node;

/// The privilege of owners, and of the admin on the metastore.
pub const OWNER: &str = "OWNER";

/// The OPA server that decides privileges.
#[derive(Debug, Clone)]
pub struct Opa {
    client: OpaClient,
}

impl Opa {
    /// The OPA server of the decision URL `url`, such as
    /// `http://opa:8181/v1/data/uc/authz`. `timeout` is for each decision.
    pub fn new(url: &str, timeout: Duration) -> std::result::Result<Self, String> {
        Ok(Self {
            client: OpaClient::new(url, timeout)?,
        })
    }

    /// The OPA server that takes `token` (`--authentication=token`).
    pub fn with_token(self, token: &str) -> std::result::Result<Self, String> {
        Ok(Self {
            client: self.client.with_token(token)?,
        })
    }

    /// Whether the token goes over plain `http`.
    pub fn sends_token_in_clear(&self) -> bool {
        self.client.sends_token_in_clear()
    }

    /// OPA's decision on `input`, or why it made none ([`unavailable`]).
    pub async fn decide(&self, input: &Value) -> std::result::Result<bool, String> {
        self.client.allow(input).await
    }
}

/// The error of a failed decision.
pub fn unavailable(cause: &str) -> Error {
    Error::new(
        ErrorCode::Unavailable,
        format!("Could not get an authorization decision from OPA: {cause}"),
    )
}

/// The caller in the input: the user's id, its principal as `email`, its
/// `external_id` when it has one, and `groups`.
pub fn principal(user: &ScimUser, groups: &[String]) -> Value {
    let mut principal = json!({
        "id": user.id.clone().unwrap_or_default(),
        "email": user.principal(),
        "groups": groups,
    });
    if let Some(external_id) = &user.external_id {
        principal["external_id"] = json!(external_id);
    }
    principal
}

/// The name of `privilege` in the input: upstream's enum name, such as
/// `USE_CATALOG`.
pub fn privilege(privilege: Privilege) -> String {
    let name = serde_json::to_value(privilege)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{privilege:?}"));
    name.replace(' ', "_")
}

/// The type of a securable in the input: upstream's enum name.
pub fn securable_type(kind: SecurableType) -> &'static str {
    match kind {
        SecurableType::Metastore => "METASTORE",
        SecurableType::Catalog => "CATALOG",
        SecurableType::Schema => "SCHEMA",
        SecurableType::Table => "TABLE",
        SecurableType::Volume => "VOLUME",
        SecurableType::Function => "FUNCTION",
        SecurableType::RegisteredModel => "REGISTERED_MODEL",
        SecurableType::ExternalLocation => "EXTERNAL_LOCATION",
        SecurableType::Credential => "CREDENTIAL",
    }
}

/// A securable in the input: id, type, and its name and owner when it has
/// them. The metastore has neither.
fn describe(node: &Node) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("id".into(), json!(node.id));
    object.insert("type".into(), json!(securable_type(node.kind)));
    if !node.name.is_empty() {
        object.insert("name".into(), json!(node.name));
    }
    object
}

/// The securable `chain[0]` in the input, with its owner and its parents:
/// the rest of `chain` without the metastore, from the schema to the
/// catalog. Upstream gives a catalog no parents.
pub fn resource(chain: &[&Node]) -> Value {
    let node = chain[0];
    let mut resource = describe(node);
    if let Some(owner) = &node.owner {
        resource.insert("owner".into(), json!(owner));
    }
    let parents: Vec<Value> = (chain[1..].iter())
        .filter(|parent| parent.kind != SecurableType::Metastore)
        .map(|parent| Value::Object(describe(parent)))
        .collect();
    resource.insert("parents".into(), Value::Array(parents));
    Value::Object(resource)
}

/// The request context in the input. Values that are Cedar extensions
/// (`source_ip`, `request_time`) are plain text here.
pub fn context(context: &Map<String, Value>) -> Value {
    let plain = context.iter().map(|(key, value)| {
        let value = match value.pointer("/__extn/arg") {
            Some(arg) => arg.clone(),
            None => value.clone(),
        };
        (key.clone(), value)
    });
    Value::Object(plain.collect())
}

/// The input of one question: may `principal` use `privilege` on
/// `resource`?
pub fn input(principal: &Value, privilege: &str, resource: Value, context: &Value) -> Value {
    json!({
        "principal": principal,
        "privilege": privilege,
        "resource": resource,
        "context": context,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(kind: SecurableType, id: &str, name: &str, owner: Option<&str>) -> Node {
        Node {
            kind,
            id: id.into(),
            name: name.into(),
            owner: owner.map(str::to_string),
        }
    }

    #[test]
    fn names_privileges_and_types_as_upstream() {
        assert_eq!(privilege(Privilege::UseCatalog), "USE_CATALOG");
        assert_eq!(
            privilege(Privilege::CreateExternalLocation),
            "CREATE_EXTERNAL_LOCATION"
        );
        assert_eq!(privilege(Privilege::Select), "SELECT");
        assert_eq!(
            securable_type(SecurableType::RegisteredModel),
            "REGISTERED_MODEL"
        );
    }

    #[test]
    fn builds_upstreams_input_with_ours_on_top() {
        let user = ScimUser {
            id: Some("u-1".into()),
            user_name: "alice@example.com".into(),
            external_id: Some("entra:abc".into()),
            ..Default::default()
        };
        let table = node(
            SecurableType::Table,
            "t-1",
            "orders",
            Some("bob@example.com"),
        );
        let schema = node(SecurableType::Schema, "s-1", "sales", None);
        let catalog = node(SecurableType::Catalog, "c-1", "main", Some("carol"));
        let metastore = node(SecurableType::Metastore, "m-1", "", Some("admin"));
        let mut request = Map::new();
        request.insert("auth_method".into(), json!("jwt"));
        request.insert(
            "source_ip".into(),
            json!({ "__extn": { "fn": "ip", "arg": "10.1.2.3" } }),
        );
        let input = input(
            &principal(&user, &["entra:analysts".into()]),
            &privilege(Privilege::Select),
            resource(&[&table, &schema, &catalog, &metastore]),
            &context(&request),
        );
        assert_eq!(
            input,
            json!({
                "principal": {
                    "id": "u-1", "email": "alice@example.com", "external_id": "entra:abc",
                    "groups": ["entra:analysts"],
                },
                "privilege": "SELECT",
                "resource": {
                    "id": "t-1", "type": "TABLE", "name": "orders", "owner": "bob@example.com",
                    "parents": [
                        { "id": "s-1", "type": "SCHEMA", "name": "sales" },
                        { "id": "c-1", "type": "CATALOG", "name": "main" },
                    ],
                },
                "context": { "auth_method": "jwt", "source_ip": "10.1.2.3" },
            })
        );
        assert_eq!(
            resource(&[&metastore]),
            json!({ "id": "m-1", "type": "METASTORE", "owner": "admin", "parents": [] })
        );
        assert_eq!(
            resource(&[&catalog, &metastore]),
            json!({ "id": "c-1", "type": "CATALOG", "name": "main", "owner": "carol", "parents": [] })
        );
    }
}
