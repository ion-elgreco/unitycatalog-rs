"""
The token sources of cloud identities, for `Client(url, token_provider=...)`.
"""

from . import TokenSource

__all__ = ["azure", "client_credentials", "detect", "google", "token_env", "token_file"]

def azure(scope: str) -> TokenSource:
    """
    An Azure identity of the environment, for the scope `scope`, such as `api://uc/.default`: a
    service principal (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`), a workload
    identity (`AZURE_FEDERATED_TOKEN_FILE`), the sign-in of the Azure CLI when `az` is on the
    path, else the managed identity of the machine or pod.
    """

def client_credentials(
    token_url: str,
    client_id: str,
    client_secret: str,
    *,
    scope: str | None = None,
    audience: str | None = None,
) -> TokenSource:
    """
    An OAuth client of any provider, with the client-credentials grant at `token_url`.
    """

def detect(base_url: str) -> TokenSource | None:
    """
    The token source of the environment for the server at `base_url`, if it has one: `UC_TOKEN`
    or `UC_TOKEN_FILE`, an OAuth client in `UC_OAUTH_*`, else an Azure or Google identity at one
    of the server's issuers. `Client(base_url)` without a token does this itself.
    """

def google(audience: str) -> TokenSource:
    """
    A Google identity of the environment, as an ID token for `audience`: the service-account
    key file of `GOOGLE_APPLICATION_CREDENTIALS`, the sign-in of the Google Cloud CLI when
    `gcloud` is on the path, else the metadata server of the machine or pod.
    """

def token_env(var: str) -> TokenSource:
    """
    The token in the environment variable `var`, read again at each refresh.
    """

def token_file(path: str) -> TokenSource:
    """
    The token in the file `path`, read again at each refresh: a projected Kubernetes
    service-account token, for one.
    """
