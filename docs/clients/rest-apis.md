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
| `/api/2.0/fs` | Files API | The files of volumes and, read-only, of tables, as the Databricks Files API has them. |

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

## Files API

The server reads and writes the files with the credentials of the volume or table, so the caller
needs no storage credentials. A path is `/Volumes/<catalog>/<schema>/<volume>/...`, or
`/Tables/<catalog>/<schema>/<table>/...` for the files of a table:

```bash
fs=http://localhost:8080/api/2.0/fs
curl -X PUT --data-binary @a.csv "$fs/files/Volumes/main/default/landing/data/a.csv?overwrite=true"
curl "$fs/directories/Volumes/main/default/landing/data"
curl -o a.csv "$fs/files/Volumes/main/default/landing/data/a.csv"
curl "$fs/directories/Tables/main/default/events/_delta_log"
```

| Method | Path | Does |
| --- | --- | --- |
| `GET` | `/directories/<path>` | Lists a directory: `contents`, a page at a time (`page_size`, at most 1000, and `page_token`). |
| `HEAD` | `/directories/<path>` | 200 when the directory exists. |
| `PUT` | `/directories/<path>` | Creates a directory. |
| `DELETE` | `/directories/<path>` | Deletes an empty directory; with `recursive=true`, a directory and what it holds. |
| `GET` | `/files/<path>` | Downloads a file; takes a `Range` header. |
| `HEAD` | `/files/<path>` | The file's `Content-Length`, `Content-Type` and `Last-Modified`. |
| `PUT` | `/files/<path>` | Uploads the body as the file; `overwrite=true` replaces a file that is there. |
| `DELETE` | `/files/<path>` | Deletes a file. |

- **Privileges**: reads need `READ VOLUME` on a volume or `SELECT` on a table, as their temporary
  credentials. Writes need ownership of the volume: upstream has no `WRITE VOLUME` privilege.
- **`recursive`** is ours: the Databricks Files API deletes only empty directories.
- **Tables are read-only**: a file written or deleted outside the table's commits would break the
  table, so writes under `/Tables` fail.
- **Directories**: in object storage, a directory exists only through its files. Creating one
  there only checks the path, and a directory without files is not found. Local storage has real
  directories.
- **Databricks SDK**: `WorkspaceClient(...).files` works for these calls. For a file of 50 MB or
  more, the SDK starts a multipart upload, which the server does not have. Set
  `disable_experimental_files_api_client=True` in the SDK's `Config`: it then sends each upload
  as one `PUT`.

## Delta Sharing

`/shares` and `/recipients`, in the Unity Catalog REST API, make shares and recipients. A recipient
reads the shares with its own token, through the Delta Sharing protocol.

!!! warning "Not complete"
    The protocol does not yet give the data: a query answers no files.
