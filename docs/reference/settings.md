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

## Cloud identities and locations

`[identity.<cloud>]` is how the server signs in to AWS, Azure or GCP. A `[locations.<name>]` gives
the paths under its URL an identity, when no external location covers them.
[Cloud storage](../server/cloud-storage.md) tells how they work with external locations, with
examples per cloud.

The variables of these tables include the cloud or the name: `UC_IDENTITY_AWS_SECRET_KEY` is
`secret_key` in `[identity.aws]`, and `UC_LOCATIONS_ANALYTICS_URL` is `url` in
`[locations.analytics]`. A name has only lowercase letters and digits, and at least one letter:
the server reads a name of only digits, such as `UC_LOCATIONS_0_URL`, as a list index.

<!-- The settings tables: their variables do not break inside a name (extra.css). -->
<div class="uc-settings" markdown>

## `[server]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| **Where it runs** |  |  | Its database folder and its address. |
| `data_dir` | `UC_SERVER_DATA_DIR` | `.uc` | The folder of the embedded database, unless `db.url` names another database. |
| `bind` | `UC_SERVER_BIND` | `0.0.0.0:8080` | The address of the APIs and the web UI. `--port` changes its port. |
| **Requests** |  |  | Limits on the requests that it serves. |
| `max_request_length` | `UC_SERVER_MAX_REQUEST_LENGTH` | `10485760` | The largest request body, in bytes. A larger one gets 413. |
| `request_timeout` | `UC_SERVER_REQUEST_TIMEOUT` | `10s` | The time that a request can take; then the server answers 503. `0s` for no limit. |
| `rate_limit_per_sec` | `UC_SERVER_RATE_LIMIT_PER_SEC` | `0` | Requests per second from one client address; more get 429. `0` for no limit. |
| `cors_allow_origins` | `UC_SERVER_CORS_ALLOW_ORIGINS` | all origins | The origins that browsers can call the API from. Empty or `*` allows every origin. |
| **Health checks and metrics** |  |  | `/livez`, `/readyz` and `/metrics`. |
| `obs_port` | `UC_SERVER_OBS_PORT` | none | The port of `/livez`, `/readyz` and `/metrics`, on all interfaces. Not the API's port. |
| `readiness_interval` | `UC_SERVER_READINESS_INTERVAL` | `5s` | How often `/readyz` checks the database. |
| `readiness_db_timeout` | `UC_SERVER_READINESS_DB_TIMEOUT` | `2s` | How long that check can take. |
| **Threads** |  |  | The threads that serve requests and outbound IO. |
| `workers` | `UC_SERVER_WORKERS` | one per CPU | The threads that serve requests. Each has a runtime of its own and its share of the PostgreSQL connections (`db.pool_size`), so a request runs on one thread. |
| `io_threads` | `UC_SERVER_IO_THREADS` | `1` | The threads for outbound IO: the connections to object stores and HTTP services. |

## `[db]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| `url` | `UC_DB_URL` | `uc.db` in `data_dir`; `uc dev`: `memory` | `file:<path>`, `memory`, `postgresql://…` or `jdbc:postgresql://…`. See [Database](../server/database.md). |
| `user` | `UC_DB_USER` | from the URL | The PostgreSQL user, instead of the URL's. |
| `password` | `UC_DB_PASSWORD` | from the URL | The PostgreSQL password, instead of the URL's. |
| `pool_size` | `UC_DB_POOL_SIZE` | `20` | The most PostgreSQL connections at one time. |

