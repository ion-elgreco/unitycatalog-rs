# CLI commands

Each command of [`uc`](../clients/cli.md) is `uc <entity> <operation> [flags]`.
`uc <entity> <operation> --help` shows the same flags as this page. The flags in **bold** are
required.

Every command also takes the global options:

| Option | Description |
| --- | --- |
| `--server <url>` | The server. Default: `UC_CLI_SERVER`, else `cli.server`, else `http://localhost:8080`. |
| `--auth_token <token>` | The token of the requests. Default: `UC_CLI_TOKEN`, else `cli.token`, else the sign-in of `uc auth login` to the server. |
| `--output <format>` | `table` (the default), `json` or `jsonPretty`. |
| `--config <file>` | The settings file, instead of `UC_CONFIG` or `uc.toml`. |

The list commands take `--max_results` (default 100) and `--page_token`. After a full page,
`uc` prints the option for the next page to standard error, such as
`Next page: --page_token sales`. The lists of users and groups page with `--start_index`.

## catalog

| Operation | Flags |
| --- | --- |
| `create` | **`--name`**, `--comment`, `--properties`, `--storage_root` |
| `list` | `--max_results`, `--page_token` |
| `get` | **`--name`** |
| `update` | **`--name`**, and one or more of `--new_name`, `--comment`, `--properties` |
| `delete` | **`--name`**, `--force` (also delete its schemas and what they hold) |

`--properties` is a JSON object: `'{"team": "sales"}'`.

## schema

| Operation | Flags |
| --- | --- |
| `create` | **`--catalog`**, **`--name`**, `--comment`, `--properties`, `--storage_root` |
| `list` | **`--catalog`**, `--max_results`, `--page_token` |
| `get` | **`--full_name`** (`catalog.schema`) |
| `update` | **`--full_name`**, and one or more of `--new_name`, `--comment`, `--properties` |
| `delete` | **`--full_name`**, `--force` (also delete what it holds) |

## table

| Operation | Flags |
| --- | --- |
| `create` | **`--full_name`** (`catalog.schema.table`), **`--columns`**, `--format`, `--table_type`, `--storage_location`, `--properties` |
| `list` | **`--catalog`**, **`--schema`**, `--max_results`, `--page_token` |
| `get` | **`--full_name`** |
| `delete` | **`--full_name`** |
| `read` | **`--full_name`**, `--max_results` (rows; default 100) |
| `write` | **`--full_name`**: writes 15 random rows |

- `--columns`: `"id INT, name STRING"`. The types are `BOOLEAN`, `BYTE`, `SHORT`, `INT`, `LONG`,
  `FLOAT`, `DOUBLE`, `DATE`, `TIMESTAMP`, `TIMESTAMP_NTZ`, `STRING`, `BINARY` and `DECIMAL`.
- `--format`: `DELTA` (the default), `ICEBERG`, `CSV`, `JSON`, `AVRO`, `PARQUET`, `ORC` or `TEXT`.
  Only `DELTA` gets a Delta table at the location; `read` and `write` work with Delta tables.
- `--table_type`: `EXTERNAL` (the default), which needs `--storage_location`, or `MANAGED`.
- When a Delta table is already at the location of an `EXTERNAL` table, `create` keeps it.
  `--columns` must then have its columns, with the same names and types, in the same order.

## volume

| Operation | Flags |
| --- | --- |
| `create` | **`--full_name`**, `--volume_type` (`EXTERNAL`, the default, or `MANAGED`), `--storage_location` (for `EXTERNAL`), `--comment` |
| `list` | **`--catalog`**, **`--schema`**, `--max_results`, `--page_token` |
| `get` | **`--full_name`** |
| `update` | **`--full_name`**, and one or both of `--new_name`, `--comment` |
| `delete` | **`--full_name`** |
| `read` | **`--full_name`**, `--path`: prints a file, or lists a folder |
| `write` | **`--full_name`**, `--source` (a local file, or `-` for standard input), `--path` (default: the name of the file) |

## function

| Operation | Flags |
| --- | --- |
| `create` | **`--full_name`**, **`--input_params`** (`"a INT, b STRING"`), **`--data_type`**, `--def` (the body), `--language` (default `python`), `--comment` |
| `list` | **`--catalog`**, **`--schema`**, `--max_results`, `--page_token` |
| `get` | **`--full_name`** |
| `delete` | **`--full_name`** |
| `call` | **`--full_name`**, **`--input_params`** (`"1,2"`): runs a Python function with `python3` |

## registered_model

| Operation | Flags |
| --- | --- |
| `create` | **`--catalog`**, **`--schema`**, **`--name`**, `--comment` |
| `list` | `--catalog`, `--schema` (both or neither), `--max_results`, `--page_token` |
| `get` | **`--full_name`** |
| `update` | **`--full_name`**, and one or both of `--new_name`, `--comment` |
| `delete` | **`--full_name`**, `--force` (also delete its versions) |

