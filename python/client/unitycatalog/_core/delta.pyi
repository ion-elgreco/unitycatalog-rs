"""
The classes of the `delta` API.
"""

from .tables import ColumnInfo
from _typeshed import SupportsGetItem
from typing import Any, SupportsIndex, final
from unitycatalog._enums import (
    DeltaCredentialOperation,
    DeltaMaintenanceOperation,
    DeltaTableType,
)

__all__ = [
    "ColumnInfos",
    "DeltaApiCommit",
    "DeltaCatalogConfig",
    "DeltaClusteringDomainMetadata",
    "DeltaCommitInfo",
    "DeltaCommitMetadataProperties",
    "DeltaCommitReport",
    "DeltaCredentialsResponse",
    "DeltaDomainMetadataUpdates",
    "DeltaFileSizeHistogram",
    "DeltaGetCommitsResponse",
    "DeltaLoadTableResponse",
    "DeltaMetadata",
    "DeltaMetricsReport",
    "DeltaProtocol",
    "DeltaRowTrackingDomainMetadata",
    "DeltaStagingTableResponse",
    "DeltaStorageCredential",
    "DeltaStorageCredentialConfig",
    "DeltaStructField",
    "DeltaStructType",
    "DeltaSuggestedProtocol",
    "DeltaTableMetadata",
    "DeltaTableRequirement",
    "DeltaTableUpdate",
    "DeltaUniform",
    "DeltaUniformIceberg",
    "DeltaUniformMetadata",
    "DeltaUniformMetadataIceberg",
]

@final
class ColumnInfos:
    """
    The columns of a Delta metadata change.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, columns: SupportsGetItem[int, ColumnInfo] | None = None
    ) -> ColumnInfos: ...
    def __repr__(self, /) -> str: ...
    @property
    def columns(self, /) -> list[ColumnInfo]:
        """
        The columns of the table.
        """

@final
class DeltaApiCommit:
    """
    A commit of a catalog-managed table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        version: SupportsIndex,
        timestamp: SupportsIndex,
        file_name: str,
        file_size: SupportsIndex,
        file_modification_timestamp: SupportsIndex,
    ) -> DeltaApiCommit: ...
    def __repr__(self, /) -> str: ...
    @property
    def file_modification_timestamp(self, /) -> int:
        """
        Time at which the commit file was last changed, in epoch milliseconds.
        """
    @property
    def file_name(self, /) -> str:
        """
        Name of the staged commit file under `_delta_log/_staged_commits`, without a path.
        """
    @property
    def file_size(self, /) -> int:
        """
        Size of the commit file, in bytes.
        """
    @property
    def timestamp(self, /) -> int:
        """
        The in-commit timestamp, in epoch milliseconds.
        """
    @property
    def version(self, /) -> int:
        """
        Version of the commit.
        """

@final
class DeltaCatalogConfig:
    """
    The configuration of the Delta API for a catalog.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def endpoints(self, /) -> list[str]:
        """
        The endpoints that the server supports, such as
        `GET /v1/catalogs/main/schemas/{schema}/tables/{table}`.
        """
    @property
    def protocol_version(self, /) -> str:
        """
        The protocol version of the endpoints, such as `1.0`.
        """

@final
class DeltaClusteringDomainMetadata:
    """
    The domain metadata of a clustered table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, clustering_columns: Any | None = None
    ) -> DeltaClusteringDomainMetadata: ...
    def __repr__(self, /) -> str: ...
    @property
    def clustering_columns(self, /) -> list[list[Any]]:
        """
        The clustering columns. Each one is a list of strings: the path from the root of the schema to
        the column, with physical names when column mapping is on. An empty list is sent too: the
        table is clustered by no columns.
        """

