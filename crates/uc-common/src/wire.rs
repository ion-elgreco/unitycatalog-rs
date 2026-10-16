//! The names of API values on the wire: the strings of their serde
//! renames, such as `TIMESTAMP_NTZ` for `ColumnTypeName::TimestampNtz`.
//! `Debug` gives the Rust names (`TimestampNtz`), which clients must not
//! show.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// The name on the wire of an API value, such as `TIMESTAMP_NTZ`.
pub fn wire_name<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(name)) => name,
        _ => String::new(),
    }
}

/// The API value whose name on the wire is `name`, such as
/// `ColumnTypeName::TimestampNtz` for `TIMESTAMP_NTZ`.
pub fn from_wire<T: DeserializeOwned>(name: &str) -> Option<T> {
    serde_json::from_value(Value::String(name.to_string())).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    enum Status {
        PendingRegistration,
    }

    #[test]
    fn names_are_the_serde_renames() {
        assert_eq!(
            wire_name(&Status::PendingRegistration),
            "PENDING_REGISTRATION"
        );
        assert_eq!(
            from_wire::<Status>("PENDING_REGISTRATION"),
            Some(Status::PendingRegistration)
        );
        assert_eq!(from_wire::<Status>("PendingRegistration"), None);
    }
}
