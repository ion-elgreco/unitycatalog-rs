"""
The classes of the `iceberg` API.
"""

from _typeshed import SupportsGetItem
from typing import Any, SupportsIndex, final

__all__ = [
    "IcebergBlobMetadata",
    "IcebergConfigResponse",
    "IcebergEncryptedKey",
    "IcebergListNamespacesResponse",
    "IcebergListTablesResponse",
    "IcebergLoadCredentialsResponse",
    "IcebergLoadTableResponse",
    "IcebergMetadataLogEntry",
    "IcebergNamespaceResponse",
    "IcebergPartitionField",
    "IcebergPartitionSpec",
    "IcebergPartitionStatisticsFile",
    "IcebergSchema",
    "IcebergSnapshot",
    "IcebergSnapshotLogEntry",
    "IcebergSnapshotReference",
    "IcebergSortField",
    "IcebergSortOrder",
    "IcebergStatisticsFile",
    "IcebergStorageCredential",
    "IcebergStructField",
    "IcebergTableIdentifier",
    "IcebergTableMetadata",
    "IcebergTableRequirement",
    "IcebergTableUpdate",
    "IcebergUpdateNamespacePropertiesResponse",
]

@final
class IcebergBlobMetadata:
    """
    A blob of a statistics file.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        blob_type: str | None = None,
        snapshot_id: SupportsIndex | None = None,
        sequence_number: SupportsIndex | None = None,
        fields: SupportsGetItem[int, SupportsIndex] | None = None,
        properties: dict[str, str] | None = None,
    ) -> IcebergBlobMetadata: ...
    def __repr__(self, /) -> str: ...
    @property
    def blob_type(self, /) -> str:
        """
        The type of the blob (JSON `type`), such as `apache-datasketches-theta-v1`.
        """
    @property
    def fields(self, /) -> list[int]:
        """
        Ids of the fields that the blob was computed from.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        Properties of the blob.
        """
    @property
    def sequence_number(self, /) -> int:
        """
        Sequence number of the snapshot that the blob was computed from.
        """
    @property
    def snapshot_id(self, /) -> int:
        """
        Id of the snapshot that the blob was computed from.
        """

@final
class IcebergConfigResponse:
    """
    The client configuration of a catalog.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def defaults(self, /) -> dict[str, str]:
        """
        Properties that a client uses unless its own configuration sets them. Always empty.
        """
    @property
    def endpoints(self, /) -> list[str]:
        """
        The routes that the server serves, such as `GET /v1/{prefix}/namespaces`. The write routes
        are in the list only when Iceberg writes are on.
        """
    @property
    def overrides(self, /) -> dict[str, str]:
        """
        Properties that replace the client's own configuration: `prefix`, the path prefix of the
        routes of the catalog (`catalogs/<catalog>`).
        """

@final
class IcebergEncryptedKey:
    """
    An encryption key of a table, itself encrypted.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        key_id: str | None = None,
        encrypted_key_metadata: str | None = None,
        encrypted_by_id: str | None = None,
        properties: dict[str, str] | None = None,
    ) -> IcebergEncryptedKey: ...
    def __repr__(self, /) -> str: ...
    @property
    def encrypted_by_id(self, /) -> str | None:
        """
        Id of the key that encrypted this key.
        """
    @property
    def encrypted_key_metadata(self, /) -> str:
        """
        The encrypted key metadata, Base64-encoded.
        """
    @property
    def key_id(self, /) -> str:
        """
        Id of the key.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        Properties of the key.
        """

@final
class IcebergListNamespacesResponse:
    """
    A page of namespaces.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def namespaces(self, /) -> list[list[Any]]:
        """
        The namespaces, each as the list of its levels: one level, the schema name.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page and when no page was asked for.
        """

@final
class IcebergListTablesResponse:
    """
    A page of tables: the tables of a namespace that have Iceberg metadata.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def identifiers(self, /) -> list[IcebergTableIdentifier]:
        """
        The tables.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page and when no page was asked for.
        """

@final
class IcebergLoadCredentialsResponse:
    """
    Fresh credentials for the files of a table.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def storage_credentials(self, /) -> list[IcebergStorageCredential]:
        """
        The credentials. Empty for a local table.
        """

