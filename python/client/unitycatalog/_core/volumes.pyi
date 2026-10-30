"""
The classes of the `volumes` API.
"""

from asyncio import Future
from typing import final
from unitycatalog._enums import VolumeType

__all__ = ["ListVolumesResponseContent", "VolumeClient", "VolumeInfo"]

@final
class ListVolumesResponseContent:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
    @property
    def volumes(self, /) -> list[VolumeInfo]:
        """
        The volumes.
        """

@final
class VolumeClient:
    """
    The requests on one `VolumeInfo`.
    """
    def delete(self, /) -> None:
        """
        Delete a volume.
        """
    def delete_async(self, /) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> VolumeInfo:
        """
        Get a volume.

        Returns:
            A volume: files in the third layer of the three-level namespace, below a schema.
        """
    def get_async(self, /) -> Future[VolumeInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self, /, *, comment: str | None = None, new_name: str | None = None
    ) -> VolumeInfo:
        """
        Change a volume.

        Args:
            comment: User-provided free-form text description.
            new_name: New name of the volume, relative to the parent schema.

        Returns:
            A volume: files in the third layer of the three-level namespace, below a schema.
        """
    def update_async(
        self, /, *, comment: str | None = None, new_name: str | None = None
    ) -> Future[VolumeInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class VolumeInfo:
    """
    A volume: files in the third layer of the three-level namespace, below a schema.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def catalog_name(self, /) -> str:
        """
        Name of the parent catalog.
        """
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the volume was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the volume.
        """
    @property
    def full_name(self, /) -> str:
        """
        Full name of the volume, in the form `<catalog>.<schema>.<volume>`.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the volume, relative to the parent schema.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the volume.
        """
    @property
    def schema_name(self, /) -> str:
        """
        Name of the parent schema.
        """
    @property
    def storage_location(self, /) -> str:
        """
        Storage location URL of the volume.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the volume was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the volume.
        """
    @property
    def volume_id(self, /) -> str:
        """
        Unique identifier of the volume.
        """
    @property
    def volume_type(self, /) -> VolumeType | None:
        """
        Type of the volume.
        """
