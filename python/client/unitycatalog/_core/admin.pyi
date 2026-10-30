"""
The classes of the `admin` API.
"""

from typing import Any, final

__all__ = ["BackupResponse", "RestoreResponse"]

@final
class BackupResponse:
    """
    A snapshot of the whole catalog: the rows of each table of the store. The audit log is not in
    it.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def tables(self, /) -> dict[str, list[Any]]:
        """
        The rows of each table, by table name. A row is a JSON object of its columns, as the table
        of the store has them.
        """
    @property
    def version(self, /) -> int:
        """
        Version of the snapshot format. A restore takes only version 1.
        """

@final
class RestoreResponse:
    """
    What a restore imported.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def imported(self, /) -> dict[str, int]:
        """
        Number of rows imported, by table name.
        """
    @property
    def skipped(self, /) -> list[str]:
        """
        The rows left out, each as `<table>: <reason>`, such as a row without its key.
        """
