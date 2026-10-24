# Settings

`uc-server` and `uc` read the same settings. Each setting is a key in a `[group]` table of
`uc.toml`, and an environment variable `UC_<GROUP>_<KEY>`: `enabled` in `[auth]` is
`UC_AUTH_ENABLED`.

```toml
# uc.toml
[server]
data_dir = "/var/lib/uc"

[db]
url = "postgresql://uc@postgres.example.com:5432/uc?sslmode=verify-full"

[auth]
enabled = true
allowed_issuers = ["https://accounts.google.com"]
```

The same with variables:

```bash
export UC_SERVER_DATA_DIR=/var/lib/uc
export UC_DB_URL='postgresql://uc@postgres.example.com:5432/uc?sslmode=verify-full'
export UC_AUTH_ENABLED=true
export UC_AUTH_ALLOWED_ISSUERS=https://accounts.google.com
```

## Where the settings come from

A setting takes its value from the first of:

1. a command-line flag, such as `--port`;
2. its environment variable;
3. `uc.toml`;
4. its default.

The settings file is the file of `--config`, else the file that `UC_CONFIG` names, else the first
`uc.toml` in the working directory or a folder above it. Without a file, the variables and the
defaults apply.

## Values

- A key that is not a setting is an error, in the file and in a variable of a group. Errors name
  the setting, its variable and the file: `auth.enabld (UC_AUTH_ENABLD): unknown field`.
- Other `UC_` variables are not settings, such as `UC_CONFIG`.
- Variables are text. The server reads numbers and booleans from the text. An empty variable is
  unset.
- A duration is text such as `30s`, `5m`, `24h` or `5d`.
- A list is a TOML array, or text with commas: `UC_AUTH_ALLOWED_ISSUERS=https://a.example,https://b.example`.

## Cloud credentials

The credentials of a bucket or storage account are a named table, such as `[s3.analytics]`.
Their variables include the name: `UC_S3_ANALYTICS_SECRET_KEY` is `secret_key` in
`[s3.analytics]`. A name has only lowercase letters and digits.

```toml
[s3.analytics]
bucket = "s3://analytics"
region = "us-east-1"
role_arn = "arn:aws:iam::123456789012:role/uc"
```

[Cloud storage](../server/cloud-storage.md) tells how the server uses them.

## `[server]`

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `data_dir` | `UC_SERVER_DATA_DIR` | `.uc` | The folder of the embedded database, unless `db.url` names another database. |
| `bind` | `UC_SERVER_BIND` | `0.0.0.0:8080` | The address of the APIs and the web UI. `--port` changes its port. |
| `obs_port` | `UC_SERVER_OBS_PORT` | none | The port of `/livez`, `/readyz` and `/metrics`, on all interfaces. Not the API's port. |
| `max_request_length` | `UC_SERVER_MAX_REQUEST_LENGTH` | `10485760` | The largest request body, in bytes. A larger one gets 413. |
| `request_timeout` | `UC_SERVER_REQUEST_TIMEOUT` | `10s` | The time that a request can take; then the server answers 503. `0s` for no limit. |
| `rate_limit_per_sec` | `UC_SERVER_RATE_LIMIT_PER_SEC` | `0` | Requests per second from one client address; more get 429. `0` for no limit. |
| `cors_allow_origins` | `UC_SERVER_CORS_ALLOW_ORIGINS` | all origins | The origins that browsers can call the API from. Empty or `*` allows every origin. |
| `readiness_interval` | `UC_SERVER_READINESS_INTERVAL` | `5s` | How often `/readyz` checks the database. |
| `readiness_db_timeout` | `UC_SERVER_READINESS_DB_TIMEOUT` | `2s` | How long that check can take. |
| `workers` | `UC_SERVER_WORKERS` | one per CPU | The threads that serve requests. Each has a runtime of its own and its share of the PostgreSQL connections (`db.pool_size`), so a request runs on one thread. |
| `io_threads` | `UC_SERVER_IO_THREADS` | `1` | The threads for outbound IO: the connections to object stores and HTTP services. |

## `[db]`

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `url` | `UC_DB_URL` | `uc.db` in `data_dir` | `file:<path>`, `memory`, `postgresql://…` or `jdbc:postgresql://…`. See [Database](../server/database.md). |
| `user` | `UC_DB_USER` | from the URL | The PostgreSQL user, instead of the URL's. |
| `password` | `UC_DB_PASSWORD` | from the URL | The PostgreSQL password, instead of the URL's. |
| `pool_size` | `UC_DB_POOL_SIZE` | `20` | The most PostgreSQL connections at one time. |

