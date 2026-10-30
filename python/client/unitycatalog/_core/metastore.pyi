"""
The classes of the `metastore` API.
"""

from typing import final

__all__ = ["GetMetastoreSummaryResponse"]

@final
class GetMetastoreSummaryResponse:
    """
    A summary of the metastore of the server.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def metastore_id(self, /) -> str:
        """
        Unique identifier of the metastore.
        """
