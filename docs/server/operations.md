# Operations

## Probes

With `--obs-port <port>` (or `server.obs_port`), the server answers these paths on a second port,
on all interfaces, without sign-in. Without it, the server has no such port.

| Path | Answer |
| --- | --- |
| `/livez` | `{"healthy":true}` while the process serves. It does not ask the database. |
| `/readyz` | `{"healthy":true}` when the last check of the database passed; else HTTP 503 and `{"healthy":false}`. |
| `/metrics` | Prometheus metrics. |

The server checks the database once before it serves, then every `server.readiness_interval`
(default `5s`), with a time limit of `server.readiness_db_timeout` (default `2s`).

On the API's port, `/healthz` answers `{"status":"ok"}` and `/version` the server's version.

## Metrics

`/metrics` has the metrics of upstream's server that have a counterpart here:

| Metric | What it counts |
| --- | --- |
| `http_server_requests_total` | Requests, by `service`, `method` and `http_status`. |
| `uc_securable_table_creations_total` | Tables that clients made, through the Unity Catalog, Delta and Iceberg APIs. |
| `process_cpu_usage`, `process_resident_memory_bytes`, `process_start_time_seconds`, `process_uptime_seconds` | The server's process. |
| `system_cpu_usage`, `system_cpu_count`, `system_load_average_1m` | The machine. |

```yaml
# A Prometheus scrape job
scrape_configs:
  - job_name: unitycatalog
    static_configs:
      - targets: ["uc.example.com:8081"]
```

## Logs

The server logs to stdout. `RUST_LOG` sets the level: `info` by default, or such as `debug` or
`uc_server=debug,info`.

With `log.dir` (`UC_LOG_DIR`), the server also writes `server.log` in that folder, in the format
of upstream's log file. At 10 MB it starts a new file and keeps the newest 5 old files, compressed.

## Traces

A server built with the cargo feature `otel` sends its traces with OTLP over gRPC when
`OTEL_EXPORTER_OTLP_ENDPOINT` is set:

```bash
cargo build --release -p uc-server --features otel
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 target/release/uc-server
```

The released binaries, wheels and images do not have this feature.

## Audit log

The server records each request that changes something (`POST`, `PUT`, `PATCH` and `DELETE`): the
caller, the method, the path, the HTTP status and the time. It keeps the records in its database.
An admin reads them:

```bash
uc admin audit --principal alice@example.com --method DELETE --max_results 20
```

The API, `/api/2.1/unity-catalog/admin/audit`, also takes `since` and `until`, in epoch
milliseconds.

## Back up and restore

An admin makes a snapshot of the whole catalog, as JSON, while the server runs:

```bash
uc admin backup > snapshot.json
uc admin restore --path snapshot.json
```

A restore replaces everything in the catalog with the snapshot. The snapshot has credentials,
users and the hashes of tokens: keep it secret. It holds the catalog only, not the data of your
tables and volumes.

`backup` and `restore` need a signed-in admin, also when authorization is off: the `admin` user
of `uc-server token`, the owner of the metastore, or a user of `auth.admin_principals`.

## Limits

| Setting | Default | What it limits |
| --- | --- | --- |
| `server.max_request_length` | 10 MiB | The size of a request body. Larger bodies get HTTP 413. |
| `server.request_timeout` | `10s` | The time of a request. Then the server answers HTTP 503. |
| `server.rate_limit_per_sec` | none | Requests per second from one client address. More get HTTP 429. |
| `server.cors_allow_origins` | all origins | The origins that browsers can call the API from. |

## Stop the server

`SIGTERM` or Ctrl+C stops the server: it stops taking connections, finishes the requests that it
has, then stops.
