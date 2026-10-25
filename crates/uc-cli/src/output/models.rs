//! Upstream's client models: the fields, in order, that its CLI prints. As
//! the generated Java client, a field the server leaves out prints as
//! `null`, or as `[]` and `{}` for lists and maps; unknown fields are
//! dropped. A map prints in key order.

use serde_json::{Map, Value};

use super::sorted;

pub enum Kind {
    Plain,
    Map,
    List(Option<&'static Model>),
    Object(&'static Model),
}

pub struct Model(&'static [(&'static str, Kind)]);

use Kind::{Map as M, Object as O, Plain as P};

const fn list(model: &'static Model) -> Kind {
    Kind::List(Some(model))
}

const LIST: Kind = Kind::List(None);

pub static CATALOG: Model = Model(&[
    ("name", P),
    ("comment", P),
    ("properties", M),
    ("owner", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("id", P),
    ("storage_root", P),
    ("storage_location", P),
]);

pub static SCHEMA: Model = Model(&[
    ("name", P),
    ("catalog_name", P),
    ("comment", P),
    ("properties", M),
    ("full_name", P),
    ("owner", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("schema_id", P),
    ("storage_root", P),
    ("storage_location", P),
]);

pub static COLUMN: Model = Model(&[
    ("name", P),
    ("type_text", P),
    ("type_json", P),
    ("type_name", P),
    ("type_precision", P),
    ("type_scale", P),
    ("type_interval_type", P),
    ("position", P),
    ("comment", P),
    ("nullable", P),
    ("partition_index", P),
]);

static TABLE_DEPENDENCY: Model = Model(&[("table_full_name", P)]);
static FUNCTION_DEPENDENCY: Model = Model(&[("function_full_name", P)]);
static DEPENDENCY: Model = Model(&[
    ("table", O(&TABLE_DEPENDENCY)),
    ("function", O(&FUNCTION_DEPENDENCY)),
]);
static DEPENDENCY_LIST: Model = Model(&[("dependencies", list(&DEPENDENCY))]);

pub static TABLE: Model = Model(&[
    ("name", P),
    ("catalog_name", P),
    ("schema_name", P),
    ("table_type", P),
    ("data_source_format", P),
    ("columns", list(&COLUMN)),
    ("storage_location", P),
    ("comment", P),
    ("properties", M),
    ("owner", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("table_id", P),
    ("view_definition", P),
    ("view_dependencies", O(&DEPENDENCY_LIST)),
]);

pub static VOLUME: Model = Model(&[
    ("catalog_name", P),
    ("schema_name", P),
    ("name", P),
    ("comment", P),
    ("owner", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("volume_id", P),
    ("volume_type", P),
    ("storage_location", P),
    ("full_name", P),
]);

static PARAMETER: Model = Model(&[
    ("name", P),
    ("type_text", P),
    ("type_json", P),
    ("type_name", P),
    ("type_precision", P),
    ("type_scale", P),
    ("type_interval_type", P),
    ("position", P),
    ("parameter_mode", P),
    ("parameter_type", P),
    ("parameter_default", P),
    ("comment", P),
]);
static PARAMETERS: Model = Model(&[("parameters", list(&PARAMETER))]);

pub static FUNCTION: Model = Model(&[
    ("name", P),
    ("catalog_name", P),
    ("schema_name", P),
    ("input_params", O(&PARAMETERS)),
    ("data_type", P),
    ("full_data_type", P),
    ("return_params", O(&PARAMETERS)),
    ("routine_body", P),
    ("routine_definition", P),
    ("routine_dependencies", O(&DEPENDENCY_LIST)),
    ("parameter_style", P),
    ("is_deterministic", P),
    ("sql_data_access", P),
    ("is_null_call", P),
    ("security_type", P),
    ("specific_name", P),
    ("comment", P),
    ("properties", P),
    ("full_name", P),
    ("owner", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("function_id", P),
    ("external_language", P),
]);

pub static REGISTERED_MODEL: Model = Model(&[
    ("name", P),
    ("catalog_name", P),
    ("schema_name", P),
    ("storage_location", P),
    ("full_name", P),
    ("comment", P),
    ("owner", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("id", P),
]);

pub static MODEL_VERSION: Model = Model(&[
    ("model_name", P),
    ("catalog_name", P),
    ("schema_name", P),
    ("version", P),
    ("source", P),
    ("run_id", P),
    ("status", P),
    ("storage_location", P),
    ("comment", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("id", P),
]);

static AWS_IAM_ROLE: Model = Model(&[
    ("role_arn", P),
    ("unity_catalog_iam_arn", P),
    ("external_id", P),
]);

static AZURE_SERVICE_PRINCIPAL: Model = Model(&[("directory_id", P), ("application_id", P)]);

static AZURE_MANAGED_IDENTITY: Model = Model(&[("managed_identity_id", P)]);

pub static CREDENTIAL: Model = Model(&[
    ("name", P),
    ("aws_iam_role", O(&AWS_IAM_ROLE)),
    ("comment", P),
    ("owner", P),
    ("full_name", P),
    ("id", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
    ("purpose", P),
    ("azure_service_principal", O(&AZURE_SERVICE_PRINCIPAL)),
    ("azure_managed_identity", O(&AZURE_MANAGED_IDENTITY)),
]);

pub static EXTERNAL_LOCATION: Model = Model(&[
    ("name", P),
    ("id", P),
    ("url", P),
    ("credential_name", P),
    ("comment", P),
    ("owner", P),
    ("credential_id", P),
    ("created_at", P),
    ("created_by", P),
    ("updated_at", P),
    ("updated_by", P),
]);

pub static PRIVILEGE_ASSIGNMENT: Model = Model(&[("principal", P), ("privileges", LIST)]);

pub static METASTORE_SUMMARY: Model = Model(&[("metastore_id", P)]);

/// The control API's `User`, which upstream's CLI prints for SCIM users.
pub static USER: Model = Model(&[
    ("id", P),
    ("name", P),
    ("email", P),
    ("external_id", P),
    ("state", P),
    ("picture_url", P),
    ("created_at", P),
    ("updated_at", P),
]);

/// `value` with the fields of `model`, in its order.
pub fn shape(model: &Model, value: &Value) -> Value {
    let mut out = Map::new();
    for (name, kind) in model.0 {
        let field = value.get(*name);
        let shaped = match (kind, field) {
            (Kind::Map, None) => Value::Object(Map::new()),
            (Kind::List(_), None) => Value::Array(Vec::new()),
            (_, None | Some(Value::Null)) => Value::Null,
            (Kind::List(Some(item)), Some(Value::Array(items))) => {
                Value::Array(items.iter().map(|i| shape(item, i)).collect())
            }
            (Kind::Object(inner), Some(v @ Value::Object(_))) => shape(inner, v),
            (Kind::Map, Some(v @ Value::Object(_))) => sorted(v.clone()),
            (_, Some(v)) => v.clone(),
        };
        out.insert(name.to_string(), shaped);
    }
    Value::Object(out)
}

/// `value`, an object of `model`'s type, with its empty maps and lists,
/// which the API's models leave out and the server sends; its maps in key
/// order. Its other fields stay as they are.
pub fn with_empty(model: &Model, value: Value) -> Value {
    let Value::Object(mut fields) = value else {
        return value;
    };
    let mut out = Map::new();
    for (name, kind) in model.0 {
        let field = match (fields.shift_remove(*name), kind) {
            (Some(v @ Value::Object(_)), Kind::Map) => sorted(v),
            (Some(v), _) => v,
            (None, Kind::Map) => Value::Object(Map::new()),
            (None, Kind::List(_)) => Value::Array(Vec::new()),
            (None, _) => continue,
        };
        out.insert(name.to_string(), field);
    }
    out.extend(fields);
    Value::Object(out)
}

/// Each element of the list `value[key]` shaped as `model`; a missing list
/// is empty.
pub fn shape_list(model: &Model, value: &Value, key: &str) -> Value {
    let items = value.get(key).and_then(Value::as_array);
    Value::Array(
        items
            .into_iter()
            .flatten()
            .map(|item| shape(model, item))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shapes_in_upstream_order_with_defaults() {
        let shaped = shape(
            &TABLE,
            &json!({ "table_id": "t", "name": "n", "extra": 1, "columns": [{ "position": 0, "name": "a" }] }),
        );
        let keys: Vec<&String> = shaped.as_object().unwrap().keys().collect();
        assert_eq!(keys[0], "name");
        assert_eq!(keys.len(), 17);
        assert_eq!(shaped["properties"], json!({}));
        assert_eq!(shaped["view_dependencies"], Value::Null);
        assert_eq!(
            shaped["columns"][0]
                .as_object()
                .unwrap()
                .keys()
                .next()
                .unwrap(),
            "name"
        );
        assert_eq!(shaped["columns"][0]["nullable"], Value::Null);
        assert!(shaped.get("extra").is_none());
        // Maps print in key order.
        let shaped = shape(&CATALOG, &json!({ "properties": { "b": "1", "a": "2" } }));
        let keys: Vec<&String> = shaped["properties"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["a", "b"]);
    }

    #[test]
    fn empty_maps_and_lists_come_back_in_place() {
        let table = with_empty(
            &TABLE,
            json!({ "x": 0, "name": "t", "table_type": "EXTERNAL", "owner": "o", "y": 1 }),
        );
        assert_eq!(
            table,
            json!({ "name": "t", "table_type": "EXTERNAL", "columns": [], "properties": {},
                "owner": "o", "x": 0, "y": 1 })
        );
        let keys: Vec<&String> = table.as_object().unwrap().keys().collect();
        let expected = [
            "name",
            "table_type",
            "columns",
            "properties",
            "owner",
            "x",
            "y",
        ];
        assert_eq!(keys, expected);
        let catalog = with_empty(&CATALOG, json!({ "properties": { "b": "1", "a": "2" } }));
        let keys: Vec<&String> = catalog["properties"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["a", "b"]);
    }
}
