# CLI

`uc` has the commands of upstream's `bin/uc`, with the same flags, and more: a
[terminal UI](tui.md), personal access tokens, backups, the Iceberg catalog, and a local server.

```bash
pip install 'unitycatalog-rs[cli]'
uc catalog list
uc catalog create --help
```

[CLI commands](../reference/cli.md) lists every command and its flags.

## Connect to a server

`uc` calls `http://localhost:8080` by default. For another server, give `--server`, set
`UC_CLI_SERVER`, or write it in `uc.toml`:

```toml
# uc.toml, in this folder or a folder above it
[cli]
server = "https://uc.example.com"
```

`--output json` or `--output jsonPretty` prints JSON instead of a table.

## Sign in

When the server needs a sign-in:

```bash
uc auth login
```

`uc auth login` signs you in to your identity provider in your browser, and keeps the server's
token for the next commands, until it expires (24 hours by default). It needs the provider's
OAuth settings, `oauth_*` in [`[cli]`](../reference/settings.md#cli). `uc auth logout` removes the
sign-in.

For scripts, give a token with `--auth_token` or `UC_CLI_TOKEN`, such as a personal access token:

```bash
uc token create --name laptop
```

## Tables, volumes and functions

```bash
uc table create --full_name main.default.events \
  --columns "id INT, name STRING" --storage_location /tmp/uc/events
uc table write --full_name main.default.events   # 15 random rows
uc table read --full_name main.default.events

uc volume create --full_name main.default.files --storage_location /tmp/uc/files
uc volume write --full_name main.default.files --source ./report.csv
uc volume read --full_name main.default.files --path report.csv

uc function create --full_name main.default.add \
  --input_params "a INT, b INT" --data_type INT --def "return a + b"
uc function call --full_name main.default.add --input_params "1,2"
```

- `uc table create` writes a Delta table at the location. With `--table_type MANAGED`, the server
  chooses the location.
- `uc table` and `uc volume` read and write with the server's temporary credentials: local paths,
  S3, ADLS and GCS.
- `uc function call` runs the function on your machine, with `python3`.

## Run a server

| Command | What it does |
| --- | --- |
| `uc dev` | Runs `uc-server` on `127.0.0.1:8080`, with the catalog in memory (unless `db.url` is set) and its managed data in `.uc`, and opens the web UI. |
| `uc serve` | Runs `uc-server` with its flags, such as `--port`. |
| `uc web` | Serves the web UI on `http://localhost:3000`, for the server of `--server`. |

These need `pip install 'unitycatalog-rs[server]'`, and `[ui]` for the web UI.

## Differences from upstream's CLI

- The settings are in `uc.toml` and `UC_CLI_*` variables, not in `cli.properties`.
- `uc` with no command opens the terminal UI.
- New commands: `token`, `group`, `admin`, `iceberg`, `tui`, `serve`, `dev` and `web`.
