"""Spark uses the server as upstream's users do: Delta tables and views through
upstream's connector (`UCSingleCatalog`) and delta-spark, and Iceberg tables
through Iceberg's REST catalog. On S3, Spark writes with the credentials that
the server vends.

Skipped without PySpark (`just test-spark` adds it) or Java. The jars follow
the version of PySpark: Spark 4.2 has the view catalog API, with which the
connector creates and drops views; Iceberg has no runtime for 4.2 yet."""

import json
import shutil
import typing
from pathlib import Path

import pytest
from _helpers import unique

from unitycatalog import Client, NotFoundError
from unitycatalog.tables import ColumnInfo

pyspark = pytest.importorskip("pyspark")
sql = pytest.importorskip("pyspark.sql")
if shutil.which("java") is None:
    pytest.skip("Spark needs Java", allow_module_level=True)

SPARK = ".".join(pyspark.__version__.split(".")[:2])
# The Hadoop of each Spark version, for its S3 file system, and Iceberg's
# runtime for it, if any.
HADOOP = {"4.1": "3.4.2", "4.2": "3.5.0"}
ICEBERG = {"4.1": "1.12.0"}
if SPARK not in HADOOP:
    pytest.skip(f"no jars for Spark {SPARK}", allow_module_level=True)

views_of_spark = pytest.mark.skipif(
    SPARK == "4.1", reason="Spark 4.1 has no view catalog API"
)
iceberg = pytest.mark.skipif(
    SPARK not in ICEBERG, reason=f"Iceberg has no runtime for Spark {SPARK}"
)


@pytest.fixture(scope="module")
def spark(s3: str) -> typing.Iterator[typing.Any]:
    """A local Spark with delta-spark, upstream's connector and Iceberg's
    runtime; `s3://` paths are in moto's S3."""
    packages = [
        f"io.delta:delta-spark_{SPARK}_2.13:4.4.1",
        f"io.unitycatalog:unitycatalog-spark_{SPARK}_2.13:0.7.0",
        f"org.apache.hadoop:hadoop-aws:{HADOOP[SPARK]}",
    ]
    extensions = ["io.delta.sql.DeltaSparkSessionExtension"]
    if SPARK in ICEBERG:
        packages.append(
            f"org.apache.iceberg:iceberg-spark-runtime-{SPARK}_2.13:{ICEBERG[SPARK]}"
        )
        extensions.append(
            "org.apache.iceberg.spark.extensions.IcebergSparkSessionExtensions"
        )
    session = (
        sql.SparkSession.builder.master("local[2]")
        .appName("uc-tests")
        .config("spark.jars.packages", ",".join(packages))
        .config("spark.sql.extensions", ",".join(extensions))
        .config(
            "spark.sql.catalog.spark_catalog",
            "org.apache.spark.sql.delta.catalog.DeltaCatalog",
        )
        # Delta gives the catalog the columns of a table; otherwise an external
        # table that Spark makes has none (upstream's issue 810).
        .config("spark.databricks.delta.catalog.update.enabled", "true")
        .config("spark.sql.shuffle.partitions", "2")
        .config("spark.ui.enabled", "false")
        .config("spark.hadoop.fs.s3.impl", "org.apache.hadoop.fs.s3a.S3AFileSystem")
        .config("spark.hadoop.fs.s3a.endpoint", s3)
        .config("spark.hadoop.fs.s3a.endpoint.region", "us-east-1")
        .config("spark.hadoop.fs.s3a.path.style.access", "true")
        .config("spark.hadoop.fs.s3a.connection.ssl.enabled", "false")
        .getOrCreate()
    )
    session.sparkContext.setLogLevel("ERROR")
    yield session
    session.stop()


def attach(spark: typing.Any, server: typing.Any, catalog: str) -> None:
    """Spark's catalog `catalog`: the catalog of that name of `server`, through
    upstream's connector."""
    key = f"spark.sql.catalog.{catalog}"
    spark.conf.set(key, "io.unitycatalog.spark.UCSingleCatalog")
    spark.conf.set(f"{key}.uri", server.address)
    spark.conf.set(f"{key}.token", server.token)


def rows(spark: typing.Any, query: str) -> list[tuple[typing.Any, ...]]:
    return [tuple(row) for row in spark.sql(query).collect()]


def orders(spark: typing.Any, table: str, location: str) -> None:
    """The external Delta table `table` at `location`, with ids 1 to 3."""
    spark.sql(
        f"CREATE TABLE {table} (id INT, amount DOUBLE) USING DELTA LOCATION '{location}'"
    )
    spark.sql(f"INSERT INTO {table} VALUES (1, 9.5), (2, 3.25), (3, 12.0)")


def test_spark_writes_and_reads_an_external_delta_table(
    spark, server, schema: tuple[str, str], tmp_path: Path
) -> None:
    catalog, name = schema
    client = Client(server.address)
    client.create_schema("other", catalog)
    attach(spark, server, catalog)
    assert sorted(rows(spark, f"SHOW SCHEMAS IN {catalog}")) == [
        ("default",),
        ("other",),
    ]
    table = f"{catalog}.{name}.orders"
    orders(spark, table, (tmp_path / "orders").as_uri())
    assert rows(spark, f"SELECT * FROM {table} ORDER BY id") == [
        (1, 9.5),
        (2, 3.25),
        (3, 12.0),
    ]
    assert [row[1] for row in rows(spark, f"SHOW TABLES IN {catalog}.{name}")] == [
        "orders"
    ]
    info = client.table(table).get()
    assert (info.table_type, info.data_source_format) == ("EXTERNAL", "DELTA")
    assert [c.name for c in info.columns] == ["id", "amount"]
    # Dropping an external table keeps its data.
    spark.sql(f"DROP TABLE {table}")
    with pytest.raises(NotFoundError):
        client.table(table).get()
    assert (tmp_path / "orders" / "_delta_log").is_dir()


