//! The SCIM schemas of the server (RFC 7643, section 7): the attributes of
//! users and groups. The discovery endpoints show them, requests are read
//! by them (names in any case, booleans also as text), and the
//! `attributes` and `excludedAttributes` parameters pick from them.

use serde_json::{Map, Value, json};

use uc_api::scim::schema_uri;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    String,
    Boolean,
    Complex,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    ReadOnly,
    ReadWrite,
    Immutable,
}

/// An attribute of a schema.
#[derive(Debug, Clone, Copy)]
pub struct Attribute {
    pub name: &'static str,
    pub kind: Type,
    pub description: &'static str,
    pub multi_valued: bool,
    pub required: bool,
    pub mutability: Mutability,
    /// Unique among the resources of its type.
    pub unique: bool,
    pub canonical_values: &'static [&'static str],
    pub reference_types: &'static [&'static str],
    pub sub_attributes: &'static [Attribute],
}

const fn attribute(name: &'static str, kind: Type, description: &'static str) -> Attribute {
    Attribute {
        name,
        kind,
        description,
        multi_valued: false,
        required: false,
        mutability: Mutability::ReadWrite,
        unique: false,
        canonical_values: &[],
        reference_types: &[],
        sub_attributes: &[],
    }
}

const fn string(name: &'static str, description: &'static str) -> Attribute {
    attribute(name, Type::String, description)
}

const fn boolean(name: &'static str, description: &'static str) -> Attribute {
    attribute(name, Type::Boolean, description)
}

const fn complex(
    name: &'static str,
    description: &'static str,
    sub_attributes: &'static [Attribute],
) -> Attribute {
    Attribute {
        sub_attributes,
        ..attribute(name, Type::Complex, description)
    }
}

impl Attribute {
    const fn multi_valued(self) -> Self {
        Self {
            multi_valued: true,
            ..self
        }
    }

    const fn required(self) -> Self {
        Self {
            required: true,
            ..self
        }
    }

    const fn unique(self) -> Self {
        Self {
            unique: true,
            ..self
        }
    }

    const fn mutability(self, mutability: Mutability) -> Self {
        Self { mutability, ..self }
    }

    const fn canonical(self, canonical_values: &'static [&'static str]) -> Self {
        Self {
            canonical_values,
            ..self
        }
    }

    const fn references(self, reference_types: &'static [&'static str]) -> Self {
        Self {
            reference_types,
            ..self
        }
    }

    /// The sub-attribute `name`, in any case.
    pub fn sub(&self, name: &str) -> Option<&'static Attribute> {
        find(self.sub_attributes, name)
    }

    fn to_json(self) -> Value {
        let kind = match self.kind {
            Type::String => "string",
            Type::Boolean => "boolean",
            Type::Complex => "complex",
            Type::Reference => "reference",
        };
        let mutability = match self.mutability {
            Mutability::ReadOnly => "readOnly",
            Mutability::ReadWrite => "readWrite",
            Mutability::Immutable => "immutable",
        };
        let mut doc = json!({
            "name": self.name,
            "type": kind,
            "multiValued": self.multi_valued,
            "description": self.description,
            "required": self.required,
            "caseExact": false,
            "mutability": mutability,
            "returned": "default",
            "uniqueness": if self.unique { "server" } else { "none" },
        });
        if !self.canonical_values.is_empty() {
            doc["canonicalValues"] = json!(self.canonical_values);
        }
        if !self.reference_types.is_empty() {
            doc["referenceTypes"] = json!(self.reference_types);
        }
        if !self.sub_attributes.is_empty() {
            doc["subAttributes"] =
                Value::Array(self.sub_attributes.iter().map(|a| a.to_json()).collect());
        }
        doc
    }
}

