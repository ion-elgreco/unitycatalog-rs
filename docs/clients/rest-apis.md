# REST APIs

The server has upstream's REST APIs, so clients of upstream's server use it as they are. The
[HTTP API reference](../reference/http-api.md) shows each operation.

| Base path | API | For |
| --- | --- | --- |
| `/api/2.1/unity-catalog` | Unity Catalog REST API | Catalogs, schemas, tables, volumes, functions, models, grants and credentials. |
| `/api/2.1/unity-catalog/delta/v1` | Delta REST API | Managed Delta tables, from Delta 4.3 and delta-kernel-rs. |
| `/api/2.1/unity-catalog/iceberg/v1` | Iceberg REST catalog | Iceberg clients, such as PyIceberg. |
| `/api/v1/delta-sharing` | Delta Sharing | Recipients of shares: tables. |
| `/api/1.0/opensharing` | Open Sharing | Recipients of shares: tables, volumes and agent skills. |
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

## Delta Sharing and Open Sharing

`/shares` and `/recipients`, in the Unity Catalog REST API, make shares and recipients, in the
shape of the Databricks sharing API. A share holds tables, volumes and agent skills. An agent
skill is a volume with a `SKILL.md` file. A recipient reads its shares with its own token, which
starts with `dsr_`:

- `/api/v1/delta-sharing` is the Delta Sharing protocol, for clients such as `delta-sharing` and
  `deltalake`. It has the tables of a share. A query answers the files of a Delta table as URLs
  that the recipient reads for one hour at most.
- `/api/1.0/opensharing` is the Open Sharing protocol. It has the tables too, and the volumes and
  agent skills, and it gives temporary read-only credentials for each.

An admin does these steps:

1. `POST /shares` makes a share.
2. `PATCH /shares/{name}` changes its objects, with `updates`: each has an `action` (`ADD`,
   `UPDATE` or `REMOVE`) and a `data_object`. The `name` of an object is its full name in the
   catalog, and `shared_as` (`<schema>.<name>`) is what the recipient sees. For a volume or an
   agent skill, the field is `string_shared_as`.
3. `POST /recipients` makes a recipient. The answer has no token. It has an activation link, in
   `tokens[].activation_url`.
4. `PATCH /shares/{name}/permissions` lets the recipient read the share: a change with the
   recipient as `principal` and `SELECT` in `add`, or in `remove` to end it.
5. The admin sends the activation link to the recipient.

The recipient opens the link once, without sign-in, and gets the profile file of a Delta Sharing
client, with the token. The link then stops working, and the server keeps a hash of the token
only. `POST /recipients/{name}/rotate-token` makes a new token and a new link. The older tokens
end at once, or after `existing_token_expire_in_seconds`. `expiration_time` of a recipient is
when its tokens end.

A table in a share gives its latest version only. With `history_data_sharing_status` `ENABLED`,
recipients read older versions too, by version or by time; `start_version` is the lowest version
they read.

A share that a recipient was not granted answers 404, as a share that does not exist.

!!! warning "Limits"
    - Delta tables only. A query has no change data feed (`/changes`) and no `delta` response
      format.
    - File URLs are for S3 and Azure. For Google Cloud Storage, recipients use the temporary
      credentials of the Open Sharing protocol.
    - A table under a local path answers `file://` URLs, which only a recipient on the same
      machine can read.
    - A share object is a table, a volume or an agent skill. Partitions and `cdf_enabled` of an
      object, `storage_root` of a share and OAuth recipients are refused.
    - The answers of `/shares` and `/recipients` have their 64-bit numbers, such as
      `created_at`, as text.
