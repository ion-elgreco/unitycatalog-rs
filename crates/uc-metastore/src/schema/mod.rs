pub(crate) mod sql;

use std::collections::BTreeMap;

use async_trait::async_trait;

use uc_api::{CreateSchema, SchemaInfo, UpdateSchema};
use uc_common::{Pagination, Result};

/// The outcome of a property patch: the keys set, the keys removed, and the
/// keys asked to be removed that were not there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PropertyChanges {
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub missing: Vec<String>,
}

#[async_trait]
pub trait SchemaOps: Send + Sync {
    async fn create(&self, req: CreateSchema, created_by: Option<&str>) -> Result<SchemaInfo>;

    /// `full_name` is `"<catalog>.<schema>"`.
    async fn get(&self, full_name: &str) -> Result<SchemaInfo>;

    async fn update(
        &self,
        full_name: &str,
        req: UpdateSchema,
        updated_by: Option<&str>,
    ) -> Result<SchemaInfo>;

    /// `force=true` cascades to child tables.
    async fn delete(&self, full_name: &str, force: bool) -> Result<()>;

    /// Upstream `applyPropertyChanges`: set `updates`, drop `removals` and
    /// leave the other properties alone, atomically. Unlike [`Self::update`]
    /// this can remove the last property.
    async fn apply_property_changes(
        &self,
        full_name: &str,
        updates: BTreeMap<String, String>,
        removals: Vec<String>,
        updated_by: Option<&str>,
    ) -> Result<PropertyChanges>;

    async fn list(
        &self,
        catalog_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<SchemaInfo>, Option<String>)>;
}
