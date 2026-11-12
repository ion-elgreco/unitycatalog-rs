"""
The classes of the `shares` API.
"""

from _typeshed import SupportsGetItem
from asyncio import Future
from typing import SupportsIndex, final
from unitycatalog._enums import HistoryDataSharingStatus, SharedObjectType

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
            A share: a named set of tables, volumes and agent skills that recipients can read.
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
            add_objects: Objects to add to the share. An object that the share holds already is replaced,
                         which changes its shared name and its history settings.
            remove_objects: Objects to remove from the share, matched by `source_full_name` and
                            `data_object_type`.

        Returns:
            A share: a named set of tables, volumes and agent skills that recipients can read.
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
    A share: a named set of tables, volumes and agent skills that recipients can read.
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
        The objects of the share.
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
    An object of a share: a Unity Catalog table or volume and the name under which the recipients
    see it.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        source_full_name: str,
        shared_as_schema: str,
        shared_as_name: str,
        *,
        data_object_type: SharedObjectType | str | None = None,
        history_data_sharing_status: HistoryDataSharingStatus | str | None = None,
        start_version: SupportsIndex | None = None,
    ) -> SharedDataObject: ...
    def __repr__(self, /) -> str: ...
    @property
    def data_object_type(self, /) -> SharedObjectType | None:
        """
        What the object is. A table, if not set.
        """
    @property
    def history_data_sharing_status(self, /) -> HistoryDataSharingStatus | None:
        """
        Of a table: whether recipients read its history. `DISABLED`, if not set.
        """
    @property
    def shared_as_name(self, /) -> str:
        """
        Name under which the recipients see the object.
        """
    @property
    def shared_as_schema(self, /) -> str:
        """
        Schema under which the recipients see the object, such as `default`.
        """
    @property
    def source_full_name(self, /) -> str:
        """
        Three-level name of the Unity Catalog table or volume: `catalog.schema.name`. An agent skill
        names the volume that holds it. A share holds an object once per type.
        """
    @property
    def start_version(self, /) -> int | None:
        """
        Of a table that shares its history: the lowest version that recipients read. Without it,
        they read from version 0.
        """