@final
class DeltaCommitInfo:
    """
    One Delta commit file.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        version: SupportsIndex,
        timestamp: SupportsIndex,
        file_name: str,
        file_size: SupportsIndex,
        file_modification_timestamp: SupportsIndex,
    ) -> DeltaCommitInfo: ...
    def __repr__(self, /) -> str: ...
    @property
    def file_modification_timestamp(self, /) -> int:
        """
        Time at which the commit file was last changed, in epoch milliseconds. It must be positive.
        """
    @property
    def file_name(self, /) -> str:
        """
        Name of the commit file under `_delta_log/_staged_commits`, without a path.
        """
    @property
    def file_size(self, /) -> int:
        """
        Size of the commit file, in bytes. It must be positive.
        """
    @property
    def timestamp(self, /) -> int:
        """
        The in-commit timestamp that the client wrote, in epoch milliseconds. It must be positive.
        """
    @property
    def version(self, /) -> int:
        """
        Version of the commit. It must be positive.
        """

@final
class DeltaCommitMetadataProperties:
    """
    The properties of a Delta metadata change.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, properties: dict[str, str] | None = None
    ) -> DeltaCommitMetadataProperties: ...
    def __repr__(self, /) -> str: ...
    @property
    def properties(self, /) -> dict[str, str]:
        """
        The properties of the table. They replace the properties of the table.
        """

@final
class DeltaCommitReport:
    """
    The metrics of a commit.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        num_files_added: SupportsIndex | None = None,
        num_bytes_added: SupportsIndex | None = None,
        num_files_removed: SupportsIndex | None = None,
        num_bytes_removed: SupportsIndex | None = None,
        num_rows_inserted: SupportsIndex | None = None,
        num_rows_removed: SupportsIndex | None = None,
        num_rows_updated: SupportsIndex | None = None,
        file_size_histogram: DeltaFileSizeHistogram | None = None,
    ) -> DeltaCommitReport: ...
    def __repr__(self, /) -> str: ...
    @property
    def file_size_histogram(self, /) -> DeltaFileSizeHistogram | None:
        """
        File counts and total bytes by file size.
        """
    @property
    def num_bytes_added(self, /) -> int | None:
        """
        Number of bytes that the commit added.
        """
    @property
    def num_bytes_removed(self, /) -> int | None:
        """
        Number of bytes that the commit removed.
        """
    @property
    def num_files_added(self, /) -> int | None:
        """
        Number of files that the commit added.
        """
    @property
    def num_files_removed(self, /) -> int | None:
        """
        Number of files that the commit removed.
        """
    @property
    def num_rows_inserted(self, /) -> int | None:
        """
        Number of rows that the commit inserted.
        """
    @property
    def num_rows_removed(self, /) -> int | None:
        """
        Number of rows that the commit removed.
        """
    @property
    def num_rows_updated(self, /) -> int | None:
        """
        Number of rows that the commit updated.
        """

@final
class DeltaCredentialsResponse:
    """
    Temporary credentials for the data of a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def storage_credentials(self, /) -> list[DeltaStorageCredential]:
        """
        The temporary cloud storage credentials.
        """

@final
class DeltaDomainMetadataUpdates:
    """
    Domain metadata entries, by domain name.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        delta_clustering: DeltaClusteringDomainMetadata | None = None,
        delta_row_tracking: DeltaRowTrackingDomainMetadata | None = None,
    ) -> DeltaDomainMetadataUpdates: ...
    def __repr__(self, /) -> str: ...
    @property
    def delta_clustering(self, /) -> DeltaClusteringDomainMetadata | None:
        """
        The `delta.clustering` domain.
        """
    @property
    def delta_row_tracking(self, /) -> DeltaRowTrackingDomainMetadata | None:
        """
        The `delta.rowTracking` domain.
        """

@final
class DeltaFileSizeHistogram:
    """
    A file size histogram of a commit.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        sorted_bin_boundaries: SupportsGetItem[int, SupportsIndex] | None = None,
        file_counts: SupportsGetItem[int, SupportsIndex] | None = None,
        total_bytes: SupportsGetItem[int, SupportsIndex] | None = None,
        commit_version: SupportsIndex | None = None,
    ) -> DeltaFileSizeHistogram: ...
    def __repr__(self, /) -> str: ...
    @property
    def commit_version(self, /) -> int | None:
        """
        Version of the commit that the histogram is for.
        """
    @property
    def file_counts(self, /) -> list[int]:
        """
        Number of files in each bin.
        """
    @property
    def sorted_bin_boundaries(self, /) -> list[int]:
        """
        Start of each bin (inclusive), in bytes, sorted. The first one is 0.
        """
    @property
    def total_bytes(self, /) -> list[int]:
        """
        Total bytes in each bin.
        """

