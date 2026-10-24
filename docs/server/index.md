# Run the server

`uc-server` serves the REST APIs and the web UI. To run it in containers, see [Docker](docker.md)
or [Kubernetes](kubernetes.md).

```bash
pip install 'unitycatalog-rs[server,ui]'
uc-server
```

The server listens on port 8080 and keeps the catalog in `.uc/uc.db`, in the working directory.
Open <http://localhost:8080> for the web UI. Ctrl+C or `SIGTERM` stops the server after it has
finished its requests.

## Settings

| Flag | What it does |
| --- | --- |
| `-p`, `--port <port>` | The port of the APIs and the web UI. Default: 8080. |
| `--obs-port <port>` | Serve `/livez`, `/readyz` and `/metrics` on a second port. |
| `--config <file>` | The settings file. Default: `UC_CONFIG`, else `uc.toml` in the working directory or a folder above it. |

All other settings are in `uc.toml`, or in `UC_<GROUP>_<KEY>` environment variables:

```toml
# uc.toml
[server]
data_dir = "/var/lib/uc"   # UC_SERVER_DATA_DIR

[auth]
enabled = true             # UC_AUTH_ENABLED
```

[Settings](../reference/settings.md) lists them all.

## Run it with uc

`uc dev` runs the server for local work: on `127.0.0.1:8080`, with its data in `.uc` and the web
UI, which it opens in your browser. `uc serve` runs `uc-server` with the flags above.

## Paths

| Path | What it is |
| --- | --- |
| `/` | The web UI. |
| `/api/2.1/unity-catalog/`, `/api/1.0/unity-control/` | The [REST APIs](../clients/rest-apis.md). |
| `/docs`, `/api-docs/openapi.json` | Swagger UI, and the OpenAPI document. |
| `/healthz`, `/version` | A health check, and the version. |