## `[auth]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| **Authorization** |  |  | Sign-in and privilege checks for the API, and the first users and admins. |
| `enabled` | `UC_AUTH_ENABLED` | `false` | Require sign-in, and check privileges. |
| `admin_principals` | `UC_AUTH_ADMIN_PRINCIPALS` | none | The emails of the admins, besides the metastore's owner: see [Privileges](../server/auth.md#privileges). |
| `users` | none | none | `[[auth.users]]`: the users that the server makes at each start when they are missing. Only in `uc.toml`. See [The first users](../server/auth.md#the-first-users). |
| `cedar` | `UC_AUTH_CEDAR` | `false` | Check privileges with the Cedar policy engine. |
| `cedar_policies` | `UC_AUTH_CEDAR_POLICIES` | none | Files of Cedar policies, on top of the default policies. |
| **Tokens** |  |  | The server's own tokens, and the identity tokens that clients exchange for them, such as with `uc auth login`. |
| `signing_key_file` | `UC_AUTH_SIGNING_KEY_FILE` | none | The PEM file of the RSA keys of the server's tokens, newest first: the first signs. Without it, the server makes a key in memory at each start. See [The signing key](../server/auth.md#the-signing-key). |
| `access_token_timeout` | `UC_AUTH_ACCESS_TOKEN_TIMEOUT` | `24h` | The life of the server's access tokens. |
| `cookie_timeout` | `UC_AUTH_COOKIE_TIMEOUT` | `5d` | The life of the `UC_TOKEN` cookie of the web UI. |
| `allowed_issuers` | `UC_AUTH_ALLOWED_ISSUERS` | none | The issuers of identity tokens that the server exchanges: exact, or with `*` for one DNS label. |
| `audiences` | `UC_AUTH_AUDIENCES` | none | The audiences that those tokens must have. `*` alone accepts every audience. |
| **Web UI sign-in** |  |  | How users sign in to the web UI. The settings of each mode follow. |
| `mode` | `UC_AUTH_MODE` | `none` | The web UI's sign-in: `none`, `oidc` or `forward`. |
| **OpenID Connect** |  |  | `mode = "oidc"`: the server signs users in with an OpenID Connect provider. |
| `public_url` | `UC_AUTH_PUBLIC_URL` | none | The server's external URL. The provider returns the browser to `<public_url>/auth/callback`, and with `oidc_end_session` to `<public_url>/` after "Log out". |
| `oidc_issuer` | `UC_AUTH_OIDC_ISSUER` | none | The OpenID Connect provider's issuer. |
| `oidc_client_id` | `UC_AUTH_OIDC_CLIENT_ID` | none | The OAuth client. |
| `oidc_client_secret` | `UC_AUTH_OIDC_CLIENT_SECRET` | none | Its secret; none for a public client. |
| `oidc_scopes` | `UC_AUTH_OIDC_SCOPES` | `openid email profile` | The scopes that the server asks for. |
| `oidc_end_session` | `UC_AUTH_OIDC_END_SESSION` | `false` | "Log out" also ends the provider's session: see [OpenID Connect](../server/auth.md#openid-connect). |
| **Auth proxy** |  |  | `mode = "forward"`: an auth proxy in front of the server signs users in, and sends their email in a header. |
| `forward_trusted_proxies` | `UC_AUTH_FORWARD_TRUSTED_PROXIES` | none | The auth proxies whose headers count: CIDRs or addresses. |
| `forward_email_header` | `UC_AUTH_FORWARD_EMAIL_HEADER` | `Remote-Email` | The header with the user's email. |
| `forward_user_header` | `UC_AUTH_FORWARD_USER_HEADER` | `Remote-User` | The header with the user, without the email header. |
| `forward_logout_url` | `UC_AUTH_FORWARD_LOGOUT_URL` | none | The proxy's sign-out, where the web UI's "Log out" goes. |

[Sign-in and authorization](../server/auth.md) tells how these settings work together.

## `[storage]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| **Managed storage** |  |  | Where managed tables and models go by default. |
| `tables_root` | `UC_STORAGE_TABLES_ROOT` | none; `uc dev`: `storage` in its `--dir` | Where managed tables go when their catalog and schema have no storage root. |
| `models_root` | `UC_STORAGE_MODELS_ROOT` | none; `uc dev`: `storage` in its `--dir` | Where models go when their catalog and schema have no storage root. |
| **Local external tables** |  |  | Local folders for external Iceberg tables. |
| `external_local_roots` | `UC_STORAGE_EXTERNAL_LOCAL_ROOTS` | none | The local folders under which external Iceberg tables can be. |
| **Cleanup of dropped data** |  |  | When you delete a managed table, volume or model, the server deletes its files later. |
| `cleanup_initial_delay` | `UC_STORAGE_CLEANUP_INITIAL_DELAY` | `7d` | How long the server waits before it deletes the data of a dropped object. |
| `cleanup_retry_backoff` | `UC_STORAGE_CLEANUP_RETRY_BACKOFF` | `1h` | The wait before a failed cleanup runs again. |
| `cleanup_poll_interval` | `UC_STORAGE_CLEANUP_POLL_INTERVAL` | `1m` | How often the cleanup of dropped managed data looks for work. |
| `cleanup_attempt_timeout` | `UC_STORAGE_CLEANUP_ATTEMPT_TIMEOUT` | `30m` | How long one cleanup can take. |
| `cleanup_lease_duration` | `UC_STORAGE_CLEANUP_LEASE_DURATION` | `2h` | How long a server holds a cleanup task. |

## `[secrets]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| `store` | `UC_SECRETS_STORE` | `database` | Where the secrets of credentials go. `database`: in the database, sealed with `key_file`. |
| `key_file` | `UC_SECRETS_KEY_FILE` | none | The PEM file of the P-256 keys that seal the secrets, newest first. Without it, the server makes a new key in memory at each start, so the secrets cannot be opened after it stops. See [The secrets key](../server/database.md#the-secrets-key). |
| `cache_capacity` | `UC_SECRETS_CACHE_CAPACITY` | `500` | The most secrets, and Azure sign-ins, that the server keeps in memory. `0` keeps none. |
| `cache_ttl` | `UC_SECRETS_CACHE_TTL` | `10m` | How long a secret stays in memory after the store gives it. |

## `[tables]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| `managed` | `UC_TABLES_MANAGED` | `true` | Allow MANAGED tables, staging tables and Delta commits. |
| `delta_api_only` | `UC_TABLES_DELTA_API_ONLY` | `false` | Write managed Delta tables only through the Delta REST API. |
| `allow_missing_deletion_vectors` | `UC_TABLES_ALLOW_MISSING_DELETION_VECTORS` | `false` | Allow UniForm Iceberg v2 tables without deletion vectors. |
| `iceberg_writes` | `UC_TABLES_ICEBERG_WRITES` | `true` | Allow writes through the Iceberg REST catalog. |

## `[identity.aws]`

The identity that the server signs in to AWS with. It assumes the roles of storage credentials.
Without keys, the server uses the credentials of the machine or pod where it runs: the `AWS_*`
variables, the role of a Kubernetes service account, an ECS task role, EKS Pod Identity or an
instance role.

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| `access_key` | `UC_IDENTITY_AWS_ACCESS_KEY` | the credentials where the server runs | The access key. |
| `secret_key` | `UC_IDENTITY_AWS_SECRET_KEY` | the credentials where the server runs | The secret key. |
| `region` | `UC_IDENTITY_AWS_REGION` | `AWS_REGION`, else `AWS_DEFAULT_REGION`, else `us-east-1` | The region of the STS calls. |
| `arn` | `UC_IDENTITY_AWS_ARN` | none | The ARN of this identity, shown as `unity_catalog_iam_arn` on storage credentials, for the trust policies of their roles. A role here is also the one of `aws_iam_role_arn = true`; its trust policy must then allow the role itself. |
| `endpoint` | `UC_IDENTITY_AWS_ENDPOINT` | AWS | The S3 and STS API of an S3-compatible store, such as MinIO or RustFS: `http://` or `https://`. uc's table and volume commands use it too. See [Cloud storage](../server/cloud-storage.md#per-cloud). |

## `[identity.azure]`

The identity that the server signs in to Azure with: a service principal, or a user-assigned
managed identity. Without one, the server uses the credentials of the machine or pod where it
runs: AKS workload identity, the `AZURE_*` variables, or the managed identity of the VM or App
Service.

| Setting | Variable | Description |
| --- | --- | --- |
| `directory_id` | `UC_IDENTITY_AZURE_DIRECTORY_ID` | The service principal's tenant ID. |
| `application_id` | `UC_IDENTITY_AZURE_APPLICATION_ID` | The service principal's client ID. |
| `client_secret` | `UC_IDENTITY_AZURE_CLIENT_SECRET` | The service principal's secret. |
| `managed_identity_id` | `UC_IDENTITY_AZURE_MANAGED_IDENTITY_ID` | Or the resource ID of a user-assigned managed identity, from the instance metadata service. |

## `[identity.gcp]`

The identity that the server signs in to GCP with. Without a key, the server uses the
credentials of the machine or pod where it runs, which Google calls Application Default
Credentials: the `GOOGLE_APPLICATION_CREDENTIALS` file, gcloud's application default
credentials, or the metadata server, such as GKE Workload Identity.

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| `json_key_file` | `UC_IDENTITY_GCP_JSON_KEY_FILE` | Application Default Credentials | A service account's JSON key file. |
| `endpoint` | `UC_IDENTITY_GCP_ENDPOINT` | Google | The STS and IAM Credentials API of a GCP emulator, such as Floci: `http://` or `https://`. |

## `[locations.<name>]`

A URL and the identity that reaches the paths under it, for paths that no external location
covers. With authorization on, it serves only the paths that the admin wrote, `storage.tables_root`
and `storage.models_root`; see
[Which location serves a path](../server/cloud-storage.md#which-location-serves-a-path). The URL's
scheme picks the cloud, and an entry holds exactly one identity of that cloud. When several entries
cover a path, the one with the longest URL wins.

| Setting | Variable | Cloud | Description |
| --- | --- | --- | --- |
| `url` | `UC_LOCATIONS_<NAME>_URL` | all | `s3://bucket`, `abfss://container@account.dfs.core.windows.net` or `gs://bucket`, with a path or without. Required. |
| `region` | `UC_LOCATIONS_<NAME>_REGION` | S3 | The bucket's region, which Iceberg clients get. Without it, the server asks S3. |
| `aws_iam_role_arn` | `UC_LOCATIONS_<NAME>_AWS_IAM_ROLE_ARN` | S3 | `true` for the server's own role, `identity.aws.arn`, or the ARN of a role that `[identity.aws]` assumes. Required for S3. |
| `azure_directory_id` | `UC_LOCATIONS_<NAME>_AZURE_DIRECTORY_ID` | ADLS | A service principal's tenant ID, with `azure_application_id` and `azure_client_secret`. |
| `azure_application_id` | `UC_LOCATIONS_<NAME>_AZURE_APPLICATION_ID` | ADLS | Its client ID. |
| `azure_client_secret` | `UC_LOCATIONS_<NAME>_AZURE_CLIENT_SECRET` | ADLS | Its secret. |
| `azure_managed_identity` | `UC_LOCATIONS_<NAME>_AZURE_MANAGED_IDENTITY` | ADLS | `true` for `[identity.azure]`, or the resource ID of a user-assigned managed identity. |
| `gcp_service_account` | `UC_LOCATIONS_<NAME>_GCP_SERVICE_ACCOUNT` | GCS | `true` for `[identity.gcp]`, or the email of a service account that `[identity.gcp]` impersonates. |
| `gcp_service_account_key` | `UC_LOCATIONS_<NAME>_GCP_SERVICE_ACCOUNT_KEY` | GCS | A service account's JSON key file. |

## `[ui]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| `enabled` | `UC_UI_ENABLED` | `true` | Serve the web UI at `/`. |
| `dir` | `UC_UI_DIR` | `share/unitycatalog-rs/ui` next to the binary's `bin/` folder | The web UI's folder. |

## `[log]`

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| `dir` | `UC_LOG_DIR` | none | The folder of `server.log`, rolled at 10 MB, with the newest 5 files kept. Logs also go to stdout. |

## `[cli]`

The settings of `uc`. The server does not read them.

| Setting | Variable | Default | Description |
| --- | --- | --- | --- |
| **Server and output** |  |  | The server of the commands, and how `uc` prints. |
| `server` | `UC_CLI_SERVER` | `http://localhost:8080` | The server of the commands. |
| `token` | `UC_CLI_TOKEN` | the sign-in of `uc auth login` | The token of the requests. |
| `server_bin` | `UC_CLI_SERVER_BIN` | `uc-server` next to `uc`, else on `PATH` | The server that `uc serve` and `uc dev` run. |
| `output_width` | `UC_CLI_OUTPUT_WIDTH` | `COLUMNS`, else the terminal's width; at least 120 | The width of tables. |
| **Browser sign-in** |  |  | `uc auth login` in the browser: the identity provider's OAuth endpoints and client. `uc auth login` signs in there, then exchanges the provider's token for one of the server. |
| `oauth_authorization_url` | `UC_CLI_OAUTH_AUTHORIZATION_URL` | none | `uc auth login` in the browser: the provider's authorization endpoint. |
| `oauth_token_url` | `UC_CLI_OAUTH_TOKEN_URL` | none | The provider's token endpoint. |
| `oauth_client_id` | `UC_CLI_OAUTH_CLIENT_ID` | none | The OAuth client. |
| `oauth_client_secret` | `UC_CLI_OAUTH_CLIENT_SECRET` | none | Its secret. |
| `oauth_redirect_port` | `UC_CLI_OAUTH_REDIRECT_PORT` | a free port | The port of the redirect to `http://localhost:<port>`. |

</div>

## Other environment variables

| Variable | Description |
| --- | --- |
| `UC_CONFIG` | The settings file. |
| `RUST_LOG` | The log level, such as `info` (the default) or `uc_server=debug`. |
