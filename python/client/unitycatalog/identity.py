"""The token sources of cloud identities, for `Client(url, token_provider=...)`.

Each function makes a `TokenSource` in Rust that gets a token when the client
first sends a request, and a new one before it expires. `Client(url)` without a
token calls `detect` itself, so a program names a source only to choose one.
"""

from unitycatalog._core import TokenSource
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
    "TokenSource",
    "aws",
    "azure",
    "client_credentials",
    "detect",
    "google",
    "token_env",
    "token_file",
]