@final
class IcebergLoadTableResponse:
    """
    The metadata of a table, with the properties a client needs to read and write its files.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def config(self, /) -> dict[str, str]:
        """
        FileIO properties for the location of the table: vended credentials under Iceberg's
        property names, and the endpoint that renews them. Empty for a local table.
        """
    @property
    def etag(self, /) -> str | None:
        """
        Weak tag of the metadata version and the snapshots mode, from the `ETag` header. Only a
        table load sends it.
        """
    @property
    def metadata(self, /) -> IcebergTableMetadata | None:
        """
        The table metadata.
        """
    @property
    def metadata_location(self, /) -> str | None:
        """
        Location of the current metadata file of the table. Absent for a staged create.
        """

@final
class IcebergMetadataLogEntry:
    """
    An entry of the metadata log: a previous metadata file.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def metadata_file(self, /) -> str:
        """
        Location of the metadata file.
        """
    @property
    def timestamp_ms(self, /) -> int:
        """
        Time at which the metadata file was written, in epoch milliseconds.
        """

@final
class IcebergNamespaceResponse:
    """
    A namespace and its properties.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def namespace(self, /) -> list[str]:
        """
        The levels of the namespace: one level, the schema name.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        Properties of the namespace.
        """

@final
class IcebergPartitionField:
    """
    A partition field: a transform of a source column.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        source_id: SupportsIndex | None = None,
        field_id: SupportsIndex | None = None,
        name: str | None = None,
        transform: str | None = None,
    ) -> IcebergPartitionField: ...
    def __repr__(self, /) -> str: ...
    @property
    def field_id(self, /) -> int:
        """
        Id of the partition field. In requests, 0 lets the server assign it.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the partition field.
        """
    @property
    def source_id(self, /) -> int:
        """
        Id of the source column.
        """
    @property
    def transform(self, /) -> str:
        """
        The transform: `identity`, `year`, `month`, `day`, `hour`, `void`, `bucket[N]` or
        `truncate[W]`.
        """

@final
class IcebergPartitionSpec:
    """
    How a table is partitioned.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        spec_id: SupportsIndex | None = None,
        fields: SupportsGetItem[int, IcebergPartitionField] | None = None,
    ) -> IcebergPartitionSpec: ...
    def __repr__(self, /) -> str: ...
    @property
    def fields(self, /) -> list[IcebergPartitionField]:
        """
        The partition fields. None for an unpartitioned table.
        """
    @property
    def spec_id(self, /) -> int:
        """
        Id of the spec. In requests, the server assigns it.
        """

@final
class IcebergPartitionStatisticsFile:
    """
    A file of partition statistics for a snapshot.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        snapshot_id: SupportsIndex | None = None,
        statistics_path: str | None = None,
        file_size_in_bytes: SupportsIndex | None = None,
    ) -> IcebergPartitionStatisticsFile: ...
    def __repr__(self, /) -> str: ...
    @property
    def file_size_in_bytes(self, /) -> int:
        """
        Size of the file, in bytes.
        """
    @property
    def snapshot_id(self, /) -> int:
        """
        Id of the snapshot that the statistics describe.
        """
    @property
    def statistics_path(self, /) -> str:
        """
        Location of the statistics file.
        """

