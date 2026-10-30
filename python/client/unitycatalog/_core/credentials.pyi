"""
The classes of the `credentials` API.
"""

from asyncio import Future
from typing import final
from unitycatalog._enums import CredentialPurpose

__all__ = [
    "AwsCredentials",
    "AwsIamRoleRequest",
    "AwsIamRoleResponse",
    "AzureManagedIdentity",
    "AzureServicePrincipalRequest",
    "AzureServicePrincipalResponse",
    "AzureUserDelegationSAS",
    "CredentialClient",
    "CredentialInfo",
    "GcpOauthToken",
    "GcpServiceAccount",
    "GcpServiceAccountKeyRequest",
    "GcpServiceAccountKeyResponse",
    "ListCredentialsResponse",
    "TemporaryCredentials",
]

@final
class AwsCredentials:
    """
    Temporary AWS credentials.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def access_key_id(self, /) -> str:
        """
        The access key ID of the temporary credentials.
        """
    @property
    def secret_access_key(self, /) -> str:
        """
        The secret access key of the temporary credentials.
        """
    @property
    def session_token(self, /) -> str:
        """
        The session token of the temporary credentials.
        """

@final
class AwsIamRoleRequest:
    """
    The AWS IAM role of a new or changed credential.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, role_arn: str) -> AwsIamRoleRequest: ...
    def __repr__(self, /) -> str: ...
    @property
    def role_arn(self, /) -> str:
        """
        ARN of the AWS IAM role that the server assumes to make temporary credentials.
        """

@final
class AwsIamRoleResponse:
    """
    The AWS IAM role of a credential.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def external_id(self, /) -> str:
        """
        External ID that the server sends when it assumes the role. The trust policy of the role
        should require it. Each new role gets a new external ID.
        """
    @property
    def role_arn(self, /) -> str:
        """
        ARN of the AWS IAM role that the server assumes to make temporary credentials.
        """
    @property
    def unity_catalog_iam_arn(self, /) -> str | None:
        """
        ARN of the server's own AWS role, which assumes `role_arn`. Absent when the server has no
        role of its own.
        """

@final
class AzureManagedIdentity:
    """
    The Azure managed identity of a credential: the server's own Azure identity.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, managed_identity_id: str | None = None
    ) -> AzureManagedIdentity: ...
    def __repr__(self, /) -> str: ...
    @property
    def managed_identity_id(self, /) -> str | None:
        """
        Azure resource ID of a user-assigned managed identity, which the server gets from the
        instance metadata service. Absent: the identity that the server's environment gives, such as
        AKS workload identity or the managed identity of the VM.
        """

@final
class AzureServicePrincipalRequest:
    """
    The Azure service principal of a new or changed credential.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, directory_id: str, application_id: str, client_secret: str
    ) -> AzureServicePrincipalRequest: ...
    def __repr__(self, /) -> str: ...
    @property
    def application_id(self, /) -> str:
        """
        Application (client) ID of the service principal.
        """
    @property
    def client_secret(self, /) -> str:
        """
        Client secret of the application. The server stores it and never returns it.
        """
    @property
    def directory_id(self, /) -> str:
        """
        ID of the Microsoft Entra ID tenant (directory) of the application.
        """

@final
class AzureServicePrincipalResponse:
    """
    The Azure service principal of a credential. The client secret is not returned.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def application_id(self, /) -> str:
        """
        Application (client) ID of the service principal.
        """
    @property
    def directory_id(self, /) -> str:
        """
        ID of the Microsoft Entra ID tenant (directory) of the application.
        """

