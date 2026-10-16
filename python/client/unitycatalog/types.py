"""The values of the API's enums, as ``Literal`` types.

The client takes and returns enum values as strings: the names in the JSON
body, such as ``"MANAGED"`` or ``"USE CATALOG"``. Any other string raises
``ValueError``.
"""

from typing import Literal, TypeAlias

TableType: TypeAlias = Literal[
    "MANAGED",
    "EXTERNAL",
    "STREAMING_TABLE",
    "MATERIALIZED_VIEW",
    "METRIC_VIEW",
    "VIEW",
]
"""The kind of a table."""

DataSourceFormat: TypeAlias = Literal[
    "DELTA", "ICEBERG", "CSV", "JSON", "AVRO", "PARQUET", "ORC", "TEXT"
]
"""The file format of a table."""

ColumnTypeName: TypeAlias = Literal[
    "BOOLEAN",
    "BYTE",
    "SHORT",
    "INT",
    "LONG",
    "FLOAT",
    "DOUBLE",
    "DATE",
    "TIMESTAMP",
    "TIMESTAMP_NTZ",
    "STRING",
    "BINARY",
    "DECIMAL",
    "INTERVAL",
    "ARRAY",
    "STRUCT",
    "MAP",
    "VARIANT",
    "CHAR",
    "NULL",
    "USER_DEFINED_TYPE",
    "TABLE_TYPE",
]
"""The type of a column or a function parameter."""

VolumeType: TypeAlias = Literal["MANAGED", "EXTERNAL"]
"""The kind of a volume."""

RoutineBody: TypeAlias = Literal["SQL", "EXTERNAL"]
"""The language of a function's body: SQL, or the ``external_language``."""

ParameterStyle: TypeAlias = Literal["S"]
"""The parameter style of a function: ``"S"``, SQL."""

SecurityType: TypeAlias = Literal["DEFINER"]
"""Whose rights a function runs with."""

SqlDataAccess: TypeAlias = Literal["CONTAINS_SQL", "READS_SQL_DATA", "NO_SQL"]
"""How a function accesses SQL data."""

FunctionParameterMode: TypeAlias = Literal["IN"]
"""The mode of a function parameter."""

FunctionParameterType: TypeAlias = Literal["PARAM", "COLUMN"]
"""A function parameter, or a column of the table that a function returns."""

ModelVersionStatus: TypeAlias = Literal[
    "MODEL_VERSION_STATUS_UNKNOWN",
    "PENDING_REGISTRATION",
    "FAILED_REGISTRATION",
    "READY",
]
"""The status of a model version."""

CredentialPurpose: TypeAlias = Literal["SERVICE_PRINCIPAL", "STORAGE"]
"""What a credential is for."""

TableOperation: TypeAlias = Literal["UNKNOWN_TABLE_OPERATION", "READ", "READ_WRITE"]
"""What temporary credentials for a table are for."""

VolumeOperation: TypeAlias = Literal[
    "UNKNOWN_VOLUME_OPERATION", "READ_VOLUME", "WRITE_VOLUME"
]
"""What temporary credentials for a volume are for."""

PathOperation: TypeAlias = Literal[
    "UNKNOWN_PATH_OPERATION", "PATH_READ", "PATH_READ_WRITE", "PATH_CREATE_TABLE"
]
"""What temporary credentials for a storage path are for."""

SecurableType: TypeAlias = Literal[
    "metastore",
    "catalog",
    "schema",
    "table",
    "volume",
    "function",
    "registered_model",
    "external_location",
    "credential",
]
"""The kind of object that privileges are granted on."""

Privilege: TypeAlias = Literal[
    "CREATE CATALOG",
    "USE CATALOG",
    "CREATE SCHEMA",
    "USE SCHEMA",
    "CREATE TABLE",
    "SELECT",
    "MODIFY",
    "CREATE FUNCTION",
    "EXECUTE",
    "CREATE VOLUME",
    "READ VOLUME",
    "CREATE MODEL",
    "CREATE EXTERNAL LOCATION",
    "READ FILES",
    "WRITE FILES",
    "CREATE EXTERNAL TABLE",
    "CREATE EXTERNAL VOLUME",
    "CREATE MANAGED STORAGE",
    "CREATE STORAGE CREDENTIAL",
]
"""A privilege on a securable."""

__all__ = [
    "ColumnTypeName",
    "CredentialPurpose",
    "DataSourceFormat",
    "FunctionParameterMode",
    "FunctionParameterType",
    "ModelVersionStatus",
    "ParameterStyle",
    "PathOperation",
    "Privilege",
    "RoutineBody",
    "SecurableType",
    "SecurityType",
    "SqlDataAccess",
    "TableOperation",
    "TableType",
    "VolumeOperation",
    "VolumeType",
]