@final
class IcebergSchema:
    """
    An Iceberg table schema: a struct type with an id.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        schema_type: str,
        fields: SupportsGetItem[int, IcebergStructField],
        *,
        schema_id: SupportsIndex | None = None,
        identifier_field_ids: SupportsGetItem[int, SupportsIndex] | None = None,
    ) -> IcebergSchema: ...
    def __repr__(self, /) -> str: ...
    @property
    def fields(self, /) -> list[IcebergStructField]:
        """
        The fields of the schema.
        """
    @property
    def identifier_field_ids(self, /) -> list[int]:
        """
        Ids of the fields that identify a row.
        """
    @property
    def schema_id(self, /) -> int:
        """
        Id of the schema. In requests, the server assigns the ids.
        """
    @property
    def schema_type(self, /) -> str:
        """
        The type of the schema (JSON `type`): always `struct`.
        """

@final
class IcebergSnapshot:
    """
    A snapshot: the state of a table at a point in time.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        snapshot_id: SupportsIndex | None = None,
        parent_snapshot_id: SupportsIndex | None = None,
        sequence_number: SupportsIndex | None = None,
        timestamp_ms: SupportsIndex | None = None,
        manifest_list: str | None = None,
        summary: dict[str, str] | None = None,
        schema_id: SupportsIndex | None = None,
        first_row_id: SupportsIndex | None = None,
        added_rows: SupportsIndex | None = None,
        key_id: str | None = None,
    ) -> IcebergSnapshot: ...
    def __repr__(self, /) -> str: ...
    @property
    def added_rows(self, /) -> int | None:
        """
        Number of rows the snapshot adds. Format version 3 only.
        """
    @property
    def first_row_id(self, /) -> int | None:
        """
        First row id of the rows the snapshot adds. Format version 3 only.
        """
    @property
    def key_id(self, /) -> str | None:
        """
        Id of the encryption key of the manifest list. Format version 3 only.
        """
    @property
    def manifest_list(self, /) -> str:
        """
        Location of the manifest list of the snapshot.
        """
    @property
    def parent_snapshot_id(self, /) -> int | None:
        """
        Id of the parent snapshot. Absent for a snapshot without a parent.
        """
    @property
    def schema_id(self, /) -> int | None:
        """
        Id of the current schema when the snapshot was made.
        """
    @property
    def sequence_number(self, /) -> int:
        """
        Sequence number of the snapshot. Absent (0) in format version 1.
        """
    @property
    def snapshot_id(self, /) -> int:
        """
        Id of the snapshot.
        """
    @property
    def summary(self, /) -> dict[str, str]:
        """
        Summary of the changes: `operation` (`append`, `replace`, `overwrite` or `delete`) and other
        properties. Requests must include `operation`.
        """
    @property
    def timestamp_ms(self, /) -> int:
        """
        Time at which the snapshot was made, in epoch milliseconds.
        """

@final
class IcebergSnapshotLogEntry:
    """
    An entry of the snapshot log: when a snapshot became current.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def snapshot_id(self, /) -> int:
        """
        Id of the snapshot.
        """
    @property
    def timestamp_ms(self, /) -> int:
        """
        Time at which the snapshot became current, in epoch milliseconds.
        """

@final
class IcebergSnapshotReference:
    """
    A named reference to a snapshot: a branch or a tag.

    `type` is `branch` (with `min-snapshots-to-keep`, `max-snapshot-age-ms` and `max-ref-age-ms`)
    or `tag` (with `max-ref-age-ms`).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def max_ref_age_ms(self, /) -> int | None:
        """
        The maximum age of the reference, in milliseconds. The main branch never expires.
        """
    @property
    def max_snapshot_age_ms(self, /) -> int | None:
        """
        Branch only: the maximum age of the snapshots to keep, in milliseconds.
        """
    @property
    def min_snapshots_to_keep(self, /) -> int | None:
        """
        Branch only: the minimum number of snapshots to keep when snapshots expire.
        """
    @property
    def snapshot_id(self, /) -> int:
        """
        Id of the snapshot that the reference points at.
        """
    @property
    def type(self, /) -> str:
        """
        The kind of reference: `branch` or `tag`.
        """

@final
class IcebergSortField:
    """
    A sort field: a transform of a source column and a direction.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        source_id: SupportsIndex | None = None,
        transform: str | None = None,
        direction: str | None = None,
        null_order: str | None = None,
    ) -> IcebergSortField: ...
    def __repr__(self, /) -> str: ...
    @property
    def direction(self, /) -> str:
        """
        The direction: `asc` or `desc`.
        """
    @property
    def null_order(self, /) -> str:
        """
        Where nulls go: `nulls-first` or `nulls-last`.
        """
    @property
    def source_id(self, /) -> int:
        """
        Id of the source column.
        """
    @property
    def transform(self, /) -> str:
        """
        The transform, as in a partition field, such as `identity`.
        """

@final
class IcebergSortOrder:
    """
    How the data files of a table are sorted.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        order_id: SupportsIndex | None = None,
        fields: SupportsGetItem[int, IcebergSortField] | None = None,
    ) -> IcebergSortOrder: ...
    def __repr__(self, /) -> str: ...
    @property
    def fields(self, /) -> list[IcebergSortField]:
        """
        The sort fields, in order. None for the unsorted order.
        """
    @property
    def order_id(self, /) -> int:
        """
        Id of the sort order. 0 is the unsorted order.
        """

