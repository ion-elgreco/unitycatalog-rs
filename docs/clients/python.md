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
from unitycatalog import Client

client = Client("http://localhost:8080/")
```

With sign-in, the client sends a bearer token with each request. It gets one in one of these
ways:

- **The identity of the environment.** `Client(url)` without a token signs in with what the
  environment has, at an identity provider that the server takes
  ([Identity providers](../server/auth.md#identity-providers)). Nothing in the code names it:
  the variables are the configuration, and moving from a laptop to a pod changes none of it.

    | Identity | What the environment has |
    | --- | --- |
    | Azure service principal | `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET` |
    | Azure workload identity | `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_FEDERATED_TOKEN_FILE` |
    | Azure managed identity | `AZURE_CLIENT_ID` for a user-assigned one, else `IDENTITY_ENDPOINT` or `MSI_ENDPOINT` |
    | Azure CLI | `az` on the path, signed in, without the variables of a managed identity |
    | Google service account | `GOOGLE_APPLICATION_CREDENTIALS` |
    | Google Cloud CLI | `gcloud` on the path, signed in |
    | Google machine or pod | `GCE_METADATA_HOST` or `K_SERVICE` |
    | AWS access keys | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, with `AWS_SESSION_TOKEN` for temporary ones |
    | AWS web identity (EKS) | `AWS_ROLE_ARN`, `AWS_WEB_IDENTITY_TOKEN_FILE` |
    | AWS container (ECS, EKS Pod Identity) | `AWS_CONTAINER_CREDENTIALS_FULL_URI` or `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI` |
    | AWS instance role (EC2) | `AWS_REGION` or `AWS_DEFAULT_REGION` |
    | Any OAuth client | `UC_OAUTH_TOKEN_URL`, `UC_OAUTH_CLIENT_ID`, `UC_OAUTH_CLIENT_SECRET`, with `UC_OAUTH_SCOPE` and `UC_OAUTH_AUDIENCE` when the provider needs them |
    | A token | `UC_TOKEN`, or a file with one in `UC_TOKEN_FILE` |

    The Azure and Google identities ask for a token for an audience of the server's issuer, so
    the server must list the provider with `direct = true`. An AWS identity signs a request for
    STS with the server id of the server's AWS block ([AWS](../server/auth.md#aws)). The audience
    or server id is the one that `UC_AUDIENCE` names, else the server's URL as the client calls
    it: the client asks for no other, so a server cannot have the identity sign for another
    service. The AWS CLI's profiles are not read: `aws configure export-credentials --format env`
    puts a profile's keys in the environment.

    `Client(url)` asks the server which providers it takes. A server that cannot answer yet is
    asked again at each call, and the calls go without a token until it answers.

- **A token source.** `token_provider=` names one: one of `unitycatalog.identity`, with the scope
  or audience of the server's issuer, or a callable that returns the token, or a pair of the
  token and when it expires, in seconds since the epoch.

    ```python
    import time

    from unitycatalog import Client, identity

    Client(url, token_provider=identity.azure("https://uc.example.com/.default"))
    Client(url, token_provider=identity.google("https://uc.example.com"))
    Client(url, token_provider=identity.aws("https://uc.example.com"))
    Client(url, token_provider=identity.client_credentials(token_url, client_id, client_secret))
    Client(url, token_provider=lambda: (fetch_token(), time.time() + 3600))
    ```

- **A fixed token.** `token=`: a personal access token, or a token of the server.

    ```python
    import os

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
