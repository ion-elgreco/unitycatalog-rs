use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    CreateExternalLocation, CredentialPurpose, ExternalLocationInfo, UpdateExternalLocation,
};
use uc_common::{
    Error, ErrorCode, NormalizedUrl, Pagination, Result, now_epoch_millis, require_non_empty,
    validate_sql_object_name,
};

use crate::ExternalLocationOps;
use crate::credential::sql::CredentialRow;
use crate::sql::rows::{CREDENTIAL, EXTERNAL_LOCATION};
use crate::sql::{
    SqlMetastore, char_len, create_err_as, db_err, insert, placeholders, select, select_one,
    starts_with, touch_audit, update,
};
use uc_storage::Value;

/// An external location row, keyed by its UUID and linked to its storage
/// credential by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExternalLocationRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) credential_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) owner: Option<String>,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated_by: Option<String>,
}

impl ExternalLocationRow {
    fn into_info(self, credential_name: Option<String>) -> ExternalLocationInfo {
        ExternalLocationInfo {
            name: Some(self.name),
            url: Some(self.url),
            credential_name,
            owner: self.owner,
            comment: self.comment,
            created_at: Some(self.created_at),
            created_by: self.created_by,
            updated_at: Some(self.updated_at),
            updated_by: self.updated_by,
            id: Some(self.id_string),
            credential_id: Some(self.credential_id),
        }
    }
}

fn location_exists(name: &str) -> Error {
    Error::new(
        ErrorCode::ExternalLocationAlreadyExists,
        format!("External location already exists: {name}"),
    )
}

impl SqlMetastore {
    pub(crate) async fn location_row(&self, name: &str) -> Result<Option<ExternalLocationRow>> {
        select_one(
            self.db.as_ref(),
            &EXTERNAL_LOCATION,
            "WHERE name = $1",
            &[name.into()],
        )
        .await
    }