@final
class IcebergStatisticsFile:
    """
    A Puffin file of table statistics for a snapshot.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        snapshot_id: SupportsIndex | None = None,
        statistics_path: str | None = None,
        file_size_in_bytes: SupportsIndex | None = None,
        file_footer_size_in_bytes: SupportsIndex | None = None,
        key_metadata: str | None = None,
        blob_metadata: SupportsGetItem[int, IcebergBlobMetadata] | None = None,
    ) -> IcebergStatisticsFile: ...
    def __repr__(self, /) -> str: ...
    @property
    def blob_metadata(self, /) -> list[IcebergBlobMetadata]:
        """
        The blobs of the file.
        """
    @property
    def file_footer_size_in_bytes(self, /) -> int:
        """
        Size of the file footer, in bytes.
        """
    @property
    def file_size_in_bytes(self, /) -> int:
        """
        Size of the file, in bytes.
        """
    @property
    def key_metadata(self, /) -> str | None:
        """
        Base64-encoded key metadata of an encrypted file.
        """
    @property
    def snapshot_id(self, /) -> int:
        """
        Id of the snapshot that the statistics describe.
        """
    @property
    def statistics_path(self, /) -> str:
        """
        Location of the statistics file.
        """

@final
class IcebergStorageCredential:
    """
    Credentials for the files below a location prefix.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def config(self, /) -> dict[str, str]:
        """
        FileIO properties: the credentials under Iceberg's property names.
        """
    @property
    def prefix(self, /) -> str:
        """
        The location prefix that the credentials are for.
        """

@final
class IcebergStructField:
    """
    A field of a struct type, such as a column of a schema.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        id: SupportsIndex,
        name: str,
        required: bool,
        field_type: Any,
        *,
        doc: str | None = None,
        initial_default: Any | None = None,
        write_default: Any | None = None,
    ) -> IcebergStructField: ...
    def __repr__(self, /) -> str: ...
    @property
    def doc(self, /) -> str | None:
        """
        Description of the field.
        """
    @property
    def field_type(self, /) -> Any:
        """
        The Iceberg type of the field (JSON `type`): a string for a primitive type (`long`,
        `decimal(10,2)`, `fixed[16]`), an object for a `struct`, `list` or `map` type.
        """
    @property
    def id(self, /) -> int:
        """
        Id of the field, unique in the table schema.
        """
    @property
    def initial_default(self, /) -> Any:
        """
        Value of the field in the rows written before the field was added, in Iceberg's JSON form
        for single values.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the field.
        """
    @property
    def required(self, /) -> bool:
        """
        Whether the field cannot be null.
        """
    @property
    def write_default(self, /) -> Any:
        """
        Value of the field when a writer does not give one, in Iceberg's JSON form for single
        values.
        """

@final
class IcebergTableIdentifier:
    """
    A table by its namespace and name.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, namespace: SupportsGetItem[int, str], name: str
    ) -> IcebergTableIdentifier: ...
    def __repr__(self, /) -> str: ...
    @property
    def name(self, /) -> str:
        """
        Name of the table.
        """
    @property
    def namespace(self, /) -> list[str]:
        """
        The levels of the namespace: one level, the schema name.
        """

