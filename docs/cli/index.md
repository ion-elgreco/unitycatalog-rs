# CLI

`uc` is the command-line client. It has the commands of upstream's `bin/uc`, with the same
entities, operations and flags, and more: a terminal UI, personal access tokens, backups, the
Iceberg catalog, and commands that run a server on your machine.

## Install

=== "pip"

    ```bash
    pip install 'unitycatalog-rs[cli]'
    ```

=== "Release tarball"

    The tarballs of the [GitHub releases](https://github.com/ion-elgreco/unitycatalog-rs/releases)
    have `bin/uc`, next to `bin/uc-server`. See [Server](../server/index.md#install).

=== "Docker"

    The image has `uc`: `docker exec <container> uc catalog list`.

```bash
uc --version
uc --help
uc catalog --help
uc catalog create --help
```

## Connect to a server

`uc` calls `http://localhost:8080` by default. To call another server, and to send a token:

```bash
uc --server https://uc.example.com --auth_token "$TOKEN" catalog list
```

The global options `--server`, `--auth_token`, `--output` and `--config` go before or after the
command. Without `--server` and `--auth_token`, `uc` takes the server and the token from the
variables `UC_CLI_SERVER` and `UC_CLI_TOKEN`, else from the `[cli]` table of `uc.toml`. Without a
token, `uc` sends the token of its [sign-in](#sign-in) to the server.

```toml
# uc.toml, in this folder or a folder above it
[cli]
server = "https://uc.example.com"
token = "<token>"
```

`--config` names another file. [Settings](../server/settings.md#cli) lists the settings of
`[cli]`.

## Output

`--output` selects the format: `table` (the default), `json` or `jsonPretty`.

```bash
uc catalog list
uc catalog get --name main --output jsonPretty
uc schema list --catalog main --output json | jq -r '.[].full_name'
```

A table shows a list as rows, and one object as keys and values. Its width is
`cli.output_width`, else `COLUMNS`, else the width of the terminal; at least 120.

## Sign in

With authorization on, each command needs a token. Sign in once:

```bash
uc auth login
```

`uc auth login` signs you in to your identity provider in your browser, and exchanges the
provider's token for a token of the server. It needs the provider's OAuth settings in `[cli]`:
`oauth_authorization_url`, `oauth_token_url`, `oauth_client_id`, and `oauth_client_secret`. The
provider must allow the redirect to `http://localhost:<port>`, with `oauth_redirect_port` as the
port. With `--identity_token "$ID_TOKEN"`, it exchanges that token instead.

`uc` keeps the token as its sign-in to the server, in `tokens.json` in `$XDG_STATE_HOME/uc`, else in
`~/.local/state/uc` (on Windows without `HOME`: `%LOCALAPPDATA%\uc`). Only you can read the file.
The next commands, the terminal UI and `uc web` send the token when you give no other one; the
terminal UI also takes a new sign-in at its next load. A sign-in is valid for the server's
`auth.access_token_timeout` (24 hours by default), and until the server has a new signing key. A
command without a valid sign-in says so, with the command that signs in again. `uc auth logout`
removes the sign-in.

As upstream's, `uc auth login` prints the token to standard output, and its messages to standard
error:

```bash
uc auth login --identity_token "$ID_TOKEN" --output json | jq -r .access_token
```

A token that you give wins over the sign-in: `--auth_token` for one command, else `UC_CLI_TOKEN` or
`token` in `[cli]`. A personal access token does not expire with the sign-in; make one when you are
signed in:

```bash
uc token create --name laptop
```

The [web UI](../ui.md#access-tokens) and the terminal UI also make tokens, and ask when they expire.

[Sign-in and authorization](../server/auth.md) tells how the server checks tokens.

## Catalogs, schemas and tables

```bash
uc catalog create --name main --comment "primary"
uc schema create --catalog main --name default
uc table create --full_name main.default.events \
  --columns "id INT, name STRING, ts TIMESTAMP" \
  --storage_location /tmp/uc/events
uc table write --full_name main.default.events   # 15 random rows
uc table read --full_name main.default.events
uc table list --catalog main --schema default
```

`uc table create` makes an `EXTERNAL` table by default, in the Delta format: it writes the Delta
table at the location, with the columns. If a Delta table is already there, it keeps it, when the
columns are the same. With `--table_type MANAGED`, the server chooses the
location under the storage root of the schema or the catalog. `uc table read` and `write` read and
write Delta tables, with the temporary credentials of the server: local paths, S3, ADLS and GCS.
With `aws.endpoint` (`UC_AWS_ENDPOINT`), S3 paths go to that S3-compatible store, as on the server.

## Volumes

```bash
uc volume create --full_name main.default.files --storage_location /tmp/uc/files
uc volume write --full_name main.default.files --source ./report.csv
uc volume read --full_name main.default.files                    # list the files
uc volume read --full_name main.default.files --path report.csv  # print a file
```

`--source -` reads the file from standard input. Without `--source`, `uc volume write` writes a
test file.

## Functions

```bash
uc function create --full_name main.default.add \
  --input_params "a INT, b INT" --data_type INT --def "return a + b"
uc function call --full_name main.default.add --input_params "1,2"
```

`uc function call` runs the Python function on your machine with `python3`.

## The terminal UI

`uc` with no command, or `uc tui`, opens a terminal UI. It has a tab for each kind of object:
catalogs, schemas, tables, volumes, functions, models, credentials, external locations, Iceberg
namespaces, users and tokens. The list is on the left, and the selected object, as JSON, on the
right; when no object is selected, such as in an empty list, the right side shows the logo. It has
the colors of the web UI. Where `COLORTERM` is not `truecolor` or `24bit`, the terminal gets the
nearest of its 256 colors.

| Key | Action |
| --- | --- |
| ++tab++ or ++right++, ++shift+tab++ or ++left++ | Next or previous tab. |
| ++down++ or ++j++, ++up++ or ++k++ | Select an item. |
| ++enter++ | Open the item: the schemas of a catalog, the tables of a schema. |
| ++backspace++ | Go back up. |
| ++shift+down++, ++shift+up++ | Scroll the details a line down or up. A scroll bar shows which part is in view. |
| ++page-down++ or ++space++, ++page-up++ | Scroll the details 10 lines down or up. |
| ++c++ | Create an item. |
| ++d++ | Delete the item, after a question. |
| ++r++ | Load the list again. |
| ++s++ | Change the server. |
| ++question++ | Show the keys. |
| ++q++ or ++esc++ | Quit. |

On the tokens tab, ++c++ asks for the token's name, then when it expires: a number of days (30 at
first), a date such as `2026-12-31` (the token works until the end of that day), or `never`. The new
token shows once, until ++enter++ or ++esc++.

The server deletes a catalog, schema or model only when it holds nothing. For one that holds
something, the terminal UI asks two more questions, as the web UI does: **Not empty** says why the
server refused and what the object holds, and ++f++ goes on. **Are you sure?** names everything that
goes, also the data of managed tables and volumes, and only ++y++ deletes it all. A change that
fails shows in red in the status line.

When you quit, the terminal UI keeps the server, the tab and the item that you opened in
`${XDG_STATE_HOME:-~/.local/state}/uc/state.json`, for the next start.

## Run a server on your machine

| Command | What it does |
| --- | --- |
| `uc dev` | Runs `uc-server` on `127.0.0.1:8080`, with its data in `.uc` and the web UI, and opens the UI in your browser. `--port`, `--dir` and `--no_browser` change this. Managed tables and models of a catalog without a storage root go to `storage` in that folder, unless `storage.tables_root` or `storage.models_root` is set. Ctrl+C stops it. |
| `uc serve` | Runs `uc-server` with its own flags, such as `uc serve --port 8080 --obs-port 8081`. |
| `uc web` | Serves the web UI on `http://localhost:3000` (`--port`) for the server of `--server`. The UI's API calls go to that server, with the token of `uc`, and the API docs (`/docs`) come from it. |

`uc dev` and `uc serve` run the `uc-server` next to `uc`, else the one on `PATH`, or the binary of
`cli.server_bin`. Install it with `pip install 'unitycatalog-rs[server]'`. `uc web` and the web
UI of `uc dev` need `pip install 'unitycatalog-rs[ui]'`.

## Differences from upstream's CLI

- The settings are in `uc.toml` and `UC_CLI_*` variables, not in `cli.properties`.
- Help and usage errors go to stderr, with the exit code 2.
- `uc` with no command opens the terminal UI.
- `uc table read` prints rows in another table layout.
- `uc volume write` can upload a file (`--source`).
- These commands are new: `token`, `group`, `admin`, `iceberg`, `tui`, `serve`, `dev` and `web`.

[Commands](commands.md) lists every command and its flags.
