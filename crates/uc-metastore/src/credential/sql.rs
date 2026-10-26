use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use uc_api::{
    AwsIamRoleRequest, AwsIamRoleResponse, AzureManagedIdentity, AzureServicePrincipalRequest,
    AzureServicePrincipalResponse, CreateCredentialRequest, CredentialInfo, CredentialPurpose,
    GcpServiceAccount, GcpServiceAccountKeyRequest, GcpServiceAccountKeyResponse,
    UpdateCredentialRequest,
};
use uc_common::{
    Error, ErrorCode, NormalizedUrl, Pagination, Result, UriScheme, now_epoch_millis,
    require_non_empty, validate_sql_object_name,
};

use crate::sql::rows::CREDENTIAL;
use crate::sql::{
    SqlMetastore, create_err_as, db_err, enum_str, insert, parse_enum, select, select_one,
    touch_audit, update,
};
use crate::{CredentialOps, RefreshReport, SecretRef, SecretValue};
use uc_storage::Value;

/// A storage credential row, keyed by its UUID. External locations link to
/// it by id. It holds one identity: an AWS role, an Azure service principal,
/// an Azure managed identity, a GCP service account or a GCP service account
/// key.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) azure_directory_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) azure_application_id: Option<String>,
    /// The reference of the secret store ([`crate::secrets`]): for the
    /// database store, the sealed secret. Only credential vending reads the
    /// secret, and no answer of the API holds it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) azure_client_secret: Option<String>,
    pub(crate) azure_managed_identity: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) azure_managed_identity_id: Option<String>,
    pub(crate) gcp_service_account: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gcp_service_account_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gcp_key_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gcp_key_id: Option<String>,
    /// The reference of the secret store, as `azure_client_secret`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gcp_json_key: Option<String>,
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
            azure_service_principal: r.azure_application_id.map(|application_id| {
                AzureServicePrincipalResponse {
                    directory_id: r.azure_directory_id,
                    application_id: Some(application_id),
                }
            }),
            azure_managed_identity: r.azure_managed_identity.then_some(AzureManagedIdentity {
                managed_identity_id: r.azure_managed_identity_id,
            }),
            gcp_service_account: r.gcp_service_account.then_some(GcpServiceAccount {
                email: r.gcp_service_account_email,
            }),
            gcp_service_account_key: r.gcp_json_key.is_some().then_some(
                GcpServiceAccountKeyResponse {
                    email: r.gcp_key_email,
                    private_key_id: r.gcp_key_id,
                },
            ),
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

/// The identity of a new or changed credential.
enum Identity {
    AwsIamRole(AwsIamRoleRequest),
    AzureServicePrincipal(AzureServicePrincipalRequest),
    AzureManagedIdentity(AzureManagedIdentity),
    GcpServiceAccount(GcpServiceAccount),
    GcpServiceAccountKey(GcpKey),
}

/// A GCP service account key, and the fields that answers show.
struct GcpKey {
    json: String,
    email: String,
    id: Option<String>,
}

impl GcpKey {
    fn parse(key: GcpServiceAccountKeyRequest) -> Result<Self> {
        let invalid = |reason: &str| {
            Error::invalid_argument(format!("json_key is not a service account key: {reason}."))
        };
        let fields: serde_json::Value =
            serde_json::from_str(&key.json_key).map_err(|e| invalid(&e.to_string()))?;
        let field = |name: &str| fields[name].as_str().filter(|value| !value.is_empty());
        if field("type") != Some("service_account") {
            return Err(invalid("its type is not service_account"));
        }
        if field("private_key").is_none() {
            return Err(invalid("it has no private_key"));
        }
        let email = field("client_email")
            .ok_or_else(|| invalid("it has no client_email"))?
            .to_string();
        let id = field("private_key_id").map(str::to_string);
        Ok(Self {
            json: key.json_key,
            email,
            id,
        })
    }
}

/// The identities of a request.
struct Identities {
    aws_iam_role: Option<AwsIamRoleRequest>,
    azure_service_principal: Option<AzureServicePrincipalRequest>,
    azure_managed_identity: Option<AzureManagedIdentity>,
    gcp_service_account: Option<GcpServiceAccount>,
    gcp_service_account_key: Option<GcpServiceAccountKeyRequest>,
}

