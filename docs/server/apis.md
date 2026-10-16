# APIs

The server answers all its APIs on one port (default 8080). `/docs` shows them in Swagger UI, and
`/api-docs/openapi.json` is their OpenAPI document.

| Base path | API |
| --- | --- |
| `/api/2.1/unity-catalog` | [Unity Catalog REST API](#unity-catalog-rest-api) |
| `/api/2.1/unity-catalog/delta/v1` | [Delta REST API](#delta-rest-api) |
| `/api/2.1/unity-catalog/iceberg/v1` | [Iceberg REST catalog](#iceberg-rest-catalog) |
| `/api/2.1/unity-catalog/delta-sharing` | [Delta Sharing](#delta-sharing) |
| `/api/1.0/unity-control` | [Control plane](#control-plane): token exchange and SCIM |

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

The server can share tables with the Delta Sharing protocol.

- `/shares` and `/recipients` make shares and recipients. A recipient gets a bearer token
  (`dsr_…`); `/recipients/{name}/profile` gives its profile file, and
  `/recipients/{name}/rotate-token` a new token.
- A recipient reads with the protocol under `/api/2.1/unity-catalog/delta-sharing`: shares,
  schemas, tables, and the version, metadata and query of a table.

!!! warning "Not complete"
    The protocol does not yet give the data: a query answers no files, and the version of a table
    is always 0. The lists send all items at once.

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
