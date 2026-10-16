"""Python client for unitycatalog-rs, the Rust reimplementation of Unity Catalog OSS.

``Client`` wraps the Rust ``uc-client``. Each method comes twice: ``name``
waits for the response, and ``name_async`` returns an awaitable::

    import asyncio
    from unitycatalog import Client

    client = Client("http://localhost:8080/api/2.1/unity-catalog/")
    client.create_catalog("main", comment="primary")
    print([c.name for c in client.list_catalogs()])

    async def main():
        catalog = await client.get_catalog_async("main")
        print(catalog.id)

    asyncio.run(main())

Errors raise :class:`UcError`, with the upstream error code. Enum values are
strings; :mod:`unitycatalog.types` has them as ``Literal`` types.
"""

from unitycatalog._core import Client, __version__
from unitycatalog._core.admin import RestoreResponse
from unitycatalog._core.audit import AuditEvent
from unitycatalog._core.catalogs import CatalogInfo
from unitycatalog._core.credentials import (
    AwsCredentials,
    AwsIamRole,
    AzureUserDelegationSas,
    CredentialInfo,
    GcpOauthToken,
    TemporaryCredentials,
)
from unitycatalog._core.external_locations import ExternalLocationInfo
from unitycatalog._core.functions import FunctionInfo, FunctionParameterInfo
from unitycatalog._core.iceberg_rest import IcebergConfig, IcebergTableIdentifier
from unitycatalog._core.metastore import MetastoreSummary
from unitycatalog._core.models import ModelVersionInfo, RegisteredModelInfo
from unitycatalog._core.permissions import PermissionsChange, PrivilegeAssignment
from unitycatalog._core.schemas import SchemaInfo
from unitycatalog._core.scim import ScimGroup, ScimUser
from unitycatalog._core.tables import (
    ColumnInfo,
    Dependency,
    StagingTableInfo,
    TableInfo,
)
from unitycatalog._core.tokens import PatInfo
from unitycatalog._core.volumes import VolumeInfo
from unitycatalog.exceptions import UcError

__all__ = [
    "AuditEvent",
    "AwsCredentials",
    "AwsIamRole",
    "AzureUserDelegationSas",
    "CatalogInfo",
    "Client",
    "ColumnInfo",
    "CredentialInfo",
    "Dependency",
    "ExternalLocationInfo",
    "FunctionInfo",
    "FunctionParameterInfo",
    "GcpOauthToken",
    "IcebergConfig",
    "IcebergTableIdentifier",
    "MetastoreSummary",
    "ModelVersionInfo",
    "PatInfo",
    "PermissionsChange",
    "PrivilegeAssignment",
    "RegisteredModelInfo",
    "RestoreResponse",
    "SchemaInfo",
    "ScimGroup",
    "ScimUser",
    "StagingTableInfo",
    "TableInfo",
    "TemporaryCredentials",
    "UcError",
    "VolumeInfo",
    "__version__",
]
