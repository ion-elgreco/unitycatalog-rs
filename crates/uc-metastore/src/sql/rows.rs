//! The catalog's tables, column by column, and rows as serde values: the
//! domain modules read rows straight into their serde structs
//! ([`TableDef::decode`]), and write them from those structs through JSON
//! objects. JSON documents (properties, columns, grants, ...) are stored as
//! text. The JSON objects are also the rows of a backup
//! ([`crate::backup::Snapshot`]).

use serde::Serialize;
use serde::de::value::MapAccessDeserializer;
use serde::de::{self, DeserializeOwned, DeserializeSeed, IntoDeserializer, MapAccess};
use serde_json::{Map, Value as JsonValue};

use uc_common::{Error, Result};

use uc_storage::{Cell, Row, Value};

/// How a column holds its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Text,
    Int,
    Bool,
    /// A JSON document, stored as text.
    Json,
}

use Kind::{Bool, Int, Json, Text};

/// A table: its name, primary key and columns.
#[derive(Debug)]
pub(crate) struct TableDef {
    pub(crate) name: &'static str,
    pub(crate) key: &'static [&'static str],
    pub(crate) columns: &'static [(&'static str, Kind)],
}

pub(crate) const CATALOG: TableDef = TableDef {
    name: "catalog",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("comment", Text),
        ("properties", Json),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
        ("storage_root", Text),
        ("storage_location", Text),
    ],
};

pub(crate) const SCHEMA: TableDef = TableDef {
    name: "schema",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("catalog_id", Text),
        ("comment", Text),
        ("properties", Json),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
        ("storage_root", Text),
        ("storage_location", Text),
    ],
};

pub(crate) const TBL: TableDef = TableDef {
    name: "tbl",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("schema_id", Text),
        ("table_type", Text),
        ("data_source_format", Text),
        ("columns", Json),
        ("storage_location", Text),
        ("comment", Text),
        ("properties", Json),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
        ("view_definition", Text),
        ("view_dependencies", Json),
        ("iceberg_metadata_location", Text),
        ("uniform_iceberg_converted_delta_version", Int),
        ("uniform_iceberg_converted_delta_timestamp", Int),
    ],
};

pub(crate) const STAGING_TABLE: TableDef = TableDef {
    name: "staging_table",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("schema_id", Text),
        ("staging_location", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("accessed_at", Int),
        ("stage_committed", Bool),
        ("stage_committed_at", Int),
        ("purge_state", Int),
        ("num_cleanup_retries", Int),
        ("last_cleanup_at", Int),
    ],
};

pub(crate) const DELTA_COMMIT: TableDef = TableDef {
    name: "delta_commit",
    key: &["table_id", "commit_version"],
    columns: &[
        ("table_id", Text),
        ("commit_version", Int),
        ("commit_filename", Text),
        ("commit_filesize", Int),
        ("commit_file_modification_timestamp", Int),
        ("commit_timestamp", Int),
        ("is_backfilled_latest_commit", Bool),
    ],
};

pub(crate) const VOLUME: TableDef = TableDef {
    name: "volume",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("schema_id", Text),
        ("volume_type", Text),
        ("comment", Text),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
        ("storage_location", Text),
    ],
};

pub(crate) const FUNC: TableDef = TableDef {
    name: "func",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("schema_id", Text),
        ("input_params", Json),
        ("data_type", Text),
        ("full_data_type", Text),
        ("return_params", Json),
        ("routine_body", Text),
        ("routine_definition", Text),
        ("routine_dependencies", Json),
        ("parameter_style", Text),
        ("is_deterministic", Bool),
        ("sql_data_access", Text),
        ("is_null_call", Bool),
        ("security_type", Text),
        ("specific_name", Text),
        ("comment", Text),
        ("properties", Text),
        ("external_language", Text),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
    ],
};

pub(crate) const REGISTERED_MODEL: TableDef = TableDef {
    name: "registered_model",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("schema_id", Text),
        ("comment", Text),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
        ("storage_location", Text),
        ("max_version_number", Int),
    ],
};

pub(crate) const MODEL_VERSION: TableDef = TableDef {
    name: "model_version",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("registered_model_id", Text),
        ("version", Int),
        ("source", Text),
        ("run_id", Text),
        ("status", Text),
        ("storage_location", Text),
        ("comment", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
    ],
};

pub(crate) const CREDENTIAL: TableDef = TableDef {
    name: "credential",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("comment", Text),
        ("owner", Text),
        ("purpose", Text),
        ("aws_role_arn", Text),
        ("aws_external_id", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
    ],
};

pub(crate) const EXTERNAL_LOCATION: TableDef = TableDef {
    name: "external_location",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("url", Text),
        ("credential_id", Text),
        ("comment", Text),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
    ],
};