@final
class DeltaGetCommitsResponse:
    """
    The commits that the server tracks for a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def commits(self, /) -> list[DeltaCommitInfo]:
        """
        The commits that are not backfilled yet, newest first.
        """
    @property
    def latest_table_version(self, /) -> int:
        """
        The latest version that the server tracks; 0 before the first commit.
        """

@final
class DeltaLoadTableResponse:
    """
    A Delta table: its metadata and, for a MANAGED table, its commits that are not backfilled yet.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def allowed_maintenance_operations(self, /) -> list[DeltaMaintenanceOperation]:
        """
        Maintenance operations that the catalog allows in addition to the Delta protocol defaults.
        """
    @property
    def commits(self, /) -> list[DeltaApiCommit]:
        """
        The commits that are not backfilled yet, newest first. Only for MANAGED tables.
        """
    @property
    def latest_table_version(self, /) -> int | None:
        """
        The latest version that the server tracks, data-only commits included. Only for MANAGED
        tables.
        """
    @property
    def metadata(self, /) -> DeltaTableMetadata | None:
        """
        The metadata of the table.
        """
    @property
    def uniform(self, /) -> DeltaUniformMetadata | None:
        """
        UniForm conversion metadata. Only for UniForm tables.
        """

@final
class DeltaMetadata:
    """
    A Delta metadata change to apply to the table with the commit. At least one field must be
    set.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        description: str | None = None,
        schema: ColumnInfos | None = None,
        properties: DeltaCommitMetadataProperties | None = None,
    ) -> DeltaMetadata: ...
    def __repr__(self, /) -> str: ...
    @property
    def description(self, /) -> str | None:
        """
        The comment of the table.
        """
    @property
    def properties(self, /) -> DeltaCommitMetadataProperties | None:
        """
        The properties of the table.
        """
    @property
    def schema(self, /) -> ColumnInfos | None:
        """
        The columns of the table. They replace the columns of the table.
        """

@final
class DeltaMetricsReport:
    """
    The reported metrics.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, commit_report: DeltaCommitReport | None = None
    ) -> DeltaMetricsReport: ...
    def __repr__(self, /) -> str: ...
    @property
    def commit_report(self, /) -> DeltaCommitReport | None:
        """
        The metrics of a commit.
        """

@final
class DeltaProtocol:
    """
    Delta protocol versions and features.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        min_reader_version: SupportsIndex | None = None,
        min_writer_version: SupportsIndex | None = None,
        reader_features: SupportsGetItem[int, str] | None = None,
        writer_features: SupportsGetItem[int, str] | None = None,
    ) -> DeltaProtocol: ...
    def __repr__(self, /) -> str: ...
    @property
    def min_reader_version(self, /) -> int | None:
        """
        Minimum reader version.
        """
    @property
    def min_writer_version(self, /) -> int | None:
        """
        Minimum writer version.
        """
    @property
    def reader_features(self, /) -> list[str]:
        """
        The reader features, such as `deletionVectors`.
        """
    @property
    def writer_features(self, /) -> list[str]:
        """
        The writer features, such as `catalogManaged`.
        """

@final
class DeltaRowTrackingDomainMetadata:
    """
    The domain metadata of a table with row tracking.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, row_id_high_water_mark: SupportsIndex | None = None
    ) -> DeltaRowTrackingDomainMetadata: ...
    def __repr__(self, /) -> str: ...
    @property
    def row_id_high_water_mark(self, /) -> int | None:
        """
        The highest row id given so far.
        """

