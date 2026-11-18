# unitycatalog-rs (Python)

The Python client of [unitycatalog-rs](https://github.com/open-lakehouse/unitycatalog-rs), a
Unity Catalog server written in Rust. It calls the server's APIs from Python, with or without
`asyncio`, and has a method for each API route. It also works with upstream's Unity Catalog OSS
server for the APIs that both servers have.

It installs as `unitycatalog-rs` and imports as `unitycatalog`. It needs Python 3.12 or later.

```bash
pip install unitycatalog-rs           # the client
pip install 'unitycatalog-rs[all]'    # also the uc CLI, the server and the web UI
```

The extras install the other packages of unitycatalog-rs, at the same version:

| Extra | Installs |
| --- | --- |
| `cli` | the `uc` CLI |
| `server` | `uc-server`, and `uc` |
| `ui` | the web UI, and `uc` |
| `all` | all of the above |

With the `server` extra, `uc dev` starts a server on http://localhost:8080, with the web UI.

## Quick start

```python
from unitycatalog import Client, NotFoundError

client = Client("http://localhost:8080/")

client.create_catalog("main", comment="primary")
client.create_schema("default", "main")
print([c.name for c in client.list_catalogs()])

client.catalog("main").update(comment="the main catalog")

try:
    client.catalog("missing").get()
except NotFoundError as e:
    print(e.code, e.message)  # CATALOG_NOT_FOUND Catalog not found: missing
```

## Sign-in

When the server needs sign-in, the client sends a token with each request. It gets one in one of
these ways:

```python
import os

from unitycatalog import Client, identity

# A fixed token: a personal access token, or a token of the server.
client = Client("https://uc.example.com/", token=os.environ["UC_TOKEN"])

# A token provider: a cloud identity, an OAuth client, or a function that returns a token.
client = Client("https://uc.example.com/", token_provider=identity.azure("https://uc.example.com/.default"))
client = Client("https://uc.example.com/", token_provider=identity.aws("https://uc.example.com"))

# No token: the client uses the identity of the environment, such as AZURE_*, AWS_* or
# GOOGLE_APPLICATION_CREDENTIALS variables, or UC_TOKEN.
client = Client("https://uc.example.com/")
```

## Async

Each method that sends a request also comes as `<name>_async`, which returns an awaitable. Call
it in a running event loop:

```python
import asyncio


async def main() -> None:
    catalog, schemas = await asyncio.gather(
        client.catalog("main").get_async(),
        client.list_schemas_async("main"),
    )


asyncio.run(main())
```

The sync methods release the GIL while they wait, so other Python threads keep running.

## How the API works

- **Handles.** The requests on one object are on its handle: `client.catalog(name)`,
  `client.schema(full_name)`, `client.table(full_name)` and so on. Making a handle sends no
  request.
- **Arguments.** They have the names of the REST API's fields. Required arguments are positional;
  the others are keyword-only.
- **Results.** Results are frozen classes, such as `CatalogInfo` and `TableInfo`, with each field
  of the API's JSON as an attribute. They compare by value, and their repr shows the fields that
  are set. The classes are in a module per API area, such as `unitycatalog.tables` (`ColumnInfo`,
  `TableInfo`).
- **Enums.** Enums are `enum.StrEnum` classes, such as `TableType`. A parameter takes a member or
  its name (`"MANAGED"`). Another string raises `ValueError`.
- **JSON values.** Arguments that hold free-form JSON, such as the rows of `restore_backup` or the
  `value` of a `ScimPatchOperation`, take `dict`, `list`, `str`, `int`, `float`, `bool` and
  `None`.
- **Lists.** The list methods return every item: they read one page after the other.
  `max_results` limits the number of items. `list_audit_events`, `search_objects` and the
  Iceberg lists return one page, with its `next_page_token`. `list_users` and `list_groups`
  return `count` items from `start_index`.
- **Exists checks.** `delta_table_exists`, `iceberg_namespace_exists` and `iceberg_table_exists`
  return `True`, or `False` when the server answers 404.

## Errors

An error from the server raises `UnityCatalogError`, or its subclass for the HTTP status:

| Exception | Status |
| --- | --- |
| `BadRequestError` | 400 |
| `UnauthenticatedError` | 401 |
| `PermissionDeniedError` | 403 |
| `NotFoundError` | 404 |
| `ConflictError` | 409 |
| `RateLimitError` | 429 |
| `ServerError` | 5xx |

Each has the error code (`code`), the message (`message`), the status (`status`) and the details
(`details`). When the server does not answer, the client raises `UnityCatalogError` with no code
and no status.

## Methods

Each method also comes as `<name>_async`. A handle method, such as `catalog(name)`, returns the
handle, and the handle's methods also come as `<name>_async`.

| Resource | Methods |
| --- | --- |
| Catalogs | `create_catalog`, `list_catalogs`; `catalog(name)`: `get`, `update`, `delete` |
| Schemas | `create_schema`, `list_schemas`; `schema(full_name)`: `get`, `update`, `delete` |
| Tables | `create_table`, `create_staging_table`, `list_tables`; `table(full_name)`: `get`, `delete` |
| Volumes | `create_volume`, `list_volumes`; `volume(full_name)`: `get`, `update`, `delete` |
| Files | `upload_file`, `download_file`, `get_file_metadata`, `delete_file`, `create_directory`, `list_directory_contents`, `get_directory_metadata`, `delete_directory` |
| Functions | `create_function`, `list_functions`; `function(full_name)`: `get`, `delete` |
| Registered models | `create_registered_model`, `list_registered_models`; `registered_model(full_name)`: `get`, `update`, `delete` |
| Model versions | `create_model_version`, `get_model_version`, `list_model_versions`, `update_model_version`, `delete_model_version`, `finalize_model_version` |
| Search | `search_objects` |
| Credentials | `create_credential`, `list_credentials`; `credential(name)`: `get`, `update`, `delete` |
| External locations | `create_external_location`, `list_external_locations`; `external_location(name)`: `get`, `update`, `delete` |
| Temporary credentials | `generate_temporary_table_credentials`, `generate_temporary_volume_credentials`, `generate_temporary_path_credentials`, `generate_temporary_model_version_credentials` |
| Permissions | `get_permissions`, `update_permissions` |
| Metastore, health | `get_metastore_summary`, `get_health`, `get_version` |
| Personal access tokens | `create_token`, `list_tokens`; `token(id)`: `delete` |
| Sign-in | `exchange_token`, `request_access_token`, `get_auth_mode`, `logout` |
| SCIM users and groups | `create_user`, `get_user`, `list_users`, `search_users`, `replace_user`, `patch_user`, `delete_user`, `get_me`, `create_group`, `get_group`, `list_groups`, `search_groups`, `replace_group`, `patch_group`, `delete_group`, `search_resources`, `scim_bulk`, `list_scim_schemas`, `get_scim_schema`, `list_scim_resource_types`, `get_scim_resource_type`, `get_service_provider_config` |
| Admin (a signed-in admin) | `get_backup`, `restore_backup`, `list_audit_events` |
| Shares and recipients | `create_share`, `list_shares`, `create_recipient`, `list_recipients`, `activate_recipient_token`; `share(name)`: `get`, `update`, `delete`, `get_share_permissions`, `update_share_permissions`; `recipient(name)`: `get`, `update`, `delete`, `rotate_recipient_token` |
| Iceberg REST catalog | `get_iceberg_config`, `list_iceberg_namespaces`, `create_iceberg_namespace`, `load_iceberg_namespace`, `iceberg_namespace_exists`, `update_iceberg_namespace_properties`, `drop_iceberg_namespace`, `list_iceberg_tables`, `create_iceberg_table`, `load_iceberg_table`, `iceberg_table_exists`, `commit_iceberg_table`, `rename_iceberg_table`, `drop_iceberg_table`, `load_iceberg_table_credentials`, `report_iceberg_metrics`, `load_iceberg_view` |
| Delta API | `get_delta_config`, `create_delta_staging_table`, `get_delta_staging_table_credentials`, `create_delta_table`, `load_delta_table`, `update_delta_table`, `delta_table_exists`, `rename_delta_table`, `delete_delta_table`, `get_delta_table_credentials`, `report_delta_table_metrics` |
| Delta commits | `get_delta_commits`, `commit_delta` |

`help(Client.create_table)` shows the arguments of a method. The
[documentation](https://open-lakehouse.github.io/unitycatalog-rs/) has a guide and the full
Python API reference.

## License

Apache-2.0
