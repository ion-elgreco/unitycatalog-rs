//! The models of the API, generated from `proto/` by `just generate`: a module per proto
//! package, such as `catalogs::v1`.

mod _gen;

pub use _gen::*;

impl scim::v1::ScimUser {
    /// The user's name in grants and owners, as the server names it: the
    /// primary email, else the first email, else the user name.
    pub fn principal(&self) -> &str {
        self.emails
            .iter()
            .find(|e| e.primary == Some(true))
            .or(self.emails.first())
            .map_or(self.user_name.as_str(), |e| e.value.as_str())
    }
}
