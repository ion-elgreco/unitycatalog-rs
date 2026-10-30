"""
The classes of the `functions` API.
"""

from .tables import DependencyList
from _typeshed import SupportsGetItem
from asyncio import Future
from typing import SupportsIndex, final
from unitycatalog._enums import (
    ColumnTypeName,
    FunctionParameterMode,
    FunctionParameterType,
    ParameterStyle,
    RoutineBody,
    SecurityType,
    SqlDataAccess,
)

__all__ = [
    "CreateFunction",
    "FunctionClient",
    "FunctionInfo",
    "FunctionParameterInfo",
    "FunctionParameterInfos",
    "ListFunctionsResponse",
]

@final
class CreateFunction:
    """
    The function to create.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        name: str,
        catalog_name: str,
        schema_name: str,
        routine_body: RoutineBody | str,
        parameter_style: ParameterStyle | str,
        security_type: SecurityType | str,
        *,
        input_params: FunctionParameterInfos | None = None,
        data_type: ColumnTypeName | str | None = None,
        full_data_type: str | None = None,
        return_params: FunctionParameterInfos | None = None,
        routine_definition: str | None = None,
        routine_dependencies: DependencyList | None = None,
        is_deterministic: bool | None = None,
        sql_data_access: SqlDataAccess | str | None = None,
        is_null_call: bool | None = None,
        specific_name: str | None = None,
        comment: str | None = None,
        properties: str | None = None,
        external_language: str | None = None,
    ) -> CreateFunction: ...
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
    def data_type(self, /) -> ColumnTypeName | None:
        """
        Name of the data type of the result.
        """
    @property
    def external_language(self, /) -> str | None:
        """
        Name of the external language of the body, such as `python`.
        """
    @property
    def full_data_type(self, /) -> str | None:
        """
        Full data type of the result as SQL text.
        """
    @property
    def input_params(self, /) -> FunctionParameterInfos | None:
        """
        The input parameters of the function.
        """
    @property
    def is_deterministic(self, /) -> bool | None:
        """
        Whether the function is deterministic.
        """
    @property
    def is_null_call(self, /) -> bool | None:
        """
        Whether the function returns null when an argument is null.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the function, relative to the parent schema.
        """
    @property
    def parameter_style(self, /) -> ParameterStyle | None:
        """
        Parameter style of the function.
        """
    @property
    def properties(self, /) -> str | None:
        """
        Key-value properties of the function, as a JSON-encoded object.
        """
    @property
    def return_params(self, /) -> FunctionParameterInfos | None:
        """
        The result columns of a table function.
        """
    @property
    def routine_body(self, /) -> RoutineBody | None:
        """
        Language of the body of the function.
        """
    @property
    def routine_definition(self, /) -> str | None:
        """
        Body of the function.
        """
    @property
    def routine_dependencies(self, /) -> DependencyList | None:
        """
        The tables and functions that the function reads.
        """
    @property
    def schema_name(self, /) -> str:
        """
        Name of the parent schema.
        """
    @property
    def security_type(self, /) -> SecurityType | None:
        """
        Security type of the function.
        """
    @property
    def specific_name(self, /) -> str | None:
        """
        Specific name of the function.
        """
    @property
    def sql_data_access(self, /) -> SqlDataAccess | None:
        """
        How the function accesses SQL data.
        """

@final
class FunctionClient:
    """
    The requests on one `FunctionInfo`.
    """
    def delete(self, /) -> None:
        """
        Delete a function.
        """
    def delete_async(self, /) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> FunctionInfo:
        """
        Get a function.

        Returns:
            A function: a user-defined function in the third layer of the three-level namespace, below
            a schema.
        """
    def get_async(self, /) -> Future[FunctionInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class FunctionInfo:
    """
    A function: a user-defined function in the third layer of the three-level namespace, below a
    schema.
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
        Time at which the function was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the function.
        """
    @property
    def data_type(self, /) -> ColumnTypeName | None:
        """
        Name of the data type of the result.
        """
    @property
    def external_language(self, /) -> str | None:
        """
        Name of the external language of the body, such as `python`.
        """
    @property
    def full_data_type(self, /) -> str | None:
        """
        Full data type of the result as SQL text.
        """
    @property
    def full_name(self, /) -> str:
        """
        Full name of the function, in the form `<catalog>.<schema>.<function>`.
        """
    @property
    def function_id(self, /) -> str:
        """
        Unique identifier of the function.
        """
    @property
    def input_params(self, /) -> FunctionParameterInfos | None:
        """
        The input parameters of the function. Absent when there are none.
        """
    @property
    def is_deterministic(self, /) -> bool | None:
        """
        Whether the function is deterministic.
        """
    @property
    def is_null_call(self, /) -> bool | None:
        """
        Whether the function returns null when an argument is null.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the function, relative to the parent schema.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the function.
        """
    @property
    def parameter_style(self, /) -> ParameterStyle | None:
        """
        Parameter style of the function.
        """
    @property
    def properties(self, /) -> str | None:
        """
        Key-value properties of the function, as a JSON-encoded object.
        """
    @property
    def return_params(self, /) -> FunctionParameterInfos | None:
        """
        The result columns of a table function. Absent when there are none.
        """
    @property
    def routine_body(self, /) -> RoutineBody | None:
        """
        Language of the body of the function.
        """
    @property
    def routine_definition(self, /) -> str | None:
        """
        Body of the function.
        """
    @property
    def routine_dependencies(self, /) -> DependencyList | None:
        """
        The tables and functions that the function reads.
        """
    @property
    def schema_name(self, /) -> str:
        """
        Name of the parent schema.
        """
    @property
    def security_type(self, /) -> SecurityType | None:
        """
        Security type of the function.
        """
    @property
    def specific_name(self, /) -> str | None:
        """
        Specific name of the function.
        """
    @property
    def sql_data_access(self, /) -> SqlDataAccess | None:
        """
        How the function accesses SQL data.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the function was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the function.
        """

@final
class FunctionParameterInfo:
    """
    A parameter or result column of a function.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        name: str,
        type_text: str,
        type_name: ColumnTypeName | str,
        *,
        type_json: str | None = None,
        type_precision: SupportsIndex | None = None,
        type_scale: SupportsIndex | None = None,
        type_interval_type: str | None = None,
        position: SupportsIndex | None = None,
        parameter_mode: FunctionParameterMode | str | None = None,
        parameter_type: FunctionParameterType | str | None = None,
        parameter_default: str | None = None,
        comment: str | None = None,
    ) -> FunctionParameterInfo: ...
    def __repr__(self, /) -> str: ...
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the parameter.
        """
    @property
    def parameter_default(self, /) -> str | None:
        """
        Default value of the parameter.
        """
    @property
    def parameter_mode(self, /) -> FunctionParameterMode | None:
        """
        Mode of the parameter.
        """
    @property
    def parameter_type(self, /) -> FunctionParameterType | None:
        """
        Kind of the parameter.
        """
    @property
    def position(self, /) -> int | None:
        """
        Position of the parameter, starting at 0.
        """
    @property
    def type_interval_type(self, /) -> str | None:
        """
        Format of an interval type.
        """
    @property
    def type_json(self, /) -> str | None:
        """
        Full data type of the parameter as JSON.
        """
    @property
    def type_name(self, /) -> ColumnTypeName | None:
        """
        Name of the data type of the parameter.
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
    def type_text(self, /) -> str:
        """
        Full data type of the parameter as SQL text.
        """

@final
class FunctionParameterInfos:
    """
    The parameters of a function.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, parameters: SupportsGetItem[int, FunctionParameterInfo] | None = None
    ) -> FunctionParameterInfos: ...
    def __repr__(self, /) -> str: ...
    @property
    def parameters(self, /) -> list[FunctionParameterInfo]:
        """
        The parameters.
        """

@final
class ListFunctionsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def functions(self, /) -> list[FunctionInfo]:
        """
        The functions.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
