"""
The classes of the `schemas` API.
"""

from asyncio import Future
from typing import final

__all__ = ["ListSchemasResponse", "SchemaClient", "SchemaInfo"]

@final
class ListSchemasResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
    @property
    def schemas(self, /) -> list[SchemaInfo]:
        """
        The schemas.
        """

@final
class SchemaClient:
    """
    The requests on one `SchemaInfo`.
    """
    def delete(self, /, *, force: bool | None = None) -> None:
        """
        Delete a schema.

        Args:
            force: Delete the tables, volumes, functions and models of the schema too.
        """
    def delete_async(self, /, *, force: bool | None = None) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> SchemaInfo:
        """
        Get a schema.

        Returns:
            A schema: the second layer of the three-level namespace. It holds tables, volumes, functions
            and models.
        """
    def get_async(self, /) -> Future[SchemaInfo]:
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
    ) -> SchemaInfo:
        """
        Change a schema.

        Args:
            comment: User-provided free-form text description.
            properties: Key-value properties of the schema. They replace the properties of the schema.
            new_name: New name of the schema, relative to the parent catalog.

        Returns:
            A schema: the second layer of the three-level namespace. It holds tables, volumes, functions
            and models.
        """
    def update_async(
        self,
        /,
        *,
        comment: str | None = None,
        properties: dict[str, str] | None = None,
        new_name: str | None = None,
    ) -> Future[SchemaInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class SchemaInfo:
    """
    A schema: the second layer of the three-level namespace. It holds tables, volumes, functions
    and models.
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
        Time at which the schema was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the schema.
        """
    @property
    def full_name(self, /) -> str:
        """
        Full name of the schema, in the form `<catalog>.<schema>`.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the schema, relative to the parent catalog.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the schema.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        Key-value properties of the schema.
        """
    @property
    def schema_id(self, /) -> str:
        """
        Unique identifier of the schema.
        """
    @property
    def storage_location(self, /) -> str | None:
        """
        Storage location URL (full path) of the managed storage of the schema.
        """
    @property
    def storage_root(self, /) -> str | None:
        """
        Storage root URL for the managed storage of the schema, such as `s3://bucket/ucroot`.
        Without it, managed tables and volumes of the schema go below the storage of the catalog.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the schema was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the schema.
        """