@final
class AzureUserDelegationSAS:
    """
    A shared access signature (SAS) for Azure storage, signed with a user delegation key.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def sas_token(self, /) -> str:
        """
        The SAS token.
        """

@final
class CredentialClient:
    """
    The requests on one `CredentialInfo`.
    """
    def delete(self, /, *, force: bool | None = None) -> None:
        """
        Delete a credential. A credential that external locations use needs `force`.

        Args:
            force: Delete the credential even when external locations use it.
        """
    def delete_async(self, /, *, force: bool | None = None) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> CredentialInfo:
        """
        Get a credential.

        Returns:
            A credential that gives access to storage or to external services.
        """
    def get_async(self, /) -> Future[CredentialInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self,
        /,
        *,
        comment: str | None = None,
        aws_iam_role: AwsIamRoleRequest | None = None,
        new_name: str | None = None,
        azure_service_principal: AzureServicePrincipalRequest | None = None,
        azure_managed_identity: AzureManagedIdentity | None = None,
        gcp_service_account: GcpServiceAccount | None = None,
        gcp_service_account_key: GcpServiceAccountKeyRequest | None = None,
    ) -> CredentialInfo:
        """
        Change a credential.

        Args:
            comment: User-provided free-form text description.
            aws_iam_role: New AWS IAM role of the credential. The role gets a new external ID. Give at
                          most one of `aws_iam_role`, `azure_service_principal`, `azure_managed_identity`,
                          `gcp_service_account` and `gcp_service_account_key`: it replaces the identity of
                          the credential.
            new_name: New name of the credential.
            azure_service_principal: New Azure service principal of the credential.
            azure_managed_identity: New Azure managed identity of the credential.
            gcp_service_account: New GCP service account of the credential.
            gcp_service_account_key: New GCP service account key of the credential.

        Returns:
            A credential that gives access to storage or to external services.
        """
    def update_async(
        self,
        /,
        *,
        comment: str | None = None,
        aws_iam_role: AwsIamRoleRequest | None = None,
        new_name: str | None = None,
        azure_service_principal: AzureServicePrincipalRequest | None = None,
        azure_managed_identity: AzureManagedIdentity | None = None,
        gcp_service_account: GcpServiceAccount | None = None,
        gcp_service_account_key: GcpServiceAccountKeyRequest | None = None,
    ) -> Future[CredentialInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class CredentialInfo:
    """
    A credential that gives access to storage or to external services.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def aws_iam_role(self, /) -> AwsIamRoleResponse | None:
        """
        The AWS IAM role of the credential.
        """
    @property
    def azure_managed_identity(self, /) -> AzureManagedIdentity | None:
        """
        The Azure managed identity of the credential.
        """
    @property
    def azure_service_principal(self, /) -> AzureServicePrincipalResponse | None:
        """
        The Azure service principal of the credential.
        """
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the credential was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the credential.
        """
    @property
    def full_name(self, /) -> str:
        """
        Full name of the credential: the same as `name`.
        """
    @property
    def gcp_service_account(self, /) -> GcpServiceAccount | None:
        """
        The GCP service account of the credential.
        """
    @property
    def gcp_service_account_key(self, /) -> GcpServiceAccountKeyResponse | None:
        """
        The GCP service account key of the credential.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the credential.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the credential.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the credential.
        """
    @property
    def purpose(self, /) -> CredentialPurpose | None:
        """
        What the credential is used for. Absent when the credential was created without one.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the credential was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the credential.
        """

@final
class GcpOauthToken:
    """
    An OAuth token for Google Cloud Storage.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def oauth_token(self, /) -> str:
        """
        The OAuth token.
        """

@final
class GcpServiceAccount:
    """
    The GCP service account of a credential: the server's own identity, or a service account that
    the server impersonates.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, *, email: str | None = None) -> GcpServiceAccount: ...
    def __repr__(self, /) -> str: ...
    @property
    def email(self, /) -> str | None:
        """
        Email of the service account that the server impersonates. The server's identity needs the
        role Service Account Token Creator on it. Absent: the server's own identity, from Application
        Default Credentials or the metadata server.
        """

@final
class GcpServiceAccountKeyRequest:
    """
    The GCP service account key of a new or changed credential.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, json_key: str) -> GcpServiceAccountKeyRequest: ...
    def __repr__(self, /) -> str: ...
    @property
    def json_key(self, /) -> str:
        """
        The JSON key file of the service account. The server stores it and never returns it.
        """

@final
class GcpServiceAccountKeyResponse:
    """
    The GCP service account key of a credential. The key is not returned.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def email(self, /) -> str:
        """
        Email of the service account.
        """
    @property
    def private_key_id(self, /) -> str:
        """
        ID of the key.
        """

@final
class ListCredentialsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def credentials(self, /) -> list[CredentialInfo]:
        """
        The credentials.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """

@final
class TemporaryCredentials:
    """
    Temporary credentials for the cloud of a storage location.
    Empty for a local `file:` location.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def aws_temp_credentials(self, /) -> AwsCredentials | None:
        """
        Temporary credentials for AWS S3.
        """
    @property
    def azure_user_delegation_sas(self, /) -> AzureUserDelegationSAS | None:
        """
        A SAS token for Azure storage.
        """
    @property
    def expiration_time(self, /) -> int | None:
        """
        Time at which the credentials expire, in epoch milliseconds. Absent when the credentials do
        not expire.
        """
    @property
    def gcp_oauth_token(self, /) -> GcpOauthToken | None:
        """
        An OAuth token for Google Cloud Storage.
        """
    @property
    def url(self, /) -> str:
        """
        The storage path that the credentials give access to.
        """
