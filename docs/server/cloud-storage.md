# Cloud storage

The data of tables, volumes and models is in your storage: S3, Azure Data Lake Storage (ADLS),
Google Cloud Storage (GCS) or a local folder. The server gives clients temporary credentials for
it, so that clients need no keys of their own.

## Managed and external storage

- An `EXTERNAL` table or volume is at the location that you give.
- The server chooses the location of a `MANAGED` table, volume or model, under a storage root:
  the `storage_root` of its schema, else of its catalog, else `storage.tables_root` (tables) or
  `storage.models_root` (models).

```bash
uc catalog create --name main --storage_root s3://analytics/main
```

## Temporary credentials

Clients ask for credentials for one object and one operation, such as `READ` on a table. The
credentials last one hour, and allow only the object's path. The user needs the privilege of the
operation, such as `SELECT` for `READ`.

```python
table = client.table("main.sales.orders").get()
credentials = client.generate_temporary_table_credentials(table.table_id, "READ")
```

The Delta REST API and the Iceberg REST catalog give credentials with their tables.

## Where the credentials come from

1. An external location that covers the path: its storage credential. A credential holds one
   identity: an AWS IAM role, an Azure service principal or an Azure managed identity. It must
   reach the location's storage: an AWS role for `s3://`, an Azure identity for `abfs://` and
   `abfss://`. A storage credential cannot hold a GCP identity yet.
2. Else the settings of the bucket or account: `[s3.<name>]`, `[adls.<name>]` or `[gcs.<name>]`.
3. Local paths get empty credentials.

With authorization on, an external table or volume, the `storage_root` of a catalog or schema,
and the credentials of a path need an external location that covers the path, also for admins.
The settings of a bucket then serve only `storage.tables_root` and `storage.models_root`.

## AWS S3

With an external location:

```bash
uc credential create --name analytics \
  --aws_iam_role_arn arn:aws:iam::123456789012:role/uc-analytics
uc external_location create --name analytics \
  --url s3://analytics/data --credential_name analytics
```

The answer of `uc credential create` has an `external_id`. In AWS, let the server's identity
assume the role with that external id, in the role's trust policy. The server's identity is the
`[aws]` settings, else the environment: the `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
variables, the role of a Kubernetes service account, an ECS task role, EKS Pod Identity or an
instance role. AWS profiles in `~/.aws` are not read.

Or, without an external location, give the role of a bucket in the settings:

```toml
[s3.analytics]
bucket = "s3://analytics"
region = "us-east-1"
role_arn = "arn:aws:iam::123456789012:role/uc-analytics"
```

For an S3-compatible store, such as MinIO or RustFS, set `aws.endpoint` to its URL.

## Azure Data Lake Storage

The server signs in to Microsoft Entra ID, gets a user delegation key, and signs SAS tokens with
it. Give the identity that signs in a data role on the storage account, such as Storage Blob Data
Contributor.

With an external location, the credential is a service principal or a managed identity:

```bash
uc credential create --name lake --azure_directory_id <tenant id> \
  --azure_application_id <client id> --azure_client_secret <client secret>
uc external_location create --name lake \
  --url abfss://data@mylake.dfs.core.windows.net/lake --credential_name lake
```

- A service principal: the server signs in as the app. The server keeps the client secret in its
  database, and no answer of the API contains it. Backups contain it.
- A managed identity (`--azure_managed_identity`): the server's own Azure identity, from its
  environment: AKS workload identity, the `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` and
  `AZURE_CLIENT_SECRET` variables, or the managed identity of the VM or App Service. With a
  resource ID (`--azure_managed_identity <resource id>`), the server uses that user-assigned
  managed identity of its VM, through the instance metadata service.

Or, without an external location, give a service principal of the storage account in the
settings:

```toml
[adls.lake]
account = "mylake"
tenant_id = "<tenant id>"
client_id = "<client id>"
client_secret = "<client secret>"
```

## Google Cloud Storage

A service account of the bucket, with the Storage Object Admin role: a JSON key file, else
Google's Application Default Credentials.

```toml
[gcs.models]
bucket = "gs://models"
json_key_file = "/etc/uc/gcs-key.json"
```

## Dropped data

When you delete a managed object, the server deletes its files 7 days later
(`storage.cleanup_initial_delay`).
