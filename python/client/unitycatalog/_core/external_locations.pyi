"""
The classes of the `external_locations` API.
"""

from asyncio import Future
from typing import final

__all__ = [
    "ExternalLocationClient",
    "ExternalLocationInfo",
    "ListExternalLocationsResponse",
]

@final
class ExternalLocationClient:
    """
    The requests on one `ExternalLocationInfo`.
    """
    def delete(self, /, *, force: bool | None = None) -> None:
        """
        Delete an external location.

        Args:
            force: Delete the external location even when tables, volumes or models use its path.
        """
    def delete_async(self, /, *, force: bool | None = None) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> ExternalLocationInfo:
        """
        Get an external location.

        Returns:
            An external location: a storage path and the credential that gives access to it.
        """
    def get_async(self, /) -> Future[ExternalLocationInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self,
        /,
        *,
        url: str | None = None,
        new_name: str | None = None,
        comment: str | None = None,
        credential_name: str | None = None,
    ) -> ExternalLocationInfo:
        """
        Change an external location.

        Args:
            url: New URL of the storage path.
            new_name: New name of the external location.
            comment: User-provided free-form text description.
            credential_name: Name of the new credential that gives access to the path.

        Returns:
            An external location: a storage path and the credential that gives access to it.
        """
    def update_async(
        self,
        /,
        *,
        url: str | None = None,
        new_name: str | None = None,
        comment: str | None = None,
        credential_name: str | None = None,
    ) -> Future[ExternalLocationInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class ExternalLocationInfo:
    """
    An external location: a storage path and the credential that gives access to it.
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
        Time at which the external location was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the external location.
        """
    @property
    def credential_id(self, /) -> str:
        """
        Unique identifier of the credential that gives access to the path.
        """
    @property
    def credential_name(self, /) -> str | None:
        """
        Name of the credential that gives access to the path. Absent when the credential was
        deleted with `force`.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the external location.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the external location.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the external location.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the external location was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the external location.
        """
    @property
    def url(self, /) -> str:
        """
        URL of the storage path, such as `s3://bucket/path`.
        """

@final
class ListExternalLocationsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def external_locations(self, /) -> list[ExternalLocationInfo]:
        """
        The external locations.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
