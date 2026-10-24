"""pyiceberg uses each operation of the server's Iceberg REST catalog: the
config, namespaces (the schemas of a catalog), tables (Iceberg tables of
Unity Catalog), commits, credentials, metrics and views. At each commit the
server writes the metadata, and pyiceberg the data files. On S3, the server
sends pyiceberg the credentials and the endpoint of the store."""

import sys
import typing

import pytest
from _helpers import unique

from unitycatalog import Client

pa = pytest.importorskip("pyarrow")
pl = pytest.importorskip("polars")
rest = pytest.importorskip("pyiceberg.catalog.rest")
errors = pytest.importorskip("pyiceberg.exceptions")

SCHEMA = pa.schema([("id", pa.int64())])

# pyiceberg (0.12) writes the data files of a local table at the path of its
# URI: on Windows, `/C:/...` for `file:///C:/...`, which is not a path.
writes_local_data = pytest.mark.skipif(
    sys.platform == "win32",
    reason="pyiceberg cannot write to file:///C:/... locations",
)


def rest_catalog(url: str, warehouse: str, **properties: str) -> typing.Any:
    """pyiceberg's client of the Iceberg REST catalog at the API URL `url`, for
    the catalog `warehouse`."""
    return rest.RestCatalog(
        "uc", uri=f"{url}iceberg", warehouse=warehouse, **properties
    )


def ids(*values: int) -> typing.Any:
    """A table of the `id` column."""
    return pa.table({"id": pa.array(values, pa.int64())})


def test_config_gives_the_prefix_and_the_endpoints_of_a_catalog(
    server, catalog: str
) -> None:
    iceberg = rest_catalog(server.url, catalog)
    assert iceberg.properties["prefix"] == f"catalogs/{catalog}"
    # The config lists the endpoints that the server serves: no register.
    with pytest.raises(NotImplementedError, match="register"):
        iceberg.register_table(("default", "t"), "file:///t/v1.metadata.json")
    with pytest.raises(errors.BadRequestError, match="warehouse"):
        rest.RestCatalog("uc", uri=f"{server.url}iceberg")
    with pytest.raises(errors.RESTError, match="Catalog not found: missing"):
        rest_catalog(server.url, "missing")


def test_namespaces_are_the_schemas_of_the_catalog(server, catalog: str) -> None:
    client = Client(server.address)
    iceberg = rest_catalog(server.url, catalog)
    iceberg.create_namespace("sales", {"team": "finance", "tier": "gold"})
    iceberg.create_namespace("hr")
    assert client.schema(f"{catalog}.sales").get().properties == {
        "team": "finance",
        "tier": "gold",
    }
    with pytest.raises(errors.NamespaceAlreadyExistsError):
        iceberg.create_namespace("sales")
    # pyiceberg asks for one namespace a page. Without a page token, the
    # server sends all of them, as upstream.
    paged = rest_catalog(server.url, catalog, **{"rest-page-size": "1"})
    assert paged.list_namespaces() == [("hr",), ("sales",)]
    assert iceberg.namespace_exists("sales")
    assert not iceberg.namespace_exists("legal")

    assert iceberg.load_namespace_properties("sales") == {
        "team": "finance",
        "tier": "gold",
    }
    with pytest.raises(errors.NoSuchNamespaceError):
        iceberg.load_namespace_properties("legal")
    summary = iceberg.update_namespace_properties(
        "sales", removals={"tier", "region"}, updates={"owner": "ann"}
    )
    assert (summary.removed, summary.updated, summary.missing) == (
        ["tier"],
        ["owner"],
        ["region"],
    )
    assert client.schema(f"{catalog}.sales").get().properties == {
        "team": "finance",
        "owner": "ann",
    }

    iceberg.create_table(("sales", "orders"), schema=SCHEMA)
    with pytest.raises(errors.NamespaceNotEmptyError):
        iceberg.drop_namespace("sales")
    iceberg.drop_table(("sales", "orders"))
    iceberg.drop_namespace("sales")
    assert [s.name for s in client.list_schemas(catalog)] == ["hr"]
    with pytest.raises(errors.NoSuchNamespaceError):
        iceberg.drop_namespace("sales")


