//! `unitycatalog._core.credentials`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_api::CredentialPurpose;

use crate::str_enum::StrEnum;
use crate::utils::repr;

/// The AWS IAM role of a credential.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.credentials"
)]
#[derive(Clone, PartialEq)]
pub struct AwsIamRole {
    /// The ARN of the role that the server assumes.
    pub role_arn: Option<String>,
    /// The ARN of the server's own role, which assumes `role_arn`.
    pub unity_catalog_iam_arn: Option<String>,
    /// The external id that the trust policy of the role should require.
    /// It changes each time the role is set.
    pub external_id: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl AwsIamRole {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = ["role_arn", "unity_catalog_iam_arn", "external_id"];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::AwsIamRoleResponse> for AwsIamRole {
    fn from(r: uc_api::AwsIamRoleResponse) -> Self {
        Self {
            role_arn: r.role_arn,
            unity_catalog_iam_arn: r.unity_catalog_iam_arn,
            external_id: r.external_id,
        }
    }
}

/// A credential: access to cloud storage, for external locations.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.credentials"
)]
#[derive(Clone, PartialEq)]
pub struct CredentialInfo {
    /// The name of the credential.
    pub name: Option<String>,
    /// The AWS IAM role.
    pub aws_iam_role: Option<AwsIamRole>,
    /// A description.
    pub comment: Option<String>,
    /// The username of the owner.
    pub owner: Option<String>,
    /// The full name, the same as the name.
    pub full_name: Option<String>,
    /// The unique identifier.
    pub id: Option<String>,
    /// The creation time, in epoch milliseconds.
    pub created_at: Option<i64>,
    /// The username of the creator.
    pub created_by: Option<String>,
    /// The time of the last update, in epoch milliseconds.
    pub updated_at: Option<i64>,
    /// The username of the last updater.
    pub updated_by: Option<String>,
    /// What the credential is for.
    pub purpose: Option<StrEnum<CredentialPurpose>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl CredentialInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "aws_iam_role",
            "comment",
            "owner",
            "full_name",
            "id",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "purpose",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::CredentialInfo> for CredentialInfo {
    fn from(c: uc_api::CredentialInfo) -> Self {
        Self {
            name: c.name,
            aws_iam_role: c.aws_iam_role.map(AwsIamRole::from),
            comment: c.comment,
            owner: c.owner,
            full_name: c.full_name,
            id: c.id,
            created_at: c.created_at,
            created_by: c.created_by,
            updated_at: c.updated_at,
            updated_by: c.updated_by,
            purpose: c.purpose.map(StrEnum),
        }
    }
}

/// Temporary AWS credentials. The `repr` leaves out the secret and the
/// session token.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.credentials"
)]
#[derive(Clone, PartialEq)]
pub struct AwsCredentials {
    /// The access key id.
    pub access_key_id: Option<String>,
    /// The secret access key.
    pub secret_access_key: Option<String>,
    /// The session token.
    pub session_token: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl AwsCredentials {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        // Not the secrets: a repr ends up in logs.
        repr(slf.as_any(), &["access_key_id"])
    }
}

impl From<uc_api::AwsCredentials> for AwsCredentials {
    fn from(c: uc_api::AwsCredentials) -> Self {
        Self {
            access_key_id: c.access_key_id,
            secret_access_key: c.secret_access_key,
            session_token: c.session_token,
        }
    }
}

/// An Azure user delegation SAS token. The `repr` leaves it out.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.credentials"
)]
#[derive(Clone, PartialEq)]
pub struct AzureUserDelegationSas {
    /// The SAS token.
    pub sas_token: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl AzureUserDelegationSas {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &[])
    }
}

impl From<uc_api::AzureUserDelegationSAS> for AzureUserDelegationSas {
    fn from(s: uc_api::AzureUserDelegationSAS) -> Self {
        Self {
            sas_token: s.sas_token,
        }
    }
}

/// A GCP OAuth token. The `repr` leaves it out.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.credentials"
)]
#[derive(Clone, PartialEq)]
pub struct GcpOauthToken {
    /// The OAuth token.
    pub oauth_token: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GcpOauthToken {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        repr(slf.as_any(), &[])
    }
}

impl From<uc_api::GcpOauthToken> for GcpOauthToken {
    fn from(t: uc_api::GcpOauthToken) -> Self {
        Self {
            oauth_token: t.oauth_token,
        }
    }
}

/// Temporary credentials for a table, a volume or a path: those of its
/// cloud, or none for local storage.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.credentials"
)]
#[derive(Clone, PartialEq)]
pub struct TemporaryCredentials {
    /// The credentials for AWS S3.
    pub aws_temp_credentials: Option<AwsCredentials>,
    /// The credentials for Azure storage.
    pub azure_user_delegation_sas: Option<AzureUserDelegationSas>,
    /// The credentials for Google Cloud Storage.
    pub gcp_oauth_token: Option<GcpOauthToken>,
    /// The expiry time, in epoch milliseconds.
    pub expiration_time: Option<i64>,
    /// The URL of the storage that the credentials are for.
    pub url: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl TemporaryCredentials {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "aws_temp_credentials",
            "azure_user_delegation_sas",
            "gcp_oauth_token",
            "expiration_time",
            "url",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::TemporaryCredentials> for TemporaryCredentials {
    fn from(c: uc_api::TemporaryCredentials) -> Self {
        Self {
            aws_temp_credentials: c.aws_temp_credentials.map(AwsCredentials::from),
            azure_user_delegation_sas: c
                .azure_user_delegation_sas
                .map(AzureUserDelegationSas::from),
            gcp_oauth_token: c.gcp_oauth_token.map(GcpOauthToken::from),
            expiration_time: c.expiration_time,
            url: c.url,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.credentials",
    "The classes of the credential and temporary credential methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(
        parent,
        "credentials",
        [
            CredentialInfo,
            AwsIamRole,
            TemporaryCredentials,
            AwsCredentials,
            AzureUserDelegationSas,
            GcpOauthToken
        ]
    )
}