@final
class DeltaStagingTableResponse:
    """
    A staging table: the table id and storage location that the server reserves for a new MANAGED
    table, and what the first commit must contain.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def location(self, /) -> str:
        """
        The storage location of the table.
        """
    @property
    def required_properties(self, /) -> dict[str, Any]:
        """
        Table properties that the table must have. Each value is a string, or `null` when any valid
        value is allowed.
        """
    @property
    def required_protocol(self, /) -> DeltaProtocol | None:
        """
        The protocol that the first commit must have at least.
        """
    @property
    def storage_credentials(self, /) -> list[DeltaStorageCredential]:
        """
        Temporary credentials to write the first commit.
        """
    @property
    def suggested_properties(self, /) -> dict[str, Any]:
        """
        Table properties that the table should have. Each value is a string, or `null` when the client
        makes the value.
        """
    @property
    def suggested_protocol(self, /) -> DeltaSuggestedProtocol | None:
        """
        Features that the client should enable when it can.
        """
    @property
    def table_id(self, /) -> str:
        """
        Unique identifier of the table.
        """
    @property
    def table_type(self, /) -> DeltaTableType | None:
        """
        Type of the table. Always `MANAGED`.
        """

@final
class DeltaStorageCredential:
    """
    A temporary credential for a storage prefix. When more than one credential applies, use the
    one with the longest prefix.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def config(self, /) -> DeltaStorageCredentialConfig | None:
        """
        The cloud credential settings.
        """
    @property
    def expiration_time_ms(self, /) -> int | None:
        """
        Time at which the credential expires, in epoch milliseconds.
        """
    @property
    def operation(self, /) -> DeltaCredentialOperation | None:
        """
        The access level of the credential.
        """
    @property
    def prefix(self, /) -> str:
        """
        The storage path prefix that the credential is for.
        """

@final
class DeltaStorageCredentialConfig:
    """
    Cloud credential settings. Only the keys of one cloud provider are set.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def azure_sas_token(self, /) -> str | None:
        """
        Azure SAS (shared access signature) token.
        """
    @property
    def gcs_oauth_token(self, /) -> str | None:
        """
        GCP OAuth token.
        """
    @property
    def s3_access_key_id(self, /) -> str | None:
        """
        AWS access key id.
        """
    @property
    def s3_secret_access_key(self, /) -> str | None:
        """
        AWS secret access key.
        """
    @property
    def s3_session_token(self, /) -> str | None:
        """
        AWS session token of the temporary credentials.
        """

@final
class DeltaStructField:
    """
    A field of a Delta struct type, as in the Delta protocol schema format.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, name: str, data_type: Any, nullable: bool, metadata: Any
    ) -> DeltaStructField: ...
    def __repr__(self, /) -> str: ...
    @property
    def data_type(self, /) -> Any:
        """
        Data type of the field: a string for a primitive type (`long`, `decimal(10,2)`), or an object
        with a `type` of `array` (`element-type`, `contains-null`), `map` (`key-type`, `value-type`,
        `value-contains-null`), `struct` (`fields`) or `decimal` (`precision`, `scale`).
        """
    @property
    def metadata(self, /) -> dict[str, Any] | None:
        """
        Metadata of the field, such as `comment` or `delta.columnMapping.id`. An empty object when
        there is none.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the field. It is unique in its struct, case-insensitively.
        """
    @property
    def nullable(self, /) -> bool:
        """
        Whether the field can hold null values.
        """

@final
class DeltaStructType:
    """
    A Delta struct type, such as the columns of a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        fields: SupportsGetItem[int, DeltaStructField],
        *,
        type_name: str | None = None,
    ) -> DeltaStructType: ...
    def __repr__(self, /) -> str: ...
    @property
    def fields(self, /) -> list[DeltaStructField]:
        """
        The fields of the struct. At least one.
        """
    @property
    def type_name(self, /) -> str:
        """
        The type name. Always `struct`.
        """

@final
class DeltaSuggestedProtocol:
    """
    Features that a client should enable when it can.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def reader_features(self, /) -> list[str]:
        """
        The suggested reader features.
        """
    @property
    def writer_features(self, /) -> list[str]:
        """
        The suggested writer features.
        """

