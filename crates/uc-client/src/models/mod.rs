//! The models of the API, generated from `proto/` by `just generate`: a module per proto
//! package, such as `catalogs::v1`.

mod _gen;

pub use _gen::*;

impl scim::v1::ScimUser {
    /// The user's name in grants and owners, as the server names it
    /// ([`uc_common::scim::principal`]).
    pub fn principal(&self) -> &str {
        let emails = self.emails.iter();
        uc_common::scim::principal(
            &self.user_name,
            emails.map(|e| (e.value.as_str(), e.primary == Some(true))),
        )
    }
}