@final
class IcebergTableMetadata:
    """
    Iceberg table metadata, in format version 1, 2 or 3.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def current_schema_id(self, /) -> int:
        """
        Id of the current schema.
        """
    @property
    def current_snapshot_id(self, /) -> int | None:
        """
        Id of the current snapshot. Absent when the table has no snapshot.
        """
    @property
    def default_sort_order_id(self, /) -> int:
        """
        Id of the default sort order.
        """
    @property
    def default_spec_id(self, /) -> int:
        """
        Id of the default partition spec.
        """
    @property
    def encryption_keys(self, /) -> list[IcebergEncryptedKey]:
        """
        The encryption keys of the table. Format version 3 only.
        """
    @property
    def format_version(self, /) -> int:
        """
        The format version: 1, 2 or 3.
        """
    @property
    def last_column_id(self, /) -> int:
        """
        The highest field id of the table.
        """
    @property
    def last_partition_id(self, /) -> int:
        """
        The highest partition field id of the table.
        """
    @property
    def last_sequence_number(self, /) -> int:
        """
        The highest sequence number of the table. Absent (0) in format version 1.
        """
    @property
    def last_updated_ms(self, /) -> int:
        """
        Time at which the table was last changed, in epoch milliseconds.
        """
    @property
    def location(self, /) -> str:
        """
        Base location of the table.
        """
    @property
    def metadata_log(self, /) -> list[IcebergMetadataLogEntry]:
        """
        The previous metadata files.
        """
    @property
    def next_row_id(self, /) -> int | None:
        """
        The next row id to assign. Format version 3 only.
        """
    @property
    def partition_spec(self, /) -> list[IcebergPartitionField]:
        """
        Format version 1 only: the fields of the default partition spec.
        """
    @property
    def partition_specs(self, /) -> list[IcebergPartitionSpec]:
        """
        The partition specs of the table.
        """
    @property
    def partition_statistics(self, /) -> list[IcebergPartitionStatisticsFile]:
        """
        The partition statistics files.
        """
    @property
    def properties(self, /) -> dict[str, str]:
        """
        Properties of the table.
        """
    @property
    def refs(self, /) -> dict[str, IcebergSnapshotReference]:
        """
        The branches and tags of the table, by name. Absent in format version 1.
        """
    @property
    def schema(self, /) -> IcebergSchema | None:
        """
        Format version 1 only: the current schema.
        """
    @property
    def schemas(self, /) -> list[IcebergSchema]:
        """
        The schemas of the table.
        """
    @property
    def snapshot_log(self, /) -> list[IcebergSnapshotLogEntry]:
        """
        When each snapshot became current.
        """
    @property
    def snapshots(self, /) -> list[IcebergSnapshot]:
        """
        The snapshots of the table. With `snapshots=refs`, only those that a reference points at.
        """
    @property
    def sort_orders(self, /) -> list[IcebergSortOrder]:
        """
        The sort orders of the table.
        """
    @property
    def statistics(self, /) -> list[IcebergStatisticsFile]:
        """
        The table statistics files.
        """
    @property
    def table_uuid(self, /) -> str:
        """
        UUID of the table.
        """

@final
class IcebergTableRequirement:
    """
    A condition that a table must meet for a commit to apply.

    `type` (`requirement_type`) picks the condition and its field:
    - `assert-create`: the table does not exist (no field). The commit creates a staged table.
    - `assert-table-uuid`: `uuid`.
    - `assert-ref-snapshot-id`: `ref` (`ref_name`) and `snapshot-id`; without `snapshot-id` the
    reference must not exist.
    - `assert-last-assigned-field-id`: `last-assigned-field-id`.
    - `assert-current-schema-id`: `current-schema-id`.
    - `assert-last-assigned-partition-id`: `last-assigned-partition-id`.
    - `assert-default-spec-id`: `default-spec-id`.
    - `assert-default-sort-order-id`: `default-sort-order-id`.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        requirement_type: str | None = None,
        uuid: str | None = None,
        ref_name: str | None = None,
        snapshot_id: SupportsIndex | None = None,
        last_assigned_field_id: SupportsIndex | None = None,
        current_schema_id: SupportsIndex | None = None,
        last_assigned_partition_id: SupportsIndex | None = None,
        default_spec_id: SupportsIndex | None = None,
        default_sort_order_id: SupportsIndex | None = None,
    ) -> IcebergTableRequirement: ...
    def __repr__(self, /) -> str: ...
    @property
    def current_schema_id(self, /) -> int | None:
        """
        `assert-current-schema-id`: id of the current schema.
        """
    @property
    def default_sort_order_id(self, /) -> int | None:
        """
        `assert-default-sort-order-id`: id of the default sort order.
        """
    @property
    def default_spec_id(self, /) -> int | None:
        """
        `assert-default-spec-id`: id of the default partition spec.
        """
    @property
    def last_assigned_field_id(self, /) -> int | None:
        """
        `assert-last-assigned-field-id`: the highest field id of the table.
        """
    @property
    def last_assigned_partition_id(self, /) -> int | None:
        """
        `assert-last-assigned-partition-id`: the highest partition field id of the table.
        """
    @property
    def ref_name(self, /) -> str | None:
        """
        `assert-ref-snapshot-id`: name of the branch or tag (JSON `ref`).
        """
    @property
    def requirement_type(self, /) -> str:
        """
        The kind of condition (JSON `type`), such as `assert-current-schema-id`.
        """
    @property
    def snapshot_id(self, /) -> int | None:
        """
        `assert-ref-snapshot-id`: the snapshot that the reference points at.
        """
    @property
    def uuid(self, /) -> str | None:
        """
        `assert-table-uuid`: the UUID of the table.
        """

