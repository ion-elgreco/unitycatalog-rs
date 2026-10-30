"""
The classes of the `health` API.
"""

from typing import final

__all__ = ["HealthResponse", "VersionResponse"]

@final
class HealthResponse:
    """
    The state of the server.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def status(self, /) -> str:
        """
        `ok`: the server answers requests.
        """

@final
class VersionResponse:
    """
    The server and its versions.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def api_version(self, /) -> str:
        """
        Version of the Unity Catalog API that the server implements: `2.1`.
        """
    @property
    def server(self, /) -> str:
        """
        Name of the server: `unitycatalog-rs`.
        """
    @property
    def version(self, /) -> str:
        """
        Version of the server, such as `0.1.0`.
        """
