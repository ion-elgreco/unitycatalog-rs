"""
The classes of the `sharing` API.
"""

from typing import final

__all__ = [
    "RecipientProfile",
    "SharingFormat",
    "SharingGetTableVersionResponse",
    "SharingListSchemasResponse",
    "SharingListSharesResponse",
    "SharingListTablesResponse",
    "SharingMetadata",
    "SharingProtocol",
    "SharingSchema",
    "SharingShare",
    "SharingTable",
    "SharingTableLine",
]

@final
class RecipientProfile:
    """
    A Delta Sharing profile file (`profile.json`): what a Delta Sharing client needs to connect to
    the server as a recipient.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def bearer_token(self, /) -> str:
        """
        The bearer token of the recipient.
        """
    @property
    def endpoint(self, /) -> str:
        """
        URL of the Delta Sharing server: the URL the caller used, ending in
        `/api/2.1/unity-catalog/delta-sharing`.
        """
    @property
    def expiration_time(self, /) -> str | None:
        """
        Time at which the token expires, as an ISO 8601 timestamp. The server sends `null`: the tokens
        do not expire.
        """
    @property
    def share_credentials_version(self, /) -> int:
        """
        Version of the profile file format. Always 1.
        """

@final
class SharingFormat:
    """
    The format of the data files of a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def provider(self, /) -> str:
        """
        Name of the file format. Always `parquet`.
        """

@final
class SharingGetTableVersionResponse:
    """
    The version of a table. The response has no body.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def version(self, /) -> int:
        """
        Version of the table, from the `Delta-Table-Version` header. Always 0: the server does not
        read the Delta log.
        """

@final
class SharingListSchemasResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items(self, /) -> list[SharingSchema]:
        """
        The schemas, sorted by name.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. The server does not send it: the first page holds all
        schemas.
        """

@final
class SharingListSharesResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items(self, /) -> list[SharingShare]:
        """
        The shares.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. The server does not send it: the first page holds all shares.
        """

@final
class SharingListTablesResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items(self, /) -> list[SharingTable]:
        """
        The tables.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. The server does not send it: the first page holds all tables.
        """

@final
class SharingMetadata:
    """
    The metadata of a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def configuration(self, /) -> dict[str, str]:
        """
        Configuration of the table. Always empty.
        """
    @property
    def format(self, /) -> SharingFormat | None:
        """
        Format of the data files of the table.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the table. Empty when the table has none.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the Unity Catalog table.
        """
    @property
    def partition_columns(self, /) -> list[str]:
        """
        Names of the partition columns. Always empty.
        """
    @property
    def schema_string(self, /) -> str:
        """
        The columns of the Unity Catalog table: its `columns` as a JSON array in a string, not a Delta
        schema. Empty when the table has no columns.
        """

@final
class SharingProtocol:
    """
    The protocol of a table: the minimum version of the Delta Sharing reader that can read it.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def min_reader_version(self, /) -> int:
        """
        Minimum reader version. Always 1.
        """

@final
class SharingSchema:
    """
    A schema of a share: the `shared_as_schema` of one or more of its tables.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def name(self, /) -> str:
        """
        Name of the schema.
        """
    @property
    def share(self, /) -> str:
        """
        Name of the share.
        """

@final
class SharingShare:
    """
    A share that the recipient can read.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def id(self, /) -> str | None:
        """
        Unique identifier of the share. The server does not send it.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the share.
        """

@final
class SharingTable:
    """
    A table of a share, by the name under which the recipient sees it.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def id(self, /) -> str | None:
        """
        Unique identifier of the table. The server does not send it.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the table.
        """
    @property
    def schema(self, /) -> str:
        """
        Name of the schema of the table.
        """
    @property
    def share(self, /) -> str:
        """
        Name of the share.
        """

@final
class SharingTableLine:
    """
    One line of the NDJSON answer of `SharingGetTableMetadata` and `SharingQueryTable`: an object
    with one key. The first line holds `protocol`, the second `metaData`.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def metadata(self, /) -> SharingMetadata | None:
        """
        The metadata of the table.
        """
    @property
    def protocol(self, /) -> SharingProtocol | None:
        """
        The protocol of the table.
        """