def test_spark_makes_and_appends_to_a_managed_delta_table(
    spark, server, schema: tuple[str, str]
) -> None:
    catalog, name = schema
    attach(spark, server, catalog)
    table = f"{catalog}.{name}.events"
    # The server gives the location; the commits go through the Delta REST API.
    spark.sql(f"CREATE TABLE {table} (id INT, kind STRING) USING DELTA")
    spark.sql(f"INSERT INTO {table} VALUES (1, 'a')")
    spark.sql(f"INSERT INTO {table} VALUES (2, 'b')")
    assert rows(spark, f"SELECT * FROM {table} ORDER BY id") == [(1, "a"), (2, "b")]
    client = Client(server.address)
    info = client.table(table).get()
    assert info.table_type == "MANAGED"
    assert info.properties["delta.feature.catalogManaged"] == "supported"
    spark.sql(f"DROP TABLE {table}")
    with pytest.raises(NotFoundError):
        client.table(table).get()


def test_spark_reads_a_view_of_the_api(
    spark, server, schema: tuple[str, str], tmp_path: Path
) -> None:
    catalog, name = schema
    attach(spark, server, catalog)
    table = f"{catalog}.{name}.orders"
    orders(spark, table, (tmp_path / "orders").as_uri())
    # The view holds SQL, which Spark runs on the tables that it names.
    field = {"name": "id", "type": "integer", "nullable": True, "metadata": {}}
    Client(server.address).create_table(
        "big",
        catalog,
        name,
        "VIEW",
        columns=[
            ColumnInfo(
                "id", json.dumps(field), type_name="INT", type_text="int", position=0
            )
        ],
        view_definition=f"SELECT id FROM {table} WHERE amount > 5",
    )
    assert rows(spark, f"SELECT * FROM {catalog}.{name}.big ORDER BY id") == [
        (1,),
        (3,),
    ]


@views_of_spark
def test_spark_creates_lists_and_drops_a_view(
    spark, server, schema: tuple[str, str], tmp_path: Path
) -> None:
    catalog, name = schema
    attach(spark, server, catalog)
    table = f"{catalog}.{name}.orders"
    view = f"{catalog}.{name}.big"
    orders(spark, table, (tmp_path / "orders").as_uri())
    spark.sql(f"CREATE VIEW {view} AS SELECT id FROM {table} WHERE amount > 5")
    client = Client(server.address)
    info = client.table(view).get()
    assert info.table_type == "VIEW"
    assert info.view_definition and "amount > 5" in info.view_definition
    assert info.view_dependencies
    assert [
        d.table.table_full_name for d in info.view_dependencies.dependencies if d.table
    ] == [table]
    assert [row[1] for row in rows(spark, f"SHOW VIEWS IN {catalog}.{name}")] == ["big"]
    assert rows(spark, f"SELECT * FROM {view} ORDER BY id") == [(1,), (3,)]
    spark.sql(f"DROP VIEW {view}")
    with pytest.raises(NotFoundError):
        client.table(view).get()


def test_spark_writes_and_reads_delta_tables_on_s3(spark, s3_server) -> None:
    client = Client(s3_server.address)
    catalog = unique("catalog")
    client.create_catalog(catalog, storage_root=f"s3://uc-test/{catalog}")
    client.create_schema("default", catalog)
    attach(spark, s3_server, catalog)
    external = f"{catalog}.default.orders"
    orders(spark, external, f"s3://uc-test/external/{catalog}/orders")
    managed = f"{catalog}.default.events"
    spark.sql(f"CREATE TABLE {managed} (id INT) USING DELTA")
    spark.sql(f"INSERT INTO {managed} SELECT id FROM {external} WHERE id > 1")
    assert rows(spark, f"SELECT * FROM {managed} ORDER BY id") == [(2,), (3,)]
    location = client.table(managed).get().storage_location
    assert location and location.startswith(f"s3://uc-test/{catalog}/")


@iceberg
def test_spark_writes_and_reads_an_iceberg_table(
    spark, server, schema: tuple[str, str]
) -> None:
    catalog, name = schema
    # Iceberg's catalog of the Unity Catalog catalog `catalog`.
    key = f"spark.sql.catalog.iceberg_{catalog}"
    spark.conf.set(key, "org.apache.iceberg.spark.SparkCatalog")
    spark.conf.set(f"{key}.type", "rest")
    spark.conf.set(f"{key}.uri", f"{server.url}iceberg")
    spark.conf.set(f"{key}.warehouse", catalog)
    spark.conf.set(f"{key}.token", server.token)
    table = f"iceberg_{catalog}.{name}.events"
    spark.sql(f"CREATE TABLE {table} (id BIGINT, kind STRING) USING iceberg")
    spark.sql(f"INSERT INTO {table} VALUES (1, 'a'), (2, 'b')")
    spark.sql(f"ALTER TABLE {table} ADD COLUMN note STRING")
    spark.sql(f"INSERT INTO {table} VALUES (3, 'c', 'new')")
    assert rows(spark, f"SELECT * FROM {table} ORDER BY id") == [
        (1, "a", None),
        (2, "b", None),
        (3, "c", "new"),
    ]
    client = Client(server.address)
    info = client.table(f"{catalog}.{name}.events").get()
    assert info.data_source_format == "ICEBERG"
    assert [c.name for c in info.columns] == ["id", "kind", "note"]
    spark.sql(f"DROP TABLE {table}")
    with pytest.raises(NotFoundError):
        client.table(f"{catalog}.{name}.events").get()
