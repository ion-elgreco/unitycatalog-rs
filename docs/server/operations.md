# Operations

## Probes and metrics

With `--obs-port <port>` (`server.obs_port`), the server answers on a second port, without
sign-in:

| Path | Answer |
| --- | --- |
| `/livez` | `{"healthy":true}` while the process serves. |
| `/readyz` | `{"healthy":true}` when the last check of the database passed; else HTTP 503. |
| `/metrics` | Prometheus metrics: requests, table creations, and the process. |

## Logs

The server logs to stdout. `RUST_LOG` sets the level: `info` by default, or such as
`uc_server=debug,info`. With `log.dir`, it also writes `server.log` in that folder.

## Audit log

The server records each request that changes something: the caller, the method, the path, the
status and the time. An admin reads the records:

```bash
uc admin audit --principal alice@example.com --method DELETE --max_results 20
```

## Back up and restore

An admin makes a snapshot of the whole catalog, as JSON, while the server runs:

```bash
uc admin backup > snapshot.json
uc admin restore --path snapshot.json
```

A restore replaces the whole catalog. The snapshot has credentials, with their secrets sealed
([The secrets key](database.md#the-secrets-key)), and the hashes of tokens: keep it secret. The
server that restores it needs the secrets key that sealed them. The snapshot has no data of your
tables and volumes.

The audit log, backups and restores need a signed-in admin, also when sign-in is off: such as the
`admin` user of [`uc-server token`](auth.md#the-first-users).

## Traces

A server built with the cargo feature `otel` sends traces with OTLP when
`OTEL_EXPORTER_OTLP_ENDPOINT` is set. The released binaries and images do not have this feature.