impl Identity {
    /// The identity of a request, if it gives one. It may give only one.
    fn of(request: Identities) -> Result<Option<Self>> {
        let mut identities = [
            request.aws_iam_role.map(Self::AwsIamRole),
            request
                .azure_service_principal
                .map(Self::AzureServicePrincipal),
            request
                .azure_managed_identity
                .map(Self::AzureManagedIdentity),
            request.gcp_service_account.map(Self::GcpServiceAccount),
            request
                .gcp_service_account_key
                .map(GcpKey::parse)
                .transpose()?
                .map(Self::GcpServiceAccountKey),
        ]
        .into_iter()
        .flatten();
        let identity = identities.next();
        if identities.next().is_some() {
            return Err(Error::invalid_argument(
                "Give only one of aws_iam_role, azure_service_principal, \
                 azure_managed_identity, gcp_service_account and gcp_service_account_key.",
            ));
        }
        match &identity {
            Some(Self::AzureServicePrincipal(principal)) => {
                require_non_empty("directory_id", &principal.directory_id)?;
                require_non_empty("application_id", &principal.application_id)?;
                require_non_empty("client_secret", &principal.client_secret)?;
            }
            // The server puts the email in the path of a request to Google.
            Some(Self::GcpServiceAccount(GcpServiceAccount { email: Some(email) }))
                if !email
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "@.-_".contains(c)) =>
            {
                return Err(Error::invalid_argument(format!(
                    "Not a service account email: {email}"
                )));
            }
            _ => {}
        }
        Ok(identity)
    }
}

/// The column of a secret.
#[derive(Debug, Clone, Copy)]
enum SecretColumn {
    AzureClientSecret,
    GcpJsonKey,
}

impl SecretColumn {
    fn name(self) -> &'static str {
        match self {
            Self::AzureClientSecret => "azure_client_secret",
            Self::GcpJsonKey => "gcp_json_key",
        }
    }
}

/// The secret of a new identity, until the secret store has it.
struct PendingSecret {
    column: SecretColumn,
    secret: SecretValue,
}

impl CredentialRow {
    /// Replace the identity of the credential; its secret, if it has one,
    /// for the secret store. An AWS role gets a new external id, as
    /// upstream.
    fn set_identity(&mut self, identity: Identity) -> Option<PendingSecret> {
        self.aws_role_arn = None;
        self.aws_external_id = None;
        self.azure_directory_id = None;
        self.azure_application_id = None;
        self.azure_client_secret = None;
        self.azure_managed_identity = false;
        self.azure_managed_identity_id = None;
        self.gcp_service_account = false;
        self.gcp_service_account_email = None;
        self.gcp_key_email = None;
        self.gcp_key_id = None;
        self.gcp_json_key = None;
        match identity {
            Identity::AwsIamRole(role) => {
                self.aws_role_arn = Some(role.role_arn);
                self.aws_external_id = Some(Uuid::new_v4().to_string());
            }
            Identity::AzureServicePrincipal(principal) => {
                self.azure_directory_id = Some(principal.directory_id);
                self.azure_application_id = Some(principal.application_id);
                return Some(PendingSecret {
                    column: SecretColumn::AzureClientSecret,
                    secret: SecretValue::from(principal.client_secret),
                });
            }
            Identity::AzureManagedIdentity(identity) => {
                self.azure_managed_identity = true;
                self.azure_managed_identity_id =
                    identity.managed_identity_id.filter(|id| !id.is_empty());
            }
            Identity::GcpServiceAccount(account) => {
                self.gcp_service_account = true;
                self.gcp_service_account_email = account.email.filter(|email| !email.is_empty());
            }
            Identity::GcpServiceAccountKey(key) => {
                self.gcp_key_email = Some(key.email);
                self.gcp_key_id = key.id;
                return Some(PendingSecret {
                    column: SecretColumn::GcpJsonKey,
                    secret: SecretValue::from(key.json),
                });
            }
        }
        None
    }

    /// The reference of the credential's secret, if it has one.
    fn secret_ref(&self) -> Option<&String> {
        self.azure_client_secret
            .as_ref()
            .or(self.gcp_json_key.as_ref())
    }