pub(crate) const PERMISSION: TableDef = TableDef {
    name: "permission",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("securable_type", Text),
        ("resource_id", Text),
        ("principal", Text),
        ("privileges", Json),
    ],
};

/// One row; its `singleton` key column has a default.
pub(crate) const METASTORE: TableDef = TableDef {
    name: "metastore",
    key: &["metastore_id"],
    columns: &[("metastore_id", Text), ("owner", Text)],
};

pub(crate) const PAT: TableDef = TableDef {
    name: "pat",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("token_hash", Text),
        ("token_prefix", Text),
        ("owner", Text),
        ("created_at", Int),
        ("expires_at", Int),
    ],
};

pub(crate) const SCIM_USER: TableDef = TableDef {
    name: "scim_user",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("user_name", Text),
        ("display_name", Text),
        ("external_id", Text),
        ("emails", Json),
        ("active", Bool),
        ("picture_url", Text),
        ("created_at", Int),
        ("updated_at", Int),
    ],
};

pub(crate) const SCIM_GROUP: TableDef = TableDef {
    name: "scim_group",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("display_name", Text),
        ("members", Json),
    ],
};

pub(crate) const SHARE: TableDef = TableDef {
    name: "share",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("comment", Text),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
        ("objects", Json),
    ],
};

pub(crate) const RECIPIENT: TableDef = TableDef {
    name: "recipient",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("name", Text),
        ("authentication_type", Text),
        ("comment", Text),
        ("owner", Text),
        ("created_at", Int),
        ("created_by", Text),
        ("updated_at", Int),
        ("updated_by", Text),
        ("token_hash", Text),
        ("token_prefix", Text),
        ("shares", Json),
    ],
};

pub(crate) const STORAGE_CLEANUP_TASK: TableDef = TableDef {
    name: "storage_cleanup_task",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("resource_type", Text),
        ("name", Text),
        ("storage_location", Text),
        ("deleted_at", Int),
        ("lease_expires_at", Int),
        ("lease_token", Text),
        ("failure_count", Int),
        ("last_error", Text),
    ],
};

pub(crate) const AUDIT_EVENT: TableDef = TableDef {
    name: "audit_event",
    key: &["id_string"],
    columns: &[
        ("id_string", Text),
        ("principal", Text),
        ("method", Text),
        ("path", Text),
        ("status", Int),
        ("timestamp", Int),
    ],
};

/// Every table above.
#[cfg(test)]
pub(crate) const ALL: &[&TableDef] = &[
    &CATALOG,
    &SCHEMA,
    &TBL,
    &STAGING_TABLE,
    &DELTA_COMMIT,
    &VOLUME,
    &FUNC,
    &REGISTERED_MODEL,
    &MODEL_VERSION,
    &CREDENTIAL,
    &EXTERNAL_LOCATION,
    &PERMISSION,
    &METASTORE,
    &PAT,
    &SCIM_USER,
    &SCIM_GROUP,
    &SHARE,
    &RECIPIENT,
    &STORAGE_CLEANUP_TASK,
    &AUDIT_EVENT,
];

fn bad_row(table: &str, column: &str, why: impl std::fmt::Display) -> Error {
    Error::internal(format!("{table}.{column}: {why}"))
}

/// The columns of a row as the fields of a map, for [`TableDef::decode`].
struct RowFields<'a, 'r> {
    columns: std::slice::Iter<'static, (&'static str, Kind)>,
    cells: std::slice::IterMut<'a, Cell<'r>>,
    /// The column whose name was read last.
    next: Option<(&'static str, Kind, &'a mut Cell<'r>)>,
}

impl<'a, 'r: 'a> MapAccess<'a> for RowFields<'a, 'r> {
    type Error = serde_json::Error;

    fn next_key_seed<K: DeserializeSeed<'a>>(
        &mut self,
        seed: K,
    ) -> std::result::Result<Option<K::Value>, Self::Error> {
        for (&(name, kind), cell) in self.columns.by_ref().zip(self.cells.by_ref()) {
            if *cell != Cell::Value(Value::Null) {
                self.next = Some((name, kind, cell));
                return seed.deserialize(name.into_deserializer()).map(Some);
            }
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'a>>(
        &mut self,
        seed: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        let Some((name, kind, cell)) = self.next.take() else {
            return Err(de::Error::custom("a value without its column"));
        };
        let column = |e: serde_json::Error| de::Error::custom(format_args!("{name}: {e}"));
        if kind == Json {
            let cell: &'a Cell<'r> = cell;
            let text: &'a str = match cell {
                Cell::Text(text) => text,
                Cell::Value(Value::Text(text)) => text,
                Cell::Value(value) => {
                    return Err(de::Error::custom(format_args!(
                        "{name}: {value:?} is not JSON text"
                    )));
                }
            };
            let mut json = serde_json::Deserializer::from_str(text);
            let field = seed.deserialize(&mut json).map_err(column)?;
            json.end().map_err(column)?;
            return Ok(field);
        }
        let value = match std::mem::replace(cell, Cell::Value(Value::Null)) {
            Cell::Value(value) => value,
            Cell::Text(text) => Value::Text(text.to_owned()),
        };
        let field = match (kind, value) {
            (Text, Value::Text(s)) => JsonValue::String(s),
            (Int, Value::Int(n)) => JsonValue::from(n),
            (Bool, Value::Bool(b)) => JsonValue::Bool(b),
            (Bool, Value::Int(n)) => JsonValue::Bool(n != 0),
            (kind, value) => {
                return Err(de::Error::custom(format_args!(
                    "{name}: {value:?} is not {kind:?}"
                )));
            }
        };
        seed.deserialize(field).map_err(column)
    }
}