## model_version

| Operation | Flags |
| --- | --- |
| `create` | **`--catalog`**, **`--schema`**, **`--name`** (the model), **`--source`** (the URI of the artifacts), `--run_id`, `--comment` |
| `list` | **`--full_name`** (the model), `--max_results`, `--page_token` |
| `get` | **`--full_name`**, **`--version`** |
| `update` | **`--full_name`**, **`--version`**, `--comment` |
| `delete` | **`--full_name`**, **`--version`** |
| `finalize` | **`--full_name`**, **`--version`**: the version is ready |

## permission

| Operation | Flags |
| --- | --- |
| `get` | **`--securable_type`**, **`--name`**, `--principal` |
| `create` | **`--securable_type`**, **`--name`**, **`--principal`**, **`--privilege`**: grants |
| `delete` | **`--securable_type`**, **`--name`**, **`--principal`**, **`--privilege`**: revokes |

`--securable_type` is `metastore`, `catalog`, `schema`, `table`, `volume`, `function`,
`registered_model`, `external_location` or `credential`. `--privilege` is a privilege such as
`"USE CATALOG"`: see [Privileges](../server/auth.md#privileges).

## metastore

| Operation | Flags |
| --- | --- |
| `get` | Shows the metastore's id and name. |

## credential

| Operation | Flags |
| --- | --- |
| `create` | **`--name`**, `--comment`, and one identity: `--aws_iam_role_arn`; `--azure_directory_id` with `--azure_application_id` and `--azure_client_secret`; `--azure_managed_identity [RESOURCE_ID]`; `--gcp_service_account [EMAIL]`; or `--gcp_service_account_key FILE` |
| `list` | `--max_results`, `--page_token` |
| `get` | **`--name`** |
| `update` | **`--name`**, and one or more of `--new_name`, `--comment` and one new identity, as for `create` |
| `delete` | **`--name`**, `--force` |

## external_location

| Operation | Flags |
| --- | --- |
| `create` | **`--name`**, **`--url`** (such as `s3://bucket/path`), **`--credential_name`**, `--comment` |
| `list` | `--max_results`, `--page_token` |
| `get` | **`--name`** |
| `update` | **`--name`**, and one or more of `--new_name`, `--url`, `--credential_name`, `--comment` |
| `delete` | **`--name`**, `--force` |

## user

| Operation | Flags |
| --- | --- |
| `create` | **`--name`**, **`--email`**, `--external_id` |
| `list` | `--filter` (SCIM, such as `'userName eq "alice@example.com"'`), `--start_index` (from 1), `--count` (default 100) |
| `get` | **`--id`** |
| `update` | **`--id`**, `--name`, `--external_id` |
| `delete` | **`--id`**: disables the user and removes its grants |

A user's email cannot change: it is the user's name in grants and owners. `update --email` fails.

## group

| Operation | Flags |
| --- | --- |
| `create` | **`--name`** |
| `list` | `--filter`, `--start_index`, `--count` |
| `get` | **`--id`** |
| `delete` | **`--id`** |

## auth

| Operation | Flags |
| --- | --- |
| `login` | `--identity_token`: exchanges it for a token of the server; without it, signs in in the browser. Keeps the token as the sign-in to the server, for the next commands. |
| `logout` | Removes the sign-in to the server. |

## token

| Operation | Flags |
| --- | --- |
| `create` | **`--name`**, `--expires_at` (epoch milliseconds): prints the token once |
| `list` | Lists your personal access tokens. |
| `delete` | **`--id`** |

## admin

| Operation | Flags |
| --- | --- |
| `backup` | Prints a snapshot of the catalog, as JSON. |
| `restore` | **`--path`** (the snapshot file, or `-` for standard input) |
| `audit` | `--principal`, `--method`, `--path_prefix`, `--max_results` |

## iceberg

| Operation | Flags |
| --- | --- |
| `config` | **`--catalog`**: the client configuration of the catalog |
| `list_namespaces` | **`--catalog`**, `--max_results`, `--page_token` |
| `list_tables` | **`--catalog`**, **`--schema`**, `--max_results`, `--page_token` |
| `load_table` | **`--full_name`**: the table's metadata |

## Other commands

| Command | Flags |
| --- | --- |
| `uc tui` | Opens the terminal UI, as `uc` alone does. |
| `uc dev` | `--port` (default 8080), `--dir` (default `.uc`), `--no_browser` |
| `uc serve` | The flags of `uc-server`: `--port`, `--obs-port`, `--config` |
| `uc web` | `--port` (default 3000), `--no_browser` |