def test_tables_are_iceberg_tables_of_unity_catalog(
    server, schema: tuple[str, str]
) -> None:
    catalog, schema_name = schema
    client = Client(server.address)
    iceberg = rest_catalog(server.url, catalog)
    for name in ("a", "b", "c"):
        iceberg.create_table((schema_name, name), schema=SCHEMA)
    table = client.table(f"{catalog}.{schema_name}.a").get()
    assert (table.table_type, table.data_source_format) == ("MANAGED", "ICEBERG")
    with pytest.raises(errors.TableAlreadyExistsError):
        iceberg.create_table((schema_name, "a"), schema=SCHEMA)
    paged = rest_catalog(server.url, catalog, **{"rest-page-size": "1"})
    assert paged.list_tables(schema_name) == [
        (schema_name, "a"),
        (schema_name, "b"),
        (schema_name, "c"),
    ]
    assert iceberg.table_exists((schema_name, "a"))
    assert not iceberg.table_exists((schema_name, "d"))
    with pytest.raises(errors.NoSuchTableError):
        iceberg.load_table((schema_name, "d"))

    iceberg.rename_table((schema_name, "a"), (schema_name, "d"))
    with pytest.raises(errors.TableAlreadyExistsError):
        iceberg.rename_table((schema_name, "b"), (schema_name, "d"))
    with pytest.raises(errors.NoSuchTableError):
        iceberg.rename_table((schema_name, "a"), (schema_name, "e"))
    iceberg.drop_table((schema_name, "d"))
    with pytest.raises(errors.NoSuchTableError):
        iceberg.drop_table((schema_name, "d"))
    # A purge drops the table too; the server cleans up its managed storage.
    iceberg.purge_table((schema_name, "b"))
    assert [t.name for t in client.list_tables(catalog, schema_name)] == ["c"]


@writes_local_data
def test_pyiceberg_commits_appends_a_column_and_properties_that_polars_reads(
    server, schema: tuple[str, str]
) -> None:
    catalog, schema_name = schema
    client = Client(server.address)
    iceberg = rest_catalog(server.url, catalog)
    january = pa.table(
        {"order_id": pa.array([1, 2], pa.int64()), "customer": ["ann", "bo"]}
    )
    table = iceberg.create_table((schema_name, "orders"), schema=january.schema)
    # Each append is a commit, for which the server writes new metadata.
    table.append(january)
    table.append(pa.table({"order_id": pa.array([3], pa.int64()), "customer": ["cy"]}))
    assert len(iceberg.load_table((schema_name, "orders")).snapshots()) == 2
    # Only the snapshots that a ref points at (`snapshots=refs`).
    refs = rest_catalog(server.url, catalog, **{"snapshot-loading-mode": "refs"})
    assert len(refs.load_table((schema_name, "orders")).snapshots()) == 1

    # A new column is a commit too; the table of Unity Catalog gets it.
    with table.update_schema() as update:
        update.union_by_name(pa.schema([("amount", pa.float64())]))
    table.append(
        pa.table(
            {"order_id": pa.array([4], pa.int64()), "customer": ["di"], "amount": [7.5]}
        )
    )
    full_name = f"{catalog}.{schema_name}.orders"
    columns = client.table(full_name).get().columns
    assert [c.name for c in columns] == ["order_id", "customer", "amount"]
    # So are properties, which Unity Catalog keeps.
    with table.transaction() as transaction:
        transaction.set_properties(team="data")
    assert client.table(full_name).get().properties["team"] == "data"

    orders = pl.scan_iceberg(iceberg.load_table((schema_name, "orders"))).collect()
    assert orders.sort("order_id").to_dicts() == [
        {"order_id": 1, "customer": "ann", "amount": None},
        {"order_id": 2, "customer": "bo", "amount": None},
        {"order_id": 3, "customer": "cy", "amount": None},
        {"order_id": 4, "customer": "di", "amount": 7.5},
    ]


