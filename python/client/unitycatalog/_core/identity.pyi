"""
The token providers of cloud identities, for `Client(url, token_provider=...)`.
"""

from . import TokenProvider

__all__ = [
    "aws",
    "azure",
    "client_credentials",
    "detect",
    "google",
    "token_env",
    "token_file",
]

def aws(server_id: str) -> TokenProvider:
    """
    An AWS identity of the environment, for the server whose AWS issuer block has the audience
    `server_id`, such as `https://uc.example.com`. Each token is a signed `GetCallerIdentity`
    request, which the server sends to STS: the credentials stay here. They are the access keys
    of `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` (with `AWS_SESSION_TOKEN`), a web identity
    (`AWS_ROLE_ARN`, `AWS_WEB_IDENTITY_TOKEN_FILE`), the container's credentials, else the
    instance role of the machine.
    """

def azure(scope: str) -> TokenProvider:
    """
    An Azure identity of the environment, for the scope `scope`, such as
    `https://uc.example.com/.default`: a service principal (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`,
    `AZURE_CLIENT_SECRET`), a workload identity (`AZURE_FEDERATED_TOKEN_FILE`), a managed identity
    that the environment names (`AZURE_CLIENT_ID`, `IDENTITY_ENDPOINT` or `MSI_ENDPOINT`), the
    sign-in of the Azure CLI when `az` is on the path, else the managed identity of the machine or
    pod.
    """

def client_credentials(
    token_url: str,
    client_id: str,
    client_secret: str,
    *,
    scope: str | None = None,
    audience: str | None = None,
) -> TokenProvider:
    """
    An OAuth client of any provider, with the client-credentials grant at `token_url`.
    """

def detect(base_url: str) -> TokenProvider | None:
    """
    The token provider of the environment for the server at `base_url`, if it has one: `UC_TOKEN`
    or `UC_TOKEN_FILE`, an OAuth client in `UC_OAUTH_*`, else an Azure, Google or AWS identity at
    one of the server's issuers, for the audience that `UC_AUDIENCE` names, else the server's
    URL. `Client(base_url)` without a token does this itself. A server that cannot say which
    providers it takes yet is asked again at each request, which goes without a token until it
    answers.
    """

def google(audience: str) -> TokenProvider:
    """
    A Google identity of the environment, as an ID token for `audience`: the service-account
    key file of `GOOGLE_APPLICATION_CREDENTIALS`, the sign-in of the Google Cloud CLI when
    `gcloud` is on the path, else the metadata server of the machine or pod.
    """

def token_env(var: str) -> TokenProvider:
    """
    The token in the environment variable `var`, read again at each refresh.
    """

def token_file(path: str) -> TokenProvider:
    """
    The token in the file `path`, read again at each refresh: a projected Kubernetes
    service-account token, for one.
    """
