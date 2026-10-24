# unitycatalog-rs (Python)

The Python client of [unitycatalog-rs](https://github.com/ion-elgreco/unitycatalog-rs):
the Rust `uc-client` crate, as a Python extension built with
[PyO3](https://pyo3.rs) and [maturin](https://www.maturin.rs). trestle generates
both, the Rust client and its Python bindings, from the protos in `proto/`. It
installs as `unitycatalog-rs` and imports as `unitycatalog`.

```bash
pip install unitycatalog-rs           # the client
pip install 'unitycatalog-rs[all]'    # also uc, uc-server and the web UI
```

The extras add the other Python packages of unitycatalog-rs, of the same
version: `cli` (`uc`), `server` (`uc-server`, and `uc`), `ui` (the web UI,
and `uc`) and `all`. `uc dev` then runs uc-server on localhost, with the web
UI.

Each request of `uc-client` comes twice:

- `name(...)` waits for the response. It releases the GIL while it waits.
- `name_async(...)` returns an `asyncio.Future` of the response, through
  [pyo3-async-runtimes](https://docs.rs/pyo3-async-runtimes). Call it in a
  running event loop.

Both send the request on one Tokio runtime in the extension.

## Usage

```python
import asyncio

from unitycatalog import Client, NotFoundError

client = Client("http://localhost:8080/")
# client = Client("http://localhost:8080/", token="<personal access token>")

catalog = client.create_catalog("main", comment="primary")
client.create_schema("default", "main")
print([c.name for c in client.list_catalogs()])

client.catalog("main").update(comment="the main catalog")
try:
    client.catalog("missing").get()
except NotFoundError as e:
    print(e.code, e.message)  # CATALOG_NOT_FOUND Catalog not found: missing


async def main() -> None:
    catalog = await client.catalog("main").get_async()
    schema, schemas = await asyncio.gather(
        client.schema("main.default").get_async(),
        client.list_schemas_async("main"),
    )


asyncio.run(main())
```

- `Client` takes the root URL of the server, and a token. The paths of the
  APIs come from the protos.
- The requests on one object are on its handle: `client.catalog(name)`,
  `client.schema(full_name)`, `client.table(full_name)` and so on. A handle
  sends no request itself.
- Required arguments are positional; the others are keyword-only.
- Results are frozen classes, such as `CatalogInfo`, with each field of the
  API's JSON. They compare by value, and their repr shows the fields that are
  set. The classes are in a submodule of `unitycatalog._core` per API package,
  such as `unitycatalog._core.tables` (`ColumnInfo`, `TableInfo`). The classes
  of the requests, such as `ColumnInfo`, take the required fields as
  arguments and the others as keywords.
- Enums are `enum.StrEnum` classes, such as `TableType`, exported by
  `unitycatalog`. A parameter takes a member or its name (`"MANAGED"`).
  Another string raises `ValueError`.
- An error answer raises `UnityCatalogError`, or its subclass for the HTTP status:
  `BadRequestError` (400), `UnauthenticatedError` (401),
  `PermissionDeniedError` (403), `NotFoundError` (404), `ConflictError` (409),
  `RateLimitError` (429) or `ServerError` (5xx). It has the error code
  (`code`), the message (`message`), the status (`status`) and the details
  (`details`). A request without an answer raises `UnityCatalogError` with no code and
  no status.
- The list methods return every item: they read one page after the other.
  `max_results` limits the number of items. `list_audit_events` and the
  Iceberg lists return one page, with its `next_page_token`; `list_users` and
  `list_groups` return `count` items from `start_index`.
- `delta_table_exists`, `iceberg_namespace_exists` and `iceberg_table_exists`
  return `True`, or `False` when the server answers 404.

## Methods

Each one also comes as `name_async`. A handle method, such as `catalog(name)`,
returns the handle; its methods also come as `name_async`.

| Resource | Methods |
| --- | --- |
| Catalogs | `create_catalog`, `list_catalogs`; `catalog(name)`: `get`, `update`, `delete` |
| Schemas | `create_schema`, `list_schemas`; `schema(full_name)`: `get`, `update`, `delete` |
| Tables | `create_table`, `create_staging_table`, `list_tables`; `table(full_name)`: `get`, `delete` |
| Volumes | `create_volume`, `list_volumes`; `volume(full_name)`: `get`, `update`, `delete` |
| Functions | `create_function`, `list_functions`; `function(full_name)`: `get`, `delete` |
| Registered models | `create_registered_model`, `list_registered_models`; `registered_model(full_name)`: `get`, `update`, `delete` |
| Model versions | `create_model_version`, `get_model_version`, `list_model_versions`, `update_model_version`, `delete_model_version`, `finalize_model_version` |
| Credentials | `create_credential`, `list_credentials`; `credential(name)`: `get`, `update`, `delete` |
| External locations | `create_external_location`, `list_external_locations`; `external_location(name)`: `get`, `update`, `delete` |
| Temporary credentials | `generate_temporary_table_credentials`, `generate_temporary_volume_credentials`, `generate_temporary_path_credentials`, `generate_temporary_model_version_credentials` |
| Permissions | `get_permissions`, `update_permissions` |
| Metastore, health | `get_metastore_summary`, `get_health`, `get_version` |
| Personal access tokens | `create_token`, `list_tokens`; `token(id)`: `delete` |
| Sign-in | `exchange_token`, `request_access_token`, `get_auth_mode`, `logout` |
| SCIM users and groups | `create_user`, `get_user`, `list_users`, `replace_user`, `patch_user`, `delete_user`, `get_me`, `create_group`, `get_group`, `list_groups`, `replace_group`, `patch_group`, `delete_group`, `scim_bulk`, `list_scim_schemas`, `list_scim_resource_types`, `get_service_provider_config` |
| Admin (a signed-in admin) | `get_backup`, `restore_backup`, `list_audit_events` |
| Shares and recipients | `create_share`, `list_shares`, `create_recipient`, `list_recipients`; `share(name)`: `get`, `update`, `delete`; `recipient(name)`: `get`, `update`, `delete`, `rotate_recipient_token`, `create_recipient_profile` |
| Delta Sharing (a recipient's token) | `sharing_list_shares`, `sharing_list_schemas`, `sharing_list_tables`, `sharing_list_all_tables`, `sharing_get_table_version`, `sharing_get_table_metadata`, `sharing_query_table` |
| Iceberg REST catalog | `get_iceberg_config`, `list_iceberg_namespaces`, `create_iceberg_namespace`, `load_iceberg_namespace`, `iceberg_namespace_exists`, `update_iceberg_namespace_properties`, `drop_iceberg_namespace`, `list_iceberg_tables`, `create_iceberg_table`, `load_iceberg_table`, `iceberg_table_exists`, `commit_iceberg_table`, `rename_iceberg_table`, `drop_iceberg_table`, `load_iceberg_table_credentials`, `report_iceberg_metrics`, `load_iceberg_view` |
| Delta API | `get_delta_config`, `create_delta_staging_table`, `get_delta_staging_table_credentials`, `create_delta_table`, `load_delta_table`, `update_delta_table`, `delta_table_exists`, `rename_delta_table`, `delete_delta_table`, `get_delta_table_credentials`, `report_delta_table_metrics` |
| Delta commits | `get_delta_commits`, `commit_delta` |

The arguments that hold free-form JSON, such as the rows of
`restore_backup` or the `value` of a `ScimPatchOperation`, take Python
values: `dict`, `list`, `str`, `int`, `float`, `bool` and `None`.

## Layout

```
python/client/
├── Cargo.toml           crate uc-py: cdylib `_core`, and the stub generator
├── pyproject.toml       maturin; module unitycatalog._core; the extras
├── src/
│   ├── lib.rs           the module `unitycatalog._core`
│   ├── codegen/         the bindings, generated (just generate): Client, the handles, the exceptions, the Tokio runtime
│   └── bin/stub_gen.rs  the stub generator (pyo3-stub-gen)
├── unitycatalog/
│   ├── __init__.py      the public names
│   ├── _enums.py        the enums, generated (just generate)
│   └── _core/           the generated stubs
└── tests/               pytest, against a uc-server
    └── integrations/    polars, deltalake, obstore and pyiceberg against it; S3 from moto
```

The classes of the models come from the `python` feature of `uc-client`
(`crates/uc-client/src/models/_gen`). They are in a submodule of
`unitycatalog._core` per API package: `admin`, `audit`, `auth`, `catalogs`,
`credentials`, `delta`, `external_locations`, `functions`, `health`, `iceberg`,
`metastore`, `models`, `permissions`, `recipients`, `schemas`, `scim`,
`shares`, `sharing`, `tables`, `tokens` and `volumes`. Each handle class is
in the submodule of its resource, such as `catalogs.CatalogClient`.

Its folder sits next to the other Python packages, `python/cli`,
`python/server` and `python/ui`, in the uv workspace of the repository
(`pyproject.toml` at its root, with the dev tools; `uv.lock`).

## Development

The recipes of the justfile at the repository root, as CI runs them:

```bash
just develop-client     # .venv with the dev tools and the engines of the integration tests (uv.lock); builds the extension into it
just test               # builds and starts a uc-server
just test-upstream      # the tests marked `upstream`, against the upstream Java server in Docker
just pre-commit-check   # ruff and mypy; uv.lock up to date
just test-packages      # the wheels of the four packages, installed into a new venv and run
```

With `uc`, `uc-server` and the web UI too:

```bash
just develop            # the four packages, built with maturin (with cargo's progress) and installed into .venv
just demo               # just develop, then uc dev from .venv on http://localhost:8080, with dummy data
just demo-auth          # the same with sign-in (FerrisKey) and S3 (RustFS), in Docker: users alice and bob
```

Without just:

```bash
uv sync --no-install-workspace --all-packages   # .venv at the repository root, with the dev tools and the engines
cd python/client
uv run --no-sync maturin develop                # builds the extension into .venv
uv run --no-sync pytest tests                   # builds and starts a uc-server
```

The tests start the server that `UC_TEST_SERVER_BIN` names, or build
`target/debug/uc-server` with cargo. Each test of a method runs twice: with
`name` and with `name_async`. `tests/test_api.py` also checks that each
method of the generated `uc-client` is here, with its `_async` twin, that
the enums list the values that the extension takes, and runs
`mypy.stubtest` on the stubs. CI runs them on Linux, macOS and Windows, on
Python 3.12 to 3.14 and on free-threaded 3.14t.

With `UC_TEST_SERVER_URL` the tests use that server instead, and only those
marked `upstream` run. Mark a test `upstream` when the upstream Java server
passes it too: it uses only the API that both servers have. `just
test-upstream` runs them against upstream's latest release (`upstream_image`
in the justfile), as CI does.

`tests/integrations/` runs engines against the server: polars and deltalake
write Delta tables, polars and obstore write files into volumes, and
pyiceberg uses each operation of the Iceberg REST catalog, on local
storage. pyiceberg 0.12 sends no metrics reports, so the test sends one over
pyiceberg's HTTP session. On S3, deltalake writes and reads a table by name (`uc://`) and
pyiceberg writes a table, with credentials that the server vends: moto, a
local S3 and STS, runs in a process of its own. deltalake 1.6.6's first
`uc://` call in a process sometimes never returns, so the test runs those
calls in a new process, up to 3 times. The `integrations` dependency group
installs the engines and moto. A module whose engine is not installed is
skipped. polars and deltalake have no free-threaded wheels: CI leaves the
group out there with `UV_NO_GROUP=integrations`.

`just generate` writes the bindings in `src/codegen/`, the model classes of
`uc-client` and `unitycatalog/_enums.py` from the protos. Do not change
them by hand. The stubs in `unitycatalog/_core/` come from the Rust code:
its doc comments and types. After `just generate`, run
`just generate-stubs` (it uses the venv in .venv) and commit the
stubs. It formats them with ruff; ruff does not lint them. CI fails when
they differ from the code.

On a `v*` tag, `.github/workflows/release-python.yml` builds the wheels
(abi3 for Python 3.12+ and free-threaded 3.14t; Linux glibc and musl, macOS,
Windows), and those of `uc`, `uc-server` and the web UI, and publishes them
to PyPI.

Plain `cargo build` and `cargo test` skip this crate (workspace
`default-members`). `cargo build -p uc-py` builds the extension without a
Python interpreter; the crate has no Rust tests.