## `[auth]`

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `enabled` | `UC_AUTH_ENABLED` | `false` | Require sign-in, and check privileges. |
| `signing_key_file` | `UC_AUTH_SIGNING_KEY_FILE` | none | The PEM file of the RSA key that signs the server's tokens. Without it, the server makes a key in memory at each start. See [The signing key](../server/auth.md#the-signing-key). |
| `users` | none | none | `[[auth.users]]`: the users that the server makes at each start when they are missing. Only in `uc.toml`. See [The first users](../server/auth.md#the-first-users). |
| `allowed_issuers` | `UC_AUTH_ALLOWED_ISSUERS` | none | The issuers of identity tokens that the server exchanges: exact, or with `*` for one DNS label. |
| `audiences` | `UC_AUTH_AUDIENCES` | none | The audiences that those tokens must have. `*` alone accepts every audience. |
| `access_token_timeout` | `UC_AUTH_ACCESS_TOKEN_TIMEOUT` | `24h` | The life of the server's access tokens. |
| `cookie_timeout` | `UC_AUTH_COOKIE_TIMEOUT` | `5d` | The life of the `UC_TOKEN` cookie of the web UI. |
| `admin_principals` | `UC_AUTH_ADMIN_PRINCIPALS` | none | The emails of the admins, besides the metastore's owner: see [Privileges](../server/auth.md#privileges). |
| `cedar` | `UC_AUTH_CEDAR` | `false` | Check privileges with the Cedar policy engine. |
| `cedar_policies` | `UC_AUTH_CEDAR_POLICIES` | none | Files of Cedar policies, on top of the default policies. |
| `mode` | `UC_AUTH_MODE` | `none` | The web UI's sign-in: `none`, `oidc` or `forward`. |
| `public_url` | `UC_AUTH_PUBLIC_URL` | none | The server's external URL. The provider returns the browser to `<public_url>/auth/callback`, and with `oidc_end_session` to `<public_url>/` after "Log out". |
| `oidc_issuer` | `UC_AUTH_OIDC_ISSUER` | none | The OpenID Connect provider's issuer. |
| `oidc_client_id` | `UC_AUTH_OIDC_CLIENT_ID` | none | The OAuth client. |
| `oidc_client_secret` | `UC_AUTH_OIDC_CLIENT_SECRET` | none | Its secret; none for a public client. |
| `oidc_scopes` | `UC_AUTH_OIDC_SCOPES` | `openid email profile` | The scopes that the server asks for. |
| `oidc_end_session` | `UC_AUTH_OIDC_END_SESSION` | `false` | "Log out" also ends the provider's session: see [OpenID Connect](../server/auth.md#openid-connect). |
| `forward_trusted_proxies` | `UC_AUTH_FORWARD_TRUSTED_PROXIES` | none | The auth proxies whose headers count: CIDRs or addresses. |
| `forward_email_header` | `UC_AUTH_FORWARD_EMAIL_HEADER` | `Remote-Email` | The header with the user's email. |
| `forward_user_header` | `UC_AUTH_FORWARD_USER_HEADER` | `Remote-User` | The header with the user, without the email header. |
| `forward_logout_url` | `UC_AUTH_FORWARD_LOGOUT_URL` | none | The proxy's sign-out, where the web UI's "Log out" goes. |

[Sign-in and authorization](../server/auth.md) tells how these settings work together.

## `[storage]`

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `tables_root` | `UC_STORAGE_TABLES_ROOT` | none; `uc dev`: `storage` in its `--dir` | The root of managed tables without a catalog or schema root. |
| `models_root` | `UC_STORAGE_MODELS_ROOT` | none; `uc dev`: `storage` in its `--dir` | The root of models without a catalog or schema root. |
| `external_local_roots` | `UC_STORAGE_EXTERNAL_LOCAL_ROOTS` | none | The local folders under which external Iceberg tables can be. |
| `cleanup_poll_interval` | `UC_STORAGE_CLEANUP_POLL_INTERVAL` | `1m` | How often the cleanup of dropped managed data looks for work. |
| `cleanup_attempt_timeout` | `UC_STORAGE_CLEANUP_ATTEMPT_TIMEOUT` | `30m` | How long one cleanup can take. |
| `cleanup_lease_duration` | `UC_STORAGE_CLEANUP_LEASE_DURATION` | `2h` | How long a server holds a cleanup task. |
| `cleanup_initial_delay` | `UC_STORAGE_CLEANUP_INITIAL_DELAY` | `7d` | How long the server waits before it deletes the data of a dropped object. |
| `cleanup_retry_backoff` | `UC_STORAGE_CLEANUP_RETRY_BACKOFF` | `1h` | The wait before a failed cleanup runs again. |

## `[tables]`

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `managed` | `UC_TABLES_MANAGED` | `true` | Allow MANAGED tables, staging tables and Delta commits. |
| `delta_api_only` | `UC_TABLES_DELTA_API_ONLY` | `false` | Write managed Delta tables only through the Delta REST API. |
| `allow_missing_deletion_vectors` | `UC_TABLES_ALLOW_MISSING_DELETION_VECTORS` | `false` | Allow UniForm Iceberg v2 tables without deletion vectors. |
| `iceberg_writes` | `UC_TABLES_ICEBERG_WRITES` | `true` | Allow writes through the Iceberg REST catalog. |

## `[aws]`

The server's own AWS identity, which assumes the roles of storage credentials.

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `region` | `UC_AWS_REGION` | the AWS SDK's region, else `us-east-1` | The region of the STS calls. |
| `access_key` | `UC_AWS_ACCESS_KEY` | the AWS SDK's default chain | The access key. |
| `secret_key` | `UC_AWS_SECRET_KEY` | the AWS SDK's default chain | The secret key. |
| `master_role_arn` | `UC_AWS_MASTER_ROLE_ARN` | none | The server's role, shown as `unity_catalog_iam_arn` on storage credentials. |
| `endpoint` | `UC_AWS_ENDPOINT` | AWS | The S3 and STS API of an S3-compatible store, such as MinIO or RustFS: `http://` or `https://`. See [Cloud storage](../server/cloud-storage.md#aws-s3). |

## `[s3.<name>]`

The credentials of one S3 bucket. An entry needs `region` and `role_arn`, or `access_key`,
`secret_key` and `session_token`.

| Key | Variable | Description |
| --- | --- | --- |
| `bucket` | `UC_S3_<NAME>_BUCKET` | The bucket: `s3://bucket`. Required. |
| `region` | `UC_S3_<NAME>_REGION` | Its region. Iceberg clients get it too. |
| `role_arn` | `UC_S3_<NAME>_ROLE_ARN` | The role that the server assumes. |
| `access_key` | `UC_S3_<NAME>_ACCESS_KEY` | Keys that assume the role; without them, the AWS SDK's default chain. |
| `secret_key` | `UC_S3_<NAME>_SECRET_KEY` | |
| `session_token` | `UC_S3_<NAME>_SESSION_TOKEN` | With the keys: a session that the server gives out as it is. For tests. |

## `[adls.<name>]`

The credentials of one Azure storage account: a service principal. All keys but `test_mode` are
required.

| Key | Variable | Description |
| --- | --- | --- |
| `account` | `UC_ADLS_<NAME>_ACCOUNT` | The storage account's name. |
| `tenant_id` | `UC_ADLS_<NAME>_TENANT_ID` | The Entra ID tenant. |
| `client_id` | `UC_ADLS_<NAME>_CLIENT_ID` | The service principal. |
| `client_secret` | `UC_ADLS_<NAME>_CLIENT_SECRET` | Its secret. |
| `test_mode` | `UC_ADLS_<NAME>_TEST_MODE` | Give fake SAS tokens, for tests. |

## `[gcs.<name>]`

The credentials of one GCS bucket.

| Key | Variable | Description |
| --- | --- | --- |
| `bucket` | `UC_GCS_<NAME>_BUCKET` | The bucket: `gs://bucket`. Required. |
| `json_key_file` | `UC_GCS_<NAME>_JSON_KEY_FILE` | A service account's JSON key; without it, Google's Application Default Credentials. |
| `credential_generator` | `UC_GCS_<NAME>_CREDENTIAL_GENERATOR` | A test generator of tokens, for tests. |

## `[ui]`

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `enabled` | `UC_UI_ENABLED` | `true` | Serve the web UI at `/`. |
| `dir` | `UC_UI_DIR` | `share/unitycatalog-rs/ui` next to the binary's `bin/` folder | The web UI's folder. |

## `[log]`

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `dir` | `UC_LOG_DIR` | none | The folder of `server.log`, rolled at 10 MB, with the newest 5 files kept. Logs also go to stdout. |

## `[cli]`

The settings of `uc`. The server does not read them.

| Key | Variable | Default | Description |
| --- | --- | --- | --- |
| `server` | `UC_CLI_SERVER` | `http://localhost:8080` | The server of the commands. |
| `token` | `UC_CLI_TOKEN` | the sign-in of `uc auth login` | The token of the requests. |
| `server_bin` | `UC_CLI_SERVER_BIN` | `uc-server` next to `uc`, else on `PATH` | The server that `uc serve` and `uc dev` run. |
| `output_width` | `UC_CLI_OUTPUT_WIDTH` | `COLUMNS`, else the terminal's width; at least 120 | The width of tables. |
| `oauth_authorization_url` | `UC_CLI_OAUTH_AUTHORIZATION_URL` | none | `uc auth login` in the browser: the provider's authorization endpoint. |
| `oauth_token_url` | `UC_CLI_OAUTH_TOKEN_URL` | none | The provider's token endpoint. |
| `oauth_client_id` | `UC_CLI_OAUTH_CLIENT_ID` | none | The OAuth client. |
| `oauth_client_secret` | `UC_CLI_OAUTH_CLIENT_SECRET` | none | Its secret. |
| `oauth_redirect_port` | `UC_CLI_OAUTH_REDIRECT_PORT` | a free port | The port of the redirect to `http://localhost:<port>`. |

## Other environment variables

| Variable | Description |
| --- | --- |
| `UC_CONFIG` | The settings file. |
| `RUST_LOG` | The log level, such as `info` (the default) or `uc_server=debug`. |
