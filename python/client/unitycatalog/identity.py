"""The token providers of cloud identities, for `Client(url, token_provider=...)`.

Each function makes a `TokenProvider` in Rust that gets a token when the client
first sends a request, and a new one before it expires. `Client(url)` without a
token calls `detect` itself, so a program names a provider only to choose one.
"""

from unitycatalog._core import TokenProvider
from unitycatalog._core.identity import (
    aws,
    azure,
    client_credentials,
    detect,
    google,
    token_env,
    token_file,
)

__all__ = [
    "TokenProvider",
    "aws",
    "azure",
    "client_credentials",
    "detect",
    "google",
    "token_env",
    "token_file",
]
