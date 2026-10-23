use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    CreateRecipient, CreateShare, RecipientInfo, ShareInfo, SharedDataObject, UpdateRecipient,
    UpdateShare,
};
use uc_common::{Error, Pagination, Result, now_epoch_millis, validate_sql_object_name};
use uc_storage::Exec;

use crate::SharingOps;
use crate::sql::rows::{RECIPIENT, SHARE};
use crate::sql::{
    SqlMetastore, create_err, db_err, insert, random_token, select, select_one, sha256_hex,
    touch_audit, update,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShareRow {
    name: String,
    id_string: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    objects: Option<Vec<SharedDataObject>>,
}

impl From<ShareRow> for ShareInfo {
    fn from(r: ShareRow) -> Self {
        Self {
            name: r.name,
            id: Some(r.id_string),
            comment: r.comment,
            owner: r.owner,
            created_at: Some(r.created_at),
            created_by: r.created_by,
            updated_at: Some(r.updated_at),
            updated_by: r.updated_by,
            objects: r.objects.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecipientRow {
    name: String,
    id_string: String,
    authentication_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(default)]
    updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shares: Option<Vec<String>>,
}

impl From<RecipientRow> for RecipientInfo {
    fn from(r: RecipientRow) -> Self {
        Self {
            name: r.name,
            id: Some(r.id_string),
            authentication_type: r.authentication_type,
            comment: r.comment,
            owner: r.owner,
            created_at: Some(r.created_at),
            created_by: r.created_by,
            bearer_token: None,
            token_prefix: r.token_prefix,
            shares: r.shares.unwrap_or_default(),
        }
    }
}

/// Grant the recipients of share `old` the share `new` instead, or nothing
/// for none.
async fn regrant(ex: &dyn Exec, old: &str, new: Option<&str>) -> Result<()> {
    let recipients: Vec<RecipientRow> =
        select(ex, &RECIPIENT, "WHERE shares IS NOT NULL", &[]).await?;
    for mut row in recipients {
        let Some(shares) = row.shares.as_mut() else {
            continue;
        };
        if !shares.iter().any(|s| s == old) {
            continue;
        }
        shares.retain(|s| s != old);
        if let Some(new) = new
            && !shares.iter().any(|s| s == new)
        {
            shares.push(new.to_string());
        }
        if shares.is_empty() {
            row.shares = None;
        }
        update(ex, &RECIPIENT, &row)
            .await
            .map_err(db_err("update recipient"))?;
    }
    Ok(())
}

/// Mint a Delta Sharing recipient token (`dsr_<64 hex>`): returns
/// `(plaintext, 8-char prefix, sha256 hash)`.
fn mint_token() -> (String, String, String) {
    let token = format!("dsr_{}", random_token());
    let prefix: String = token.chars().take(8).collect();
    let hash = sha256_hex(&token);
    (token, prefix, hash)
}

impl SqlMetastore {
    async fn share_row(&self, name: &str) -> Result<ShareRow> {
        select_one(self.db.as_ref(), &SHARE, "WHERE name = $1", &[name.into()])
            .await?
            .ok_or_else(|| Error::not_found(format!("share '{name}' not found")))
    }

    async fn recipient_row(&self, name: &str) -> Result<RecipientRow> {
        select_one(
            self.db.as_ref(),
            &RECIPIENT,
            "WHERE name = $1",
            &[name.into()],
        )
        .await?
        .ok_or_else(|| Error::not_found(format!("recipient '{name}' not found")))
    }

    /// Delete the row of `table` named `name`, or fail with `not_found`.
    async fn delete_named(&self, table: &str, name: &str, not_found: Error) -> Result<()> {
        let removed = self
            .db
            .execute(
                &format!("DELETE FROM {table} WHERE name = $1"),
                &[name.into()],
            )
            .await
            .map_err(db_err("delete"))?;
        if removed == 0 {
            return Err(not_found);
        }
        Ok(())
    }
}

#[async_trait]
impl SharingOps for SqlMetastore {
    async fn create_share(&self, req: CreateShare, created_by: Option<&str>) -> Result<ShareInfo> {
        validate_sql_object_name(&req.name)?;
        let now = now_epoch_millis();
        let row = ShareRow {
            name: req.name.clone(),
            id_string: Uuid::new_v4().to_string(),
            comment: req.comment,
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
            objects: None,
        };
        insert(self.db.as_ref(), &SHARE, &row)
            .await
            .map_err(create_err)?;
        Ok(row.into())
    }

    async fn get_share(&self, name: &str) -> Result<ShareInfo> {
        self.share_row(name).await.map(Into::into)
    }

    async fn list_shares(&self, page: &Pagination) -> Result<(Vec<ShareInfo>, Option<String>)> {
        let (rows, next) = self
            .paged_list::<ShareRow, _>(&SHARE, &[], vec![], "name", page, |r| r.name.clone())
            .await?;
        Ok((rows.into_iter().map(Into::into).collect(), next))
    }

    async fn update_share(
        &self,
        name: &str,
        req: UpdateShare,
        updated_by: Option<&str>,
    ) -> Result<ShareInfo> {
        let mut row = self.share_row(name).await?;

        if let Some(c) = req.comment {
            row.comment = Some(c);
        }

        // Objects are unique by `source_table_full_name`. Removals come
        // first, so that an update can remove an object and add it anew.
        let mut objs = row.objects.unwrap_or_default();
        for rm in &req.remove_objects {
            objs.retain(|o| o.source_table_full_name != rm.source_table_full_name);
        }
        for add in req.add_objects {
            if !objs
                .iter()
                .any(|o| o.source_table_full_name == add.source_table_full_name)
            {
                objs.push(add);
            }
        }
        row.objects = if objs.is_empty() { None } else { Some(objs) };
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);

        let renamed = req.new_name.filter(|new_name| *new_name != row.name);
        if let Some(new_name) = &renamed {
            validate_sql_object_name(new_name)?;
            row.name = new_name.clone();
        }
        // A rename moves the recipients' grants along, in one transaction.
        let failed = db_err("update share");
        let tx = self.db.begin().await.map_err(&failed)?;
        let written = update(tx.as_ref(), &SHARE, &row)
            .await
            .map_err(create_err)?;
        if written == 0 {
            return Err(Error::not_found(format!("share '{name}' not found")));
        }
        if let Some(new_name) = &renamed {
            regrant(tx.as_ref(), name, Some(new_name)).await?;
        }
        tx.commit().await.map_err(&failed)?;
        Ok(row.into())
    }

    async fn delete_share(&self, name: &str) -> Result<()> {
        // A later share of the same name is not granted to the recipients
        // of this one.
        let failed = db_err("delete share");
        let tx = self.db.begin().await.map_err(&failed)?;
        let removed = tx
            .execute("DELETE FROM share WHERE name = $1", &[name.into()])
            .await
            .map_err(&failed)?;
        if removed == 0 {
            return Err(Error::not_found(format!("Share not found: {name}")));
        }
        regrant(tx.as_ref(), name, None).await?;
        tx.commit().await.map_err(&failed)
    }

    async fn create_recipient(
        &self,
        req: CreateRecipient,
        created_by: Option<&str>,
    ) -> Result<RecipientInfo> {
        validate_sql_object_name(&req.name)?;
        let auth = req
            .authentication_type
            .unwrap_or_else(|| "TOKEN".to_string());
        let (plaintext, prefix, hash) = mint_token();
        let now = now_epoch_millis();
        let row = RecipientRow {
            name: req.name.clone(),
            id_string: Uuid::new_v4().to_string(),
            authentication_type: auth,
            comment: req.comment,
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
            token_hash: Some(hash),
            token_prefix: Some(prefix),
            shares: None,
        };
        insert(self.db.as_ref(), &RECIPIENT, &row)
            .await
            .map_err(create_err)?;
        let mut out: RecipientInfo = row.into();
        out.bearer_token = Some(plaintext);
        Ok(out)
    }

    async fn get_recipient(&self, name: &str) -> Result<RecipientInfo> {
        self.recipient_row(name).await.map(Into::into)
    }

    async fn list_recipients(
        &self,
        page: &Pagination,
    ) -> Result<(Vec<RecipientInfo>, Option<String>)> {
        let (rows, next) = self
            .paged_list::<RecipientRow, _>(&RECIPIENT, &[], vec![], "name", page, |r| {
                r.name.clone()
            })
            .await?;
        Ok((rows.into_iter().map(Into::into).collect(), next))
    }

    async fn update_recipient(
        &self,
        name: &str,
        req: UpdateRecipient,
        updated_by: Option<&str>,
    ) -> Result<RecipientInfo> {
        let mut row = self.recipient_row(name).await?;

        if let Some(c) = req.comment {
            row.comment = Some(c);
        }
        let mut shares = row.shares.unwrap_or_default();
        for s in req.add_shares {
            if !shares.contains(&s) {
                shares.push(s);
            }
        }
        shares.retain(|s| !req.remove_shares.contains(s));
        row.shares = if shares.is_empty() {
            None
        } else {
            Some(shares)
        };
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);

        let written = update(self.db.as_ref(), &RECIPIENT, &row)
            .await
            .map_err(db_err("update recipient"))?;
        if written == 0 {
            return Err(Error::not_found(format!("recipient '{name}' not found")));
        }
        Ok(row.into())
    }

    async fn delete_recipient(&self, name: &str) -> Result<()> {
        self.delete_named(
            "recipient",
            name,
            Error::not_found(format!("Recipient not found: {name}")),
        )
        .await
    }

    async fn rotate_recipient_token(&self, name: &str) -> Result<RecipientInfo> {
        let mut row = self.recipient_row(name).await?;
        let (plaintext, prefix, hash) = mint_token();
        row.token_hash = Some(hash);
        row.token_prefix = Some(prefix);

        let written = update(self.db.as_ref(), &RECIPIENT, &row)
            .await
            .map_err(db_err("rotate recipient token"))?;
        if written == 0 {
            return Err(Error::not_found(format!("recipient '{name}' not found")));
        }
        let mut out: RecipientInfo = row.into();
        out.bearer_token = Some(plaintext);
        Ok(out)
    }

    async fn recipient_for_token(&self, token: &str) -> Result<Option<RecipientInfo>> {
        let row: Option<RecipientRow> = select_one(
            self.db.as_ref(),
            &RECIPIENT,
            "WHERE token_hash = $1",
            &[sha256_hex(token).into()],
        )
        .await?;
        Ok(row.map(Into::into))
    }
}
