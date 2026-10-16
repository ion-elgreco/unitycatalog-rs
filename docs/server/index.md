# Server

`uc-server` serves Unity Catalog's REST APIs and the web UI. This page tells how to install and
run it on your own machine or server. For containers, see [Kubernetes](../kubernetes.md) and
[Docker](../docker.md).

## Install

=== "pip"

    ```bash
    pip install 'unitycatalog-rs[server,ui]'   # uc-server, uc and the web UI
    ```

    The extra `server` installs `uc-server` and `uc`; the extra `ui` installs the web UI, and
    `all` installs everything, with the Python client. The wheels are for Linux (x86_64 and
    aarch64, glibc and musl), macOS (x86_64 and arm64) and Windows (x86_64).

=== "Release tarball"

    Each [GitHub release](https://github.com/ion-elgreco/unitycatalog-rs/releases) has a tarball
    for Linux (x86_64 and aarch64, glibc 2.28 or later) and macOS (x86_64 and arm64), with
    `bin/uc-server`, `bin/uc` and the web UI in `share/unitycatalog-rs/ui`.

    ```bash
    release=https://github.com/ion-elgreco/unitycatalog-rs/releases/download/v0.0.0-dev
    name=unitycatalog-rs-v0.0.0-dev-x86_64-unknown-linux-gnu
    curl -LO "$release/$name.tar.gz"
    tar -xzf "$name.tar.gz"
    "$name/bin/uc-server"
    ```

    The other names end with `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and
    `aarch64-apple-darwin`.

    Keep the folders together: the binaries find the web UI next to their `bin/` folder.

=== "From source"

    In a checkout of the repository, with Rust and [just](https://just.systems):

    ```bash
    cargo install --locked wasm-bindgen-cli --version 0.2.121
    just wasm                                     # the web UI, into crates/uc-ui/dist
    cargo build --release -p uc-server -p uc-cli  # target/release/uc-server and uc
    UC_UI_DIR=crates/uc-ui/dist target/release/uc-server
    ```

    `just wasm` also needs the `brotli` tool.

## Run the server

```bash
uc-server
```

The server listens on port 8080 of all network interfaces. It keeps its data in `.uc`, in the
working directory: the catalog database `uc.db`. It makes the folder when it is not there. It signs
its tokens with a key in memory, unless `auth.signing_key_file` names one: see
[Sign-in and authorization](auth.md#the-signing-key). Open <http://localhost:8080> for the web UI.

Ctrl+C or `SIGTERM` stops the server: it finishes the requests that it has, then stops.

| Flag | What it does |
| --- | --- |
| `-p`, `--port <port>` | The port of the APIs and the web UI, on all interfaces. Default: `server.bind`, `0.0.0.0:8080`. |
| `--obs-port <port>` | Serve `/livez`, `/readyz` and `/metrics` on this port. None without it. |
| `--config <file>` | The settings file. Default: `UC_CONFIG`, else `uc.toml` in the working directory or a folder above it. |
| `-v`, `--version` | Show the version. |
| `-h`, `--help` | Show the flags. |

All other settings are in `uc.toml` or in `UC_*` environment variables: see
[Settings](settings.md).

```toml
# uc.toml
[server]
data_dir = "/var/lib/uc"

[auth]
enabled = true
```

## Run it with uc

`uc serve` runs `uc-server` with the same flags: the `uc-server` next to `uc`, else the one on
`PATH`. `uc dev` runs it for local work:

```bash
uc dev                     # http://localhost:8080, data in .uc
uc dev --port 9090 --dir /tmp/uc --no_browser
```

`uc dev` listens on `127.0.0.1` only, keeps its data in `--dir`, serves the web UI and opens it in
your browser. Ctrl+C stops it. Managed tables and models of a catalog without a storage root, such
as a catalog made in the web UI, go to `storage` in `--dir`, unless `storage.tables_root` or
`storage.models_root` is set.

## What the server serves

| Path | What it is |
| --- | --- |
| `/` | The web UI, for browsers. Other clients get the text `Hello, Unity Catalog!`. |
| `/api/2.1/unity-catalog/` | The Unity Catalog REST API. |
| `/api/2.1/unity-catalog/delta/v1/` | The Delta REST API, for managed Delta tables. |
| `/api/2.1/unity-catalog/iceberg/v1/` | The Iceberg REST catalog. |
| `/api/2.1/unity-catalog/delta-sharing/` | The Delta Sharing protocol. |
| `/api/1.0/unity-control/` | Token exchange, sign-out, and SCIM users and groups. |
| `/auth/login`, `/auth/callback`, `/auth/logout`, `/auth/mode` | The web UI's sign-in. |
| `/docs` | The API docs (Swagger UI). |
| `/api-docs/openapi.json` | The OpenAPI document of the APIs. |
| `/healthz`, `/version` | A health check and the version, on the API's port. |
| `/livez`, `/readyz`, `/metrics` | Probes and metrics, on the `--obs-port` port only. |

[APIs](apis.md) tells more about each API.

## Next steps

- [Settings](settings.md): every setting, in `uc.toml` and as a variable.
- [Database](database.md): the embedded database, or PostgreSQL.
- [Sign-in and authorization](auth.md): users, tokens and privileges.
- [Cloud storage](cloud-storage.md): storage roots, and credentials for S3, ADLS and GCS.
- [Operations](operations.md): probes, metrics, logs, the audit log, and backups.
