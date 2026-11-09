//! SCIM PATCH (RFC 7644, section 3.5.2) on a resource as JSON: `add`,
//! `replace` and `remove`, in any case, with the paths `attr`, `attr.sub`,
//! `attr[filter]` and `attr[filter].sub`. Operations on attributes that
//! the schema does not have, or that are read-only, change nothing.
//!
//! As Entra ID needs: a `remove` of a list with a `value` removes only
//! those values, and an `add` or `replace` of `attr[filter].sub` that
//! matches nothing adds a value made of the filter's `eq` terms, such as
//! `emails[type eq "work"].value`. A filter on `value`, such as
//! `members[value eq "<id>"].display`, names an existing value only: one
//! that matches nothing is `noTarget` (RFC 7644, section 3.5.2.3).

use serde_json::{Map, Value};

use uc_api::ScimPatchOperation;

use super::scim_filter::{self, Filter};
use super::scim_schema::{self, Attribute, Mutability, ResourceType, Type};

/// Why a patch failed: the SCIM `scimType` and the detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError {
    pub scim_type: &'static str,
    pub detail: String,
}

fn error(scim_type: &'static str, detail: impl Into<String>) -> PatchError {
    PatchError {
        scim_type,
        detail: detail.into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Replace,
    Remove,
}

/// A parsed path.
struct Target {
    attribute: &'static Attribute,
    filter: Option<Filter>,
    sub: Option<&'static Attribute>,
}

/// The common attribute `externalId`, which PATCH may change.
const EXTERNAL_ID: Attribute = Attribute {
    name: "externalId",
    kind: Type::String,
    description: "",
    multi_valued: false,
    required: false,
    mutability: Mutability::ReadWrite,
    unique: false,
    canonical_values: &[],
    reference_types: &[],
    sub_attributes: &[],
};

/// The attribute `name` of `resource_type`, or `externalId`; `None` for an
/// attribute it does not have or may not change.
fn writable(resource_type: &ResourceType, name: &str) -> Option<&'static Attribute> {
    if name.eq_ignore_ascii_case(EXTERNAL_ID.name) {
        return Some(&EXTERNAL_ID);
    }
    scim_schema::find(resource_type.attributes, name)
        .filter(|a| a.mutability != Mutability::ReadOnly)
}

/// Parse `path`; `None` when it names an attribute that [`writable`]
/// leaves out, such as one of an extension schema.
fn parse_path(path: &str, resource_type: &ResourceType) -> Result<Option<Target>, PatchError> {
    let invalid = || error("invalidPath", format!("Invalid path: {path}"));
    let path = scim_schema::strip_schema(path.trim(), resource_type.schema);
    if path
        .get(..4)
        .is_some_and(|start| start.eq_ignore_ascii_case("urn:"))
    {
        return Ok(None);
    }
    let end = path.find(['[', '.']).unwrap_or(path.len());
    let (name, mut rest) = path.split_at(end);
    let mut filter = None;
    if let Some(inside) = rest.strip_prefix('[') {
        let close = closing_bracket(inside).ok_or_else(invalid)?;
        let parsed = scim_filter::parse(&inside[..close])
            .map_err(|e| error("invalidPath", format!("Invalid filter in path {path}: {e}")))?;
        filter = Some(parsed);
        rest = &inside[close + 1..];
    }
    let sub = match rest {
        "" => None,
        _ => Some(rest.strip_prefix('.').ok_or_else(invalid)?),
    };
    let Some(attribute) = writable(resource_type, name) else {
        return Ok(None);
    };
    if filter.is_some() && !attribute.multi_valued {
        return Err(invalid());
    }
    let sub = match sub {
        None => None,
        Some(_) if attribute.kind != Type::Complex => return Err(invalid()),
        Some(sub) => match attribute.sub(sub) {
            Some(sub) => Some(sub),
            None => return Ok(None),
        },
    };
    Ok(Some(Target {
        attribute,
        filter,
        sub,
    }))
}

/// The index in `text` of the `]` that closes a value filter, outside
/// quotes.
fn closing_bracket(text: &str) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    for (i, c) in text.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            ']' if !quoted => return Some(i),
            _ => {}
        }
    }
    None
}