@writes_local_data
def test_a_commit_on_an_old_snapshot_fails_and_pyiceberg_retries(
    server, schema: tuple[str, str], caplog: pytest.LogCaptureFixture
) -> None:
    catalog, schema_name = schema
    iceberg = rest_catalog(server.url, catalog)
    iceberg.create_table((schema_name, "events"), schema=SCHEMA)
    first = iceberg.load_table((schema_name, "events"))
    second = iceberg.load_table((schema_name, "events"))
    first.append(ids(1))
    # The server refuses the commit of `second`, whose snapshot is not the
    # current one any more; pyiceberg loads the table again and retries.
    second.append(ids(2))
    assert "concurrent update" in caplog.text
    read = iceberg.load_table((schema_name, "events")).scan().to_arrow()
    assert sorted(read["id"].to_pylist()) == [1, 2]


@writes_local_data
def test_a_staged_create_makes_the_table_at_its_commit(
    server, schema: tuple[str, str]
) -> None:
    catalog, schema_name = schema
    iceberg = rest_catalog(server.url, catalog)
    transaction = iceberg.create_table_transaction(
        (schema_name, "orders"), schema=SCHEMA
    )
    assert not iceberg.table_exists((schema_name, "orders"))
    transaction.append(ids(1, 2))
    transaction.commit_transaction()

    table = Client(server.address).table(f"{catalog}.{schema_name}.orders").get()
    assert (table.table_type, table.data_source_format) == ("MANAGED", "ICEBERG")
    read = iceberg.load_table((schema_name, "orders")).scan().to_arrow()
    assert read["id"].to_pylist() == [1, 2]


@writes_local_data
def test_metrics_reports_are_acknowledged(server, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    iceberg = rest_catalog(server.url, catalog)
    table = iceberg.create_table((schema_name, "orders"), schema=SCHEMA)
    table.append(ids(1))
    snapshot = table.current_snapshot()
    report = {
        "report-type": "commit-report",
        "table-name": f"{schema_name}.orders",
        "snapshot-id": snapshot.snapshot_id,
        "sequence-number": snapshot.sequence_number,
        "operation": snapshot.summary.operation.value,
        "metrics": {},
    }
    # pyiceberg 0.12 sends no metrics reports, so the test sends one, as
    # Iceberg's Java client does, over pyiceberg's HTTP session.
    url = iceberg.url(
        "namespaces/{namespace}/tables/{table}/metrics",
        namespace=schema_name,
        table="orders",
    )
    assert iceberg._session.post(url, json=report).status_code == 204
    incomplete = iceberg._session.post(url, json={"report-type": "commit-report"})
    assert incomplete.status_code == 400
    assert incomplete.json()["error"]["type"] == "BadRequestException"


def test_the_catalog_serves_no_views(server, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    iceberg = rest_catalog(server.url, catalog)
    with pytest.raises(errors.NoSuchViewError):
        iceberg.load_view((schema_name, "orders_by_day"))
    assert not iceberg.view_exists((schema_name, "orders_by_day"))


def test_local_tables_get_no_credentials(server, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    iceberg = rest_catalog(server.url, catalog)
    table = iceberg.create_table((schema_name, "orders"), schema=SCHEMA)
    assert iceberg.load_credentials((schema_name, "orders"), table.location()) == {}


def test_pyiceberg_writes_a_table_on_s3_with_what_the_server_sends(
    s3: str, s3_server
) -> None:
    # pyiceberg has no settings for S3: each table comes with credentials,
    # the endpoint of moto and the region.
    catalog = unique("catalog")
    Client(s3_server.address).create_catalog(
        catalog, storage_root=f"s3://uc-test/{catalog}"
    )
    iceberg = rest_catalog(s3_server.url, catalog)
    iceberg.create_namespace("default")
    table = iceberg.create_table(("default", "orders"), schema=SCHEMA)
    assert table.io.properties["s3.endpoint"] == s3
    assert table.io.properties["s3.session-token"]
    # The same, from the endpoint where clients renew them.
    credentials = iceberg.load_credentials(("default", "orders"), table.location())
    assert credentials["s3.endpoint"] == s3
    assert credentials["s3.session-token"]

    table.append(ids(1, 2))
    read = iceberg.load_table(("default", "orders")).scan().to_arrow()
    assert read["id"].to_pylist() == [1, 2]
