//! Input validation, as upstream (the Java Unity Catalog) does it.

use std::sync::LazyLock;

use regex::Regex;

use crate::{Error, Result};

/// Longest valid securable name, in UTF-16 code units like Java's
/// `String.length()`.
pub const MAX_NAME_LENGTH: usize = 255;

/// Upstream's name pattern, anchored as Java's `Matcher.matches()` is.
static VALID_FORMAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[a-zA-Z0-9_@-]+$").expect("valid regex"));

/// A securable name is 1 to 255 characters of `[a-zA-Z0-9_@-]`, as upstream.
/// Upstream applies it to the name of the object being created or renamed,
/// never to parent names (those are looked up instead, so a bad parent name
/// is a 404).
pub fn validate_sql_object_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_argument("Name cannot be empty"));
    }
    if name.encode_utf16().count() > MAX_NAME_LENGTH {
        return Err(Error::invalid_argument(format!(
            "Name cannot be longer than {MAX_NAME_LENGTH} characters"
        )));
    }
    if !VALID_FORMAT.is_match(name) {
        return Err(Error::invalid_argument(
            "Name cannot contain a period, space, forward-slash, or control characters",
        ));
    }
    Ok(())
}

/// Reject empty or whitespace-only strings with an `INVALID_ARGUMENT`.
pub fn require_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::invalid_argument(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::empty("", "Name cannot be empty")]
    #[case::too_long(&"x".repeat(MAX_NAME_LENGTH + 1), "Name cannot be longer than 255 characters")]
    #[case::dot(
        "catalog.schema",
        "Name cannot contain a period, space, forward-slash, or control characters"
    )]
    #[case::slash(
        "a/b",
        "Name cannot contain a period, space, forward-slash, or control characters"
    )]
    #[case::space(
        "foo bar",
        "Name cannot contain a period, space, forward-slash, or control characters"
    )]
    #[case::quote(
        "a'b",
        "Name cannot contain a period, space, forward-slash, or control characters"
    )]
    #[case::backtick(
        "a`b",
        "Name cannot contain a period, space, forward-slash, or control characters"
    )]
    #[case::unicode(
        "café",
        "Name cannot contain a period, space, forward-slash, or control characters"
    )]
    #[case::newline(
        "foo\nbar",
        "Name cannot contain a period, space, forward-slash, or control characters"
    )]
    fn rejects_invalid_names(#[case] value: &str, #[case] message: &str) {
        let err = validate_sql_object_name(value).unwrap_err();
        assert_eq!(err.message(), message, "for {value:?}");
    }

    #[rstest]
    #[case::simple("main")]
    #[case::all_allowed("My_catalog-2024@team")]
    #[case::reserved_words_are_allowed("system")]
    #[case::information_schema("information_schema")]
    #[case::max_length(&"x".repeat(MAX_NAME_LENGTH))]
    fn accepts_valid_names(#[case] value: &str) {
        assert!(
            validate_sql_object_name(value).is_ok(),
            "expected {value:?} to be accepted"
        );
    }
}