    /// An error unless the credential can reach `url`: S3 needs an AWS
    /// role, ADLS an Azure identity, GCS a GCP identity. Local paths need no
    /// identity.
    pub(crate) fn check_reaches(&self, url: &NormalizedUrl) -> Result<()> {
        let azure = self.azure_application_id.is_some() || self.azure_managed_identity;
        let gcp = self.gcp_service_account || self.gcp_key_email.is_some();
        let need = match url.scheme() {
            UriScheme::File => return Ok(()),
            UriScheme::S3 if self.aws_role_arn.is_some() => return Ok(()),
            UriScheme::Abfs | UriScheme::Abfss if azure => return Ok(()),
            UriScheme::Gs if gcp => return Ok(()),
            UriScheme::S3 => "an AWS IAM role",
            UriScheme::Abfs | UriScheme::Abfss => "an Azure service principal or managed identity",
            UriScheme::Gs => "a GCP service account or service account key",
        };
        Err(Error::invalid_argument(format!(
            "Credential {} cannot reach {url}: it needs {need}.",
            self.name
        )))
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

    /// Give the secret store the secret of `row`'s new identity, and keep
    /// its reference in the row.
    async fn store_secret(&self, row: &mut CredentialRow, pending: PendingSecret) -> Result<()> {
        let reference = self
            .secrets
            .put(&row.id_string, &pending.secret)
            .await?
            .into_string();
        match pending.column {
            SecretColumn::AzureClientSecret => row.azure_client_secret = Some(reference),
            SecretColumn::GcpJsonKey => row.gcp_json_key = Some(reference),
        }
        Ok(())
    }

    /// Remove from the secret store a secret that no row keeps. A failure
    /// only leaves the secret in the store.
    async fn drop_secret(&self, owner: &str, reference: Option<String>) {
        let Some(reference) = reference else {
            return;
        };
        if let Err(error) = self.secrets.delete(owner, &SecretRef::new(reference)).await {
            warn!(credential = owner, %error, "cannot remove an old secret from the secret store");
        }
    }

    /// The secret of `row`, if it has one, from the secret store.
    async fn open_secret(&self, row: &CredentialRow) -> Result<Option<SecretValue>> {
        let Some(reference) = row.secret_ref() else {
            return Ok(None);
        };
        let cannot = |reason: &str| {
            format!(
                "The server cannot open the secret of credential {}: {reason}.",
                row.name
            )
        };
        self.secrets
            .get(&row.id_string, &SecretRef::new(reference.as_str()))
            .await
            .map(Some)
            .map_err(|e| Error::new(e.code(), cannot(e.message())))
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
        let identity = Identity::of(Identities {
            aws_iam_role: req.aws_iam_role,
            azure_service_principal: req.azure_service_principal,
            azure_managed_identity: req.azure_managed_identity,
            gcp_service_account: req.gcp_service_account,
            gcp_service_account_key: req.gcp_service_account_key,
        })?
        .ok_or_else(|| Error::invalid_argument("Unknown credential type"))?;
        if self.credential_row(&req.name).await?.is_some() {
            return Err(credential_exists(&req.name));
        }

        let now = now_epoch_millis();
        let id = Uuid::new_v4().to_string();
        let mut row = CredentialRow {
            id_string: id,
            name: req.name.clone(),
            comment: req.comment,
            owner: created_by.map(str::to_string),
            purpose: req.purpose.map(|p| enum_str(&p)),
            aws_role_arn: None,
            aws_external_id: None,
            azure_directory_id: None,
            azure_application_id: None,
            azure_client_secret: None,
            azure_managed_identity: false,
            azure_managed_identity_id: None,
            gcp_service_account: false,
            gcp_service_account_email: None,
            gcp_key_email: None,
            gcp_key_id: None,
            gcp_json_key: None,
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
        };
        if let Some(pending) = row.set_identity(identity) {
            self.store_secret(&mut row, pending).await?;
        }

        let inserted = insert(self.db.as_ref(), &CREDENTIAL, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::StorageCredentialAlreadyExists,
                format!("Storage credential already exists: {}", req.name),
            ));
        if let Err(error) = inserted {
            self.drop_secret(&row.id_string, row.secret_ref().cloned())
                .await;
            return Err(error);
        }
        row.try_into()
    }

