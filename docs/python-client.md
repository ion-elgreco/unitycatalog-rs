# Python client

The Python client calls the server's APIs from Python. It is the Rust client of this project,
built for Python with PyO3. Both are generated from the API definitions of the server (the
protos in `proto/`), so the client has a method for each route. It installs as
`unitycatalog-rs` and imports as `unitycatalog`.

```bash
pip install unitycatalog-rs
```

It runs on Python 3.12 and later, and on free-threaded Python 3.14t. The wheels are for Linux
(x86_64 and aarch64, glibc and musl), macOS (x86_64 and arm64) and Windows (x86_64).

!!! note
    The API of this client is not the API of upstream's Python client, `unitycatalog-client`.

## Connect

```python
from unitycatalog import Client

client = Client("http://localhost:8080/")
```

The argument is the root URL of the server: the client adds the path of each API. With
authorization on, give a token: a personal access token, or an access token of the server.

```python
import os

client = Client("https://uc.example.com/", token=os.environ["UC_TOKEN"])
```

## Catalogs, schemas and tables

```python
from unitycatalog import Client
from unitycatalog._core.tables import ColumnInfo

client = Client("http://localhost:8080/")

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
            '{"name": "id", "type": "long", "nullable": false, "metadata": {}}',
            type_name="LONG",
            type_text="bigint",
            position=0,
            nullable=False,
        ),
        ColumnInfo(
            "name",
            '{"name": "name", "type": "string", "nullable": true, "metadata": {}}',
            type_name="STRING",
            type_text="string",
            position=1,
            nullable=True,
        ),
    ],
)

print([c.name for c in client.list_catalogs()])
print(client.table("main.default.events").get().columns)

client.catalog("main").update(comment="the main catalog")
client.catalog("main").delete(force=True)    # also deletes its schemas and tables
```

The methods take the names and fields of the REST API. The required arguments are positional;
the others are keywords. A full name is `catalog.schema.name`. The requests on one object are on
its handle, such as `client.catalog("main")` or `client.table("main.default.events")`: `get`,
`update` and `delete`. A handle sends no request itself. A column's `type_json` is its field in
the Delta (Spark) schema, as JSON: its name, type and nullability must agree with the other
arguments.

## Results, values and errors

- The methods return classes, such as `CatalogInfo` and `TableInfo`, with each field of the API's
  JSON as an attribute. They cannot change, and they compare by value. The classes are in a
  submodule of `unitycatalog._core` per API, such as `unitycatalog._core.tables` for
  `ColumnInfo` and `TableInfo`.
- The enums are `enum.StrEnum` classes, such as `TableType`, and `unitycatalog` exports them. A
  member is equal to its name, so an argument takes `TableType.MANAGED` or `"MANAGED"`. Another
  string raises `ValueError`.
- An error answer raises `UnityCatalogError`, or its subclass for the HTTP status: `BadRequestError`,
  `UnauthenticatedError`, `PermissionDeniedError`, `NotFoundError`, `ConflictError`,
  `RateLimitError` or `ServerError`. `code` is upstream's error code, `message` the message and
  `status` the HTTP status. When the server does not answer, `code` and `status` are `None`.
- The list methods return all the items: they read one page after the other. `max_results`
  limits the number of items. `list_audit_events`, `list_users`, `list_groups` and the lists
  of the Iceberg REST catalog return one page.

```python
from unitycatalog import NotFoundError

try:
    client.catalog("missing").get()
except NotFoundError as e:
    print(e.code, e.message)  # CATALOG_NOT_FOUND Catalog not found: missing
```

The package has type stubs: type checkers and editors know each method and class.

## Async

Each method that sends a request also comes as `<name>_async`, which returns an
`asyncio.Future`. Call it in a running event loop:

```python
import asyncio

from unitycatalog import Client


async def main() -> None:
    client = Client("http://localhost:8080/")
    catalog, schemas = await asyncio.gather(
        client.catalog("main").get_async(),
        client.list_schemas_async("main"),
    )
    print(catalog.name, [s.name for s in schemas])


asyncio.run(main())
```

The methods without `_async` wait for the answer, and let other Python threads run while they
wait.

## Privileges

```python
from unitycatalog._core.permissions import PermissionsChange

client.update_permissions(
    "schema",
    "main.default",
    changes=[PermissionsChange("bob@example.com", add=["USE SCHEMA", "SELECT"])],
)
privileges = client.get_permissions("schema", "main.default")
for assignment in privileges.privilege_assignments:
    print(assignment.principal, assignment.privileges)
```

## Temporary credentials

```python
table = client.table("main.default.events").get()
credentials = client.generate_temporary_table_credentials(table.table_id, "READ")
aws = credentials.aws_temp_credentials
storage_options = {
    "AWS_ACCESS_KEY_ID": aws.access_key_id,
    "AWS_SECRET_ACCESS_KEY": aws.secret_access_key,
    "AWS_SESSION_TOKEN": aws.session_token,
}
```

