# Cloud storage

The server keeps no data of tables, volumes or models: they point at storage, such as S3, Azure
Data Lake Storage (ADLS), Google Cloud Storage (GCS) or a local folder. The server gives clients
temporary credentials for that storage, so that clients need no keys of their own.

## Managed and external storage

- An `EXTERNAL` table or volume has the location that you give.
- The server chooses the location of a `MANAGED` table, volume, registered model or model version.
  It is under a storage root: `<root>/__unitystorage/…`.

The storage root of a managed object is the first of:

1. the `storage_root` of its schema;
2. the `storage_root` of its catalog;
3. for tables, `storage.tables_root`, and for models, `storage.models_root` (see
   [Settings](settings.md#storage)).

Managed volumes need the root of a schema or a catalog.

```bash
uc catalog create --name main --storage_root s3://analytics/main
uc schema create --catalog main --name sales --storage_root s3://analytics/sales
```

The location of an external table or volume cannot be the root of a bucket, and cannot be in
managed storage.

## Temporary credentials

Clients ask the server for credentials for an object, and say what they want to do:

| Endpoint | For | Operations |
| --- | --- | --- |
| `POST /temporary-table-credentials` | A table | `READ`, `READ_WRITE` |
| `POST /temporary-volume-credentials` | A volume | `READ_VOLUME`, `WRITE_VOLUME` |
| `POST /temporary-model-version-credentials` | A model version | `READ_MODEL_VERSION`, `READ_WRITE_MODEL_VERSION` |
| `POST /temporary-path-credentials` | A path | `PATH_READ`, `PATH_READ_WRITE`, `PATH_CREATE_TABLE` |

The paths are under `/api/2.1/unity-catalog`. The Delta REST API and the Iceberg REST catalog
give credentials with their tables. The credentials last one hour, and allow only the object's
path and the operation. With authorization on, the user needs the privileges of the operation,
such as `SELECT` on the table for `READ`.

For a table on S3:

```python
from unitycatalog import Client

client = Client("http://localhost:8080/api/2.1/unity-catalog/")
table = client.get_table("main.sales.orders")
credentials = client.generate_temporary_table_credentials(table.table_id, "READ")
print(credentials.aws_temp_credentials.access_key_id, credentials.expiration_time)
```

`uc table read` and `uc volume read` use these credentials too.

## Where the credentials come from

The scheme of the path selects the cloud: `s3://` for AWS, `abfs://` and `abfss://` for Azure,
`gs://` for Google Cloud, and `file://` (or a local path) for the local disk. Then:

1. If an external location covers the path, the server uses its storage credential: an AWS IAM
   role. When several cover it, the longest one wins.
2. Else, the settings of the bucket or storage account: `[s3.<name>]`, `[adls.<name>]` or
   `[gcs.<name>]`.
3. Local paths get empty credentials.

Storage credentials are AWS IAM roles. For ADLS and GCS, use the settings of the storage
account or bucket.

## AWS S3

### With an external location

1. Give the server an AWS identity: the `[aws]` settings, or else the AWS SDK's default chain,
   such as an instance role or the role of a Kubernetes service account.

2. Make a storage credential with the IAM role that can read and write the data:

    ```bash
    uc credential create --name analytics \
      --aws_iam_role_arn arn:aws:iam::123456789012:role/uc-analytics
    ```

    The answer has the role's `external_id`. Set `aws.master_role_arn` to show the server's role
    as `unity_catalog_iam_arn` too.

3. In AWS, let the server's identity assume the role with that external id. The role's trust
   policy:

    ```json
    {
      "Version": "2012-10-17",
      "Statement": [
        {
          "Effect": "Allow",
          "Principal": { "AWS": "arn:aws:iam::123456789012:role/uc-server" },
          "Action": "sts:AssumeRole",
          "Condition": { "StringEquals": { "sts:ExternalId": "<external_id>" } }
        }
      ]
    }
    ```

4. Make an external location for the data:

    ```bash
    uc external_location create --name analytics \
      --url s3://analytics/data --credential_name analytics
    ```

The server assumes the role for each request, with a session policy that allows only the path
and the operation.

### With the settings of the bucket

Without an external location, `[s3.<name>]` gives the role of a bucket. The entry's keys, or else
the AWS SDK's default chain, assume the role:

```toml
[s3.analytics]
bucket = "s3://analytics"
region = "us-east-1"
role_arn = "arn:aws:iam::123456789012:role/uc-analytics"
# access_key = "..."   # optional: keys that assume the role
# secret_key = "..."
```

The region also goes to Iceberg clients, with their credentials.

### An S3-compatible store

`aws.endpoint` puts an S3-compatible store, such as MinIO or RustFS, in place of AWS: the URL of
its S3 and STS API.

```toml
[aws]
endpoint = "http://storage.example.com:9000"
region = "us-east-1"        # the default
access_key = "uc-server"    # a user of the store that may call AssumeRole
secret_key = "..."
```

Then:

- For each request, the server calls AssumeRole on the store, as on AWS, with a session policy for
  the path and the operation. The policy has no KMS statement. A storage credential still needs a
  role ARN; RustFS does not use it.
- The server reads and writes its S3 paths on the store.
- Iceberg clients get `s3.endpoint`, `s3.path-style-access` and the region of `aws.region`.
- `uc table` and `uc volume` read and write on the store when their settings have `aws.endpoint`.

Other clients, such as Spark, need the store's endpoint in their own settings.

## Azure Data Lake Storage

`[adls.<name>]` gives a service principal of the storage account. The server signs in to Entra ID
as the service principal, gets a user delegation key, and signs a SAS token for the path: it can
read and list, and for writes also write and delete.

```toml
[adls.lake]
account = "mylake"
tenant_id = "<tenant id>"
client_id = "<client id>"
client_secret = "<client secret>"
```

Give the service principal a data role on the storage account, such as Storage Blob Data
Contributor: a SAS token cannot give more than the principal has. Paths look like
`abfss://<container>@mylake.dfs.core.windows.net/<path>`.

## Google Cloud Storage

`[gcs.<name>]` gives the service account of a bucket: a JSON key file, or else Google's
Application Default Credentials. The server gets a token that it limits to the path: object
viewer, or object admin for writes.

```toml
[gcs.models]
bucket = "gs://models"
json_key_file = "/etc/uc/gcs-key.json"
```

Give the service account the Storage Object Admin role on the bucket.

## Local folders

Paths on the local disk, such as `file:///data/lake/orders` or `/data/lake/orders`, need no
credentials: the server gives empty ones, and clients read the disk. Use local paths on one
machine, for tests and demos.

External Iceberg tables on the local disk must be under an external location or under a folder
of `storage.external_local_roots`.

## Cleanup of dropped data

When you delete a managed table, volume, registered model or model version, the server deletes
its files later, in the background: after `storage.cleanup_initial_delay` (default `7d`). Until
then, you can still copy the files. A failed cleanup runs again after
`storage.cleanup_retry_backoff` (default `1h`).
