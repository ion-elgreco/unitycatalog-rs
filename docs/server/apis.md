# APIs

The server answers all its APIs on one port (default 8080). `/docs` shows them in Swagger UI, and
`/api-docs/openapi.json` is their OpenAPI document.

| Base path | API |
| --- | --- |
| `/api/2.1/unity-catalog` | [Unity Catalog REST API](#unity-catalog-rest-api) |
| `/api/2.1/unity-catalog/delta/v1` | [Delta REST API](#delta-rest-api) |
| `/api/2.1/unity-catalog/iceberg/v1` | [Iceberg REST catalog](#iceberg-rest-catalog) |
| `/api/1.0/unity-control` | [Control plane](#control-plane): token exchange and SCIM |
| `/delta-sharing` | [Delta Sharing](#delta-sharing), for recipients of shares |

With authorization on, each request needs a token: see
[Sign-in and authorization](auth.md).

## Unity Catalog REST API

The paths, methods and bodies of upstream's `api/all.yaml`. Clients of upstream's server use them
as they are.

```bash
curl -X POST http://localhost:8080/api/2.1/unity-catalog/catalogs \
  -H 'Content-Type: application/json' -d '{"name": "main", "comment": "primary"}'
curl http://localhost:8080/api/2.1/unity-catalog/catalogs
```

| Path | What it holds |
| --- | --- |
| `/catalogs`, `/schemas`, `/tables`, `/volumes`, `/functions` | The securables of the catalog. |
| `/models`, `/models/{full_name}/versions` | Registered models and their versions. |
| `/staging-tables` | The location and id of a new managed table, before its creation. |
| `/delta/preview/commits` | The commits of catalog-managed Delta tables. |
| `/credentials`, `/external-locations` | Storage credentials and external locations. |
| `/temporary-table-credentials`, `/temporary-volume-credentials`, `/temporary-model-version-credentials`, `/temporary-path-credentials` | Temporary credentials: see [Cloud storage](cloud-storage.md). |
| `/permissions/{securable_type}/{full_name}` | Grants: `GET` lists them, `PATCH` changes them. |
| `/metastore_summary` | The metastore's id and name. |

These paths are not in upstream's server:

| Path | What it holds |
| --- | --- |
| `/tokens` | Personal access tokens. |
| `/oauth/token` | OAuth client credentials, with a personal access token. |
| `/admin/backup`, `/admin/restore`, `/admin/audit` | Backups and the audit log: see [Operations](operations.md). |
| `/shares`, `/recipients` | The shares and recipients of [Delta Sharing](#delta-sharing). |

Lists take `max_results` (at most 100 per page) and `page_token`, and answer with
`next_page_token` when there are more results. Errors have upstream's body and codes:

```json
{
  "error_code": "CATALOG_NOT_FOUND",
  "message": "Catalog not found: missing",
  "details": [
    {
      "@type": "type.googleapis.com/google.rpc.ErrorInfo",
      "reason": "CATALOG_NOT_FOUND"
    }
  ]
}
```

## Delta REST API

Delta 4.3 and later, and delta-kernel-rs, use this API by default for managed Delta tables: the
server coordinates their commits. It has the paths of upstream's `api/delta.yaml`, under
`/api/2.1/unity-catalog/delta/v1`:

| Path | What it does |
| --- | --- |
| `/config` | The endpoints of the API, for a catalog. |
| `/catalogs/{catalog}/schemas/{schema}/staging-tables` | Reserves the location of a new managed table, with credentials. |
| `/catalogs/{catalog}/schemas/{schema}/tables` | Creates a table. |
| `/catalogs/{catalog}/schemas/{schema}/tables/{table}` | Loads (`GET`), updates and commits (`POST`), and deletes a table. |
| `…/tables/{table}/credentials`, `/staging-tables/{table_id}/credentials` | Temporary credentials. |
| `…/tables/{table}/rename`, `…/tables/{table}/metrics` | Renames a table; takes metrics. |

Settings of `[tables]` turn parts off: `managed = false` refuses managed tables, and
`delta_api_only = true` lets clients write managed Delta tables only through this API.

## Iceberg REST catalog

The server serves each catalog as an Iceberg REST catalog: the catalog's name is the Iceberg
warehouse, and its schemas are the namespaces. The tables are native Iceberg tables, and Delta
tables with UniForm's Iceberg metadata, which are read-only. With PyIceberg:

```python
from pyiceberg.catalog import load_catalog

catalog = load_catalog(
    "uc",
    type="rest",
    uri="http://localhost:8080/api/2.1/unity-catalog/iceberg",
    warehouse="main",
    # token="<token>",                    # with authorization on
)
print(catalog.list_namespaces())
```

When a client loads a table, the server gives it temporary credentials for the table's location
(`s3.*`, `adls.*` or `gcs.*`), and a refresh endpoint. Clients can create, commit, rename and drop
native Iceberg tables; `tables.iceberg_writes = false` refuses writes.

## Delta Sharing

The server shares Delta tables with the
[Delta Sharing protocol](https://github.com/delta-io/delta-sharing/blob/main/PROTOCOL.md). A
metastore admin makes shares and recipients with `/shares` and `/recipients` of the Unity Catalog
REST API. A recipient reads with a token of its own (`dsr_…`), under `/delta-sharing`. The server
reads the Delta log of each table with the Delta kernel.

```bash
API=http://localhost:8080/api/2.1/unity-catalog
curl -X POST $API/shares -d '{"name": "analytics"}'
curl -X PATCH $API/shares/analytics -d '{"add_objects": [
  {"source_table_full_name": "main.default.events", "history_data_sharing_status": "ENABLED"}]}'
curl -X POST $API/recipients -d '{"name": "partner"}'
curl -X PATCH $API/recipients/partner -d '{"add_shares": ["analytics"]}'
curl -X POST $API/recipients/partner/profile > partner.share
```

The profile file holds the endpoint and the recipient's token. Each call of `profile` (or
`rotate-token`) makes a new token, and the token before it stops working. With the profile, the
recipient reads with any Delta Sharing client:

```python
import delta_sharing

events = delta_sharing.load_as_pandas("partner.share#analytics.default.events")
```

A shared table has these fields:

| Field | What it does |
| --- | --- |
| `source_table_full_name` | The Delta table to share. |
| `shared_as_schema`, `shared_as_name` | The schema and the name that recipients see. By default, those of the table. |
| `history_data_sharing_status` | `ENABLED`: recipients also read earlier versions and the changes of the table. `DISABLED` (the default): only the latest version. |
| `start_version` | With history: the first version that recipients read. |

To change a shared table, remove it and add it again in one `PATCH`. When a share gets a new
name, its recipients keep it; when a share is deleted, its recipients lose it.

Recipients read:

- Their shares, and the schemas and tables of a share, in pages (`maxResults`, `pageToken`).
- The version, metadata and files of a table. The answer is in the `parquet` format, or in the
  `delta` format when the client asks for it (`delta-sharing-capabilities: responseformat=delta`).
  Tables with deletion vectors or column mapping need the `delta` format.
- With history: a version by its number or time, the files that each commit added and removed
  since a version (`startingVersion`), and the change data feed (`changes`). The change data feed
  needs `delta.enableChangeDataFeed = true` on the table.
- For MANAGED tables, also the commits that the server holds and that are not in the Delta log
  yet.

`jsonPredicateHints` and `limitHint` let the server skip files. The URLs of the files work for one
hour at most:

| Storage | URLs |
| --- | --- |
| S3 | Presigned, with credentials for reading the table, as `/temporary-table-credentials` vends them. They expire with the credentials. |
| ADLS | The blob URL with a SAS for the table. |
| GCS | Signed with the server's own credentials, such as a service account key. |
| Local | URLs of the server (`/delta-sharing/files/…`), signed with its signing key. Servers with the same `auth.signing_key_file` accept each other's URLs. |

The server does not support these parts of the protocol: `predicateHints` (SQL), asynchronous
queries, pages of files (`maxFiles`), refresh tokens, the protocol's
`temporary-table-credentials` (directory access), and OAuth recipients. A query answers all its
files at once.

## Control plane

Under `/api/1.0/unity-control`, as upstream's `api/control.yaml`:

| Path | What it does |
| --- | --- |
| `/auth/tokens` | Exchanges a token of your identity provider for a token of the server (RFC 8693). |
| `/auth/logout` | Ends the web UI's sign-in: it deletes the `UC_TOKEN` cookie. |
| `/scim2/Users`, `/scim2/Users/{id}` | SCIM 2.0 users, with filters, such as `userName eq "alice@example.com"`. |
| `/scim2/Me` | The signed-in user. |
| `/scim2/Groups`, `/scim2/Bulk` | SCIM groups and bulk changes. Not in upstream's server. |
| `/scim2/ServiceProviderConfig`, `/scim2/ResourceTypes`, `/scim2/Schemas` | SCIM discovery. |

## Clients

- [`uc`](../cli/index.md) and the [Python client](../python-client.md) of this project call these
  APIs.
- Iceberg clients, such as PyIceberg, use the Iceberg REST catalog.
- Delta clients use the Delta REST API and the temporary credentials.

Upstream's Spark and Hadoop connectors use the same APIs, but they are not tested against this
server.
