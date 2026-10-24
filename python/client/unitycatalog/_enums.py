# @generated — do not edit by hand.
"""The enums of the API.

Each member is a `str` equal to its name, so a member and its name compare equal, and a
parameter takes either.
"""

import enum


class CredentialPurpose(enum.StrEnum):
    """What a credential is used for."""

    SERVICE_PRINCIPAL = "SERVICE_PRINCIPAL"
    """The credential gives access to external services."""
    STORAGE = "STORAGE"
    """The credential gives access to storage."""


class ModelVersionOperation(enum.StrEnum):
    """The operation on a model version that temporary credentials are for."""

    UNKNOWN_MODEL_VERSION_OPERATION = "UNKNOWN_MODEL_VERSION_OPERATION"
    """The operation is not known. The server rejects it."""
    READ_MODEL_VERSION = "READ_MODEL_VERSION"
    """Read the model version."""
    READ_WRITE_MODEL_VERSION = "READ_WRITE_MODEL_VERSION"
    """Read and write the model version. Only for a model version with the status
    PENDING_REGISTRATION.
    """


class PathOperation(enum.StrEnum):
    """The operation on a storage path that temporary credentials are for."""

    UNKNOWN_PATH_OPERATION = "UNKNOWN_PATH_OPERATION"
    """The operation is not known."""
    PATH_READ = "PATH_READ"
    """Read the path."""
    PATH_READ_WRITE = "PATH_READ_WRITE"
    """Read and write the path."""
    PATH_CREATE_TABLE = "PATH_CREATE_TABLE"
    """Create a table at the path."""


class TableOperation(enum.StrEnum):
    """The operation on a table that temporary credentials are for."""

    UNKNOWN_TABLE_OPERATION = "UNKNOWN_TABLE_OPERATION"
    """The operation is not known."""
    READ = "READ"
    """Read the table."""
    READ_WRITE = "READ_WRITE"
    """Read and write the table."""


class VolumeOperation(enum.StrEnum):
    """The operation on a volume that temporary credentials are for."""

    UNKNOWN_VOLUME_OPERATION = "UNKNOWN_VOLUME_OPERATION"
    """The operation is not known."""
    READ_VOLUME = "READ_VOLUME"
    """Read the volume."""
    WRITE_VOLUME = "WRITE_VOLUME"
    """Read and write the volume."""


class DeltaCredentialOperation(enum.StrEnum):
    """The access level of a storage credential."""

    READ = "READ"
    """Read the data."""
    READ_WRITE = "READ_WRITE"
    """Read and write the data."""


class DeltaMaintenanceOperation(enum.StrEnum):
    """A maintenance operation that the catalog allows on a table, in addition to the operations that
    the Delta protocol allows.
    """

    DATA_REORGANIZATION = "DATA_REORGANIZATION"
    """Rewrite the data files of the latest version to group, cluster or lay out the data, without
    a change to the data.
    """
    DATA_CLEANUP = "DATA_CLEANUP"
    """Delete old data files, without a change to the data of the latest version."""
    METADATA_CLEANUP = "METADATA_CLEANUP"
    """Delete old metadata files, without a change to the data of the latest version."""


class DeltaTableType(enum.StrEnum):
    """The type of a Delta table."""

    MANAGED = "MANAGED"
    """A table whose data Unity Catalog manages."""
    EXTERNAL = "EXTERNAL"
    """A table whose data is at a storage location that the user gives."""


class FunctionParameterMode(enum.StrEnum):
    """The mode of a function parameter."""

    IN = "IN"
    """An input parameter."""


class FunctionParameterType(enum.StrEnum):
    """The kind of a function parameter."""

    PARAM = "PARAM"
    """A named parameter."""
    COLUMN = "COLUMN"
    """A column of a table function result."""


class ParameterStyle(enum.StrEnum):
    """The parameter style of a function."""

    S = "S"
    """SQL parameter style."""


