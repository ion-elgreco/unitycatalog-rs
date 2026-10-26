# Plan

Written on 2026-10-26. Part 1 is the file browser, built the same day. Part 2 lists the
Databricks APIs that we do not serve yet.

## 1. Files API and file browser

Built on 2026-10-26, on branch `feat/files-browser`. The UI shows the files of volumes and tables.
The server lists, reads and writes the files with `object_store`; the browser never gets storage
credentials. The API is a subset of the
[Databricks Files API](https://docs.databricks.com/api/workspace/files), with a second path root
for tables. Users' docs: `docs/clients/rest-apis.md` (Files API), `docs/clients/web-ui.md`
(Files), `docs/clients/python.md` (Files).

### Routes

The base is `/api/2.0/fs`, the same as in Databricks. `{path}` holds several segments, with or
without its leading `/`.

| Method | Path | `/Volumes/...` | `/Tables/...` |
| --- | --- | --- | --- |
| `GET` | `/directories/{path}` | list | list |
| `HEAD` | `/directories/{path}` | the directory exists | the directory exists |
| `PUT` | `/directories/{path}` | create | refused |
| `DELETE` | `/directories/{path}` | delete, if empty; with `recursive=true`, also what it holds | refused |
| `GET` | `/files/{path}` | download, with `Range` | download, with `Range` |
| `HEAD` | `/files/{path}` | metadata | metadata |
| `PUT` | `/files/{path}` | upload (`overwrite`) | refused |
| `DELETE` | `/files/{path}` | delete | refused |

Path roots:

- `/Volumes/<catalog>/<schema>/<volume>/<rest>`: the Databricks form.
- `/Tables/<catalog>/<schema>/<table>/<rest>`: ours. Read-only, because a file written or deleted
  outside the commit protocol breaks the table. A write is 400 `INVALID_ARGUMENT`.

Shapes, as `databricks-sdk` 0.157.0 (`service/files.py`) has them: the list answers
`{"contents": [{"path", "name", "is_directory", "file_size", "last_modified"}], "next_page_token"}`
(directories first in each page, their paths end in `/`, `last_modified` in milliseconds);
downloads and `HEAD` send `Content-Length`, `Content-Type` (`application/octet-stream`) and
`Last-Modified`; uploads and deletes answer 204.

### Privileges

| Root | Read (list, metadata, download) | Write (upload, create, delete) |
| --- | --- | --- |
| `/Volumes` | `READ VOLUME`, or owner | owner |
| `/Tables` | `SELECT`, or owner | none |

The rules are those of the temporary credentials (`rules::vend_volume`, `rules::vend_table`), with
`USE CATALOG` and `USE SCHEMA`: the API gives nothing that the credentials do not give. Changed
from the first plan: writes need ownership, because upstream has no `WRITE VOLUME` privilege.

### Server

- `crates/uc-server/src/routes/files.rs`. Paths with `.`, `..`, a backslash or NUL are refused.
  Keys are built from the storage root with `Path::child`, so no key leaves the volume or table.
- `ServerStorage::open_store` returns the store, its `PaginatedListStore` (S3, GCS, Azure) and the
  expiry of its credentials. `StoreCache` (in `AppState`) keeps opened stores by location and
  operation, at most 5 minutes and until 2 minutes before the credentials expire.
- Listing: one storage page per request with the storage's continuation token; local paths list
  the directory, sort it and page by offset.
- Uploads stream: smaller than 8 MiB in one `put`, else multipart with 4 parts at a time. They
  have no request timeout and no size limit (`server.request_timeout` and
  `server.max_request_length` do not apply). Downloads stream.
- Directories: local storage has real ones (`create_dir_all`, `remove_dir`). In object storage, a
  directory exists only through its files: `PUT` only checks the path, and a directory without
  files is 404.
- `DELETE /directories/...?recursive=true` (ours; Databricks deletes only empty directories):
  `remove_dir_all` on local storage, else `storage::delete_all` (list, then `delete_stream`, in
  batches), which the storage cleanup uses too. Never the volume itself. Directory deletes have no
  request timeout.
- The audit layer records uploads and deletes, as other writes; it does not record reads.
- The OpenAPI document names the path as one parameter: utoipa cannot show a wildcard.

### Protos, clients, trestle

`proto/unitycatalog/files/v1/files.proto`: `FilesService`, 8 RPCs. Trestle changes (the fork,
`~/Development/trestle`, branch `feat/uc-client-plan`, not committed yet):

- `(olai.trestle.v1.raw_body)`: a `bytes` field that is the raw request body (upload) or response
  body (download); the other fields of a response come from headers.
- A `{x=**}` path value drops its leading `/`.
- Python: `bytes` fields and parameters are Python `bytes` (getters return `Cow<[u8]>`, inputs
  take `PyBackedBytes`); the stubs say `bytes`.

The clients hold a whole file in memory: the generated client has no streamed bodies. The web UI
reads an upload into memory too; downloads are plain links, which the browser streams.

### UI

`crates/uc-ui/src/components/files.rs`: `FilesCard` in a "Files" tab after Overview, on volume and
table pages. A folder path, folders first, name, size and last modified, "Load more", a download
link per file. The owner of a volume gets Upload (a dialog with a file picker and "Replace files
that are already there"), New folder and Delete. The delete dialog of a folder counts the first
page of what it holds ("The folder holds 1 folder and 1 file.") and then shows a box that sets
`recursive`, as the catalog's force box; Delete waits for the box. A folder that is not there
shows as empty, for object storage.

### Tests

- `crates/uc-server/tests/files.rs`: files and folders of a volume, ranges, paging, read-only
  tables, path checks, privileges with sign-in.
- `crates/uc-client/tests/client/files.rs`: every RPC through the generated client.
- `python/client/tests/test_client.py::test_files`, and
  `tests/integrations/test_volumes.py::test_files_api_pages_through_a_volume_on_s3` (moto: S3
  paging, vended credentials, directories).
- Trestle: `raw_bodies_and_paths_of_several_segments` and the Python and stub checks of the
  `rest_client_e2e` fixture.
- By hand (kit in `/tmp/uc-files-ui`): 12 browser checks of the tab; `databricks-sdk`
  `WorkspaceClient.files` for all 8 calls, and `download_to` of a 60 MB file.

### Not done

- `databricks-sdk` uploads of 50 MB or more start a multipart upload
  (`POST /files{path}?action=initiate-upload`, `/create-upload-part-urls`), which gives 405. With
  `disable_experimental_files_api_client=True` the SDK sends one `PUT`, which works. Serving the
  multipart protocol would need part URLs that the server takes, and storage of the parts until
  the upload completes.
- CLI: `uc fs ls|get|put|rm`.
- `/Locations/<external location>/...` with `READ FILES` and `WRITE FILES`.
- Drag and drop in the UI.
- A snapshot view of a table: the data files of its current version, from the Delta log or the
  Iceberg manifests, not the folder.

## 2. Databricks APIs we do not serve

Compared on 2026-10-26: the endpoints in `databricks-sdk` 0.157.0, against our 131 routes. Only
the catalog side: compute, jobs, SQL, apps, serving, pipelines and billing are out of scope.

### Small: the data is already there

| API | Endpoints | What it gives |
| --- | --- | --- |
| Effective permissions | `GET /effective-permissions/{type}/{name}` | The privileges of each principal, with the parent they come from. The permissions card can show inherited grants. |
| Table exists | `GET /tables/{full_name}/exists` | A check without a 404. |
| Table summaries | `GET /table-summaries` | Names and types of all tables in a catalog, with name patterns, in one call. |
| Update function | `PATCH /functions/{name}` | Change the owner. |
| Storage credentials | `/storage-credentials` (CRUD), `POST /validate-storage-credentials` | The older name for `/credentials` with purpose `STORAGE`. Terraform and older scripts use it. |
| Validate credentials | `POST /validate-credentials` | Tests a credential on a URL (read, list, write, delete). Uses `ServerStorage`. |
| Current metastore | `GET /current-metastore-assignment`, `GET /metastores/{id}` | `WorkspaceClient.metastores.current()` works. Read-only, for our one metastore. |
| Share permissions | `GET`/`PATCH /shares/{name}/permissions`, `GET /recipients/{name}/share-permissions` | Grant a share to a recipient as Databricks does. Maps onto the recipient's share list. |
| SCIM at Databricks paths | `/api/2.0/preview/scim/v2/{Users,Groups,Me}` | `WorkspaceClient.current_user.me()` and the users and groups calls work. Same handlers as `/scim2`. |

### Medium: new features without compute

| API | Endpoints | What it gives |
| --- | --- | --- |
| Files | `/api/2.0/fs/...` | Part 1: done. |
| Model aliases | `PUT`/`DELETE`/`GET /models/{full_name}/aliases/{alias}`, `aliases` on models and versions | MLflow aliases such as `champion`. |
| Tags | `/entity-tag-assignments/...`, `/api/2.1/tag-policies` | Key and value tags on catalogs, schemas, tables, columns and volumes; allowed keys and values. Search can use them. |
| Table constraints | `POST /constraints`, `DELETE /constraints/{full_name}` | Primary and foreign keys on tables, for BI tools. |
| Lineage | `/api/2.0/lineage-tracking/external-metadata`, `/external-lineage` | Jobs report lineage between tables and outside objects; the UI shows it. Upstream issue #887 asks for lineage. |
| Recipient activation | `GET /public/data_sharing_activation_info/{url}`, `GET /public/data_sharing_activation/{url}` | A one-time link that gives the recipient its credential file. |
| Service credentials | `POST /temporary-service-credentials` | Vend credentials with purpose `SERVICE`, for cloud services instead of storage. |
| Secrets | `/secrets` (CRUD) | Secrets as securables with grants, on our secret store. |
| Service principals | `/api/2.0/preview/scim/v2/ServicePrincipals` | Machine identities apart from users. |
| Request for access | `/api/3.0/rfa/...` | A "Request access" button; per securable, where the request goes. |

### Not now

| API | Why |
| --- | --- |
| ABAC policies (`/policies`), row filters, column masks | The engine must enforce them; open source engines do not. |
| Connections (Lakehouse Federation) | Only useful with an engine that federates queries. |
| Providers, recipient federation policies | The receiving side of Delta Sharing: the server reads other servers' shares. Large. |
| Quality monitors, data quality, data classification, online tables | Need compute. |
| System schemas | Possible later: the audit log as a `system.access.audit` table. |
| Metastore CRUD and assignment, workspace bindings | We have one metastore and no workspaces. |
| Resource quotas, artifact allowlists, AI gateway (`mcp-services`, `skills`, `model-services`) | Databricks platform features. |
| DBFS (`/api/2.0/dbfs`) | The Files API replaces it. |
