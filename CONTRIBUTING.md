# Contributing

unitycatalog-rs is a Rust reimplementation of
[Unity Catalog OSS](https://github.com/unitycatalog/unitycatalog). Its client-facing APIs stay
compatible with upstream's, so existing clients, such as Spark, keep working.

## Set up

Install:

- [rustup](https://rustup.rs). `rust-toolchain.toml` pins the Rust version and adds the
  `wasm32-unknown-unknown` target of the web UI.
- [just](https://just.systems), to run the recipes of the `justfile`.
- [uv](https://docs.astral.sh/uv/), for the Python packages and the tests.
- [protoc](https://protobuf.dev/installation/), only to run `just generate`.
- Docker, only for `just demo-auth`, `just test-docker` and `just test-upstream`.

Then:

```bash
cargo build              # the server, the CLI and the libraries
just test-rust           # the Rust tests
just demo                # build everything and run a server with the web UI and sample data on http://localhost:8080
```

## Repository layout

| Path | What it is |
| --- | --- |
| `crates/uc-server` | The HTTP server, `uc-server`: the routes of each API (`src/routes`), sign-in, authorization, Delta Sharing |
| `crates/uc-metastore` | The catalog's metadata: one `Ops` trait per domain (catalogs, tables, …) behind the `Metastore` trait, and the SQL migrations |
| `crates/uc-storage` | The database under the metastore: Turso (embedded) or PostgreSQL |
| `crates/uc-auth` | Who sent a request (OIDC tokens, AWS signed requests, auth proxy headers) and what they may do (Cedar, Open Policy Agent) |
| `crates/uc-delta` | The rules of catalog-managed Delta tables, without storage |
| `crates/uc-api` | The request and response types of the server's REST API |
| `crates/uc-common` | Settings, errors and IDs that all crates share |
| `crates/uc-client` | The Rust client, generated from `proto/` |
| `crates/uc-cli` | The `uc` CLI and its terminal UI |
| `crates/uc-ui` | The web UI, in Leptos, built to WebAssembly (`just wasm`); see its README |
| `crates/uc-codegen` | The generator of the Rust and Python clients (`just generate`) |
| `proto/unitycatalog` | The API definitions: one folder per API area, with each route of the server |
| `proto/olai` | The proto options of trestle, the code generator |
| `python/client` | The Python client, `unitycatalog-rs` on PyPI: `uc-client` as a Python extension |
| `python/cli`, `python/server`, `python/ui` | Python packages that ship `uc`, `uc-server` and the web UI as wheels |
| `deploy/docker`, `deploy/helm` | The container image and the Helm chart |
| `docs`, `mkdocs.yml` | The documentation site (`just docs-serve`) |
| `examples` | The demo setups and their sample data |

## How the parts fit

- The server implements the routes in `crates/uc-server/src/routes`, with the types of
  `crates/uc-api`, and keeps the metadata through `crates/uc-metastore`.
- The protos in `proto/unitycatalog` describe every route of the server. From them,
  `just generate` writes the Rust client (`crates/uc-client`) and the Python client's bindings
  (`python/client`).
- The CLI, the web UI and the Python client all call the server through the generated client.

### Add or change an API route

1. Change the route in `crates/uc-server/src/routes` and add a test in `crates/uc-server/tests`.
2. Change the matching proto in `proto/unitycatalog/<area>`.
3. Run `just generate`, then `just generate-stubs` for the Python stubs.
4. Call the new client method in a test in `crates/uc-client/tests/client`. A test there fails
   when a route of the client has no call.

### Change the database schema

Add a new migration file in `crates/uc-metastore/migrations` (`V<n>__<name>.sql`). The Rust tests
run on Turso, and on PostgreSQL when `UC_TEST_POSTGRES_URL` is set.

## Generated code

Do not edit these by hand. Change the protos or the generator, then run `just generate`:

- `crates/uc-client/src/codegen` and `crates/uc-client/src/models/_gen`
- `python/client/src/codegen` and `python/client/unitycatalog/_enums.py`

The Python stubs in `python/client/unitycatalog/_core` come from `just generate-stubs`. CI fails
when any generated file differs from what the generator writes.

## Checks

Run these before you open a pull request. CI runs them too.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
just test-rust           # the Rust tests
just test                # the Python client's tests, against a uc-server
just pre-commit-check    # ruff and mypy on the Python client; uv.lock up to date
uvx typos                # spelling
```

Other test recipes, for the parts you change:

| Recipe | Tests |
| --- | --- |
| `just test-upstream` | The Python tests marked `upstream`, against upstream's Java server in Docker |
| `just test-spark` | Upstream's Spark connector against our server |
| `just test-opa`, `just test-azure`, `just test-authentik`, `just test-aws-emulator` | Open Policy Agent, Azure, SCIM with authentik, and AWS sign-in |
| `just test-docker` | The container image |
| `just test-helm` | The Helm chart (needs the helm-unittest plugin) |
| `just test-packages` | The four Python wheels, in a new virtual environment |

## Python client

The Python client in `python/client` is the Rust `uc-client` crate built as a Python extension,
with [PyO3](https://pyo3.rs) and [maturin](https://www.maturin.rs). Its folder is in the uv
workspace of the repository, next to `python/cli`, `python/server` and `python/ui`.

```bash
just develop-client      # .venv with the dev tools and the test engines; builds the extension into it
just test                # its tests, against a uc-server that the tests build and start
```

- `unitycatalog/` holds the Python package. The modules of the API areas, `_enums.py` and the
  stubs in `_core/` are generated.
- `src/identity.rs` holds the cloud identity token sources, written by hand. After a change there,
  run `just generate-stubs`.
- The tests run each method twice: as `name` and as `name_async`. With `UC_TEST_SERVER_URL`, they
  use that server, and only the tests marked `upstream` run. Mark a test `upstream` when it uses
  only the API that upstream's Java server has too.
- `tests/integrations/` runs polars, deltalake, obstore and pyiceberg against the server, on local
  storage and on S3 from moto.
- `cargo build` and `cargo test` skip this crate. `cargo build -p uc-py` builds it.

## Release

A `v*` tag starts the release workflows: `release.yml` builds the binaries and the container
image, and `release-python.yml` builds the Python wheels and publishes them to PyPI.
