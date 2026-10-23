"""The delta-sharing client reads a table that the server shares: its latest
version, an earlier version and its changes, in the parquet and the delta
response formats. On S3 the file URLs are presigned; local files come from
the server itself."""

import json
import typing
from pathlib import Path

import pytest
from _helpers import unique

from unitycatalog import Client, ColumnInfo

delta_sharing = pytest.importorskip("delta_sharing")
deltalake = pytest.importorskip("deltalake")
pd = pytest.importorskip("pandas")


def write_events(location: str, storage_options: dict[str, str] | None = None) -> None:
    """Version 0 adds ids 1 to 3, version 1 adds 4, version 2 deletes 1. The
    change data feed is on, so the delete writes change data files."""
    options = {"storage_options": storage_options} if storage_options else {}
    first = pd.DataFrame({"id": [1, 2, 3], "region": ["eu", "eu", "us"]})
    deltalake.write_deltalake(
        location,
        first,
        partition_by=["region"],
        configuration={"delta.enableChangeDataFeed": "true"},
        **options,
    )
    second = pd.DataFrame({"id": [4], "region": ["us"]})
    deltalake.write_deltalake(location, second, mode="append", **options)
    deltalake.DeltaTable(location, **options).delete("id = 1")


def column(name: str, position: int, delta_type: str) -> ColumnInfo:
    type_name: typing.Any = {"long": "LONG", "string": "STRING"}[delta_type]
    field = {"name": name, "type": delta_type, "nullable": True, "metadata": {}}
    return ColumnInfo(
        name,
        type_name=type_name,
        type_text="bigint" if delta_type == "long" else delta_type,
        type_json=json.dumps(field),
        position=position,
        nullable=True,
    )


def share_events(client: Client, catalog: str, location: str, profile: Path) -> str:
    """Register the table at `location` as `<catalog>.default.events`, share it
    with its history, and write the profile of a new recipient of the share to
    `profile`. The URL of the shared table for delta-sharing."""
    client.create_table(
        "events",
        catalog,
        "default",
        "EXTERNAL",
        columns=[column("id", 0, "long"), column("region", 1, "string")],
        data_source_format="DELTA",
        storage_location=location,
    )
    share, recipient = unique("share"), unique("recipient")
    client.raw_json("POST", "shares", body={"name": share})
    table = {
        "source_table_full_name": f"{catalog}.default.events",
        "history_data_sharing_status": "ENABLED",
    }
    client.raw_json("PATCH", f"shares/{share}", body={"add_objects": [table]})
    client.raw_json("POST", "recipients", body={"name": recipient})
    client.raw_json("PATCH", f"recipients/{recipient}", body={"add_shares": [share]})
    profile.write_text(
        json.dumps(client.raw_json("POST", f"recipients/{recipient}/profile"))
    )
    return f"{profile}#{share}.default.events"


def assert_reads_events(url: str) -> None:
    assert delta_sharing.get_table_version(url) == 2
    assert sorted(delta_sharing.load_as_pandas(url)["id"]) == [2, 3, 4]
    assert sorted(delta_sharing.load_as_pandas(url, version=0)["id"]) == [1, 2, 3]
    assert len(delta_sharing.load_as_pandas(url, limit=1)) == 1
    delta = delta_sharing.load_as_pandas(url, use_delta_format=True)
    assert sorted(delta["id"]) == [2, 3, 4]
    changes = delta_sharing.load_table_changes_as_pandas(url, starting_version=0)
    rows = zip(changes["_commit_version"], changes["id"], changes["_change_type"])
    assert sorted(rows) == [
        (0, 1, "insert"),
        (0, 2, "insert"),
        (0, 3, "insert"),
        (1, 4, "insert"),
        (2, 1, "delete"),
    ]


def test_delta_sharing_reads_a_local_table(
    server, schema: tuple[str, str], tmp_path: Path
) -> None:
    client = Client(server.url).with_bearer(server.token)
    location = tmp_path / "events"
    write_events(str(location))
    url = share_events(client, schema[0], location.as_uri(), tmp_path / "profile.share")
    client_tables = delta_sharing.SharingClient(url.split("#")[0]).list_all_tables()
    assert [t.name for t in client_tables] == ["events"]
    assert_reads_events(url)


def test_delta_sharing_reads_a_table_on_s3(s3: str, s3_server, tmp_path: Path) -> None:
    client = Client(s3_server.url).with_bearer(s3_server.token)
    catalog = unique("catalog")
    client.create_catalog(catalog)
    client.create_schema("default", catalog)
    location = f"s3://uc-test/{catalog}/events"
    # moto takes any keys.
    write_events(
        location,
        {
            "AWS_ENDPOINT_URL": s3,
            "AWS_ALLOW_HTTP": "true",
            "AWS_REGION": "us-east-1",
            "AWS_ACCESS_KEY_ID": "testing",
            "AWS_SECRET_ACCESS_KEY": "testing",
            "AWS_S3_ALLOW_UNSAFE_RENAME": "true",
        },
    )
    url = share_events(client, catalog, location, tmp_path / "profile.share")
    # The files come from the store, by presigned URLs.
    table = delta_sharing.Table("events", url.split("#")[1].split(".")[0], "default")
    rest = delta_sharing.SharingClient(url.split("#")[0])._rest_client
    files = rest.list_files_in_table(table).add_files
    assert files and all(f.url.startswith(f"{s3}/uc-test/") for f in files)
    assert all("X-Amz-Signature=" in f.url for f in files)
    assert_reads_events(url)
