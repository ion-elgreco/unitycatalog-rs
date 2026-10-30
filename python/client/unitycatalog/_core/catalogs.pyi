"""
The classes of the `catalogs` API.
"""

from asyncio import Future
from typing import final

__all__ = ["CatalogClient", "CatalogInfo", "ListCatalogsResponse"]

@final
class CatalogClient:
    """
    The requests on one `CatalogInfo`.
    """
    def delete(self, /, *, force: bool | None = None) -> None:
        """
        Delete a catalog.

        Args:
            force: Delete the schemas, tables and volumes of the catalog too.
        """
    def delete_async(self, /, *, force: bool | None = None) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> CatalogInfo:
        """
        Get a catalog.

        Returns:
            A catalog: the first layer of the three-level namespace. It holds schemas.
        """
    def get_async(self, /) -> Future[CatalogInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self,
        /,
        *,
        comment: str | None = None,
        properties: dict[str, str] | None = None,
        new_name: str | None = None,
    ) -> CatalogInfo:
        """
        Change a catalog.

        Args:
            comment: User-provided free-form text description.
            properties: Key-value properties of the catalog. They replace the properties of the catalog.
            new_name: New name of the catalog.

        Returns:
            A catalog: the first layer of the three-level namespace. It holds schemas.
        """
    def update_async(
        self,
        /,
        *,
        comment: str | None = None,
        properties: dict[str, str] | None = None,
        new_name: str | None = None,
    ) -> Future[CatalogInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class CatalogInfo:
    """
    A catalog: the first layer of the three-level namespace. It holds schemas.
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
        Time at which the catalog was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the catalog.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the catalog.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the catalog.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the catalog.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        Key-value properties of the catalog.
        """
    @property
    def storage_location(self, /) -> str | None:
        """
        Storage location URL (full path) of the managed storage of the catalog.
        """
    @property
    def storage_root(self, /) -> str | None:
        """
        Storage root URL for the managed storage of the catalog, such as `s3://bucket/ucroot`.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the catalog was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the catalog.
        """

@final
class ListCatalogsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def catalogs(self, /) -> list[CatalogInfo]:
        """
        The catalogs.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