impl TableDef {
    /// `a, b, c`
    pub(crate) fn column_list(&self) -> String {
        let names: Vec<&str> = self.columns.iter().map(|(name, _)| *name).collect();
        names.join(", ")
    }

    /// `SELECT a, b, c FROM t`; add `WHERE …` and the rest.
    pub(crate) fn select(&self) -> String {
        format!("SELECT {} FROM {}", self.column_list(), self.name)
    }

    /// `INSERT INTO t (a, b, c) VALUES ($1, $2, $3)`, for [`Self::encode`].
    pub(crate) fn insert(&self) -> String {
        self.insert_rows(1)
    }

    /// `INSERT INTO t (a, b) VALUES ($1, $2), ($3, $4)`: `rows` rows of
    /// [`Self::encode`] values, one after the other.
    pub(crate) fn insert_rows(&self, rows: usize) -> String {
        let width = self.columns.len();
        let tuples: Vec<String> = (0..rows)
            .map(|row| format!("({})", super::placeholders(row * width + 1, width)))
            .collect();
        format!(
            "INSERT INTO {} ({}) VALUES {}",
            self.name,
            self.column_list(),
            tuples.join(", ")
        )
    }

    /// `UPDATE t SET b = $2, c = $3 WHERE a = $1`: every column by its key,
    /// for [`Self::encode`].
    pub(crate) fn update(&self) -> String {
        let mut set = Vec::new();
        let mut key = Vec::new();
        for (i, (name, _)) in self.columns.iter().enumerate() {
            let term = format!("{name} = ${}", i + 1);
            match self.key.contains(name) {
                true => key.push(term),
                false => set.push(term),
            }
        }
        format!(
            "UPDATE {} SET {} WHERE {}",
            self.name,
            set.join(", "),
            key.join(" AND ")
        )
    }

    /// A row of [`Self::select`] as a JSON object; null columns are left out.
    pub(crate) fn object(&self, row: Row) -> Result<Map<String, JsonValue>> {
        let values = row
            .into_values()
            .map_err(|e| Error::internal(format!("{}: {e}", self.name)))?;
        if values.len() != self.columns.len() {
            return Err(Error::internal(format!(
                "{}: {} columns, expected {}",
                self.name,
                values.len(),
                self.columns.len()
            )));
        }
        let mut object = Map::with_capacity(values.len());
        for ((name, kind), value) in self.columns.iter().zip(values) {
            let json = match (kind, value) {
                (_, Value::Null) => continue,
                (Text, Value::Text(s)) => JsonValue::String(s),
                (Int, Value::Int(n)) => JsonValue::from(n),
                (Bool, Value::Bool(b)) => JsonValue::Bool(b),
                (Bool, Value::Int(n)) => JsonValue::Bool(n != 0),
                (Json, Value::Text(s)) => {
                    serde_json::from_str(&s).map_err(|e| bad_row(self.name, name, e))?
                }
                (kind, value) => {
                    return Err(bad_row(
                        self.name,
                        name,
                        format!("{value:?} is not {kind:?}"),
                    ));
                }
            };
            object.insert((*name).to_string(), json);
        }
        Ok(object)
    }

    /// A row of [`Self::select`] as `R`; null columns are left out. A JSON
    /// column is read into its field's type, without a [`JsonValue`] tree.
    pub(crate) fn decode<R: DeserializeOwned>(&self, row: Row) -> Result<R> {
        let mut row = row;
        if row.len() != self.columns.len() {
            return Err(Error::internal(format!(
                "{}: {} columns, expected {}",
                self.name,
                row.len(),
                self.columns.len()
            )));
        }
        let mut cells = row
            .cells()
            .map_err(|e| Error::internal(format!("{}: {e}", self.name)))?;
        let fields = RowFields {
            columns: self.columns.iter(),
            cells: cells.iter_mut(),
            next: None,
        };
        R::deserialize(MapAccessDeserializer::new(fields))
            .map_err(|e| Error::internal(format!("{}: {e}", self.name)))
    }