@final
class DeltaTableMetadata:
    """
    The metadata of a Delta table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def columns(self, /) -> DeltaStructType | None:
        """
        The columns of the table.
        """
    @property
    def created_time(self, /) -> int:
        """
        Time at which the table was created, in epoch milliseconds.
        """
    @property
    def etag(self, /) -> str:
        """
        Entity tag of the metadata. It changes when the metadata changes.
        """
    @property
    def last_commit_timestamp_ms(self, /) -> int | None:
        """
        Timestamp of the last commit that changed the metadata, in epoch milliseconds.
        """
    @property
    def last_commit_version(self, /) -> int | None:
        """
        Version of the last commit that changed the metadata.
        """
    @property
    def location(self, /) -> str:
        """
        Storage location of the table.
        """
    @property
    def partition_columns(self, /) -> list[str]:
        """
        Names of the partition columns.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        The table properties, with keys that the server derives (`delta.minReaderVersion`,
        `delta.feature.*`, `delta.lastUpdateVersion`, ...). Clients must not write those back.
        """
    @property
    def table_type(self, /) -> DeltaTableType | None:
        """
        Type of the table.
        """
    @property
    def table_uuid(self, /) -> str:
        """
        Unique identifier of the table.
        """
    @property
    def updated_time(self, /) -> int:
        """
        Time at which the table was last changed, in epoch milliseconds.
        """

@final
class DeltaTableRequirement:
    """
    A precondition of a table update. The `type` says which field is set:
    - `assert-table-uuid`: `uuid`, the expected table id.
    - `assert-etag`: `etag`, the expected entity tag of the metadata.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        requirement_type: str,
        *,
        uuid: str | None = None,
        etag: str | None = None,
    ) -> DeltaTableRequirement: ...
    def __repr__(self, /) -> str: ...
    @property
    def etag(self, /) -> str | None:
        """
        The expected entity tag of the table metadata. For `assert-etag`.
        """
    @property
    def requirement_type(self, /) -> str:
        """
        Kind of the requirement, the JSON `type`: `assert-table-uuid` or `assert-etag`.
        """
    @property
    def uuid(self, /) -> str | None:
        """
        The expected unique identifier of the table. For `assert-table-uuid`.
        """

@final
class DeltaTableUpdate:
    """
    One update action. The `action` says which fields are set:
    - `set-properties`: `updates`, property names to string values.
    - `remove-properties`: `removals`.
    - `set-columns`: `columns`.
    - `set-table-comment`: `comment`.
    - `add-commit`: `commit` and `uniform`.
    - `set-latest-backfilled-version`: `latest_published_version`.
    - `set-protocol`: `protocol`.
    - `set-domain-metadata`: `updates`, domain names to their metadata:
    `delta.clustering` to `{"clusteringColumns": [["a"], ["b", "c"]]}`, `delta.rowTracking` to
    `{"rowIdHighWaterMark": 9}`.
    - `remove-domain-metadata`: `domains`.
    - `set-partition-columns`: `partition_columns`.
    - `update-metadata-snapshot-version`: `last_commit_version` and `last_commit_timestamp_ms`.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        action: str,
        *,
        updates: Any | None = None,
        removals: SupportsGetItem[int, str] | None = None,
        columns: DeltaStructType | None = None,
        comment: str | None = None,
        commit: DeltaApiCommit | None = None,
        uniform: DeltaUniformMetadata | None = None,
        latest_published_version: SupportsIndex | None = None,
        protocol: DeltaProtocol | None = None,
        domains: SupportsGetItem[int, str] | None = None,
        partition_columns: SupportsGetItem[int, str] | None = None,
        last_commit_version: SupportsIndex | None = None,
        last_commit_timestamp_ms: SupportsIndex | None = None,
    ) -> DeltaTableUpdate: ...
    def __repr__(self, /) -> str: ...
    @property
    def action(self, /) -> str:
        """
        Kind of the update: one of the actions above.
        """
    @property
    def columns(self, /) -> DeltaStructType | None:
        """
        For `set-columns`: the new columns of the table.
        """
    @property
    def comment(self, /) -> str | None:
        """
        For `set-table-comment`: the new comment of the table.
        """
    @property
    def commit(self, /) -> DeltaApiCommit | None:
        """
        For `add-commit`: the commit to add.
        """
    @property
    def domains(self, /) -> list[str]:
        """
        For `remove-domain-metadata`: the domain names to remove.
        """
    @property
    def last_commit_timestamp_ms(self, /) -> int | None:
        """
        For `update-metadata-snapshot-version`: timestamp of the last commit, in epoch milliseconds
        (`delta.lastCommitTimestamp`).
        """
    @property
    def last_commit_version(self, /) -> int | None:
        """
        For `update-metadata-snapshot-version`: version of the last commit
        (`delta.lastUpdateVersion`).
        """
    @property
    def latest_published_version(self, /) -> int | None:
        """
        For `set-latest-backfilled-version`: the commits up to this version are backfilled.
        """
    @property
    def partition_columns(self, /) -> list[str]:
        """
        For `set-partition-columns`: names of the partition columns. An empty list is sent too: it
        removes the partitioning.
        """
    @property
    def protocol(self, /) -> DeltaProtocol | None:
        """
        For `set-protocol`: the new protocol. It replaces the protocol of the table.
        """
    @property
    def removals(self, /) -> list[str]:
        """
        For `remove-properties`: the property keys to remove.
        """
    @property
    def uniform(self, /) -> DeltaUniformMetadata | None:
        """
        For `add-commit`: UniForm metadata to commit with the commit.
        """
    @property
    def updates(self, /) -> dict[str, Any]:
        """
        For `set-properties`: property names to string values. For `set-domain-metadata`: domain
        names to their metadata objects.
        """

@final
class DeltaUniform:
    """
    UniForm metadata committed with a Delta commit.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, iceberg: DeltaUniformIceberg | None = None
    ) -> DeltaUniform: ...
    def __repr__(self, /) -> str: ...
    @property
    def iceberg(self, /) -> DeltaUniformIceberg | None:
        """
        The Iceberg metadata of the table.
        """

