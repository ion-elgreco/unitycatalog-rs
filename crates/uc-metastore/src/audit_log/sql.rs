use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::AuditEvent;
use uc_common::{Pagination, Result};

use crate::sql::rows::AUDIT_EVENT;
use crate::sql::{SqlMetastore, char_len, create_err, insert, select, starts_with};
use crate::{AuditLogOps, AuditQuery};
use uc_storage::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEventRow {
    id_string: String,
    principal: String,
    method: String,
    path: String,
    status: u16,
    timestamp: i64,
}

impl From<AuditEventRow> for AuditEvent {
    fn from(r: AuditEventRow) -> Self {
        Self {
            id: Some(r.id_string),
            principal: r.principal,
            method: r.method,
            path: r.path,
            status: r.status,
            timestamp: r.timestamp,
        }
    }
}

#[async_trait]
impl AuditLogOps for SqlMetastore {
    async fn record(&self, event: AuditEvent) -> Result<()> {
        let row = AuditEventRow {
            id_string: event.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            principal: event.principal,
            method: event.method,
            path: event.path,
            status: event.status,
            timestamp: event.timestamp,
        };
        insert(self.db.as_ref(), &AUDIT_EVENT, &row)
            .await
            .map_err(create_err)
    }

    async fn list(
        &self,
        query: AuditQuery<'_>,
        page: &Pagination,
    ) -> Result<(Vec<AuditEvent>, Option<String>)> {
        // Newest-first by timestamp. Cursor encodes the boundary as
        // `<timestamp>:<id_string>` so paging is stable even when many
        // events share a millisecond.
        let mut filters: Vec<String> = Vec::new();
        let mut params: Vec<Value> = Vec::new();
        let bind = |params: &mut Vec<Value>, v: Value| {
            params.push(v);
            params.len()
        };
        if let Some(p) = query.principal {
            filters.push(format!("principal = ${}", bind(&mut params, p.into())));
        }
        if let Some(m) = query.method {
            filters.push(format!("method = ${}", bind(&mut params, m.into())));
        }
        if let Some(pre) = query.path_prefix {
            let prefix = bind(&mut params, pre.into());
            let len = bind(&mut params, char_len(pre));
            filters.push(starts_with("path", prefix, len));
        }
        if let Some(t) = query.since {
            filters.push(format!("timestamp >= ${}", bind(&mut params, t.into())));
        }
        if let Some(t) = query.until {
            filters.push(format!("timestamp <= ${}", bind(&mut params, t.into())));
        }
        // Cursor: walk earlier-than (timestamp, id_string) tuples.
        if let Some((ts, id)) = page.page_token.as_deref().and_then(parse_audit_cursor) {
            let ts = bind(&mut params, ts.into());
            let id = bind(&mut params, id.into());
            filters.push(format!(
                "(timestamp < ${ts} OR (timestamp = ${ts} AND id_string < ${id}))"
            ));
        }

        let where_sql = if filters.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", filters.join(" AND "))
        };
        let limit = page.page_size()?;
        let limit_param = bind(&mut params, (limit + 1).into());
        let tail =
            format!("{where_sql} ORDER BY timestamp DESC, id_string DESC LIMIT ${limit_param}");
        let mut rows: Vec<AuditEventRow> =
            select(self.db.as_ref(), &AUDIT_EVENT, &tail, &params).await?;
        let next = if rows.len() as i64 > limit {
            rows.pop();
            rows.last()
                .map(|r| format!("{}:{}", r.timestamp, r.id_string))
        } else {
            None
        };
        Ok((rows.into_iter().map(Into::into).collect(), next))
    }
}

fn parse_audit_cursor(s: &str) -> Option<(i64, String)> {
    let (ts, id) = s.split_once(':')?;
    Some((ts.parse().ok()?, id.to_string()))
}
