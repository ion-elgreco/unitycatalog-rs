//! Cedar policies as the [`Authorizer`](uc_auth::authz::Authorizer).
//!
//! Plugs in with `auth.cedar` ([`AppState::with_cedar`]): Cedar policies
//! then decide what a caller may do on a resource, instead of the grants
//! alone ([`uc_auth::authz::CedarAuthorizer`]).
//!
//! ## Default policy
//!
//! [`DEFAULT_POLICY`] permits what the `permission` table grants, as the
//! grants alone do. Operators add their own `.cedar` files with
//! `auth.cedar_policies`. A policy that fails to evaluate denies.
//!
//! [`AppState::with_cedar`]: crate::server::state::AppState::with_cedar
//!
//! ## Requests
//!
//! Every UC privilege is a Cedar `Action::"<priv>"` (e.g. `Action::"MODIFY"`).
//! Principals are `User::"<username>"`. Resources are `Catalog::"<id>"`,
//! `Schema::"<id>"` and so on: one entity type per securable type, named by
//! the securable's id. A resource's parents (schema, catalog, metastore) are
//! its Cedar parents, so policies can say `resource in Catalog::"<id>"`;
//! `granted_privileges` holds what is granted on it or a parent, and `owner`
//! its owner when it has one.
//!
//! The context is the request's: `auth_method`, `provider`, `groups`,
//! `source_ip` (an `ipaddr`, when known) and `request_time` (a
//! `datetime`), so policies can also say where and when.

use std::path::PathBuf;

use uc_auth::authz::{CedarAuthorizer, NoEntities};
use uc_common::{Error, Result};

/// A principal may do an action that the resource's `granted_privileges`,
/// from the `permission` table, contain.
pub const DEFAULT_POLICY: &str = r#"
@id("granted")
permit (principal, action, resource)
when { resource.granted_privileges.contains(action) };
"#;

/// The authorizer of [`DEFAULT_POLICY`] and the policies of `policies`, each
/// a Cedar policy source. The policies of each source get their positions as
/// ids, unless they have an `@id`.
pub fn authorizer(policies: &[String]) -> Result<CedarAuthorizer> {
    let mut all = DEFAULT_POLICY.to_string();
    for source in policies {
        all.push('\n');
        all.push_str(source);
    }
    CedarAuthorizer::new(&all, NoEntities)
        .map_err(|e| Error::invalid_argument(format!("invalid Cedar policy: {e}")))
}

/// The `.cedar` files `paths`, each with its text.
pub fn read_policy_files(paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>> {
    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path).map_err(|e| {
                Error::invalid_argument(format!("read Cedar policy file '{}': {e}", path.display()))
            })?;
            Ok((path.clone(), text))
        })
        .collect()
}

/// The authorizer of [`DEFAULT_POLICY`] and the `.cedar` files `files`,
/// each a path and its text.
pub fn with_policy_files(files: &[(PathBuf, String)]) -> Result<CedarAuthorizer> {
    let mut sources = Vec::with_capacity(files.len());
    for (path, source) in files {
        // Each file on its own, then with the ones before it, so an error
        // names the file: an `@id` that the default policy or an earlier
        // file has too names this one.
        let in_file = |e: &dyn std::fmt::Display| {
            Error::invalid_argument(format!("policy file '{}': {e}", path.display()))
        };
        CedarAuthorizer::new(source, NoEntities).map_err(|e| in_file(&e))?;
        sources.push(source.clone());
        authorizer(&sources).map_err(|e| in_file(&e.message()))?;
    }
    authorizer(&sources)
}