/// Apply `operations`, in order, to `resource`, a resource of
/// `resource_type` as JSON.
pub fn apply(
    resource: &mut Map<String, Value>,
    resource_type: &ResourceType,
    operations: &[ScimPatchOperation],
) -> Result<(), PatchError> {
    for operation in operations {
        let op = match operation.op.to_ascii_lowercase().as_str() {
            "add" => Op::Add,
            "replace" => Op::Replace,
            "remove" => Op::Remove,
            other => {
                return Err(error(
                    "invalidSyntax",
                    format!("Unknown patch operation: {other}"),
                ));
            }
        };
        let value = operation.value.clone();
        match operation.path.as_deref().filter(|p| !p.trim().is_empty()) {
            None => apply_without_path(resource, resource_type, op, value)?,
            Some(path) => {
                if let Some(target) = parse_path(path, resource_type)? {
                    apply_to(resource, &target, op, value)?;
                }
            }
        }
    }
    Ok(())
}

/// An `add` or `replace` whose value holds the attributes. A key may be a
/// path, such as `name.familyName` or `emails[type eq "work"].value`.
fn apply_without_path(
    resource: &mut Map<String, Value>,
    resource_type: &ResourceType,
    op: Op,
    value: Option<Value>,
) -> Result<(), PatchError> {
    if op == Op::Remove {
        return Err(error("noTarget", "A remove operation needs a path"));
    }
    let Some(Value::Object(attributes)) = value.map(|v| resource_type.read(v)) else {
        return Err(error(
            "invalidValue",
            "An operation without a path needs an object of attributes as value",
        ));
    };
    for (name, value) in attributes {
        if let Some(target) = parse_path(&name, resource_type)? {
            apply_to(resource, &target, op, Some(value))?;
        }
    }
    Ok(())
}

fn apply_to(
    resource: &mut Map<String, Value>,
    target: &Target,
    op: Op,
    value: Option<Value>,
) -> Result<(), PatchError> {
    let attribute = target.attribute;
    let name = attribute.name;
    let value = match target.sub {
        Some(sub) => value.map(|v| scim_schema::read_value(v, sub)),
        None => value.map(|v| scim_schema::read_value(v, attribute)),
    };
    let needs_value = || {
        error(
            "invalidValue",
            format!("The operation on {name} needs a value"),
        )
    };
    match (&target.filter, target.sub) {
        (None, None) => match op {
            Op::Remove => remove_whole(resource, attribute, value),
            Op::Add => add_whole(resource, attribute, value.ok_or_else(needs_value)?),
            Op::Replace => set_whole(resource, attribute, value.ok_or_else(needs_value)?),
        },
        (None, Some(sub)) if !attribute.multi_valued => {
            let object = resource
                .entry(name)
                .or_insert_with(|| Value::Object(Map::new()));
            if !object.is_object() {
                *object = Value::Object(Map::new());
            }
            set_sub(object, sub, op, value);
        }
        (None, Some(sub)) => {
            let items = list(resource, name);
            if items.is_empty() && op != Op::Remove {
                items.push(Value::Object(Map::new()));
            }
            for item in items.iter_mut() {
                set_sub(item, sub, op, value.clone());
            }
            if sub.name == "primary" && value == Some(Value::Bool(true)) {
                let last = items.len().saturating_sub(1);
                keep_one_primary(items, last);
            }
        }
        (Some(filter), sub) => {
            let items = list(resource, name);
            let matching: Vec<usize> = (0..items.len())
                .filter(|&i| scim_filter::matches(filter, &items[i]))
                .collect();
            match (op, sub) {
                (Op::Remove, None) => {
                    let mut i = 0;
                    items.retain(|_| {
                        let keep = !matching.contains(&i);
                        i += 1;
                        keep
                    });
                }
                (Op::Remove, Some(sub)) => {
                    for &i in &matching {
                        set_sub(&mut items[i], sub, op, None);
                    }
                }
                (_, None) => {
                    let Some(Value::Object(new)) = value else {
                        return Err(error(
                            "invalidValue",
                            format!("The operation on {name} needs an object as value"),
                        ));
                    };
                    if matching.is_empty() {
                        return Err(error("noTarget", format!("No value of {name} matches")));
                    }
                    for &i in &matching {
                        match op {
                            Op::Replace => items[i] = Value::Object(new.clone()),
                            _ => merge(&mut items[i], &new),
                        }
                    }
                }
                (_, Some(sub)) => {
                    let value = value.ok_or_else(needs_value)?;
                    let chosen = match matching.is_empty() {
                        true => {
                            let made = (filter.equalities())
                                .filter(|terms| {
                                    terms
                                        .iter()
                                        .all(|(attr, _)| !attr.eq_ignore_ascii_case("value"))
                                })
                                .ok_or_else(|| {
                                    error("noTarget", format!("No value of {name} matches"))
                                })?;
                            items.push(Value::Object(made.into_iter().collect()));
                            vec![items.len() - 1]
                        }
                        false => matching,
                    };
                    for &i in &chosen {
                        set_sub(&mut items[i], sub, op, Some(value.clone()));
                    }
                    if sub.name == "primary"
                        && value == Value::Bool(true)
                        && let Some(&last) = chosen.last()
                    {
                        keep_one_primary(items, last);
                    }
                }
            }
        }
    }
    tidy(resource, name);
    Ok(())
}

