use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use uc_common::{Error, Pagination, Result, now_epoch_millis, validate_sql_object_name};

use crate::sharing::{
    Activated, NewRecipient, ObjectChange, Recipient, RecipientToken, RecipientUpdate, Share,
    ShareObject, ShareObjectType, ShareUpdate, SharingOps, TokenRecipient,
};
use crate::sql::rows::{RECIPIENT, RECIPIENT_TOKEN, SHARE, SHARE_GRANT};
use crate::sql::{
    SqlMetastore, create_err, db_err, insert, random_token, select, select_one, sha256_hex,
    touch_audit, update,
};
use crate::{SecretRef, SecretValue};

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
    objects: Option<Vec<ShareObject>>,
}

impl From<ShareRow> for Share {
    fn from(r: ShareRow) -> Self {
        Self {
            id: r.id_string,
            name: r.name,
            comment: r.comment,
            owner: r.owner,
            created_at: r.created_at,
            created_by: r.created_by,
            updated_at: r.updated_at,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expiration_time: Option<i64>,
    created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(default)]
    updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenRow {
    id_string: String,
    recipient_id: String,
    token_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    activation_hash: Option<String>,
    /// The reference of the token in the secret store, until activation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sealed_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expiration_time: Option<i64>,
    created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated_by: Option<String>,
}

impl TokenRow {
    fn is_live(&self, now: i64) -> bool {
        self.expiration_time.is_none_or(|end| end > now)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GrantRow {
    share_id: String,
    recipient_id: String,
    granted_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    granted_by: Option<String>,
}

/// The code of the activation link of `token`. It does not give the token.
fn activation_code(token: &str) -> String {
    sha256_hex(&format!("activation:{token}"))
}

fn share_not_found(name: &str) -> Error {
    Error::not_found(format!("share '{name}' not found"))
}

fn recipient_not_found(name: &str) -> Error {
    Error::not_found(format!("recipient '{name}' not found"))
}

fn check_expiration(expiration_time: Option<i64>) -> Result<()> {
    match expiration_time {
        Some(end) if end <= now_epoch_millis() => Err(Error::invalid_argument(
            "expiration_time must be in the future",
        )),
        _ => Ok(()),
    }
}

impl SqlMetastore {
    async fn share_row(&self, name: &str) -> Result<ShareRow> {
        select_one(self.db.as_ref(), &SHARE, "WHERE name = $1", &[name.into()])
            .await?
            .ok_or_else(|| share_not_found(name))
    }

    async fn recipient_row(&self, name: &str) -> Result<RecipientRow> {
        select_one(
            self.db.as_ref(),
            &RECIPIENT,
            "WHERE name = $1",
            &[name.into()],
        )
        .await?
        .ok_or_else(|| recipient_not_found(name))
    }

    async fn token_rows(&self, recipient_id: &str) -> Result<Vec<TokenRow>> {
        select(
            self.db.as_ref(),
            &RECIPIENT_TOKEN,
            "WHERE recipient_id = $1 ORDER BY created_at, id_string",
            &[recipient_id.into()],
        )
        .await
    }

    /// A new token row of the recipient, with the token in the secret
    /// store for its activation.
    async fn new_token(
        &self,
        recipient_id: &str,
        expiration_time: Option<i64>,
        actor: Option<&str>,
    ) -> Result<TokenRow> {
        let token = format!("dsr_{}", random_token());
        let id = Uuid::new_v4().to_string();
        let sealed = (self.secrets)
            .put(&id, &SecretValue::from(token.as_str()))
            .await?;
        let now = now_epoch_millis();
        Ok(TokenRow {
            id_string: id,
            recipient_id: recipient_id.to_string(),
            token_hash: sha256_hex(&token),
            activation_hash: Some(sha256_hex(&activation_code(&token))),
            sealed_token: Some(sealed.into_string()),
            expiration_time,
            created_at: now,
            created_by: actor.map(str::to_string),
            updated_at: now,
            updated_by: actor.map(str::to_string),
        })
    }

    /// The token of `row` from the secret store, until its activation.
    async fn sealed_token(&self, row: &TokenRow) -> Result<Option<SecretValue>> {
        let Some(reference) = &row.sealed_token else {
            return Ok(None);
        };
        (self.secrets)
            .get(&row.id_string, &SecretRef::new(reference.as_str()))
            .await
            .map(Some)
    }

    /// Remove the token of `row` from the secret store.
    async fn drop_sealed(&self, row: &TokenRow) {
        let Some(reference) = &row.sealed_token else {
            return;
        };
        let reference = SecretRef::new(reference.as_str());
        if let Err(error) = self.secrets.delete(&row.id_string, &reference).await {
            warn!(%error, token = %row.id_string, "cannot remove the token of a recipient from the secret store");
        }
    }

    async fn recipient(&self, row: RecipientRow) -> Result<Recipient> {
        let now = now_epoch_millis();
        let mut tokens = Vec::new();
        for token in self.token_rows(&row.id_string).await? {
            if !token.is_live(now) {
                continue;
            }
            let activation_code = match self.sealed_token(&token).await {
                Ok(sealed) => sealed.map(|t| activation_code(t.as_sensitive_str())),
                Err(error) => {
                    warn!(%error, recipient = %row.name, "cannot open a token that waits for its activation");
                    None
                }
            };
            tokens.push(RecipientToken {
                id: token.id_string,
                activation_code,
                expiration_time: token.expiration_time,
                created_at: token.created_at,
                created_by: token.created_by,
                updated_at: token.updated_at,
                updated_by: token.updated_by,
            });
        }
        Ok(Recipient {
            id: row.id_string,
            name: row.name,
            comment: row.comment,
            owner: row.owner,
            properties: row.properties.unwrap_or_default(),
            created_at: row.created_at,
            created_by: row.created_by,
            updated_at: row.updated_at,
            updated_by: row.updated_by,
            tokens,
        })
    }
}

#[async_trait]
impl SharingOps for SqlMetastore {
    async fn create_share(
        &self,
        name: &str,
        comment: Option<String>,
        actor: Option<&str>,
    ) -> Result<Share> {
        validate_sql_object_name(name)?;
        let now = now_epoch_millis();
        let row = ShareRow {
            name: name.to_string(),
            id_string: Uuid::new_v4().to_string(),
            comment,
            owner: actor.map(str::to_string),
            created_at: now,
            created_by: actor.map(str::to_string),
            updated_at: now,
            updated_by: actor.map(str::to_string),
            objects: None,
        };
        insert(self.db.as_ref(), &SHARE, &row)
            .await
            .map_err(create_err)?;
        Ok(row.into())
    }

    async fn get_share(&self, name: &str) -> Result<Share> {
        self.share_row(name).await.map(Into::into)
    }

    async fn list_shares(&self, page: &Pagination) -> Result<(Vec<Share>, Option<String>)> {
        let (rows, next) = self
            .paged_list::<ShareRow, _>(&SHARE, &[], vec![], "name", page, |r| r.name.clone())
            .await?;
        Ok((rows.into_iter().map(Into::into).collect(), next))
    }

    async fn update_share(
        &self,
        name: &str,
        change: ShareUpdate,
        actor: Option<&str>,
    ) -> Result<Share> {
        let mut row = self.share_row(name).await?;
        if let Some(comment) = change.comment {
            row.comment = Some(comment);
        }
        if let Some(owner) = change.owner {
            row.owner = Some(owner);
        }
        if let Some(new_name) = change.new_name {
            validate_sql_object_name(&new_name)?;
            row.name = new_name;
        }

        let mut objects = row.objects.unwrap_or_default();
        let now = now_epoch_millis();
        for object_change in change.objects {
            match object_change {
                ObjectChange::Add(mut object) => {
                    validate_object(&object)?;
                    if objects.iter().any(|held| same_object(held, &object)) {
                        return Err(Error::already_exists(format!(
                            "share '{name}' already has '{}'",
                            object.source_full_name
                        )));
                    }
                    object.added_at = now;
                    object.added_by = actor.map(str::to_string);
                    objects.push(object);
                }
                ObjectChange::Update(mut object) => {
                    validate_object(&object)?;
                    let held = (objects.iter_mut())
                        .find(|held| same_object(held, &object))
                        .ok_or_else(|| not_in_share(name, &object.source_full_name))?;
                    object.added_at = held.added_at;
                    object.added_by = held.added_by.take();
                    *held = object;
                }
                ObjectChange::Remove(object_type, source) => {
                    let before = objects.len();
                    objects.retain(|held| {
                        held.object_type != object_type || held.source_full_name != source
                    });
                    if objects.len() == before {
                        return Err(not_in_share(name, &source));
                    }
                }
            }
        }
        unique_aliases(&objects)?;
        row.objects = (!objects.is_empty()).then_some(objects);
        touch_audit(&mut row.updated_at, &mut row.updated_by, actor);

        let written = update(self.db.as_ref(), &SHARE, &row)
            .await
            .map_err(create_err)?;
        if written == 0 {
            return Err(share_not_found(name));
        }
        Ok(row.into())
    }

    async fn delete_share(&self, name: &str) -> Result<()> {
        let row = self.share_row(name).await?;
        let failed = db_err("delete share");
        let tx = self.db.begin().await.map_err(&failed)?;
        for sql in [
            "DELETE FROM share_grant WHERE share_id = $1",
            "DELETE FROM share WHERE id_string = $1",
        ] {
            tx.execute(sql, &[row.id_string.as_str().into()])
                .await
                .map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn share_recipients(&self, share: &str) -> Result<Vec<String>> {
        let row = self.share_row(share).await?;
        let failed = db_err("list share grants");
        let rows = self
            .db
            .query(
                "SELECT r.name FROM recipient r JOIN share_grant g ON g.recipient_id = r.id_string \
                 WHERE g.share_id = $1 ORDER BY r.name",
                &[row.id_string.into()],
            )
            .await
            .map_err(&failed)?;
        (rows.iter()).map(|r| r.text(0).map_err(&failed)).collect()
    }

    async fn grant_share(&self, share: &str, recipient: &str, actor: Option<&str>) -> Result<()> {
        let share = self.share_row(share).await?;
        let recipient = self.recipient_row(recipient).await?;
        let grant = GrantRow {
            share_id: share.id_string,
            recipient_id: recipient.id_string,
            granted_at: now_epoch_millis(),
            granted_by: actor.map(str::to_string),
        };
        match insert(self.db.as_ref(), &SHARE_GRANT, &grant).await {
            Ok(()) | Err(uc_storage::StorageError::Unique(_)) => Ok(()),
            Err(e) => Err(db_err("grant share")(e)),
        }
    }

    async fn revoke_share(&self, share: &str, recipient: &str) -> Result<()> {
        let share = self.share_row(share).await?;
        let recipient = self.recipient_row(recipient).await?;
        self.db
            .execute(
                "DELETE FROM share_grant WHERE share_id = $1 AND recipient_id = $2",
                &[share.id_string.into(), recipient.id_string.into()],
            )
            .await
            .map_err(db_err("revoke share"))?;
        Ok(())
    }

    async fn create_recipient(&self, new: NewRecipient, actor: Option<&str>) -> Result<Recipient> {
        validate_sql_object_name(&new.name)?;
        check_expiration(new.expiration_time)?;
        let now = now_epoch_millis();
        let row = RecipientRow {
            name: new.name,
            id_string: Uuid::new_v4().to_string(),
            authentication_type: "TOKEN".to_string(),
            comment: new.comment,
            owner: new.owner.or_else(|| actor.map(str::to_string)),
            properties: (!new.properties.is_empty()).then_some(new.properties),
            expiration_time: new.expiration_time,
            created_at: now,
            created_by: actor.map(str::to_string),
            updated_at: now,
            updated_by: actor.map(str::to_string),
        };
        let token = (self.new_token(&row.id_string, row.expiration_time, actor)).await?;
        let failed = db_err("create recipient");
        let tx = self.db.begin().await.map_err(&failed)?;
        let written = async {
            insert(tx.as_ref(), &RECIPIENT, &row).await?;
            insert(tx.as_ref(), &RECIPIENT_TOKEN, &token).await?;
            tx.commit().await
        }
        .await;
        if let Err(e) = written {
            self.drop_sealed(&token).await;
            return Err(create_err(e));
        }
        self.recipient(row).await
    }

    async fn get_recipient(&self, name: &str) -> Result<Recipient> {
        let row = self.recipient_row(name).await?;
        self.recipient(row).await
    }

    async fn list_recipients(&self, page: &Pagination) -> Result<(Vec<Recipient>, Option<String>)> {
        let (rows, next) = self
            .paged_list::<RecipientRow, _>(&RECIPIENT, &[], vec![], "name", page, |r| {
                r.name.clone()
            })
            .await?;
        let mut recipients = Vec::with_capacity(rows.len());
        for row in rows {
            recipients.push(self.recipient(row).await?);
        }
        Ok((recipients, next))
    }

    async fn update_recipient(
        &self,
        name: &str,
        change: RecipientUpdate,
        actor: Option<&str>,
    ) -> Result<Recipient> {
        let mut row = self.recipient_row(name).await?;
        if let Some(comment) = change.comment {
            row.comment = Some(comment);
        }
        if let Some(owner) = change.owner {
            row.owner = Some(owner);
        }
        if let Some(properties) = change.properties {
            row.properties = (!properties.is_empty()).then_some(properties);
        }
        if let Some(new_name) = change.new_name {
            validate_sql_object_name(&new_name)?;
            row.name = new_name;
        }
        check_expiration(change.expiration_time)?;
        touch_audit(&mut row.updated_at, &mut row.updated_by, actor);

        // The newest token: the older ones end as their rotation set.
        let newest = match change.expiration_time {
            Some(_) => self.token_rows(&row.id_string).await?.pop(),
            None => None,
        };
        let failed = db_err("update recipient");
        let tx = self.db.begin().await.map_err(&failed)?;
        if let Some(end) = change.expiration_time {
            row.expiration_time = Some(end);
            if let Some(newest) = newest {
                tx.execute(
                    "UPDATE recipient_token SET expiration_time = $1, updated_at = $2 \
                     WHERE id_string = $3",
                    &[end.into(), row.updated_at.into(), newest.id_string.into()],
                )
                .await
                .map_err(&failed)?;
            }
        }
        let written = update(tx.as_ref(), &RECIPIENT, &row)
            .await
            .map_err(create_err)?;
        if written == 0 {
            return Err(recipient_not_found(name));
        }
        tx.commit().await.map_err(&failed)?;
        self.recipient(row).await
    }

    async fn delete_recipient(&self, name: &str) -> Result<()> {
        let row = self.recipient_row(name).await?;
        let tokens = self.token_rows(&row.id_string).await?;
        let failed = db_err("delete recipient");
        let tx = self.db.begin().await.map_err(&failed)?;
        for sql in [
            "DELETE FROM recipient_token WHERE recipient_id = $1",
            "DELETE FROM share_grant WHERE recipient_id = $1",
            "DELETE FROM recipient WHERE id_string = $1",
        ] {
            tx.execute(sql, &[row.id_string.as_str().into()])
                .await
                .map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)?;
        for token in &tokens {
            self.drop_sealed(token).await;
        }
        Ok(())
    }

    async fn rotate_recipient_token(
        &self,
        name: &str,
        existing_expire_in_seconds: Option<i64>,
        actor: Option<&str>,
    ) -> Result<Recipient> {
        let seconds = existing_expire_in_seconds.unwrap_or(0);
        if seconds < 0 {
            return Err(Error::invalid_argument(
                "existing_token_expire_in_seconds cannot be negative",
            ));
        }
        let row = self.recipient_row(name).await?;
        let existing = self.token_rows(&row.id_string).await?;
        let token = (self.new_token(&row.id_string, row.expiration_time, actor)).await?;
        let now = now_epoch_millis();
        let grace_end = now.saturating_add(seconds.saturating_mul(1000));

        let failed = db_err("rotate recipient token");
        let written = async {
            let tx = self.db.begin().await?;
            for old in &existing {
                let end = old.expiration_time.map_or(grace_end, |e| e.min(grace_end));
                if end <= now {
                    tx.execute(
                        "DELETE FROM recipient_token WHERE id_string = $1",
                        &[old.id_string.as_str().into()],
                    )
                    .await?;
                } else {
                    // An older token has no activation link any more.
                    tx.execute(
                        "UPDATE recipient_token SET expiration_time = $1, activation_hash = NULL, \
                         sealed_token = NULL, updated_at = $2 WHERE id_string = $3",
                        &[end.into(), now.into(), old.id_string.as_str().into()],
                    )
                    .await?;
                }
            }
            insert(tx.as_ref(), &RECIPIENT_TOKEN, &token).await?;
            tx.commit().await
        }
        .await;
        if let Err(e) = written {
            self.drop_sealed(&token).await;
            return Err(failed(e));
        }
        for old in &existing {
            self.drop_sealed(old).await;
        }
        self.recipient(row).await
    }

    async fn activate(&self, code: &str) -> Result<Activated> {
        let used = || Error::not_found("The activation link is not valid, or it was used.");
        let hash = sha256_hex(code);
        let row: TokenRow = select_one(
            self.db.as_ref(),
            &RECIPIENT_TOKEN,
            "WHERE activation_hash = $1",
            &[hash.as_str().into()],
        )
        .await?
        .ok_or_else(used)?;
        if !row.is_live(now_epoch_millis()) {
            return Err(used());
        }
        let token = self.sealed_token(&row).await?.ok_or_else(used)?;
        // The link works once: of two requests at the same time, one
        // changes the row.
        let taken = self
            .db
            .execute(
                "UPDATE recipient_token SET activation_hash = NULL, sealed_token = NULL \
                 WHERE id_string = $1 AND activation_hash = $2",
                &[row.id_string.as_str().into(), hash.into()],
            )
            .await
            .map_err(db_err("activate recipient token"))?;
        if taken == 0 {
            return Err(used());
        }
        self.drop_sealed(&row).await;
        Ok(Activated {
            bearer_token: token.as_sensitive_str().to_string(),
            expiration_time: row.expiration_time,
        })
    }

    async fn recipient_for_token(&self, token: &str) -> Result<Option<TokenRecipient>> {
        let row: Option<TokenRow> = select_one(
            self.db.as_ref(),
            &RECIPIENT_TOKEN,
            "WHERE token_hash = $1",
            &[sha256_hex(token).into()],
        )
        .await?;
        let Some(row) = row.filter(|r| r.is_live(now_epoch_millis())) else {
            return Ok(None);
        };
        let recipient: Option<RecipientRow> = select_one(
            self.db.as_ref(),
            &RECIPIENT,
            "WHERE id_string = $1",
            &[row.recipient_id.as_str().into()],
        )
        .await?;
        let Some(recipient) = recipient else {
            return Ok(None);
        };
        let failed = db_err("list granted shares");
        let rows = self
            .db
            .query(
                "SELECT s.name FROM share s JOIN share_grant g ON g.share_id = s.id_string \
                 WHERE g.recipient_id = $1 ORDER BY s.name",
                &[recipient.id_string.into()],
            )
            .await
            .map_err(&failed)?;
        let shares = (rows.iter())
            .map(|r| r.text(0).map_err(&failed))
            .collect::<Result<_>>()?;
        Ok(Some(TokenRecipient {
            name: recipient.name,
            shares,
        }))
    }
}

fn not_in_share(share: &str, source: &str) -> Error {
    Error::not_found(format!("share '{share}' does not have '{source}'"))
}

fn same_object(a: &ShareObject, b: &ShareObject) -> bool {
    a.object_type == b.object_type && a.source_full_name == b.source_full_name
}

/// A recipient finds an object by its type, schema and name.
fn unique_aliases(objects: &[ShareObject]) -> Result<()> {
    for (i, object) in objects.iter().enumerate() {
        let twice = objects[..i].iter().any(|other| {
            other.object_type == object.object_type
                && other.shared_as_schema == object.shared_as_schema
                && other.shared_as_name == object.shared_as_name
        });
        if twice {
            return Err(Error::invalid_argument(format!(
                "two objects are shared as '{}.{}'",
                object.shared_as_schema, object.shared_as_name
            )));
        }
    }
    Ok(())
}

/// The history settings are of tables, and a lowest version needs the
/// history shared.
fn validate_object(object: &ShareObject) -> Result<()> {
    if object.object_type != ShareObjectType::Table
        && (object.history_shared || object.start_version.is_some())
    {
        return Err(Error::invalid_argument(format!(
            "'{}' is not a table: it has no history to share",
            object.source_full_name
        )));
    }
    match object.start_version {
        Some(version) if version < 0 => {
            Err(Error::invalid_argument("start_version cannot be negative"))
        }
        Some(_) if !object.history_shared => Err(Error::invalid_argument(
            "start_version needs history_data_sharing_status ENABLED",
        )),
        _ => Ok(()),
    }
}
