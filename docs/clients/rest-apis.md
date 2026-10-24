# REST APIs

The server has upstream's REST APIs, so clients of upstream's server use it as they are. The
[HTTP API reference](../reference/http-api.md) shows each operation.

| Base path | API | For |
| --- | --- | --- |
| `/api/2.1/unity-catalog` | Unity Catalog REST API | Catalogs, schemas, tables, volumes, functions, models, grants and credentials. |
| `/api/2.1/unity-catalog/delta/v1` | Delta REST API | Managed Delta tables, from Delta 4.3 and delta-kernel-rs. |
| `/api/2.1/unity-catalog/iceberg/v1` | Iceberg REST catalog | Iceberg clients, such as PyIceberg. |
| `/api/2.1/unity-catalog/delta-sharing` | Delta Sharing | Recipients of shares. |
| `/api/1.0/unity-control` | Control plane | Token exchange, and SCIM users and groups. |

With sign-in on, each request needs a token: `Authorization: Bearer <token>`. See
[Sign-in and authorization](../server/auth.md).

## Unity Catalog REST API

```bash
curl -X POST http://localhost:8080/api/2.1/unity-catalog/catalogs \
  -H 'Content-Type: application/json' -d '{"name": "main"}'
curl http://localhost:8080/api/2.1/unity-catalog/catalogs
```

Lists take `max_results` and `page_token`, and answer with `next_page_token` when there are more
results. Errors have upstream's body:

```json
{"error_code": "CATALOG_NOT_FOUND", "message": "Catalog not found: missing"}
```

## Iceberg REST catalog

Each catalog is an Iceberg warehouse, and its schemas are the namespaces:

```python
from pyiceberg.catalog import load_catalog

catalog = load_catalog(
    "uc",
    type="rest",
    uri="http://localhost:8080/api/2.1/unity-catalog/iceberg",
    warehouse="main",
)
print(catalog.list_namespaces())
```

The server gives the client temporary credentials for each table that it loads.

## Delta Sharing

`/shares` and `/recipients`, in the Unity Catalog REST API, make shares and recipients. A recipient
reads the shares with its own token, through the Delta Sharing protocol.

!!! warning "Not complete"
    The protocol does not yet give the data: a query answers no files.
