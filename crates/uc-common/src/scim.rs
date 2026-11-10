//! The rules of SCIM users that the server and its clients share.

/// A user's name in grants and owners: its primary email, else its first
/// email, else its user name. `emails` gives each email with whether it is
/// the primary one.
pub fn principal<'a>(
    user_name: &'a str,
    emails: impl IntoIterator<Item = (&'a str, bool)>,
) -> &'a str {
    let mut first = None;
    for (email, primary) in emails {
        if primary {
            return email;
        }
        first.get_or_insert(email);
    }
    first.unwrap_or(user_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_principal_is_the_primary_email_else_the_first_else_the_user_name() {
        assert_eq!(principal("u", [("a@x", false), ("b@x", true)]), "b@x");
        assert_eq!(principal("u", [("a@x", false), ("b@x", false)]), "a@x");
        assert_eq!(principal("u", []), "u");
    }
}