    async fn get(&self, name: &str) -> Result<CredentialInfo> {
        self.credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Storage credential not found: {name}")))
            .and_then(TryInto::try_into)
    }

    async fn get_with_secret(&self, name: &str) -> Result<(CredentialInfo, Option<SecretValue>)> {
        let row = self
            .credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Storage credential not found: {name}")))?;
        let secret = self.open_secret(&row).await?;
        Ok((row.try_into()?, secret))
    }

    async fn refresh_secrets(&self) -> Result<RefreshReport> {
        let mut report = RefreshReport::default();
        if !self.secrets.refreshes() {
            return Ok(report);
        }
        let rows: Vec<CredentialRow> = select(
            self.db.as_ref(),
            &CREDENTIAL,
            "WHERE azure_client_secret IS NOT NULL OR gcp_json_key IS NOT NULL ORDER BY name",
            &[],
        )
        .await?;
        for row in rows {
            let (column, reference) = match (&row.azure_client_secret, &row.gcp_json_key) {
                (Some(reference), _) => (SecretColumn::AzureClientSecret, reference),
                (None, Some(reference)) => (SecretColumn::GcpJsonKey, reference),
                (None, None) => continue,
            };
            let refreshed = match self
                .secrets
                .refresh(&row.id_string, &SecretRef::new(reference.as_str()))
                .await
            {
                Ok(Some(refreshed)) => refreshed.into_string(),
                Ok(None) => continue,
                Err(_) => {
                    report.unreadable.push(row.name);
                    continue;
                }
            };
            // Another server may have refreshed it first; then the secret
            // that this one stored is not used.
            let column = column.name();
            let written = self
                .db
                .execute(
                    &format!(
                        "UPDATE credential SET {column} = $1 WHERE id_string = $2 AND {column} = $3"
                    ),
                    &[
                        refreshed.as_str().into(),
                        row.id_string.as_str().into(),
                        reference.as_str().into(),
                    ],
                )
                .await
                .map_err(db_err("refresh secrets"))?;
            let unused = match written {
                0 => refreshed,
                _ => reference.clone(),
            };
            report.refreshed += written as usize;
            self.drop_secret(&row.id_string, Some(unused)).await;
        }
        Ok(report)
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
        let identity = Identity::of(Identities {
            aws_iam_role: req.aws_iam_role,
            azure_service_principal: req.azure_service_principal,
            azure_managed_identity: req.azure_managed_identity,
            gcp_service_account: req.gcp_service_account,
            gcp_service_account_key: req.gcp_service_account_key,
        })?;
        let mut row = self
            .credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Storage credential not found: {name}")))?;
        let old_secret = row.secret_ref().cloned();
        let replaced = identity.is_some();

        // External locations link to the credential by id, so a rename is a
        // field update. Upstream rejects any new name that exists, the
        // current one included.
        if let Some(new_name) = req.new_name {
            if self.credential_row(&new_name).await?.is_some() {
                return Err(credential_exists(&new_name));
            }
            row.name = new_name;
        }
        if let Some(identity) = identity {
            let pending = row.set_identity(identity);
            let urls = self
                .db
                .query(
                    "SELECT url FROM external_location WHERE credential_id = $1",
                    &[row.id_string.as_str().into()],
                )
                .await
                .map_err(db_err("update credential"))?;
            for url in &urls {
                let url = url.text(0).map_err(db_err("update credential"))?;
                row.check_reaches(&NormalizedUrl::parse(&url)?)?;
            }
            if let Some(pending) = pending {
                self.store_secret(&mut row, pending).await?;
            }
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
            ))
            .and_then(|written| match written {
                0 => Err(Error::not_found(format!(
                    "Storage credential not found: {name}"
                ))),
                _ => Ok(()),
            });
        let new_secret = row.secret_ref().cloned();
        if replaced && old_secret != new_secret {
            // The row keeps one of the two secrets: remove the other.
            let unused = match written {
                Ok(()) => old_secret,
                Err(_) => new_secret,
            };
            self.drop_secret(&row.id_string, unused).await;
        }
        written?;
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
        tx.commit().await.map_err(&failed)?;
        self.drop_secret(&existing.id_string, existing.secret_ref().cloned())
            .await;
        Ok(())
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