@final
class DeltaUniformIceberg:
    """
    The Iceberg metadata of a UniForm table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        metadata_location: str | None = None,
        converted_delta_version: SupportsIndex | None = None,
        converted_delta_timestamp: str | None = None,
        base_converted_delta_version: SupportsIndex | None = None,
    ) -> DeltaUniformIceberg: ...
    def __repr__(self, /) -> str: ...
    @property
    def base_converted_delta_version(self, /) -> int | None:
        """
        The Delta version of the Iceberg metadata that the conversion started from.
        """
    @property
    def converted_delta_timestamp(self, /) -> str | None:
        """
        When the Delta version was converted, as an ISO-8601 instant in UTC.
        """
    @property
    def converted_delta_version(self, /) -> int | None:
        """
        The Delta version that the Iceberg metadata was converted from.
        """
    @property
    def metadata_location(self, /) -> str | None:
        """
        URL of the Iceberg metadata file. It must be inside the table.
        """

@final
class DeltaUniformMetadata:
    """
    UniForm conversion metadata. Only UniForm (Delta and Iceberg) tables have it.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, *, iceberg: DeltaUniformMetadataIceberg | None = None
    ) -> DeltaUniformMetadata: ...
    def __repr__(self, /) -> str: ...
    @property
    def iceberg(self, /) -> DeltaUniformMetadataIceberg | None:
        """
        The Iceberg metadata of the table.
        """

@final
class DeltaUniformMetadataIceberg:
    """
    The Iceberg metadata of a UniForm table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        metadata_location: str | None = None,
        converted_delta_version: SupportsIndex | None = None,
        converted_delta_timestamp: SupportsIndex | None = None,
        base_converted_delta_version: SupportsIndex | None = None,
    ) -> DeltaUniformMetadataIceberg: ...
    def __repr__(self, /) -> str: ...
    @property
    def base_converted_delta_version(self, /) -> int | None:
        """
        The Delta version that the incremental conversion started from.
        """
    @property
    def converted_delta_timestamp(self, /) -> int | None:
        """
        When the Delta version was converted, in epoch milliseconds.
        """
    @property
    def converted_delta_version(self, /) -> int | None:
        """
        The Delta version that was converted to Iceberg.
        """
    @property
    def metadata_location(self, /) -> str | None:
        """
        URL of the Iceberg metadata file.
        """