    /// Credential names for the given credential ids.
    async fn credential_names(&self, ids: &[&str]) -> Result<HashMap<String, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let failed = db_err("credential names");
        let ids: Vec<Value> = ids.iter().map(|id| (*id).into()).collect();
        let rows = self
            .db
            .query(
                &format!(
                    "SELECT id_string, name FROM credential WHERE id_string IN ({})",
                    placeholders(1, ids.len())
                ),
                &ids,
            )
            .await
            .map_err(&failed)?;
        rows.iter()
            .map(|r| Ok((r.text(0)?, r.text(1)?)))
            .collect::<Result<_, _>>()
            .map_err(&failed)
    }

    /// Attach credential names to rows.
    async fn location_infos(
        &self,
        rows: Vec<ExternalLocationRow>,
    ) -> Result<Vec<ExternalLocationInfo>> {
        let ids: Vec<&str> = rows.iter().map(|r| r.credential_id.as_str()).collect();
        let names = self.credential_names(&ids).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let name = names.get(&r.credential_id).cloned();
                r.into_info(name)
            })
            .collect())
    }

    async fn location_info(&self, row: ExternalLocationRow) -> Result<ExternalLocationInfo> {
        Ok(self.location_infos(vec![row]).await?.remove(0))
    }

    /// Upstream `validateAndGetCredentialDAO`: the id of the storage
    /// credential named `name`, which must reach `url`.
    async fn credential_id_for(&self, name: &str, url: &NormalizedUrl) -> Result<String> {
        let credential = self
            .credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Credential not found: {name}")))?;
        let storage = crate::sql::enum_str(&CredentialPurpose::Storage);
        if credential.purpose.as_ref().is_some_and(|p| *p != storage) {
            return Err(Error::invalid_argument(format!(
                "Credential not of type storage: {name}"
            )));
        }
        credential.check_reaches(url)?;
        Ok(credential.id_string)
    }

    /// Upstream `validateUrlNotUsedByAnyExternalLocation`: no other location
    /// may be the same as, above or below `url`.
    async fn validate_url_not_used(
        &self,
        url: &NormalizedUrl,
        current_id: Option<&str>,
    ) -> Result<()> {
        let others = self.overlapping_locations(url.as_str()).await?;
        if others.iter().any(|(id, _)| Some(id.as_str()) != current_id) {
            return Err(Error::invalid_argument(
                "Cannot accept an external location that duplicates or overlaps with existing \
                 external location",
            ));
        }
        Ok(())
    }

    /// Upstream `deleteExternalLocation` without `force`: the location must
    /// not overlap pending storage cleanup, and no table, volume or model
    /// may live at or under it.
    async fn validate_location_unused(&self, url: &str) -> Result<()> {
        let failed = db_err("external location users");
        let under = format!("{url}/");
        // `$1` the location, `$2` and `$3` what is under it.
        let mut params: Vec<Value> = vec![url.into(), under.as_str().into(), char_len(&under)];
        let at_or_under = format!(
            "storage_location = $1 OR {}",
            starts_with("storage_location", 2, 3)
        );
        let above = url_prefix_candidates(url);
        let sql = format!(
            "SELECT id_string FROM storage_cleanup_task \
             WHERE storage_location IN ({}) OR {at_or_under} LIMIT 1",
            placeholders(params.len() + 1, above.len())
        );
        let users_sql =
            |table: &str| format!("SELECT id_string FROM {table} WHERE {at_or_under} LIMIT 1");
        let base = params.clone();
        params.extend(above.into_iter().map(Value::Text));
        let cleanup = self.db.query(&sql, &params).await.map_err(&failed)?;
        if !cleanup.is_empty() {
            return Err(Error::permission_denied(
                "Input path overlaps pending storage cleanup.",
            ));
        }
        for (table, kind) in [
            ("tbl", "table"),
            ("volume", "volume"),
            ("registered_model", "registered_model"),
        ] {
            let users = self
                .db
                .query(&users_sql(table), &base)
                .await
                .map_err(&failed)?;
            if let Some(user) = users.first() {
                return Err(Error::invalid_argument(format!(
                    "External location still used by {kind} '{}'",
                    user.text(0).map_err(&failed)?
                )));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ExternalLocationOps for SqlMetastore {
    async fn create(
        &self,
        req: CreateExternalLocation,
        created_by: Option<&str>,
    ) -> Result<ExternalLocationInfo> {
        validate_sql_object_name(&req.name)?;
        require_non_empty("url", &req.url)?;

        // Upstream order: name collision, URL checks, then the credential.
        if self.location_row(&req.name).await?.is_some() {
            return Err(location_exists(&req.name));
        }
        let url = NormalizedUrl::parse(&req.url)?;
        url.validate_not_under_managed_prefix()?;
        self.validate_url_not_used(&url, None).await?;
        let credential_id = self.credential_id_for(&req.credential_name, &url).await?;

        let now = now_epoch_millis();
        let id = Uuid::new_v4().to_string();
        let row = ExternalLocationRow {
            id_string: id,
            name: req.name.clone(),
            url: url.into_string(),
            credential_id,
            comment: req.comment,
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
        };

        insert(self.db.as_ref(), &EXTERNAL_LOCATION, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::ExternalLocationAlreadyExists,
                format!("External location already exists: {}", req.name),
            ))?;
        Ok(row.into_info(Some(req.credential_name)))
    }

    async fn get(&self, name: &str) -> Result<ExternalLocationInfo> {
        let row = self
            .location_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("External location not found: {name}")))?;
        self.location_info(row).await
    }

    async fn update(
        &self,
        name: &str,
        req: UpdateExternalLocation,
        updated_by: Option<&str>,
    ) -> Result<ExternalLocationInfo> {
        if let Some(new_name) = &req.new_name {
            validate_sql_object_name(new_name)?;
        }
        let mut existing = self
            .location_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("External location not found: {name}")))?;

        // Upstream rejects any new name that exists, the current one included.
        if let Some(new_name) = req.new_name {
            if self.location_row(&new_name).await?.is_some() {
                return Err(location_exists(&new_name));
            }
            existing.name = new_name;
        }
        // Upstream checks a new URL for overlaps only (no managed-prefix
        // check on update).
        let new_url = req.url.is_some();
        if let Some(raw) = req.url {
            let url = NormalizedUrl::parse(&raw)?;
            self.validate_url_not_used(&url, Some(&existing.id_string))
                .await?;
            existing.url = url.into_string();
        }
        if let Some(c) = req.comment {
            existing.comment = Some(c);
        }
        let url = NormalizedUrl::parse(&existing.url)?;
        if let Some(credential_name) = req.credential_name {
            existing.credential_id = self.credential_id_for(&credential_name, &url).await?;
        } else if new_url {
            let credential: CredentialRow = select_one(
                self.db.as_ref(),
                &CREDENTIAL,
                "WHERE id_string = $1",
                &[existing.credential_id.as_str().into()],
            )
            .await?
            .ok_or_else(|| Error::internal("external_location row without its credential"))?;
            credential.check_reaches(&url)?;
        }
        touch_audit(
            &mut existing.updated_at,
            &mut existing.updated_by,
            updated_by,
        );

        let written = update(self.db.as_ref(), &EXTERNAL_LOCATION, &existing)
            .await
            .map_err(create_err_as(
                ErrorCode::ExternalLocationAlreadyExists,
                format!("External location already exists: {}", existing.name),
            ))?;
        if written == 0 {
            return Err(Error::internal("external_location update found no row"));
        }
        self.location_info(existing).await
    }

    async fn delete(&self, name: &str, force: bool) -> Result<()> {
        let row = self
            .location_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("External location not found: {name}")))?;
        if !force {
            self.validate_location_unused(&row.url).await?;
        }
        let failed = db_err("delete external location");
        let tx = self.db.begin().await.map_err(&failed)?;
        let id = [row.id_string.as_str().into()];
        for sql in [
            "DELETE FROM permission WHERE resource_id = $1",
            "DELETE FROM external_location WHERE id_string = $1",
        ] {
            tx.execute(sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn list(&self, page: &Pagination) -> Result<(Vec<ExternalLocationInfo>, Option<String>)> {
        let (rows, next) = self
            .paged_list::<ExternalLocationRow, _>(
                &EXTERNAL_LOCATION,
                &[],
                vec![],
                "name",
                page,
                |r| r.name.clone(),
            )
            .await?;
        Ok((self.location_infos(rows).await?, next))
    }

    async fn find_by_url_prefix(&self, path: &str) -> Result<Option<ExternalLocationInfo>> {
        // Look up every '/'-bounded prefix of `path` in the `url` index. The
        // longest match wins.
        let candidates = url_prefix_candidates(path);
        if candidates.is_empty() {
            return Ok(None);
        }
        let rows: Vec<ExternalLocationRow> = select(
            self.db.as_ref(),
            &EXTERNAL_LOCATION,
            &format!("WHERE url IN ({})", placeholders(1, candidates.len())),
            &candidates.into_iter().map(Value::Text).collect::<Vec<_>>(),
        )
        .await?;
        match rows.into_iter().max_by_key(|r| r.url.len()) {
            Some(row) => Ok(Some(self.location_info(row).await?)),
            None => Ok(None),
        }
    }
}

impl SqlMetastore {
    /// `(id, url)` of each external location that is the same as, above or
    /// below `url`. Locations stored with a trailing slash still match.
    pub(crate) async fn overlapping_locations(&self, url: &str) -> Result<Vec<(String, String)>> {
        let failed = db_err("overlapping external locations");
        let base = url.trim_end_matches('/');
        let under = format!("{base}/");
        let above = url_prefix_candidates(base);
        let sql = format!(
            "SELECT id_string, url FROM external_location \
             WHERE {} OR url IN ({}) LIMIT 2",
            starts_with("url", 1, 2),
            placeholders(3, above.len())
        );
        let mut params: Vec<Value> = vec![under.as_str().into(), char_len(&under)];
        params.extend(above.into_iter().map(Value::Text));
        let hits = self.db.query(&sql, &params).await.map_err(&failed)?;
        hits.iter()
            .map(|h| Ok((h.text(0)?, h.text(1)?)))
            .collect::<Result<_, _>>()
            .map_err(&failed)
    }
}

/// `s3://bucket/a/b/c` →
/// `["s3://bucket/a/b/c", "s3://bucket/a/b/", "s3://bucket/a/b",
///   "s3://bucket/a/", "s3://bucket/a", "s3://bucket/", "s3://bucket"]`.
///
/// Each `/` gives both forms, with and without the slash, so that
/// `external_location.url` rows stored either way match.
fn url_prefix_candidates(path: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![path.to_string()];
    let scheme_end = path.find("://").map(|i| i + 3).unwrap_or(0);
    let bytes = path.as_bytes();
    let mut i = path.len();
    while i > scheme_end {
        if bytes[i - 1] == b'/' {
            out.push(path[..i].to_string());
            if i - 1 > scheme_end {
                out.push(path[..i - 1].to_string());
            }
        }
        i -= 1;
    }
    out.sort();
    out.dedup();
    out
}