    /// The values of a JSON object in column order, for [`Self::insert`]
    /// and [`Self::update`]. Absent and null fields are NULL; fields that
    /// are not columns are left out.
    pub(crate) fn values_of(&self, object: &Map<String, JsonValue>) -> Result<Vec<Value>> {
        self.columns
            .iter()
            .map(|(name, kind)| {
                let bad = |what: &str| bad_row(self.name, name, format!("expected {what}"));
                Ok(match (kind, object.get(*name)) {
                    (_, None | Some(JsonValue::Null)) => Value::Null,
                    (Text, Some(JsonValue::String(s))) => Value::Text(s.clone()),
                    (Text, Some(_)) => return Err(bad("text")),
                    (Int, Some(v)) => Value::Int(v.as_i64().ok_or_else(|| bad("an integer"))?),
                    (Bool, Some(JsonValue::Bool(b))) => Value::Bool(*b),
                    (Bool, Some(v)) => match v.as_i64() {
                        Some(n @ (0 | 1)) => Value::Bool(n == 1),
                        _ => return Err(bad("a boolean")),
                    },
                    (Json, Some(v)) => Value::Text(v.to_string()),
                })
            })
            .collect()
    }

    /// `row`'s values in column order.
    pub(crate) fn encode<R: Serialize>(&self, row: &R) -> Result<Vec<Value>> {
        match serde_json::to_value(row) {
            Ok(JsonValue::Object(object)) => self.values_of(&object),
            Ok(_) => Err(Error::internal(format!("{}: not an object", self.name))),
            Err(e) => Err(Error::internal(format!("{}: {e}", self.name))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statements_number_columns_in_order() {
        assert_eq!(
            SCIM_GROUP.insert(),
            "INSERT INTO scim_group (id_string, display_name, members) VALUES ($1, $2, $3)"
        );
        assert_eq!(
            SCIM_GROUP.update(),
            "UPDATE scim_group SET display_name = $2, members = $3 WHERE id_string = $1"
        );
        assert_eq!(
            DELTA_COMMIT.update(),
            "UPDATE delta_commit SET commit_filename = $3, commit_filesize = $4, \
             commit_file_modification_timestamp = $5, commit_timestamp = $6, \
             is_backfilled_latest_commit = $7 WHERE table_id = $1 AND commit_version = $2"
        );
    }

    #[test]
    fn rows_round_trip_through_objects() {
        let object = serde_json::json!({
            "id_string": "g", "display_name": "admins",
            "members": [{ "value": "u-1" }], "extra": 1,
        });
        let values = SCIM_GROUP.values_of(object.as_object().unwrap()).unwrap();
        assert_eq!(
            values,
            [
                Value::Text("g".into()),
                Value::Text("admins".into()),
                Value::Text(r#"[{"value":"u-1"}]"#.into()),
            ]
        );
        let back = SCIM_GROUP.object(Row::from(values)).unwrap();
        assert_eq!(back["members"], serde_json::json!([{ "value": "u-1" }]));
        assert!(!back.contains_key("extra"));

        let task = SCIM_USER
            .values_of(serde_json::json!({ "active": 1 }).as_object().unwrap())
            .unwrap();
        assert_eq!(task[5], Value::Bool(true), "0 and 1 are booleans");
        assert!(
            SCIM_USER
                .values_of(serde_json::json!({ "user_name": 5 }).as_object().unwrap())
                .is_err()
        );
    }

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct User {
        id_string: String,
        display_name: Option<String>,
        emails: Option<Vec<serde_json::Value>>,
        active: bool,
        created_at: i64,
        #[serde(default)]
        updated_at: Option<i64>,
    }

    #[test]
    fn rows_decode_into_their_types() {
        let row = |active: Value, created_at: Value| {
            Row::from(vec![
                Value::Text("u-1".into()),
                Value::Text("ann".into()),
                Value::Null,
                Value::Null,
                Value::Text(r#"[{"value":"ann@x.org"}]"#.into()),
                active,
                Value::Null,
                created_at,
                Value::Null,
            ])
        };
        let user: User = SCIM_USER.decode(row(Value::Int(1), Value::Int(7))).unwrap();
        assert_eq!(
            user,
            User {
                id_string: "u-1".into(),
                display_name: None,
                emails: Some(vec![serde_json::json!({ "value": "ann@x.org" })]),
                active: true,
                created_at: 7,
                updated_at: None,
            },
            "null columns are left out; Turso's integers are booleans"
        );
        let wrong = SCIM_USER.decode::<User>(row(Value::Bool(false), Value::Text("7".into())));
        assert!(wrong.unwrap_err().message().contains("created_at"));
    }
}
