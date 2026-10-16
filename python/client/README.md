# unitycatalog-rs (Python)

The Python client of [unitycatalog-rs](https://github.com/ion-elgreco/unitycatalog-rs):
the Rust `uc-client` crate, as a Python extension built with
[PyO3](https://pyo3.rs) and [maturin](https://www.maturin.rs). It installs
as `unitycatalog-rs` and imports as `unitycatalog`.

```bash
pip install unitycatalog-rs           # the client
pip install 'unitycatalog-rs[all]'    # also uc, uc-server and the web UI
```

The extras add the other Python packages of unitycatalog-rs, of the same
version: `cli` (`uc`), `server` (`uc-server`, and `uc`), `ui` (the web UI,
and `uc`) and `all`. `uc dev` then runs uc-server on localhost, with the web
UI.

Each method of `uc-client` comes twice:

- `name(...)` waits for the response. It releases the GIL while it waits.
- `name_async(...)` returns an `asyncio.Future` of the response, through
  [pyo3-async-runtimes](https://docs.rs/pyo3-async-runtimes). Call it in a
  running event loop.

Both send the request on one Tokio runtime in the extension.

## Usage

```python
import asyncio
from unitycatalog import Client, ColumnInfo, UcError

client = Client("http://localhost:8080/api/2.1/unity-catalog/")
# client = client.with_bearer("<personal access token>")

catalog = client.create_catalog("main", comment="primary")
client.create_schema("default", "main")
print([c.name for c in client.list_catalogs()])

try:
    client.get_catalog("missing")
except UcError as e:
    print(e.code, e.message)  # CATALOG_NOT_FOUND Catalog not found: missing


async def main() -> None:
    catalog = await client.get_catalog_async("main")
    schemas = await asyncio.gather(
        client.get_schema_async("main.default"),
        client.list_schemas_async("main"),
    )


asyncio.run(main())
```

- Results are frozen classes, such as `CatalogInfo`, with every field of the
  API's JSON.
- Enum values are strings, such as `"MANAGED"` or `"USE CATALOG"`.
  `unitycatalog.types` has them as `Literal` types. Another string raises
  `ValueError`.
- Errors raise `UcError`, with the upstream error code (`code`) and the
  message (`message`).
- The list methods return the first page of results.
- `raw_json(method, path, query=..., body=...)` calls an endpoint that has no method;
  `post_form(path, form)` posts a form, as the token exchange takes it.

## Methods

Each one also comes as `name_async`.

| Resource | Methods |
| --- | --- |
| Catalogs | `create_catalog`, `get_catalog`, `list_catalogs`, `update_catalog`, `delete_catalog` |
| Schemas | `create_schema`, `get_schema`, `list_schemas`, `update_schema`, `delete_schema` |
| Tables | `create_table`, `get_table`, `list_tables`, `delete_table`, `create_staging_table` |
| Volumes | `create_volume`, `get_volume`, `list_volumes`, `update_volume`, `delete_volume` |
| Functions | `create_function`, `get_function`, `list_functions`, `delete_function` |
| Registered models | `create_registered_model`, `get_registered_model`, `list_registered_models`, `update_registered_model`, `delete_registered_model` |
| Model versions | `create_model_version`, `get_model_version`, `list_model_versions`, `update_model_version`, `delete_model_version`, `finalize_model_version` |
| Credentials | `create_credential`, `get_credential`, `list_credentials`, `update_credential`, `delete_credential` |
| Temporary credentials | `generate_temporary_table_credentials`, `generate_temporary_volume_credentials`, `generate_temporary_path_credentials` |
| External locations | `create_external_location`, `get_external_location`, `list_external_locations`, `update_external_location`, `delete_external_location` |
| Permissions, metastore | `get_permissions`, `update_permissions`, `metastore_summary` |
| Personal access tokens | `create_pat`, `list_pats`, `revoke_pat` |
| SCIM users and groups | `create_user`, `get_user`, `list_users`, `update_user`, `delete_user`, `create_group`, `get_group`, `list_groups`, `delete_group` |
| Admin (a signed-in admin) | `backup`, `restore`, `list_audit_events` |
| Iceberg REST catalog | `iceberg_config`, `list_iceberg_namespaces`, `list_iceberg_tables`, `load_iceberg_table` |
| Delta commits | `get_delta_commits`, `commit_delta` |
| Any endpoint | `raw_json`, `post_form` |

`backup` and `restore` take the snapshot as a `dict`; `load_iceberg_table`
and `get_delta_commits` return the JSON response as a `dict`, and
`commit_delta` takes the commit as one.

## Layout

```
python/client/
├── Cargo.toml           crate uc-py: cdylib `_core`, and the stub generator
├── pyproject.toml       maturin; module unitycatalog._core; the extras
├── src/
│   ├── lib.rs           the module `unitycatalog._core`
│   ├── client.rs        Client: each method, then its `_async` twin
│   ├── catalogs.rs …    the classes, a submodule per resource
│   ├── str_enum.rs      uc_api enums as Python strings
│   ├── py_future.rs     the `asyncio.Future` of an `_async` method
│   └── bin/stub_gen.rs  the stub generator (pyo3-stub-gen)
├── unitycatalog/
│   ├── __init__.py      the public names
│   ├── exceptions.py    UcError
│   ├── types.py         the enum values as Literal types
│   └── _core/           the generated stubs
└── tests/               pytest, against a uc-server
    └── integrations/    polars, deltalake, obstore and pyiceberg against it; S3 from moto
```

The classes are in a submodule of `unitycatalog._core` per resource, as in
`uc_api`: `catalogs`, `schemas`, `tables`, `volumes`, `functions`, `models`,
`credentials`, `external_locations`, `permissions`, `metastore`, `tokens`,
`scim`, `admin`, `audit` and `iceberg_rest`.
`unitycatalog` imports all of them.

Its folder sits next to the other Python packages, `python/cli`,
`python/server` and `python/ui`, in the uv workspace of the repository
(`pyproject.toml` at its root, with the dev tools; `uv.lock`).

## Development

The recipes of the justfile at the repository root, as CI runs them:

```bash
just develop-client     # .venv with the dev tools and the engines of the integration tests (uv.lock); builds the extension into it
just test               # builds and starts a uc-server
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
`name` and with `name_async`. `tests/test_api.py` also checks that every
method of `uc-client` is here, and runs `mypy.stubtest` on the stubs. CI
runs them on Linux, macOS and Windows, on Python 3.10 to 3.14 and on
free-threaded 3.14t.

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

The stubs in `unitycatalog/_core/` come from the Rust code: its doc comments
and types. After a change to a class or a method, run
`just generate-stubs` (it uses the venv in .venv) and commit the
stubs. It formats them with ruff; ruff does not lint them. CI fails when
they differ from the code.

On a `v*` tag, `.github/workflows/release-python.yml` builds the wheels
(abi3 for Python 3.10+ and free-threaded 3.14t; Linux glibc and musl, macOS,
Windows), and those of `uc`, `uc-server` and the web UI, and publishes them
to PyPI.

Plain `cargo build` and `cargo test` skip this crate (workspace
`default-members`). `cargo build -p uc-py` builds the extension without a
Python interpreter; the crate has no Rust tests.
