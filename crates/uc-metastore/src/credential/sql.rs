use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    AwsIamRoleResponse, CreateCredentialRequest, CredentialInfo, CredentialPurpose,
    UpdateCredentialRequest,
};
use uc_common::{Error, ErrorCode, Pagination, Result, now_epoch_millis, validate_sql_object_name};

use crate::CredentialOps;
use crate::sql::rows::CREDENTIAL;
use crate::sql::{
    SqlMetastore, create_err_as, db_err, enum_str, insert, parse_enum, select_one, touch_audit,
    update,
};
use uc_storage::Value;

/// A storage credential row, keyed by its UUID. External locations link to
/// it by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CredentialRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) aws_role_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) aws_external_id: Option<String>,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated_by: Option<String>,
}

impl TryFrom<CredentialRow> for CredentialInfo {
    type Error = Error;
    fn try_from(r: CredentialRow) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            full_name: Some(r.name.clone()),
            aws_iam_role: r.aws_role_arn.map(|arn| AwsIamRoleResponse {
                role_arn: Some(arn),
                external_id: r.aws_external_id,
                unity_catalog_iam_arn: None,
            }),
            name: Some(r.name),
            comment: r.comment,
            owner: r.owner,
            id: Some(r.id_string),
            created_at: Some(r.created_at),
            created_by: r.created_by,
            updated_at: Some(r.updated_at),
            updated_by: r.updated_by,
            purpose: r
                .purpose
                .as_deref()
                .map(parse_enum::<CredentialPurpose>)
                .transpose()?,
        })
    }
}

fn credential_exists(name: &str) -> Error {
    Error::new(
        ErrorCode::StorageCredentialAlreadyExists,
        format!("Storage credential already exists: {name}"),
    )
}

impl SqlMetastore {
    /// The credential named `name`, if any.
    pub(crate) async fn credential_row(&self, name: &str) -> Result<Option<CredentialRow>> {
        select_one(
            self.db.as_ref(),
            &CREDENTIAL,
            "WHERE name = $1",
            &[name.into()],
        )
        .await
    }
}

#[async_trait]
impl CredentialOps for SqlMetastore {
    async fn create(
        &self,
        req: CreateCredentialRequest,
        created_by: Option<&str>,
    ) -> Result<CredentialInfo> {
        validate_sql_object_name(&req.name)?;
        // Upstream `CredentialDAO.from`: an AWS IAM role is the only type.
        let role = req
            .aws_iam_role
            .ok_or_else(|| Error::invalid_argument("Unknown credential type"))?;
        if self.credential_row(&req.name).await?.is_some() {
            return Err(credential_exists(&req.name));
        }

        let now = now_epoch_millis();
        let id = Uuid::new_v4().to_string();
        let row = CredentialRow {
            id_string: id,
            name: req.name.clone(),
            comment: req.comment,
            owner: created_by.map(str::to_string),
            purpose: req.purpose.map(|p| enum_str(&p)),
            aws_role_arn: Some(role.role_arn),
            aws_external_id: Some(Uuid::new_v4().to_string()),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
        };

        insert(self.db.as_ref(), &CREDENTIAL, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::StorageCredentialAlreadyExists,
                format!("Storage credential already exists: {}", req.name),
            ))?;
        row.try_into()
    }

    async fn get(&self, name: &str) -> Result<CredentialInfo> {
        self.credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Storage credential not found: {name}")))
            .and_then(TryInto::try_into)
    }

    async fn update(
        &self,
        name: &str,
        req: UpdateCredentialRequest,
        updated_by: Option<&str>,
    ) -> Result<CredentialInfo> {
        if let Some(new_name) = &req.new_name {
            validate_sql_object_name(new_name)?;
        }
        let mut row = self
            .credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Storage credential not found: {name}")))?;

        // External locations link to the credential by id, so a rename is a
        // field update. Upstream rejects any new name that exists, the
        // current one included.
        if let Some(new_name) = req.new_name {
            if self.credential_row(&new_name).await?.is_some() {
                return Err(credential_exists(&new_name));
            }
            row.name = new_name;
        }
        // A new role gets a new external id, as upstream.
        if let Some(role) = req.aws_iam_role {
            row.aws_role_arn = Some(role.role_arn);
            row.aws_external_id = Some(Uuid::new_v4().to_string());
        }
        if let Some(c) = req.comment {
            row.comment = Some(c);
        }
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);

        let written = update(self.db.as_ref(), &CREDENTIAL, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::StorageCredentialAlreadyExists,
                format!("Storage credential already exists: {}", row.name),
            ))?;
        if written == 0 {
            return Err(Error::not_found(format!(
                "Storage credential not found: {name}"
            )));
        }
        row.try_into()
    }

    async fn delete(&self, name: &str, force: bool) -> Result<()> {
        // Upstream order: existence first, then (without force) the check
        // that no external location still uses the credential.
        let existing = self
            .credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Credential not found: {name}")))?;
        let id = [existing.id_string.as_str().into()];
        let failed = db_err("delete credential");
        if !force {
            let users = self
                .db
                .query(
                    "SELECT name FROM external_location WHERE credential_id = $1 LIMIT 1",
                    &id,
                )
                .await
                .map_err(&failed)?;
            if let Some(location) = users.first() {
                return Err(Error::invalid_argument(format!(
                    "Credential still used by external location '{}'",
                    location.text(0).map_err(&failed)?
                )));
            }
        }
        let tx = self.db.begin().await.map_err(&failed)?;
        for sql in [
            "DELETE FROM permission WHERE resource_id = $1",
            "DELETE FROM credential WHERE id_string = $1",
        ] {
            tx.execute(sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn list(
        &self,
        purpose: Option<CredentialPurpose>,
        page: &Pagination,
    ) -> Result<(Vec<CredentialInfo>, Option<String>)> {
        // Filter in the query, so that a page is never cut short.
        let (filters, params): (&[&str], Vec<Value>) = match purpose {
            Some(p) => (&["purpose = $1"], vec![enum_str(&p).into()]),
            None => (&[], vec![]),
        };
        let (rows, next) = self
            .paged_list::<CredentialRow, _>(&CREDENTIAL, filters, params, "name", page, |r| {
                r.name.clone()
            })
            .await?;
        let infos: Result<Vec<CredentialInfo>> = rows.into_iter().map(TryInto::try_into).collect();
        Ok((infos?, next))
    }
}
