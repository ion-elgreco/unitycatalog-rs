# Python client

The Python client calls the server's APIs from Python. It is the Rust client of this project,
built for Python with PyO3. It installs as `unitycatalog-rs` and imports as `unitycatalog`.

```bash
pip install unitycatalog-rs
```

It runs on Python 3.10 and later, and on free-threaded Python 3.14t. The wheels are for Linux
(x86_64 and aarch64, glibc and musl), macOS (x86_64 and arm64) and Windows (x86_64).

!!! note
    The API of this client is not the API of upstream's Python client, `unitycatalog-client`.

## Connect

```python
from unitycatalog import Client

client = Client("http://localhost:8080/api/2.1/unity-catalog/")
```

The argument is the base URL of the Unity Catalog REST API. With authorization on, give a token:
a personal access token, or an access token of the server.

```python
import os

client = Client("https://uc.example.com/api/2.1/unity-catalog/")
client = client.with_bearer(os.environ["UC_TOKEN"])
```

`with_bearer` returns a new client; the first one stays without the token.

## Catalogs, schemas and tables

```python
from unitycatalog import Client, ColumnInfo

client = Client("http://localhost:8080/api/2.1/unity-catalog/")

catalog = client.create_catalog("main", comment="primary")
schema = client.create_schema("default", "main")
table = client.create_table(
    "events",
    "main",
    "default",
    "EXTERNAL",
    data_source_format="DELTA",
    storage_location="s3://analytics/events",
    columns=[
        ColumnInfo(
            "id",
            type_name="LONG",
            type_text="bigint",
            type_json='{"name": "id", "type": "long", "nullable": false, "metadata": {}}',
            position=0,
            nullable=False,
        ),
        ColumnInfo(
            "name",
            type_name="STRING",
            type_text="string",
            type_json='{"name": "name", "type": "string", "nullable": true, "metadata": {}}',
            position=1,
            nullable=True,
        ),
    ],
)

print([c.name for c in client.list_catalogs()])
print(client.get_table("main.default.events").columns)

client.update_catalog("main", comment="the main catalog")
client.delete_catalog("main", force=True)    # also deletes its schemas and tables
```

The methods take the names and fields of the REST API. A full name is `catalog.schema.name`. A
column's `type_json` is its field in the Delta (Spark) schema, as JSON: its name, type and
nullability must agree with the other arguments.

## Results, values and errors

- The methods return classes, such as `CatalogInfo` and `TableInfo`, with each field of the API's
  JSON as an attribute. They cannot change.
- The values of enums are strings, such as `"MANAGED"` or `"USE CATALOG"`.
  `unitycatalog.types` has them as `Literal` types, for type checkers. Another string raises
  `ValueError`.
- An error raises `UcError`, with upstream's error code in `code` and the message in `message`.
  When the server does not answer, `code` is `"INTERNAL"`.
- The list methods return the first page of results: at most 100 items.

```python
from unitycatalog import UcError

try:
    client.get_catalog("missing")
except UcError as e:
    print(e.code, e.message)  # CATALOG_NOT_FOUND Catalog not found: missing
```

The package has type stubs: type checkers and editors know each method and class.

## Async

Each method also comes as `<name>_async`, which returns an `asyncio.Future`. Call it in a running
event loop:

```python
import asyncio

from unitycatalog import Client


async def main() -> None:
    client = Client("http://localhost:8080/api/2.1/unity-catalog/")
    catalog, schemas = await asyncio.gather(
        client.get_catalog_async("main"),
        client.list_schemas_async("main"),
    )
    print(catalog.name, [s.name for s in schemas])


asyncio.run(main())
```

The methods without `_async` wait for the answer, and let other Python threads run while they
wait.

## Privileges

```python
from unitycatalog import PermissionsChange

client.update_permissions(
    "schema",
    "main.default",
    [PermissionsChange("bob@example.com", add=["USE SCHEMA", "SELECT"])],
)
for assignment in client.get_permissions("schema", "main.default"):
    print(assignment.principal, assignment.privileges)
```

