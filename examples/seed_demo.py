"""Dummy data for `just demo`: three catalogs, with tables, volumes,
functions and registered models in their schemas. It uses the Python
client, which `just develop` installs into .venv:

    uv run --no-sync examples/seed_demo.py http://127.0.0.1:8080 .uc-demo/storage

The catalogs keep their managed data in the storage folder. A server that
has catalogs gets no dummy data.
"""

import argparse
import json
from pathlib import Path

from unitycatalog import Client, ColumnInfo, FunctionParameterInfo
from unitycatalog.types import ColumnTypeName

# A column type: its type name, its type text and its type in Delta's schema.
Type = tuple[ColumnTypeName, str, str]
INT: Type = ("INT", "int", "integer")
STRING: Type = ("STRING", "string", "string")
DOUBLE: Type = ("DOUBLE", "double", "double")
DATE: Type = ("DATE", "date", "date")


def type_json(name: str, json_type: str) -> str:
    """A column or a parameter as Delta's schema has it."""
    return json.dumps(
        {"name": name, "type": json_type, "nullable": True, "metadata": {}}
    )


def table(
    client: Client,
    catalog: str,
    schema: str,
    name: str,
    comment: str,
    columns: list[tuple[str, Type]],
) -> None:
    """A managed Delta table. As engines make one, a staging table comes
    first: it gives the table its id and its location."""
    staging = client.create_staging_table(name, catalog, schema)
    assert staging.id, "the server gave the staging table no id"
    client.create_table(
        name,
        catalog,
        schema,
        "MANAGED",
        columns=[
            ColumnInfo(
                column,
                type_name=type_name,
                type_text=type_text,
                type_json=type_json(column, json_type),
                position=position,
                nullable=True,
            )
            for position, (column, (type_name, type_text, json_type)) in enumerate(
                columns
            )
        ],
        data_source_format="DELTA",
        storage_location=staging.staging_location,
        comment=comment,
        properties={"io.unitycatalog.tableId": staging.id},
    )


def function(
    client: Client,
    catalog: str,
    schema: str,
    name: str,
    comment: str,
    parameter: str,
    definition: str,
) -> None:
    """A Python function from one string to a string."""
    client.create_function(
        name,
        catalog,
        schema,
        "EXTERNAL",
        input_params=[
            FunctionParameterInfo(
                parameter,
                "string",
                "STRING",
                type_json=type_json(parameter, "string"),
                position=0,
            )
        ],
        data_type="STRING",
        full_data_type="STRING",
        routine_definition=definition,
        external_language="python",
        parameter_style="S",
        is_deterministic=True,
        sql_data_access="NO_SQL",
        is_null_call=False,
        security_type="DEFINER",
        specific_name=name,
        comment=comment,
    )


def model(
    client: Client,
    catalog: str,
    schema: str,
    name: str,
    comment: str,
    versions: int,
    ready: int,
) -> None:
    """A registered model with `versions` versions. The first `ready` of
    them are finalized (READY); the others stay PENDING_REGISTRATION."""
    client.create_registered_model(name, catalog, schema, comment=comment)
    # The versions of a new model are 1, 2, 3, …
    for version in range(1, versions + 1):
        source = f"s3://models/{name}/run-{version}"
        client.create_model_version(
            name, catalog, schema, source, run_id=f"run-{version}"
        )
        if version <= ready:
            client.finalize_model_version(f"{catalog}.{schema}.{name}", version)


def seed(client: Client, root: str) -> None:
    """The dummy data. The catalogs keep their managed data under `root`, a
    URL: a folder (`file://`) or a bucket (`s3://`)."""
    for name, comment in [
        ("unity", "Main catalog"),
        ("sales", "Sales data"),
        ("ml", "Machine learning"),
    ]:
        client.create_catalog(name, comment=comment, storage_root=f"{root}/{name}")
    for catalog, schema, schema_comment in [
        ("unity", "default", "Default schema"),
        ("unity", "demo", None),
        ("sales", "eu", "Sales in Europe"),
        ("sales", "us", "Sales in the US"),
        ("ml", "features", "Feature tables"),
        ("ml", "models", "Trained models"),
    ]:
        client.create_schema(schema, catalog, comment=schema_comment)

    orders = [
        ("order_id", INT),
        ("customer", STRING),
        ("amount", DOUBLE),
        ("ordered_on", DATE),
    ]
    table(
        client,
        "unity",
        "default",
        "marksheet",
        "Marks of the students",
        [("id", INT), ("name", STRING), ("marks", INT)],
    )
    table(
        client,
        "unity",
        "default",
        "numbers",
        "Numbers of each kind",
        [("as_int", INT), ("as_double", DOUBLE)],
    )
    client.create_volume(
        "txt_files", "unity", "default", "MANAGED", comment="Text files"
    )
    function(
        client,
        "unity",
        "default",
        "lowercase",
        "Converts a string to lowercase.",
        "s",
        "g = s.lower()\nreturn g",
    )
    model(
        client,
        "unity",
        "default",
        "iris",
        "Classifies iris flowers.",
        versions=2,
        ready=1,
    )

    table(client, "sales", "eu", "orders", "Orders in Europe", orders)
    table(
        client,
        "sales",
        "eu",
        "invoices",
        "Invoices in Europe",
        [("invoice_id", INT), ("order_id", INT), ("total", DOUBLE)],
    )
    client.create_volume("exports", "sales", "eu", "MANAGED", comment="Monthly exports")
    table(client, "sales", "us", "orders", "Orders in the US", orders)
    function(
        client,
        "sales",
        "us",
        "normalize_state",
        "Upper-cases a state code.",
        "state",
        "return state.strip().upper()",
    )

    table(
        client,
        "ml",
        "features",
        "customer_features",
        "Features per customer",
        [("customer", STRING), ("orders", INT), ("spend", DOUBLE)],
    )
    client.create_volume(
        "raw", "ml", "features", "MANAGED", comment="Raw feature files"
    )
    model(
        client,
        "ml",
        "models",
        "churn",
        "Predicts customer churn.",
        versions=1,
        ready=1,
    )
    model(
        client,
        "ml",
        "models",
        "forecast",
        "Forecasts monthly sales.",
        versions=3,
        ready=2,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Put dummy data into a uc-server.")
    parser.add_argument(
        "server", help="the address of the server, such as http://127.0.0.1:8080"
    )
    parser.add_argument(
        "storage", type=Path, help="the folder of the catalogs' managed data"
    )
    args = parser.parse_args()
    client = Client(f"{args.server.rstrip('/')}/api/2.1/unity-catalog/")
    existing = sorted(c.name or "" for c in client.list_catalogs())
    if existing:
        print(
            f"The server has catalogs, so it gets no dummy data: {', '.join(existing)}."
        )
        return
    seed(client, args.storage.resolve().as_uri())
    catalogs = ", ".join(sorted(c.name or "" for c in client.list_catalogs()))
    print(f"The demo data is in place. Catalogs: {catalogs}.")


if __name__ == "__main__":
    main()