/// The attribute `name` of `attributes`, in any case.
pub fn find(attributes: &'static [Attribute], name: &str) -> Option<&'static Attribute> {
    attributes
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
}

const NAME: &[Attribute] = &[
    string("formatted", "The full name, formatted for display."),
    string("familyName", "The family name, or last name."),
    string("givenName", "The given name, or first name."),
    string("middleName", "The middle name."),
    string("honorificPrefix", "The honorific prefix, such as Ms."),
    string("honorificSuffix", "The honorific suffix, such as III."),
];

const EMAIL: &[Attribute] = &[
    string("value", "The email address."),
    string("display", "A name for the address, for display."),
    string("type", "The kind of address.").canonical(&["work", "home", "other"]),
    boolean(
        "primary",
        "Whether this is the primary address, the user's name in grants.",
    ),
];

const PHOTO: &[Attribute] = &[
    attribute("value", Type::Reference, "The URL of the picture.").references(&["external"]),
    string("display", "A name for the picture, for display."),
    string("type", "The kind of picture.").canonical(&["photo", "thumbnail"]),
    boolean("primary", "Whether this is the primary picture."),
];

const ROLE: &[Attribute] = &[string("value", "The role: admin.")];

/// The attributes of a user.
pub const USER: &[Attribute] = &[
    string(
        "userName",
        "The unique name of the user, such as a user principal name. Without an email, it is the user's name in grants.",
    )
    .required()
    .unique(),
    complex("name", "The parts of the user's name.", NAME),
    string("displayName", "The name of the user, for display."),
    boolean("active", "Whether the user can sign in. Absent: true."),
    complex(
        "emails",
        "The email addresses of the user. The primary one, else the first, is the user's name in grants.",
        EMAIL,
    )
    .multi_valued(),
    complex("photos", "Pictures of the user.", PHOTO).multi_valued(),
    complex(
        "roles",
        "The roles of the user. Only /Me has them: admin for a metastore admin.",
        ROLE,
    )
    .multi_valued()
    .mutability(Mutability::ReadOnly),
];

const MEMBER: &[Attribute] = &[
    string("value", "The id of the user.").mutability(Mutability::Immutable),
    attribute(
        "$ref",
        Type::Reference,
        "The URL of the user. The server sets it.",
    )
    .references(&["User"])
    .mutability(Mutability::Immutable),
    string("display", "The name of the user, for display."),
    string("type", "The kind of member.")
        .canonical(&["User"])
        .mutability(Mutability::Immutable),
];

/// The attributes of a group.
pub const GROUP: &[Attribute] = &[
    string("displayName", "The name of the group, its name in grants.")
        .required()
        .unique(),
    complex("members", "The users in the group.", MEMBER).multi_valued(),
];

/// The common attribute that a client writes (RFC 7643, section 3.1).
pub const EXTERNAL_ID: Attribute = string(
    "externalId",
    "The identity the resource has at an identity provider.",
);

/// The attributes that every resource has (RFC 7643, section 3.1).
pub const COMMON: &[&str] = &["schemas", "id", EXTERNAL_ID.name, "meta"];

/// A resource type: its schema URI, name, endpoint and attributes.
pub struct ResourceType {
    pub id: &'static str,
    pub schema: &'static str,
    pub endpoint: &'static str,
    pub description: &'static str,
    pub attributes: &'static [Attribute],
}

pub const USERS: ResourceType = ResourceType {
    id: "User",
    schema: schema_uri::USER,
    endpoint: "/Users",
    description: "User Account",
    attributes: USER,
};

pub const GROUPS: ResourceType = ResourceType {
    id: "Group",
    schema: schema_uri::GROUP,
    endpoint: "/Groups",
    description: "Group",
    attributes: GROUP,
};

pub const RESOURCE_TYPES: &[&ResourceType] = &[&USERS, &GROUPS];

impl ResourceType {
    /// The schema document (RFC 7643, section 7), with its URL under `base`.
    pub fn schema_doc(&self, base: &str) -> Value {
        json!({
            "schemas": [schema_uri::SCHEMA],
            "id": self.schema,
            "name": self.id,
            "description": self.description,
            "attributes": self.attributes.iter().map(|a| a.to_json()).collect::<Vec<_>>(),
            "meta": {
                "resourceType": "Schema",
                "location": format!("{base}/Schemas/{}", self.schema),
            },
        })
    }

    /// The resource type document (RFC 7643, section 6), with its URL under
    /// `base`.
    pub fn resource_type_doc(&self, base: &str) -> Value {
        json!({
            "schemas": [schema_uri::RESOURCE_TYPE],
            "id": self.id,
            "name": self.id,
            "endpoint": self.endpoint,
            "description": self.description,
            "schema": self.schema,
            "meta": {
                "resourceType": "ResourceType",
                "location": format!("{base}/ResourceTypes/{}", self.id),
            },
        })
    }

    /// The attribute `name` that a client may write, in any case: one of
    /// the schema that is not read-only, or `externalId`.
    pub fn writable(&self, name: &str) -> Option<&'static Attribute> {
        if name.eq_ignore_ascii_case(EXTERNAL_ID.name) {
            return Some(&EXTERNAL_ID);
        }
        find(self.attributes, name).filter(|a| a.mutability != Mutability::ReadOnly)
    }

    /// Whether resources of the type have the attribute `name`, in any
    /// case.
    pub fn has(&self, name: &str) -> bool {
        COMMON.iter().any(|c| c.eq_ignore_ascii_case(name)) || find(self.attributes, name).is_some()
    }

    /// The first required attribute that `resource` (as [`Self::read`] gives
    /// it) lacks or has empty.
    pub fn missing_required(&self, resource: &Map<String, Value>) -> Option<&'static str> {
        self.attributes
            .iter()
            .filter(|a| a.required)
            .find(|a| match resource.get(a.name) {
                None | Some(Value::Null) => true,
                Some(Value::String(s)) => s.trim().is_empty(),
                Some(Value::Array(items)) => items.is_empty(),
                Some(_) => false,
            })
            .map(|a| a.name)
    }

    /// `value` with the names of known attributes as the schema writes them
    /// and booleans given as text made booleans.
    pub fn read(&self, value: Value) -> Value {
        read_object(value, self.attributes, true)
    }

    /// `resource` with only the attributes that `attributes` names, or
    /// without those that `excluded` names (RFC 7644, section 3.4.2.5).
    /// `schemas` and `id` always stay. Names are `attr` or `attr.sub`, in
    /// any case, with or without the schema URI.
    pub fn select(&self, resource: Value, attributes: &[String], excluded: &[String]) -> Value {
        let Value::Object(mut object) = resource else {
            return resource;
        };
        let paths = |names: &[String]| -> Vec<(String, Option<String>)> {
            names
                .iter()
                .flat_map(|n| n.split(','))
                .map(|n| strip_schema(n.trim(), self.schema))
                .filter(|n| !n.is_empty())
                .map(|n| match n.split_once('.') {
                    Some((attr, sub)) => {
                        (attr.to_ascii_lowercase(), Some(sub.to_ascii_lowercase()))
                    }
                    None => (n.to_ascii_lowercase(), None),
                })
                .collect()
        };
        let always = |key: &str| key == "schemas" || key == "id";
        let wanted = paths(attributes);
        if !wanted.is_empty() {
            object.retain(|key, value| {
                if always(key) {
                    return true;
                }
                let key = key.to_ascii_lowercase();
                let subs: Vec<&str> = wanted
                    .iter()
                    .filter(|(attr, _)| *attr == key)
                    .map(|(_, sub)| sub.as_deref().unwrap_or(""))
                    .collect();
                if subs.is_empty() {
                    return false;
                }
                if !subs.contains(&"") {
                    keep_subs(value, &subs);
                }
                true
            });
            return Value::Object(object);
        }
        for (attr, sub) in paths(excluded) {
            let key = object
                .keys()
                .find(|k| k.eq_ignore_ascii_case(&attr))
                .cloned();
            let Some(key) = key.filter(|k| !always(k)) else {
                continue;
            };
            match sub {
                None => {
                    object.remove(&key);
                }
                Some(sub) => {
                    if let Some(value) = object.get_mut(&key) {
                        drop_sub(value, &sub);
                    }
                }
            }
        }
        Value::Object(object)
    }
}