/// The list `name` of `resource`, made when missing.
fn list<'a>(resource: &'a mut Map<String, Value>, name: &str) -> &'a mut Vec<Value> {
    let value = resource
        .entry(name)
        .or_insert_with(|| Value::Array(Vec::new()));
    if !value.is_array() {
        let single = std::mem::take(value);
        *value = Value::Array(match single {
            Value::Null => Vec::new(),
            other => vec![other],
        });
    }
    match value {
        Value::Array(items) => items,
        _ => unreachable!("made an array above"),
    }
}

/// `value` as a list of values.
fn values(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        other => vec![other],
    }
}

/// Whether two values of a list are the same value: complex values by
/// their `value`, others as they are.
fn same(a: &Value, b: &Value) -> bool {
    match (a.get("value"), b.get("value")) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

fn add_whole(resource: &mut Map<String, Value>, attribute: &Attribute, value: Value) {
    let name = attribute.name;
    if attribute.multi_valued {
        let items = list(resource, name);
        for new in values(value) {
            let primary = new.get("primary") == Some(&Value::Bool(true));
            let at = match items.iter().position(|item| same(item, &new)) {
                Some(i) => {
                    if let Value::Object(fields) = &new {
                        merge(&mut items[i], fields);
                    }
                    i
                }
                None => {
                    items.push(new);
                    items.len() - 1
                }
            };
            if primary {
                keep_one_primary(items, at);
            }
        }
    } else if attribute.kind == Type::Complex {
        match (resource.get_mut(name), value) {
            (Some(existing @ Value::Object(_)), Value::Object(new)) => merge(existing, &new),
            (_, value) => {
                resource.insert(name.to_string(), value);
            }
        }
    } else {
        resource.insert(name.to_string(), value);
    }
}

fn set_whole(resource: &mut Map<String, Value>, attribute: &Attribute, value: Value) {
    let name = attribute.name;
    match value {
        Value::Null => {
            resource.remove(name);
        }
        value if attribute.multi_valued => {
            resource.insert(name.to_string(), Value::Array(values(value)));
        }
        // A complex value replaces the sub-attributes it has.
        Value::Object(new) if attribute.kind == Type::Complex => match resource.get_mut(name) {
            Some(existing @ Value::Object(_)) => merge(existing, &new),
            _ => {
                resource.insert(name.to_string(), Value::Object(new));
            }
        },
        value => {
            resource.insert(name.to_string(), value);
        }
    }
}

fn remove_whole(resource: &mut Map<String, Value>, attribute: &Attribute, value: Option<Value>) {
    let name = attribute.name;
    match value.filter(|v| attribute.multi_valued && !v.is_null()) {
        Some(value) => {
            let gone = values(value);
            list(resource, name).retain(|item| !gone.iter().any(|g| same(item, g)));
        }
        None => {
            resource.remove(name);
        }
    }
}

/// Set or remove the sub-attribute `sub` of `item`, an object.
fn set_sub(item: &mut Value, sub: &Attribute, op: Op, value: Option<Value>) {
    let Some(object) = item.as_object_mut() else {
        return;
    };
    match (op, value) {
        (Op::Remove, _) | (_, None | Some(Value::Null)) => {
            object.remove(sub.name);
        }
        (_, Some(value)) => {
            object.insert(sub.name.to_string(), value);
        }
    }
}

/// `item` with the members of `new` set on it.
fn merge(item: &mut Value, new: &Map<String, Value>) {
    if let Some(object) = item.as_object_mut() {
        for (key, value) in new {
            match value {
                Value::Null => object.remove(key),
                value => object.insert(key.clone(), value.clone()),
            };
        }
    }
}

/// Only the value at `keep` of `items` stays primary.
fn keep_one_primary(items: &mut [Value], keep: usize) {
    for (i, item) in items.iter_mut().enumerate() {
        if i != keep
            && let Some(object) = item.as_object_mut()
            && object.get("primary") == Some(&Value::Bool(true))
        {
            object.insert("primary".to_string(), Value::Bool(false));
        }
    }
}

/// Drop the attribute `name` of `resource` when it is empty.
fn tidy(resource: &mut Map<String, Value>, name: &str) {
    if let Some(Value::Array(items)) = resource.get_mut(name) {
        items.retain(|item| !matches!(item, Value::Object(o) if o.is_empty()));
    }
    let empty = match resource.get(name) {
        Some(Value::Array(items)) => items.is_empty(),
        Some(Value::Object(object)) => object.is_empty(),
        Some(Value::Null) => true,
        _ => false,
    };
    if empty {
        resource.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::control::scim_schema::{GROUPS, USERS};
    use serde_json::json;

    fn patch(
        resource_type: &ResourceType,
        resource: Value,
        ops: Value,
    ) -> Result<Value, PatchError> {
        let ops: Vec<ScimPatchOperation> = serde_json::from_value(ops).unwrap();
        let Value::Object(mut object) = resource else {
            panic!("not an object");
        };
        apply(&mut object, resource_type, &ops)?;
        Ok(Value::Object(object))
    }

    fn user() -> Value {
        json!({
            "id": "1", "userName": "alice", "active": true,
            "name": { "givenName": "Alice", "familyName": "Doe" },
            "emails": [{ "value": "alice@x", "type": "work", "primary": true }],
        })
    }

    #[test]
    fn applies_entra_operations() {
        let patched = patch(
            &USERS,
            user(),
            json!([
                { "op": "Replace", "path": "emails[type eq \"work\"].value", "value": "alice2@x" },
                { "op": "Replace", "path": "name.familyName", "value": "Smith" },
                { "op": "Replace", "path": "userName", "value": "alice.smith" },
                { "op": "Add", "path": "displayName", "value": "Alice Smith" },
                { "op": "Replace", "path": "active", "value": "False" },
            ]),
        )
        .unwrap();
        assert_eq!(
            patched["emails"],
            json!([{ "value": "alice2@x", "type": "work", "primary": true }])
        );
        assert_eq!(
            patched["name"],
            json!({ "givenName": "Alice", "familyName": "Smith" })
        );
        assert_eq!(patched["userName"], "alice.smith");
        assert_eq!(patched["displayName"], "Alice Smith");
        assert_eq!(patched["active"], false);
    }

    #[test]
    fn a_filter_that_matches_nothing_adds_a_value() {
        let patched = patch(
            &USERS,
            user(),
            json!([
                { "op": "replace", "path": "emails[type eq \"home\"].value", "value": "a@home" },
            ]),
        )
        .unwrap();
        assert_eq!(
            patched["emails"][1],
            json!({ "type": "home", "value": "a@home" })
        );
        for path in [
            "emails[value co \"z\"].value",
            "emails[value eq \"z@x\"].type",
        ] {
            let ops = json!([{ "op": "replace", "path": path, "value": "z" }]);
            let err = patch(&USERS, user(), ops).unwrap_err();
            assert_eq!(err.scim_type, "noTarget", "{path}");
        }
        // A member is never made from a filter.
        let group = json!({ "id": "g", "displayName": "G" });
        let ops = json!([
            { "op": "replace", "path": "members[value eq \"u1\"].display", "value": "Bob" },
        ]);
        let err = patch(&GROUPS, group, ops).unwrap_err();
        assert_eq!(err.scim_type, "noTarget");
    }

    #[test]
    fn replaces_without_a_path_like_okta() {
        let patched = patch(
            &USERS,
            user(),
            json!([
                { "op": "replace", "value": { "active": false, "Name": { "givenName": "Al" } } },
            ]),
        )
        .unwrap();
        assert_eq!(patched["active"], false);
        assert_eq!(
            patched["name"],
            json!({ "givenName": "Al", "familyName": "Doe" })
        );
        let patched = patch(
            &USERS,
            user(),
            json!([
                { "op": "replace", "value": {
                    "emails[type eq \"work\"].value": "new@x",
                    "name.familyName": "Smith",
                } },
            ]),
        )
        .unwrap();
        assert_eq!(patched["emails"][0]["value"], "new@x");
        assert_eq!(patched["name"]["familyName"], "Smith");
    }

    #[test]
    fn adds_and_removes_members() {
        let group = json!({ "id": "g", "displayName": "G", "members": [{ "value": "u1" }] });
        let patched = patch(&GROUPS, group, json!([
            { "op": "Add", "path": "members", "value": [{ "$ref": null, "value": "u2" }, { "value": "u1" }] },
            { "op": "Remove", "path": "members", "value": [{ "value": "u1" }] },
        ]))
        .unwrap();
        assert_eq!(patched["members"], json!([{ "$ref": null, "value": "u2" }]));
        let patched = patch(
            &GROUPS,
            patched,
            json!([
                { "op": "remove", "path": "members[value eq \"u2\"]" },
            ]),
        )
        .unwrap();
        assert!(patched.get("members").is_none());
        let patched = patch(&GROUPS, json!({ "displayName": "G" }), json!([
            { "op": "replace", "path": "displayName", "value": "H" },
            { "op": "replace", "path": "externalId", "value": "e" },
            { "op": "replace", "path": "members", "value": [{ "value": "u3", "display": "U" }] },
        ]))
        .unwrap();
        assert_eq!(
            patched,
            json!({ "displayName": "H", "externalId": "e",
            "members": [{ "value": "u3", "display": "U" }] })
        );
    }

    #[test]
    fn leaves_unknown_and_read_only_attributes_alone() {
        let patched = patch(&USERS, user(), json!([
            { "op": "add", "path": "title", "value": "Engineer" },
            { "op": "replace", "path": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User:department", "value": "d" },
            { "op": "replace", "path": "id", "value": "2" },
            { "op": "add", "path": "roles", "value": [{ "value": "admin" }] },
        ]))
        .unwrap();
        assert_eq!(patched, user());
    }

    #[test]
    fn refuses_malformed_operations() {
        for (ops, scim_type) in [
            (json!([{ "op": "remove" }]), "noTarget"),
            (json!([{ "op": "move", "path": "active" }]), "invalidSyntax"),
            (
                json!([{ "op": "add", "path": "emails[type eq" , "value": "x" }]),
                "invalidPath",
            ),
            (
                json!([{ "op": "add", "path": "userName[value eq \"a\"]", "value": "x" }]),
                "invalidPath",
            ),
            (json!([{ "op": "replace", "value": "x" }]), "invalidValue"),
            // Byte 4 is inside a character.
            (
                json!([{ "op": "add", "path": "ab\u{65e5}[", "value": "x" }]),
                "invalidPath",
            ),
        ] {
            assert_eq!(
                patch(&USERS, user(), ops.clone()).unwrap_err().scim_type,
                scim_type,
                "{ops}"
            );
        }
    }
}
