"""
The classes of the `shares` API.
"""

from _typeshed import SupportsGetItem
from asyncio import Future
from typing import final

__all__ = ["ListSharesResponse", "ShareClient", "ShareInfo", "SharedDataObject"]

@final
class ListSharesResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items(self, /) -> list[ShareInfo]:
        """
        The shares.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """

@final
class ShareClient:
    """
    The requests on one `ShareInfo`.
    """
    def delete(self, /) -> None:
        """
        Delete a share.
        """
    def delete_async(self, /) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> ShareInfo:
        """
        Get a share.

        Returns:
            A share: a named set of tables that Delta Sharing recipients can read.
        """
    def get_async(self, /) -> Future[ShareInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self,
        /,
        *,
        comment: str | None = None,
        new_name: str | None = None,
        add_objects: SupportsGetItem[int, SharedDataObject] | None = None,
        remove_objects: SupportsGetItem[int, SharedDataObject] | None = None,
    ) -> ShareInfo:
        """
        Change a share: its comment, its name and its tables.

        Args:
            comment: User-provided free-form text description.
            new_name: New name of the share.
            add_objects: Tables to add to the share. A table that the share holds already is skipped.
            remove_objects: Tables to remove from the share, matched by `source_table_full_name`.

        Returns:
            A share: a named set of tables that Delta Sharing recipients can read.
        """
    def update_async(
        self,
        /,
        *,
        comment: str | None = None,
        new_name: str | None = None,
        add_objects: SupportsGetItem[int, SharedDataObject] | None = None,
        remove_objects: SupportsGetItem[int, SharedDataObject] | None = None,
    ) -> Future[ShareInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class ShareInfo:
    """
    A share: a named set of tables that Delta Sharing recipients can read.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the share was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the share.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the share.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the share.
        """
    @property
    def objects(self, /) -> list[SharedDataObject]:
        """
        The tables of the share.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the share.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the share was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the share.
        """

@final
class SharedDataObject:
    """
    A table of a share: a Unity Catalog table and the name under which the recipients see it.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, source_table_full_name: str, shared_as_schema: str, shared_as_name: str
    ) -> SharedDataObject: ...
    def __repr__(self, /) -> str: ...
    @property
    def shared_as_name(self, /) -> str:
        """
        Name under which the recipients see the table.
        """
    @property
    def shared_as_schema(self, /) -> str:
        """
        Schema under which the recipients see the table, such as `default`.
        """
    @property
    def source_table_full_name(self, /) -> str:
        """
        Three-level name of the Unity Catalog table: `catalog.schema.table`. A share holds a table
        once.
        """
