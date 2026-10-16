use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{CreatePat, PatInfo};
use uc_common::{Error, Pagination, Result, now_epoch_millis, require_non_empty};

use crate::TokenOps;
use crate::sql::rows::PAT;
use crate::sql::{SqlMetastore, by_id, create_err, db_err, insert};
use uc_storage::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatRow {
    id_string: String,
    name: String,
    /// SHA-256 hex of the plaintext token. The plaintext is not stored; the
    /// caller sees it only in the answer to `create_pat`.
    token_hash: String,
    token_prefix: String,
    owner: String,
    created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<i64>,
}

impl PatRow {
    fn to_info(&self, token_value: Option<String>) -> PatInfo {
        PatInfo {
            id: Some(self.id_string.clone()),
            name: Some(self.name.clone()),
            token_prefix: Some(self.token_prefix.clone()),
            owner: Some(self.owner.clone()),
            created_at: Some(self.created_at),
            expires_at: self.expires_at,
            token_value,
        }
    }
}

use crate::sql::{random_token, sha256_hex};

#[async_trait]
impl TokenOps for SqlMetastore {
    async fn create_pat(&self, req: CreatePat, owner: &str) -> Result<PatInfo> {
        require_non_empty("name", &req.name)?;

        let token = random_token();
        let token_hash = sha256_hex(&token);
        let token_prefix: String = token.chars().take(8).collect();

        let row = PatRow {
            id_string: Uuid::new_v4().to_string(),
            name: req.name,
            token_hash,
            token_prefix,
            owner: owner.to_string(),
            created_at: now_epoch_millis(),
            expires_at: req.expires_at,
        };

        insert(self.db.as_ref(), &PAT, &row)
            .await
            .map_err(create_err)?;
        Ok(row.to_info(Some(token)))
    }

    async fn get_pat(&self, id: &str) -> Result<PatInfo> {
        let row: Option<PatRow> = by_id(self.db.as_ref(), &PAT, id).await?;
        row.map(|r| r.to_info(None))
            .ok_or_else(|| Error::not_found(format!("PAT '{id}' not found")))
    }

    async fn list_pats(
        &self,
        owner_filter: Option<&str>,
        page: &Pagination,
    ) -> Result<(Vec<PatInfo>, Option<String>)> {
        let (filters, params): (&[&str], Vec<Value>) = match owner_filter {
            Some(o) => (&["owner = $1"], vec![o.into()]),
            None => (&[], vec![]),
        };
        let (rows, next) = self
            .paged_list::<PatRow, _>(&PAT, filters, params, "id_string", page, |r| {
                r.id_string.clone()
            })
            .await?;
        Ok((rows.iter().map(|r| r.to_info(None)).collect(), next))
    }

    async fn revoke_pat(&self, id: &str) -> Result<()> {
        self.delete_by_id("pat", id, || {
            Error::not_found(format!("PAT not found: {id}"))
        })
        .await
    }

    async fn principal_for_token(&self, token: &str) -> Result<Option<String>> {
        // Hot path: every request with a token runs through here. Read only
        // the owner, and let the database skip expired tokens.
        let rows = self
            .db
            .query(
                "SELECT owner FROM pat \
                 WHERE token_hash = $1 \
                   AND (expires_at IS NULL OR expires_at = 0 OR expires_at >= $2) \
                 LIMIT 1",
                &[sha256_hex(token).into(), now_epoch_millis().into()],
            )
            .await
            .map_err(db_err("look up pat"))?;
        rows.first()
            .map(|r| r.text(0))
            .transpose()
            .map_err(db_err("look up pat"))
    }
}
