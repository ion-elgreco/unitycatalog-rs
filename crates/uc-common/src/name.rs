//! UC full-name construction helpers.

/// The full name of a schema: `<catalog>.<schema>`.
pub fn schema_full_name(catalog: &str, schema: &str) -> String {
    format!("{catalog}.{schema}")
}

/// The full name of an object in a schema, such as a table, volume,
/// function, registered model or Iceberg view: `<catalog>.<schema>.<name>`.
pub fn three_part_full_name(catalog: &str, schema: &str, name: &str) -> String {
    format!("{catalog}.{schema}.{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_part() {
        assert_eq!(schema_full_name("main", "default"), "main.default");
    }

    #[test]
    fn three_part() {
        assert_eq!(three_part_full_name("c", "s", "t"), "c.s.t");
    }
}
