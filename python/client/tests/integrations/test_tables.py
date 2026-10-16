"""polars and deltalake write Delta tables of Unity Catalog: the server keeps
the table and its storage location, and the engines write the files there.
On S3, the engines reach the files with credentials that the server vends."""

import json
import subprocess
import sys
import typing
from pathlib import Path

import pytest
from _helpers import unique

from unitycatalog import Client, ColumnInfo, StagingTableInfo, TableInfo
from unitycatalog.types import ColumnTypeName

pl = pytest.importorskip("polars")
deltalake = pytest.importorskip("deltalake")

# The types of a Delta schema, as the type name and text of a column.
TYPES: dict[str, tuple[ColumnTypeName, str]] = {
    "long": ("LONG", "bigint"),
    "double": ("DOUBLE", "double"),
}


def column(position: int, field: dict[str, typing.Any]) -> ColumnInfo:
    """The column of a field of a Delta schema."""
    type_name, type_text = TYPES[field["type"]]
    return ColumnInfo(
        field["name"],
        type_name=type_name,
        type_text=type_text,
        type_json=json.dumps(field),
        position=position,
        nullable=field["nullable"],
    )


def create_managed_table(
    client: Client,
    staging: StagingTableInfo,
    storage_options: dict[str, typing.Any] | None = None,
) -> TableInfo:
    """The managed table of `staging`, whose first version deltalake wrote. As
    engines make a managed table: the staging table gives it its id and
    location, and the table takes the place of the staging table."""
    assert staging.name and staging.catalog_name and staging.schema_name
    assert staging.id
    location = staging.staging_location
    written = deltalake.DeltaTable(location, storage_options=storage_options)
    fields = json.loads(written.schema().to_json())["fields"]
    return client.create_table(
        staging.name,
        staging.catalog_name,
        staging.schema_name,
        "MANAGED",
        columns=[column(position, field) for position, field in enumerate(fields)],
        data_source_format="DELTA",
        storage_location=location,
        properties={"io.unitycatalog.tableId": staging.id},
    )


# todo: investigate upstream why it hangs in deltalake
def append_by_name(
    name: str, storage_options: dict[str, typing.Any], rows: dict[str, typing.Any]
) -> int:
    """deltalake appends `rows` to the table `name` (`uc://…`) in a process of
    its own; the version of the table after it. deltalake 1.6.6's first
    `uc://` call in a process sometimes never returns, so a run that takes
    longer than 60 s is stopped and started again, up to 3 times."""
    script = Path(__file__).with_name("_deltalake_by_name.py")
    options, data = json.dumps(storage_options), json.dumps(rows)
    for _ in range(3):
        try:
            done = subprocess.run(
                [sys.executable, str(script), name, options, data],
                check=False,
                capture_output=True,
                text=True,
                timeout=60,
            )
        except subprocess.TimeoutExpired:
            continue
        assert done.returncode == 0, done.stderr
        return int(done.stdout)
    pytest.fail(f"deltalake did not open {name} in 3 runs of 60 s")


def test_polars_creates_writes_and_reads_an_external_table(
    server, schema: tuple[str, str], tmp_path_factory: pytest.TempPathFactory
) -> None:
    catalog, schema_name = schema
    location = tmp_path_factory.mktemp("people").as_uri()
    polars = pl.Catalog(server.address, require_https=False)
    people = pl.DataFrame({"id": [1, 2], "name": ["ann", "bo"]})
    polars.create_table(
        catalog,
        schema_name,
        "people",
        schema=people.schema,
        table_type="EXTERNAL",
        data_source_format="DELTA",
        storage_root=location,
    )
    table = Client(server.url).get_table(f"{catalog}.{schema_name}.people")
    assert (table.table_type, table.storage_location) == ("EXTERNAL", location)
    assert [(c.name, c.type_name) for c in table.columns or []] == [
        ("id", "LONG"),
        ("name", "STRING"),
    ]

    # Each write is a version of the Delta table.
    polars.write_table(people, catalog, schema_name, "people", delta_mode="append")
    polars.write_table(people, catalog, schema_name, "people", delta_mode="append")
    both = polars.scan_table(catalog, schema_name, "people").collect()
    assert both.equals(pl.concat([people, people]))
    first = polars.scan_table(catalog, schema_name, "people", delta_table_version=0)
    assert first.collect().equals(people)


def test_deltalake_creates_a_managed_table_that_polars_appends_to(
    server, schema: tuple[str, str]
) -> None:
    catalog, schema_name = schema
    client = Client(server.url)
    staging = client.create_staging_table("orders", catalog, schema_name)
    january = pl.DataFrame({"order_id": [1, 2], "amount": [9.5, 3.25]})
    deltalake.write_deltalake(staging.staging_location, january)
    table = create_managed_table(client, staging)
    assert (table.table_id, table.storage_location) == (
        staging.id,
        staging.staging_location,
    )

    february = pl.DataFrame({"order_id": [3], "amount": [12.0]})
    polars = pl.Catalog(server.address, require_https=False)
    polars.write_table(february, catalog, schema_name, "orders", delta_mode="append")
    assert deltalake.DeltaTable(table.storage_location).version() == 1
    orders = polars.scan_table(catalog, schema_name, "orders").collect()
    assert orders.sort("order_id").equals(pl.concat([january, february]))


def test_deltalake_writes_and_reads_a_table_on_s3_by_name(s3: str, s3_server) -> None:
    client = Client(s3_server.url)
    catalog = unique("catalog")
    client.create_catalog(catalog, storage_root=f"s3://uc-test/{catalog}")
    client.create_schema("default", catalog)
    staging = client.create_staging_table("orders", catalog, "default")
    assert staging.id
    # The first version, with the credentials of the staging table.
    vended = client.generate_temporary_table_credentials(staging.id, "READ_WRITE")
    aws = vended.aws_temp_credentials
    assert aws and vended.expiration_time
    # Where moto is: the engines get the credentials from the server.
    endpoint = {
        "aws_endpoint_url": s3,
        "aws_allow_http": "true",
        "aws_region": "us-east-1",
    }
    credentials = {
        **endpoint,
        "aws_access_key_id": aws.access_key_id,
        "aws_secret_access_key": aws.secret_access_key,
        "aws_session_token": aws.session_token,
    }
    january = pl.DataFrame({"order_id": [1, 2], "amount": [9.5, 3.25]})
    deltalake.write_deltalake(
        staging.staging_location, january, storage_options=credentials
    )
    create_managed_table(client, staging, credentials)

    # By name, deltalake asks the server for the table's location and for
    # credentials. It sends a token also to a server without sign-in.
    by_name = {
        **endpoint,
        "unity_workspace_url": s3_server.address,
        "unity_access_token": s3_server.token,
        "unity_allow_http_url": "true",
    }
    february = {"order_id": [3], "amount": [12.0]}
    name = f"uc://{catalog}.default.orders"
    assert append_by_name(name, by_name, february) == 1
    # So does polars, through its Unity Catalog client.
    polars = pl.Catalog(s3_server.address, require_https=False)
    orders = polars.scan_table(catalog, "default", "orders", storage_options=endpoint)
    expected = pl.concat([january, pl.DataFrame(february)])
    assert orders.collect().sort("order_id").equals(expected)