@final
class IcebergTableUpdate:
    """
    A change to the metadata of a table.

    `action` picks the change and its fields:
    - `upgrade-format-version`: `format-version`.
    - `assign-uuid`: `uuid`.
    - `add-schema`: `schema`.
    - `set-current-schema`: `schema-id` (-1 for the schema added last).
    - `add-spec`: `spec`.
    - `set-default-spec`: `spec-id` (-1 for the spec added last).
    - `add-sort-order`: `sort-order`.
    - `set-default-sort-order`: `sort-order-id` (-1 for the sort order added last).
    - `add-snapshot`: `snapshot`.
    - `set-snapshot-ref`: `ref-name`, `snapshot-id`, `type` (`ref_type`) and the retention fields
    of a `branch` (`min-snapshots-to-keep`, `max-snapshot-age-ms`, `max-ref-age-ms`) or a `tag`
    (`max-ref-age-ms`).
    - `remove-snapshots`: `snapshot-ids`.
    - `remove-snapshot-ref`: `ref-name`.
    - `set-location`: `location`.
    - `set-properties`: `updates`.
    - `remove-properties`: `removals`.
    - `remove-partition-specs`: `spec-ids`.
    - `set-statistics`: `statistics`, and optionally `snapshot-id` (that of the statistics).
    - `remove-statistics`: `snapshot-id`.
    - `set-partition-statistics`: `partition-statistics`.
    - `remove-partition-statistics`: `snapshot-id`.
    - `remove-schemas`: `schema-ids`.
    - `add-encryption-key`: `encryption-key`.
    - `remove-encryption-key`: `key-id`.
    The lists and the map are always sent, empty for the other actions; the server ignores the
    keys that an action does not read.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        action: str | None = None,
        format_version: SupportsIndex | None = None,
        uuid: str | None = None,
        schema: IcebergSchema | None = None,
        schema_id: SupportsIndex | None = None,
        spec: IcebergPartitionSpec | None = None,
        spec_id: SupportsIndex | None = None,
        sort_order: IcebergSortOrder | None = None,
        sort_order_id: SupportsIndex | None = None,
        snapshot: IcebergSnapshot | None = None,
        ref_name: str | None = None,
        snapshot_id: SupportsIndex | None = None,
        ref_type: str | None = None,
        min_snapshots_to_keep: SupportsIndex | None = None,
        max_snapshot_age_ms: SupportsIndex | None = None,
        max_ref_age_ms: SupportsIndex | None = None,
        snapshot_ids: SupportsGetItem[int, SupportsIndex] | None = None,
        location: str | None = None,
        updates: dict[str, str] | None = None,
        removals: SupportsGetItem[int, str] | None = None,
        spec_ids: SupportsGetItem[int, SupportsIndex] | None = None,
        statistics: IcebergStatisticsFile | None = None,
        partition_statistics: IcebergPartitionStatisticsFile | None = None,
        schema_ids: SupportsGetItem[int, SupportsIndex] | None = None,
        encryption_key: IcebergEncryptedKey | None = None,
        key_id: str | None = None,
    ) -> IcebergTableUpdate: ...
    def __repr__(self, /) -> str: ...
    @property
    def action(self, /) -> str:
        """
        The kind of change, such as `add-snapshot`.
        """
    @property
    def encryption_key(self, /) -> IcebergEncryptedKey | None:
        """
        `add-encryption-key`: the encryption key to add.
        """
    @property
    def format_version(self, /) -> int | None:
        """
        `upgrade-format-version`: the format version to upgrade to.
        """
    @property
    def key_id(self, /) -> str | None:
        """
        `remove-encryption-key`: id of the encryption key to remove.
        """
    @property
    def location(self, /) -> str | None:
        """
        `set-location`: the new base location of the table.
        """
    @property
    def max_ref_age_ms(self, /) -> int | None:
        """
        `set-snapshot-ref`: the maximum age of the reference, in milliseconds.
        """
    @property
    def max_snapshot_age_ms(self, /) -> int | None:
        """
        `set-snapshot-ref`, branch only: the maximum age of the snapshots to keep, in milliseconds.
        """
    @property
    def min_snapshots_to_keep(self, /) -> int | None:
        """
        `set-snapshot-ref`, branch only: the minimum number of snapshots to keep when snapshots
        expire.
        """
    @property
    def partition_statistics(self, /) -> IcebergPartitionStatisticsFile | None:
        """
        `set-partition-statistics`: the partition statistics file to set.
        """
    @property
    def ref_name(self, /) -> str | None:
        """
        `set-snapshot-ref` and `remove-snapshot-ref`: name of the branch or tag.
        """
    @property
    def ref_type(self, /) -> str | None:
        """
        `set-snapshot-ref`: the kind of reference (JSON `type`), `branch` or `tag`.
        """
    @property
    def removals(self, /) -> list[str]:
        """
        `remove-properties`: keys of the properties to remove.
        """
    @property
    def schema(self, /) -> IcebergSchema | None:
        """
        `add-schema`: the schema to add.
        """
    @property
    def schema_id(self, /) -> int | None:
        """
        `set-current-schema`: id of the schema to make current, or -1 for the schema added last.
        """
    @property
    def schema_ids(self, /) -> list[int]:
        """
        `remove-schemas`: ids of the schemas to remove.
        """
    @property
    def snapshot(self, /) -> IcebergSnapshot | None:
        """
        `add-snapshot`: the snapshot to add.
        """
    @property
    def snapshot_id(self, /) -> int | None:
        """
        `set-snapshot-ref`: the snapshot that the reference points at. `set-statistics`,
        `remove-statistics` and `remove-partition-statistics`: the snapshot of the statistics.
        """
    @property
    def snapshot_ids(self, /) -> list[int]:
        """
        `remove-snapshots`: ids of the snapshots to remove.
        """
    @property
    def sort_order(self, /) -> IcebergSortOrder | None:
        """
        `add-sort-order`: the sort order to add.
        """
    @property
    def sort_order_id(self, /) -> int | None:
        """
        `set-default-sort-order`: id of the sort order to make the default, or -1 for the sort order
        added last.
        """
    @property
    def spec(self, /) -> IcebergPartitionSpec | None:
        """
        `add-spec`: the partition spec to add.
        """
    @property
    def spec_id(self, /) -> int | None:
        """
        `set-default-spec`: id of the spec to make the default, or -1 for the spec added last.
        """
    @property
    def spec_ids(self, /) -> list[int]:
        """
        `remove-partition-specs`: ids of the partition specs to remove.
        """
    @property
    def statistics(self, /) -> IcebergStatisticsFile | None:
        """
        `set-statistics`: the statistics file to set.
        """
    @property
    def updates(self, /) -> dict[str, str]:
        """
        `set-properties`: the properties to set.
        """
    @property
    def uuid(self, /) -> str | None:
        """
        `assign-uuid`: the new UUID of the table.
        """

@final
class IcebergUpdateNamespacePropertiesResponse:
    """
    The changes of a namespace properties update.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def missing(self, /) -> list[str]:
        """
        Keys to remove that the namespace did not have.
        """
    @property
    def removed(self, /) -> list[str]:
        """
        Keys that were removed.
        """
    @property
    def updated(self, /) -> list[str]:
        """
        Keys that were set.
        """