/// `name` without a leading `<schema>:`.
pub fn strip_schema<'a>(name: &'a str, schema: &str) -> &'a str {
    match name.get(..schema.len()) {
        Some(prefix) if prefix.eq_ignore_ascii_case(schema) => {
            name[schema.len()..].trim_start_matches(':')
        }
        _ => name,
    }
}

/// Keep only the sub-attributes `subs` of `value`, an object or a list of
/// objects.
fn keep_subs(value: &mut Value, subs: &[&str]) {
    let keep = |object: &mut Map<String, Value>| {
        object.retain(|key, _| subs.iter().any(|s| key.eq_ignore_ascii_case(s)));
    };
    match value {
        Value::Object(object) => keep(object),
        Value::Array(items) => items
            .iter_mut()
            .filter_map(Value::as_object_mut)
            .for_each(keep),
        _ => {}
    }
}

/// Drop the sub-attribute `sub` of `value`, an object or a list of objects.
fn drop_sub(value: &mut Value, sub: &str) {
    let drop = |object: &mut Map<String, Value>| {
        object.retain(|key, _| !key.eq_ignore_ascii_case(sub));
    };
    match value {
        Value::Object(object) => drop(object),
        Value::Array(items) => items
            .iter_mut()
            .filter_map(Value::as_object_mut)
            .for_each(drop),
        _ => {}
    }
}

