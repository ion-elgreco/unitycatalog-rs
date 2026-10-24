"""Python client for unitycatalog-rs, the Rust reimplementation of Unity Catalog OSS.

``Client`` wraps the Rust ``uc-client``. It takes the root URL of the server. Each
method comes twice: ``name`` waits for the response, and ``name_async`` returns an
awaitable. The requests on one catalog, schema, table and so on are on its handle,
such as ``client.catalog("main")``::

    import asyncio
    from unitycatalog import Client

    client = Client("http://localhost:8080/")
    client.create_catalog("main", comment="primary")
    print([c.name for c in client.list_catalogs()])

    async def main():
        catalog = await client.catalog("main").get_async()
        print(catalog.id)

    asyncio.run(main())

The classes of the requests and responses are in a submodule of
``unitycatalog._core`` per API package, such as ``unitycatalog._core.tables``.
Enums are ``enum.StrEnum`` classes; a parameter takes a member or its name. An
error answer raises :class:`UnityCatalogError`, or the subclass of its HTTP status, such as
:class:`NotFoundError`.
"""

from unitycatalog._core import (
    BadRequestError,
    Client,
    ConflictError,
    NotFoundError,
    PermissionDeniedError,
    RateLimitError,
    ServerError,
    UnauthenticatedError,
    UnityCatalogError,
    __version__,
)
from unitycatalog._enums import (
    AuthenticationType,
    ColumnTypeName,
    CredentialPurpose,
    DataSourceFormat,
    DeltaCredentialOperation,
    DeltaMaintenanceOperation,
    DeltaTableType,
    FunctionParameterMode,
    FunctionParameterType,
    ModelVersionOperation,
    ModelVersionStatus,
    ParameterStyle,
    PathOperation,
    RoutineBody,
    SecurityType,
    SqlDataAccess,
    TableOperation,
    TableType,
    VolumeOperation,
    VolumeType,
)

__all__ = [
    "AuthenticationType",
    "BadRequestError",
    "Client",
    "ColumnTypeName",
    "ConflictError",
    "CredentialPurpose",
    "DataSourceFormat",
    "DeltaCredentialOperation",
    "DeltaMaintenanceOperation",
    "DeltaTableType",
    "FunctionParameterMode",
    "FunctionParameterType",
    "ModelVersionOperation",
    "ModelVersionStatus",
    "NotFoundError",
    "ParameterStyle",
    "PathOperation",
    "PermissionDeniedError",
    "RateLimitError",
    "RoutineBody",
    "SecurityType",
    "ServerError",
    "SqlDataAccess",
    "TableOperation",
    "TableType",
    "UnauthenticatedError",
    "UnityCatalogError",
    "VolumeOperation",
    "VolumeType",
    "__version__",
]