Give these to a reader of the table, such as deltalake or Polars. For volumes, paths and model
versions, use `generate_temporary_volume_credentials`, `generate_temporary_path_credentials` and
`generate_temporary_model_version_credentials`. See [Cloud storage](server/cloud-storage.md).

## The other APIs

The client also calls the other APIs of the server: users and groups (SCIM), personal access
tokens, backup and restore, the audit log, Delta Sharing, the Iceberg REST catalog and the Delta
API. For example, the Iceberg REST catalog:

```python
if client.iceberg_table_exists("main", "default", "orders"):
    table = client.load_iceberg_table("main", "default", "orders")
    print(table.metadata_location)
```

`delta_table_exists`, `iceberg_namespace_exists` and `iceberg_table_exists` return `True`, or
`False` when the server answers 404. A Delta Sharing recipient calls the `sharing_*` methods with
its own token:

```python
recipient = Client("http://localhost:8080/", token=profile.bearer_token)
print([share.name for share in recipient.sharing_list_shares().items])
```

## Methods

Each method that sends a request also comes as `<name>_async`. `catalog(name)` and the other
handles return the requests on one object.

| Resource | Methods |
| --- | --- |
| Catalogs | `create_catalog`, `list_catalogs`; `catalog(name)`: `get`, `update`, `delete` |
| Schemas | `create_schema`, `list_schemas`; `schema(full_name)`: `get`, `update`, `delete` |
| Tables | `create_table`, `create_staging_table`, `list_tables`; `table(full_name)`: `get`, `delete` |
| Volumes | `create_volume`, `list_volumes`; `volume(full_name)`: `get`, `update`, `delete` |
| Functions | `create_function`, `list_functions`; `function(full_name)`: `get`, `delete` |
| Registered models | `create_registered_model`, `list_registered_models`; `registered_model(full_name)`: `get`, `update`, `delete` |
| Model versions | `create_model_version`, `get_model_version`, `list_model_versions`, `update_model_version`, `delete_model_version`, `finalize_model_version` |
| Storage credentials | `create_credential`, `list_credentials`; `credential(name)`: `get`, `update`, `delete` |
| External locations | `create_external_location`, `list_external_locations`; `external_location(name)`: `get`, `update`, `delete` |
| Temporary credentials | `generate_temporary_table_credentials`, `generate_temporary_volume_credentials`, `generate_temporary_path_credentials`, `generate_temporary_model_version_credentials` |
| Privileges | `get_permissions`, `update_permissions` |
| Metastore, health | `get_metastore_summary`, `get_health`, `get_version` |
| Personal access tokens | `create_token`, `list_tokens`; `token(id)`: `delete` |
| Sign-in | `exchange_token`, `request_access_token`, `get_auth_mode`, `logout` |
| Users and groups (SCIM) | `create_user`, `get_user`, `list_users`, `replace_user`, `patch_user`, `delete_user`, `get_me`, `create_group`, `get_group`, `list_groups`, `replace_group`, `patch_group`, `delete_group`, `scim_bulk`, `list_scim_schemas`, `list_scim_resource_types`, `get_service_provider_config` |
| Admin | `get_backup`, `restore_backup`, `list_audit_events` |
| Shares and recipients | `create_share`, `list_shares`, `create_recipient`, `list_recipients`; `share(name)`: `get`, `update`, `delete`; `recipient(name)`: `get`, `update`, `delete`, `rotate_recipient_token`, `create_recipient_profile` |
| Delta Sharing | `sharing_list_shares`, `sharing_list_schemas`, `sharing_list_tables`, `sharing_list_all_tables`, `sharing_get_table_version`, `sharing_get_table_metadata`, `sharing_query_table` |
| Iceberg REST catalog | `get_iceberg_config`, `list_iceberg_namespaces`, `create_iceberg_namespace`, `load_iceberg_namespace`, `iceberg_namespace_exists`, `update_iceberg_namespace_properties`, `drop_iceberg_namespace`, `list_iceberg_tables`, `create_iceberg_table`, `load_iceberg_table`, `iceberg_table_exists`, `commit_iceberg_table`, `rename_iceberg_table`, `drop_iceberg_table`, `load_iceberg_table_credentials`, `report_iceberg_metrics`, `load_iceberg_view` |
| Delta API | `get_delta_config`, `create_delta_staging_table`, `get_delta_staging_table_credentials`, `create_delta_table`, `load_delta_table`, `update_delta_table`, `delta_table_exists`, `rename_delta_table`, `delete_delta_table`, `get_delta_table_credentials`, `report_delta_table_metrics` |
| Delta commits | `get_delta_commits`, `commit_delta` |

`help(Client.create_table)` shows the arguments of a method.

## The other packages

The extras of `unitycatalog-rs` install the other packages of this project, of the same version:

| Install | Adds |
| --- | --- |
| `pip install 'unitycatalog-rs[cli]'` | `uc`, the [CLI](cli/index.md). |
| `pip install 'unitycatalog-rs[server]'` | `uc-server`, the [server](server/index.md), and `uc`. |
| `pip install 'unitycatalog-rs[ui]'` | The [web UI](ui.md), and `uc`. |
| `pip install 'unitycatalog-rs[all]'` | All of them. |
