pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::AuditEvent;
use uc_common::{Pagination, Result};

/// Filters of [`AuditLogOps::list`]. A `None` field does not filter.
#[derive(Debug, Clone, Default)]
pub struct AuditQuery<'a> {
    pub principal: Option<&'a str>,
    pub method: Option<&'a str>,
    pub path_prefix: Option<&'a str>,
    /// Inclusive lower bound on `timestamp` (epoch millis).
    pub since: Option<i64>,
    /// Inclusive upper bound on `timestamp` (epoch millis).
    pub until: Option<i64>,
}

#[async_trait]
pub trait AuditLogOps: Send + Sync {
    async fn record(&self, event: AuditEvent) -> Result<()>;

    async fn list(
        &self,
        query: AuditQuery<'_>,
        page: &Pagination,
    ) -> Result<(Vec<AuditEvent>, Option<String>)>;
}
