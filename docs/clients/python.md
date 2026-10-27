# Python client

The package `unitycatalog-rs` calls the server's APIs from Python, with or without `asyncio`. It
imports as `unitycatalog`. It is generated from the server's API definitions, so it has a method
for each route.

```bash
pip install unitycatalog-rs
```

!!! note
    Its API is not the API of upstream's Python client, `unitycatalog-client`.

## Connect

```python
import os

from unitycatalog import Client

client = Client("http://localhost:8080/")

# With sign-in: a personal access token, or a token of the server.
client = Client("https://uc.example.com/", token=os.environ["UC_TOKEN"])
```

## Catalogs, schemas and tables

```python
client.create_catalog("main", comment="primary")
client.create_schema("default", "main")
print([catalog.name for catalog in client.list_catalogs()])

client.catalog("main").update(comment="the main catalog")
table = client.table("main.default.events").get()
client.catalog("main").delete(force=True)  # also deletes its schemas and tables
```

- The arguments have the names of the REST API's fields. The required ones are positional, the
  others are keywords.
- `client.catalog(name)`, `client.table(full_name)` and the other handles have the requests on
  one object: `get`, `update` and `delete`.
- The methods return classes, such as `CatalogInfo` and `TableInfo`, with each field as an
  attribute. They cannot change, and they compare by value.
- The enums are `enum.StrEnum` classes: give `TableType.MANAGED` or `"MANAGED"`.
- The list methods return all the items, page after page. `max_results` limits them.

## Errors

```python
from unitycatalog import NotFoundError

try:
    client.catalog("missing").get()
except NotFoundError as e:
    print(e.code, e.message)  # CATALOG_NOT_FOUND Catalog not found: missing
```

Each error is a `UnityCatalogError`, with a subclass per HTTP status, such as `NotFoundError`,
`PermissionDeniedError` and `ConflictError`.

## Async

Each method that sends a request also comes as `<name>_async`:

```python
catalog, schemas = await asyncio.gather(
    client.catalog("main").get_async(),
    client.list_schemas_async("main"),
)
```

## Temporary credentials

```python
table = client.table("main.default.events").get()
credentials = client.generate_temporary_table_credentials(table.table_id, "READ")
aws = credentials.aws_temp_credentials
storage_options = {
    "AWS_ACCESS_KEY_ID": aws.access_key_id,
    "AWS_SECRET_ACCESS_KEY": aws.secret_access_key,
    "AWS_SESSION_TOKEN": aws.session_token,
}
```

Give these to a reader of the table, such as deltalake or Polars.

## Files

The server reads and writes the files of a volume, and reads the files of a table, so the client
needs no storage credentials:

```python
root = "/Volumes/main/default/landing"
client.upload_file(f"{root}/data/a.csv", b"id,name\n1,x\n", overwrite=True)
for entry in client.list_directory_contents(f"{root}/data").contents:
    print(entry.name, entry.file_size)
data = client.download_file(f"{root}/data/a.csv").contents
head = client.download_file(f"{root}/data/a.csv", range="bytes=0-1023").contents
```

See [Files API](rest-apis.md#files-api).

## Reference

The [Python API](../reference/python/index.md) lists every method, class and enum, with its
arguments. `help(Client.create_table)` shows the same.
