"""
The classes of the `tables` API.
"""

from _typeshed import SupportsGetItem
from asyncio import Future
from typing import SupportsIndex, final
from unitycatalog._enums import ColumnTypeName, DataSourceFormat, TableType

__all__ = [
    "ColumnInfo",
    "Dependency",
    "DependencyList",
    "FunctionDependency",
    "ListTablesResponse",
    "StagingTableInfo",
    "TableClient",
    "TableDependency",
    "TableInfo",
    "UpdateColumn",
]

@final
class ColumnInfo:
    """
    A column of a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        name: str,
        type_json: str,
        *,
        type_text: str | None = None,
        type_name: ColumnTypeName | str | None = None,
        type_precision: SupportsIndex | None = None,
        type_scale: SupportsIndex | None = None,
        type_interval_type: str | None = None,
        position: SupportsIndex | None = None,
        comment: str | None = None,
        nullable: bool | None = None,
        partition_index: SupportsIndex | None = None,
    ) -> ColumnInfo: ...
    def __repr__(self, /) -> str: ...
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the column.
        """
    @property
    def nullable(self, /) -> bool | None:
        """
        Whether the column can hold null values. The server sets false when it is absent.
        """
    @property
    def partition_index(self, /) -> int | None:
        """
        Position of the column in the partition key of the table.
        """
    @property
    def position(self, /) -> int | None:
        """
        Position of the column, starting at 0.
        """
    @property
    def type_interval_type(self, /) -> str | None:
        """
        Format of an interval type.
        """
    @property
    def type_json(self, /) -> str:
        """
        Full data type of the column as a JSON Spark `StructField`.
        """
    @property
    def type_name(self, /) -> ColumnTypeName | None:
        """
        Name of the data type of the column.
        """
    @property
    def type_precision(self, /) -> int | None:
        """
        Digits of precision. Required for decimal types.
        """
    @property
    def type_scale(self, /) -> int | None:
        """
        Digits to the right of the decimal point. Required for decimal types.
        """
    @property
    def type_text(self, /) -> str | None:
        """
        Full data type of the column as SQL text, such as `decimal(10,2)`.
        """

@final
class Dependency:
    """
    A dependency of a view or function. Exactly one of the fields is set.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        table: TableDependency | None = None,
        function: FunctionDependency | None = None,
    ) -> Dependency: ...
    def __repr__(self, /) -> str: ...
    @property
    def function(self, /) -> FunctionDependency | None:
        """
        A function dependency.
        """
    @property
    def table(self, /) -> TableDependency | None:
        """
        A table dependency.
        """

@final
class DependencyList:
    """
    The dependencies of a view or function.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, dependencies: SupportsGetItem[int, Dependency] | None = None
    ) -> DependencyList: ...
    def __repr__(self, /) -> str: ...
    @property
    def dependencies(self, /) -> list[Dependency]:
        """
        The dependencies.
        """

@final
class FunctionDependency:
    """
    A function that a view or function depends on.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, function_full_name: str) -> FunctionDependency: ...
    def __repr__(self, /) -> str: ...
    @property
    def function_full_name(self, /) -> str:
        """
        Full name of the function, in the form `<catalog>.<schema>.<function>`.
        """

@final
class ListTablesResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
    @property
    def tables(self, /) -> list[TableInfo]:
        """
        The tables.
        """

@final
class StagingTableInfo:
    """
    A staging table: a table id and managed storage location reserved for a MANAGED table that a
    client is about to create.
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
    def id(self, /) -> str:
        """
        Unique identifier of the staging table. The table gets it when it is created.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the table, relative to the parent schema.
        """
    @property
    def schema_name(self, /) -> str:
        """
        Name of the parent schema.
        """
    @property
    def staging_location(self, /) -> str:
        """
        Storage location URL reserved for the table. Create the table with it as its
        `storage_location`.
        """

@final
class TableClient:
    """
    The requests on one `TableInfo`.
    """
    def delete(self, /) -> None:
        """
        Delete a table.
        """
    def delete_async(self, /) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(
        self,
        /,
        *,
        read_streaming_table_as_managed: bool | None = None,
        read_materialized_view_as_managed: bool | None = None,
    ) -> TableInfo:
        """
        Get a table.

        Args:
            read_streaming_table_as_managed: Show a streaming table as a MANAGED table, as upstream's server
                                             can. This server does not read it.
            read_materialized_view_as_managed: Show a materialized view as a MANAGED table, as upstream's
                                               server can. This server does not read it.

        Returns:
            A table: data in the third layer of the three-level namespace, below a schema.
        """
    def get_async(
        self,
        /,
        *,
        read_streaming_table_as_managed: bool | None = None,
        read_materialized_view_as_managed: bool | None = None,
    ) -> Future[TableInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self,
        /,
        *,
        comment: str | None = None,
        columns: SupportsGetItem[int, UpdateColumn] | None = None,
    ) -> TableInfo:
        """
        Change the comments of a table and its columns.

        Args:
            comment: User-provided free-form text description.
            columns: Columns that get a new comment; the others keep theirs.

        Returns:
            A table: data in the third layer of the three-level namespace, below a schema.
        """
    def update_async(
        self,
        /,
        *,
        comment: str | None = None,
        columns: SupportsGetItem[int, UpdateColumn] | None = None,
    ) -> Future[TableInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class TableDependency:
    """
    A table that a view or function depends on.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, table_full_name: str) -> TableDependency: ...
    def __repr__(self, /) -> str: ...
    @property
    def table_full_name(self, /) -> str:
        """
        Full name of the table, in the form `<catalog>.<schema>.<table>`.
        """

@final
class TableInfo:
    """
    A table: data in the third layer of the three-level namespace, below a schema.
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
    def columns(self, /) -> list[ColumnInfo]:
        """
        The columns of the table.
        """
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the table was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the table.
        """
    @property
    def data_source_format(self, /) -> DataSourceFormat | None:
        """
        File format of the data of the table.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the table, relative to the parent schema.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the table.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        Key-value properties of the table.
        """
    @property
    def schema_name(self, /) -> str:
        """
        Name of the parent schema.
        """
    @property
    def storage_location(self, /) -> str | None:
        """
        Storage location URL of the table. Absent for views.
        """
    @property
    def table_id(self, /) -> str:
        """
        Unique identifier of the table.
        """
    @property
    def table_type(self, /) -> TableType | None:
        """
        Type of the table.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the table was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the table.
        """
    @property
    def view_definition(self, /) -> str | None:
        """
        Definition of a view-like table: SQL for views, YAML for metric views.
        """
    @property
    def view_dependencies(self, /) -> DependencyList | None:
        """
        The tables and functions that a view-like table reads.
        """

@final
class UpdateColumn:
    """
    A new comment for a column of a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, name: str, comment: str) -> UpdateColumn: ...
    def __repr__(self, /) -> str: ...
    @property
    def comment(self, /) -> str:
        """
        New comment of the column. Empty clears it.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the column.
        """
