pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::Protection;
use uc_common::{Error, Result, SecurableType};

/// Protection against deletes. A protected catalog, schema, table, volume or
/// registered model cannot be deleted until its protection is removed. A
/// delete that would also remove protected securables inside the deleted
/// one is refused too, and names them.
#[async_trait]
pub trait ProtectionOps: Send + Sync {
    /// The protection of the securable `full_name` of `kind`.
    async fn get(&self, kind: SecurableType, full_name: &str) -> Result<Protection>;

    /// Protect the securable `full_name` of `kind`, or remove its
    /// protection. `by` is who sets it.
    async fn set(
        &self,
        kind: SecurableType,
        full_name: &str,
        protected: bool,
        by: Option<&str>,
    ) -> Result<Protection>;
}

/// The table of a securable type that can be protected, or why it cannot.
pub fn protectable(kind: SecurableType) -> Result<&'static str> {
    match kind {
        SecurableType::Catalog => Ok("catalog"),
        SecurableType::Schema => Ok("schema"),
        SecurableType::Table => Ok("tbl"),
        SecurableType::Volume => Ok("volume"),
        SecurableType::RegisteredModel => Ok("registered_model"),
        other => Err(Error::invalid_argument(format!(
            "Only catalogs, schemas, tables, volumes and registered models can be protected, \
             not {}s.",
            label(other)
        ))),
    }
}

/// `registered model` for `registered_model`.
pub(crate) fn label(kind: SecurableType) -> String {
    kind.as_str().replace('_', " ")
}
