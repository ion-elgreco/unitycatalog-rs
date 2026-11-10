//! SCIM PATCH (RFC 7644, section 3.5.2) on a resource as JSON: `add`,
//! `replace` and `remove`, in any case, with the paths `attr`, `attr.sub`,
//! `attr[filter]` and `attr[filter].sub`. Operations on attributes that
//! the schema does not have, or that are read-only, change nothing.
//!
//! As Entra ID needs: a `remove` of a list with a `value` removes only
//! those values, and an `add` or `replace` of `attr[filter].sub` that
//! matches nothing sets a value of the filter's `eq` terms, such as
//! `emails[type eq "work"].value` ([`value_of_terms`]). A filter on
//! `value`, such as `members[value eq "<id>"].display`, names an existing
//! value only: one that matches nothing is `noTarget` (RFC 7644, section
//! 3.5.2.3). A value of a list that loses its `value`, such as an email
//! whose address is removed, leaves the list.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use uc_api::ScimPatchOperation;

use super::scim_filter::{self, Filter};
use super::scim_schema::{self, Attribute, ResourceType, Type};

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

/// Parse `path`; `None` when it names an attribute that
/// [`ResourceType::writable`] leaves out, such as one of an extension
/// schema.
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
    let Some(attribute) = resource_type.writable(name) else {
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
    let value = value.map(|v| scim_schema::read_value(v, target.sub.unwrap_or(attribute)));
    if op != Op::Remove && value.is_none() {
        return Err(error(
            "invalidValue",
            format!("The operation on {name} needs a value"),
        ));
    }
    match (&target.filter, target.sub) {
        (None, None) => match op {
            Op::Remove => remove_whole(resource, attribute, value),
            Op::Add => add_whole(resource, attribute, value.unwrap_or_default()),
            Op::Replace => set_whole(resource, attribute, value.unwrap_or_default()),
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
            if clears_value(sub, op, value.as_ref()) {
                items.clear();
            } else {
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
        }
        (Some(filter), sub) => {
            let items = list(resource, name);
            let matching: Vec<usize> = (0..items.len())
                .filter(|&i| scim_filter::matches(filter, &items[i]))
                .collect();
            match (op, sub) {
                (Op::Remove, None) => remove_at(items, &matching),
                (_, Some(sub)) if clears_value(sub, op, value.as_ref()) => {
                    remove_at(items, &matching);
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
                    if let Some(&last) = matching.last()
                        && new.get("primary") == Some(&Value::Bool(true))
                    {
                        keep_one_primary(items, last);
                    }
                }
                (_, Some(sub)) => {
                    let value = value.unwrap_or_default();
                    let chosen = match matching.is_empty() {
                        true => vec![value_of_terms(items, attribute, filter)?],
                        false => matching,
                    };
                    for &i in &chosen {
                        set_sub(&mut items[i], sub, op, Some(value.clone()));
                    }
                    if let Some(&last) = chosen.last()
                        && is_primary(&items[last])
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

/// Whether the operation takes the `value` of a value of a list away: a
/// `remove` of the sub-attribute `value`, or an `add` or `replace` of it
/// with nothing or blank text. The value then leaves the list.
fn clears_value(sub: &Attribute, op: Op, value: Option<&Value>) -> bool {
    sub.name == "value"
        && match (op, value) {
            (Op::Remove, _) | (_, None | Some(Value::Null)) => true,
            (_, Some(Value::String(text))) => text.trim().is_empty(),
            _ => false,
        }
}

/// `items` without those at `gone`, indexes in order.
fn remove_at(items: &mut Vec<Value>, gone: &[usize]) {
    let mut gone = gone.iter().copied().peekable();
    let mut i = 0;
    items.retain(|_| {
        let keep = gone.next_if_eq(&i).is_none();
        i += 1;
        keep
    });
}

/// The index of the value of the list `items` of `attribute` that an `add`
/// or `replace` of `attr[filter].sub` sets when no value matches `filter`,
/// a filter of `eq` terms. A value that has none of the terms' attributes
/// (the primary one, else the first) takes the terms: for
/// `emails[type eq "work"].value`, an email without a type is the work
/// email from then on. Without such a value, a new value has the terms.
fn value_of_terms(
    items: &mut Vec<Value>,
    attribute: &Attribute,
    filter: &Filter,
) -> Result<usize, PatchError> {
    let terms: Map<String, Value> = (filter.equalities())
        .filter(|terms| {
            terms
                .iter()
                .all(|(attr, _)| !attr.eq_ignore_ascii_case("value"))
        })
        .ok_or_else(|| {
            error(
                "noTarget",
                format!("No value of {} matches", attribute.name),
            )
        })?
        .into_iter()
        .map(|(attr, value)| match attribute.sub(&attr) {
            Some(sub) => (sub.name.to_string(), scim_schema::read_value(value, sub)),
            None => (attr, value),
        })
        .collect();
    let open = |item: &Value| {
        item.as_object().is_some_and(|object| {
            terms
                .keys()
                .all(|attr| scim_filter::member(object, attr).is_none_or(Value::is_null))
        })
    };
    let taken = (items.iter().position(|item| open(item) && is_primary(item)))
        .or_else(|| items.iter().position(open));
    Ok(match taken {
        Some(at) => {
            merge(&mut items[at], &terms);
            at
        }
        None => {
            items.push(Value::Object(terms));
            items.len() - 1
        }
    })
}

/// The list `name` of `resource`, made when missing.
fn list<'a>(resource: &'a mut Map<String, Value>, name: &str) -> &'a mut Vec<Value> {
    let value = resource
        .entry(name)
        .or_insert_with(|| Value::Array(Vec::new()));
    if !value.is_array() {
        *value = Value::Array(values(std::mem::take(value)));
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

/// What makes two values of a list the same value: complex values their
/// `value`, in any case as filters compare it, others all of them.
fn identity(item: &Value) -> (bool, String) {
    match item.get("value") {
        Some(value) => (true, value.to_string().to_lowercase()),
        None => (false, item.to_string()),
    }
}

fn add_whole(resource: &mut Map<String, Value>, attribute: &Attribute, value: Value) {
    let name = attribute.name;
    if attribute.multi_valued {
        let items = list(resource, name);
        let mut places: HashMap<(bool, String), usize> = HashMap::with_capacity(items.len());
        for (at, item) in items.iter().enumerate() {
            places.entry(identity(item)).or_insert(at);
        }
        let mut primary = None;
        for new in values(value) {
            let is_primary = is_primary(&new);
            let at = match places.get(&identity(&new)) {
                Some(&at) => {
                    if let Value::Object(fields) = &new {
                        merge(&mut items[at], fields);
                    }
                    at
                }
                None => {
                    places.insert(identity(&new), items.len());
                    items.push(new);
                    items.len() - 1
                }
            };
            if is_primary {
                primary = Some(at);
            }
        }
        if let Some(at) = primary {
            keep_one_primary(items, at);
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
            let gone: HashSet<(bool, String)> = values(value).iter().map(identity).collect();
            list(resource, name).retain(|item| !gone.contains(&identity(item)));
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

/// Whether `item`, a value of a list, is the primary one.
fn is_primary(item: &Value) -> bool {
    item.get("primary") == Some(&Value::Bool(true))
}

/// Only the value at `keep` of `items` stays primary.
fn keep_one_primary(items: &mut [Value], keep: usize) {
    for (i, item) in items.iter_mut().enumerate() {
        if i != keep && is_primary(item) {
            item["primary"] = Value::Bool(false);
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
    fn an_email_without_a_type_takes_the_type_of_the_filter() {
        // As `uc user create`, `[[auth.users]]` and the first sign-in make
        // a user.
        let user = json!({
            "id": "1", "userName": "a@corp.com",
            "emails": [{ "value": "a@corp.com", "primary": true }],
        });
        let patched = patch(
            &USERS,
            user.clone(),
            json!([
                { "op": "Replace", "path": "emails[Type eq \"work\"].value", "value": "a.new@corp.com" },
                { "op": "Add", "path": "emails[type eq \"home\"].value", "value": "a@home.org" },
            ]),
        )
        .unwrap();
        assert_eq!(
            patched["emails"],
            json!([
                { "value": "a.new@corp.com", "primary": true, "type": "work" },
                { "type": "home", "value": "a@home.org" },
            ])
        );
        // So does a value that an operation on `attr[filter]` makes primary.
        let two = json!({ "userName": "a", "emails": [
            { "value": "a@x", "type": "home", "primary": true },
            { "value": "b@x", "type": "work" },
        ] });
        for op in ["add", "replace"] {
            let patched = patch(
                &USERS,
                two.clone(),
                json!([{ "op": op, "path": "emails[type eq \"work\"]",
                    "value": { "value": "b@x", "type": "work", "primary": true } }]),
            )
            .unwrap();
            assert_eq!(
                patched["emails"],
                json!([
                    { "value": "a@x", "type": "home", "primary": false },
                    { "value": "b@x", "type": "work", "primary": true },
                ])
            );
        }
        // A new primary value takes the place of the old one.
        let patched = patch(
            &USERS,
            json!({ "userName": "a", "emails": [{ "value": "a@x", "type": "home", "primary": true }] }),
            json!([
                { "op": "add", "path": "emails[type eq \"work\" and primary eq true].value", "value": "b@x" },
            ]),
        )
        .unwrap();
        assert_eq!(
            patched["emails"],
            json!([
                { "value": "a@x", "type": "home", "primary": false },
                { "type": "work", "primary": true, "value": "b@x" },
            ])
        );
    }

    #[test]
    fn a_value_without_its_value_leaves_the_list() {
        let user = json!({
            "id": "1", "userName": "alice",
            "emails": [
                { "value": "alice@x", "type": "work", "primary": true },
                { "value": "alice@home", "type": "home" },
            ],
            "photos": [{ "value": "https://x/p.png", "type": "photo" }],
        });
        // As Entra ID clears a `mail`.
        for op in [
            json!({ "op": "Remove", "path": "emails[type eq \"work\"].value" }),
            json!({ "op": "Replace", "path": "emails[type eq \"work\"].value", "value": "" }),
        ] {
            let patched = patch(&USERS, user.clone(), json!([op])).unwrap();
            assert_eq!(
                patched["emails"],
                json!([{ "value": "alice@home", "type": "home" }]),
                "{op}"
            );
        }
        // A `null` value, which a request can give, clears too.
        let null = ScimPatchOperation {
            op: "replace".into(),
            path: Some("emails[type eq \"work\"].value".into()),
            value: Some(Value::Null),
        };
        let Value::Object(mut resource) = user.clone() else {
            unreachable!();
        };
        apply(&mut resource, &USERS, &[null]).unwrap();
        assert_eq!(
            resource["emails"],
            json!([{ "value": "alice@home", "type": "home" }])
        );
        let patched = patch(
            &USERS,
            user.clone(),
            json!([
                { "op": "remove", "path": "photos[type eq \"photo\"].value" },
                { "op": "remove", "path": "emails.value" },
                // Nothing to clear.
                { "op": "replace", "path": "emails[type eq \"other\"].value", "value": "" },
            ]),
        )
        .unwrap();
        assert!(patched.get("photos").is_none() && patched.get("emails").is_none());
        // Another sub-attribute goes alone.
        let patched = patch(
            &USERS,
            user,
            json!([{ "op": "remove", "path": "emails[type eq \"home\"].type" }]),
        )
        .unwrap();
        assert_eq!(patched["emails"][1], json!({ "value": "alice@home" }));
    }

    #[test]
    fn adds_and_removes_many_values_by_their_value() {
        let member = |i: usize| json!({ "value": format!("u{i}") });
        let members = |range: std::ops::Range<usize>| Value::Array(range.map(member).collect());
        let group = json!({ "id": "g", "displayName": "G", "members": members(0..20_000) });
        let patched = patch(
            &GROUPS,
            group,
            json!([
                { "op": "add", "path": "members", "value": members(10_000..30_000) },
                { "op": "remove", "path": "members", "value": members(0..29_998) },
            ]),
        )
        .unwrap();
        assert_eq!(patched["members"], members(29_998..30_000));
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
            (
                json!([{ "op": "replace", "path": "name.familyName" }]),
                "invalidValue",
            ),
            (
                json!([{ "op": "add", "path": "emails.type" }]),
                "invalidValue",
            ),
            (
                json!([{ "op": "add", "path": "emails[type eq \"work\"].value" }]),
                "invalidValue",
            ),
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