/// `value` read against `attributes` ([`ResourceType::read`]); `top` adds
/// the common attributes.
fn read_object(value: Value, attributes: &'static [Attribute], top: bool) -> Value {
    let Value::Object(object) = value else {
        return value;
    };
    let mut out = Map::with_capacity(object.len());
    for (key, value) in object {
        if top && let Some(common) = COMMON.iter().find(|c| c.eq_ignore_ascii_case(&key)) {
            out.insert(common.to_string(), value);
            continue;
        }
        match find(attributes, &key) {
            Some(attribute) => {
                out.insert(attribute.name.to_string(), read_value(value, attribute));
            }
            None => {
                out.insert(key, value);
            }
        }
    }
    Value::Object(out)
}

/// `value`, a value of `attribute`, read as [`ResourceType::read`] does.
pub fn read_value(value: Value, attribute: &'static Attribute) -> Value {
    match (attribute.kind, value) {
        (_, Value::Array(items)) if attribute.multi_valued => Value::Array(
            items
                .into_iter()
                .map(|item| read_single(item, attribute))
                .collect(),
        ),
        (_, value) => read_single(value, attribute),
    }
}

fn read_single(value: Value, attribute: &'static Attribute) -> Value {
    match (attribute.kind, value) {
        (Type::Complex, value @ Value::Object(_)) => {
            read_object(value, attribute.sub_attributes, false)
        }
        (Type::Boolean, Value::String(text)) if text.eq_ignore_ascii_case("true") => {
            Value::Bool(true)
        }
        (Type::Boolean, Value::String(text)) if text.eq_ignore_ascii_case("false") => {
            Value::Bool(false)
        }
        (_, value) => value,
    }
}

/// The ServiceProviderConfig document (RFC 7643, section 5), with its URL
/// under `base`. `max_payload_size` is the most bytes of a request body.
pub fn service_provider_config(base: &str, max_payload_size: usize) -> Value {
    json!({
        "schemas": [schema_uri::SERVICE_PROVIDER_CONFIG],
        "documentationUri": "https://datatracker.ietf.org/doc/html/rfc7644",
        "patch":          { "supported": true },
        "bulk":           { "supported": true, "maxOperations": MAX_BULK_OPERATIONS,
                            "maxPayloadSize": max_payload_size },
        "filter":         { "supported": true, "maxResults": MAX_RESULTS },
        "changePassword": { "supported": false },
        "sort":           { "supported": false },
        "etag":           { "supported": false },
        "authenticationSchemes": [
            { "type": "oauthbearertoken", "name": "OAuth Bearer Token",
              "description": "Authentication scheme using the OAuth Bearer Token Standard",
              "specUri": "https://datatracker.ietf.org/doc/html/rfc6750",
              "primary": true }
        ],
        "meta": {
            "resourceType": "ServiceProviderConfig",
            "location": format!("{base}/ServiceProviderConfig"),
        },
    })
}

/// The most resources in one list.
pub const MAX_RESULTS: usize = 1000;

/// The most operations in a bulk request.
pub const MAX_BULK_OPERATIONS: usize = 100;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_names_in_any_case_and_text_booleans() {
        let read = USERS.read(json!({
            "UserName": "a", "ACTIVE": "False", "ExternalID": "e",
            "emails": [{ "Value": "a@x", "Primary": "true" }],
            "name": { "GIVENNAME": "A" },
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": { "department": "d" },
        }));
        assert_eq!(read["userName"], "a");
        assert_eq!(read["active"], false);
        assert_eq!(read["externalId"], "e");
        assert_eq!(read["emails"], json!([{ "value": "a@x", "primary": true }]));
        assert_eq!(read["name"], json!({ "givenName": "A" }));
        assert!(read["urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"].is_object());
    }

    #[test]
    fn selects_attributes() {
        let user = json!({
            "schemas": [schema_uri::USER], "id": "1", "userName": "a", "displayName": "A",
            "emails": [{ "value": "a@x", "type": "work" }], "meta": { "resourceType": "User" },
        });
        let pick = |attributes: &[&str], excluded: &[&str]| {
            let owned = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
            USERS.select(user.clone(), &owned(attributes), &owned(excluded))
        };
        assert_eq!(
            pick(&["userName,emails.value"], &[]),
            json!({ "schemas": [schema_uri::USER], "id": "1", "userName": "a",
                "emails": [{ "value": "a@x" }] })
        );
        assert_eq!(
            pick(
                &["urn:ietf:params:scim:schemas:core:2.0:User:displayName"],
                &[]
            ),
            json!({ "schemas": [schema_uri::USER], "id": "1", "displayName": "A" })
        );
        let without = pick(&[], &["emails", "meta", "id"]);
        assert!(without.get("emails").is_none() && without.get("meta").is_none());
        assert_eq!(without["id"], "1");
    }
}