## Temporary credentials

```python
table = client.get_table("main.default.events")
credentials = client.generate_temporary_table_credentials(table.table_id, "READ")
aws = credentials.aws_temp_credentials
storage_options = {
    "AWS_ACCESS_KEY_ID": aws.access_key_id,
    "AWS_SECRET_ACCESS_KEY": aws.secret_access_key,
    "AWS_SESSION_TOKEN": aws.session_token,
}
```

Give these to a reader of the table, such as deltalake or Polars. For volumes and paths, use
`generate_temporary_volume_credentials` and `generate_temporary_path_credentials`. See
[Cloud storage](server/cloud-storage.md).

## Other endpoints

`raw_json` calls an endpoint that has no method, and returns the JSON answer:

```python
shares = client.raw_json("GET", "shares")
users = client.raw_json("GET", "/api/1.0/unity-control/scim2/Users")
```

A path without `/` at the start is under the base URL; with it, it is from the root of the
server. `post_form` posts a form, as the token exchange takes it.

## Methods

Each method also comes as `<name>_async`.

| Resource | Methods |
| --- | --- |
| Catalogs | `create_catalog`, `get_catalog`, `list_catalogs`, `update_catalog`, `delete_catalog` |
| Schemas | `create_schema`, `get_schema`, `list_schemas`, `update_schema`, `delete_schema` |
| Tables | `create_table`, `get_table`, `list_tables`, `delete_table`, `create_staging_table` |
| Volumes | `create_volume`, `get_volume`, `list_volumes`, `update_volume`, `delete_volume` |
| Functions | `create_function`, `get_function`, `list_functions`, `delete_function` |
| Registered models | `create_registered_model`, `get_registered_model`, `list_registered_models`, `update_registered_model`, `delete_registered_model` |
| Model versions | `create_model_version`, `get_model_version`, `list_model_versions`, `update_model_version`, `delete_model_version`, `finalize_model_version` |
| Storage credentials | `create_credential`, `get_credential`, `list_credentials`, `update_credential`, `delete_credential` |
| Temporary credentials | `generate_temporary_table_credentials`, `generate_temporary_volume_credentials`, `generate_temporary_path_credentials` |
| External locations | `create_external_location`, `get_external_location`, `list_external_locations`, `update_external_location`, `delete_external_location` |
| Privileges, metastore | `get_permissions`, `update_permissions`, `metastore_summary` |
| Personal access tokens | `create_pat`, `list_pats`, `revoke_pat` |
| Users and groups (SCIM) | `create_user`, `get_user`, `list_users`, `update_user`, `delete_user`, `create_group`, `get_group`, `list_groups`, `delete_group` |
| Admin | `backup`, `restore`, `list_audit_events` |
| Iceberg REST catalog | `iceberg_config`, `list_iceberg_namespaces`, `list_iceberg_tables`, `load_iceberg_table` |
| Delta commits | `get_delta_commits`, `commit_delta` |
| Any endpoint | `raw_json`, `post_form` |

`backup` and `restore` take the snapshot as a `dict`. `load_iceberg_table` and
`get_delta_commits` return the JSON answer as a `dict`, and `commit_delta` takes the commit as
one. `help(Client.create_table)` shows the arguments of a method.

## The other packages

The extras of `unitycatalog-rs` install the other packages of this project, of the same version:

| Install | Adds |
| --- | --- |
| `pip install 'unitycatalog-rs[cli]'` | `uc`, the [CLI](cli/index.md). |
| `pip install 'unitycatalog-rs[server]'` | `uc-server`, the [server](server/index.md), and `uc`. |
| `pip install 'unitycatalog-rs[ui]'` | The [web UI](ui.md), and `uc`. |
| `pip install 'unitycatalog-rs[all]'` | All of them. |
