pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::SearchResult;
use uc_common::{Result, SecurableType};

use crate::Lineage;

/// A result of a search, with the catalog and schema that it is in: what
/// the rule of its type's `get` checks.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub result: SearchResult,
    pub lineage: Lineage,
}

#[async_trait]
pub trait SearchOps: Send + Sync {
    /// The objects of the type `kind`, or of all the types that a search
    /// finds (catalogs, schemas, tables, volumes, functions and registered
    /// models), whose name and comment hold all the words of `text`,
    /// through the full-text indexes. First the names equal to `text`, then
    /// the names that start with it, the names that hold it, and the
    /// others; in each group by name, then full name. `offset` and `limit`
    /// take a part of that list. A type that the search does not find is an
    /// `INVALID_ARGUMENT`.
    async fn search(
        &self,
        text: &str,
        kind: Option<SecurableType>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<SearchHit>>;
}
