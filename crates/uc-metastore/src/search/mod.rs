pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::SearchResult;
use uc_common::{Result, SecurableType};

/// The types of the objects that a search finds.
pub const SEARCHABLE: [SecurableType; 6] = [
    SecurableType::Catalog,
    SecurableType::Schema,
    SecurableType::Table,
    SecurableType::Volume,
    SecurableType::Function,
    SecurableType::RegisteredModel,
];

#[async_trait]
pub trait SearchOps: Send + Sync {
    /// The objects of the type `kind`, or of all [`SEARCHABLE`] types, whose
    /// name and comment hold all the words of `text`, through the full-text
    /// indexes. First the names equal to `text`, then the names that start
    /// with it, the names that hold it, and the others; in each group by
    /// name, then full name. `offset` and `limit` take a part of that list.
    /// A type that is not searchable is an `INVALID_ARGUMENT`.
    async fn search(
        &self,
        text: &str,
        kind: Option<SecurableType>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<SearchResult>>;
}