class RoutineBody(enum.StrEnum):
    """The language of the body of a function."""

    SQL = "SQL"
    """The body is SQL."""
    EXTERNAL = "EXTERNAL"
    """The body is in an external language, such as Python."""


class SecurityType(enum.StrEnum):
    """The security type of a function."""

    DEFINER = "DEFINER"
    """The function runs with the rights of its owner."""


class SqlDataAccess(enum.StrEnum):
    """How a function accesses SQL data."""

    CONTAINS_SQL = "CONTAINS_SQL"
    """The function contains SQL."""
    READS_SQL_DATA = "READS_SQL_DATA"
    """The function reads SQL data."""
    NO_SQL = "NO_SQL"
    """The function does not use SQL."""


class ModelVersionStatus(enum.StrEnum):
    """The status of a model version: where the upload of its files stands."""

    MODEL_VERSION_STATUS_UNKNOWN = "MODEL_VERSION_STATUS_UNKNOWN"
    """The status is not known."""
    PENDING_REGISTRATION = "PENDING_REGISTRATION"
    """The client has not yet written all files of the model version. A new model version has this
    status.
    """
    FAILED_REGISTRATION = "FAILED_REGISTRATION"
    """The client failed to write all files of the model version."""
    READY = "READY"
    """The client wrote all files and finalized the model version."""


class AuthenticationType(enum.StrEnum):
    """How a recipient signs in to the Delta Sharing server."""

    TOKEN = "TOKEN"
    """A bearer token."""
    OAUTH_CLIENT_CREDENTIALS = "OAUTH_CLIENT_CREDENTIALS"
    """OAuth client credentials. Not implemented: the recipient gets a bearer token."""


class ColumnTypeName(enum.StrEnum):
    """The name of the data type of a column."""

    BOOLEAN = "BOOLEAN"
    BYTE = "BYTE"
    SHORT = "SHORT"
    INT = "INT"
    LONG = "LONG"
    FLOAT = "FLOAT"
    DOUBLE = "DOUBLE"
    DATE = "DATE"
    TIMESTAMP = "TIMESTAMP"
    TIMESTAMP_NTZ = "TIMESTAMP_NTZ"
    """Timestamp without a time zone."""
    STRING = "STRING"
    BINARY = "BINARY"
    DECIMAL = "DECIMAL"
    INTERVAL = "INTERVAL"
    ARRAY = "ARRAY"
    STRUCT = "STRUCT"
    MAP = "MAP"
    VARIANT = "VARIANT"
    CHAR = "CHAR"
    NULL = "NULL"
    USER_DEFINED_TYPE = "USER_DEFINED_TYPE"
    TABLE_TYPE = "TABLE_TYPE"
    """The return type of a table function."""


class DataSourceFormat(enum.StrEnum):
    """The file format of the data of a table."""

    DELTA = "DELTA"
    ICEBERG = "ICEBERG"
    CSV = "CSV"
    JSON = "JSON"
    AVRO = "AVRO"
    PARQUET = "PARQUET"
    ORC = "ORC"
    TEXT = "TEXT"


class TableType(enum.StrEnum):
    """The type of a table."""

    MANAGED = "MANAGED"
    """A table whose data Unity Catalog manages, below the managed storage of its schema or catalog."""
    EXTERNAL = "EXTERNAL"
    """A table whose data is at a storage location that the user gives."""
    STREAMING_TABLE = "STREAMING_TABLE"
    """A streaming table."""
    MATERIALIZED_VIEW = "MATERIALIZED_VIEW"
    """A materialized view."""
    METRIC_VIEW = "METRIC_VIEW"
    """A metric view: its definition is YAML."""
    VIEW = "VIEW"
    """A view: its definition is SQL."""


class VolumeType(enum.StrEnum):
    """The type of a volume."""

    MANAGED = "MANAGED"
    """A volume whose files Unity Catalog manages, below the managed storage of its schema or
    catalog.
    """
    EXTERNAL = "EXTERNAL"
    """A volume whose files are at a storage location that the user gives."""
