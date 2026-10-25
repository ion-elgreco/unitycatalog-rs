use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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

use crate::CredentialOps;
use crate::sql::rows::CREDENTIAL;
use crate::sql::{
    SqlMetastore, create_err_as, db_err, enum_str, insert, parse_enum, select_one, touch_audit,
    update,
};
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
    /// Only credential vending reads it: no answer of the API holds it.
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
    /// Only credential vending reads it: no answer of the API holds it.
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

impl CredentialRow {
    /// Replace the identity of the credential. An AWS role gets a new
    /// external id, as upstream.
    fn set_identity(&mut self, identity: Identity) {
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
                self.azure_client_secret = Some(principal.client_secret);
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
                self.gcp_json_key = Some(key.json);
            }
        }
    }

    /// An error unless the credential can reach `url`: S3 needs an AWS
    /// role, ADLS an Azure identity, GCS a GCP identity. Local paths need no
    /// identity.
    pub(crate) fn check_reaches(&self, url: &NormalizedUrl) -> Result<()> {
        let azure = self.azure_application_id.is_some() || self.azure_managed_identity;
        let gcp = self.gcp_service_account || self.gcp_json_key.is_some();
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
        row.set_identity(identity);

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

    async fn get_with_secret(&self, name: &str) -> Result<(CredentialInfo, Option<String>)> {
        let row = self
            .credential_row(name)
            .await?
            .ok_or_else(|| Error::not_found(format!("Storage credential not found: {name}")))?;
        let secret = row
            .azure_client_secret
            .clone()
            .or_else(|| row.gcp_json_key.clone());
        Ok((row.try_into()?, secret))
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
            row.set_identity(identity);
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
