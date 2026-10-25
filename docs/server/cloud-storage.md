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
   identity: an AWS IAM role, an Azure service principal, an Azure managed identity, a GCP
   service account or a GCP service account key. It must reach the location's storage: an AWS
   role for `s3://`, an Azure identity for `abfs://` and `abfss://`, a GCP identity for `gs://`.
2. Else a location of the settings that covers the path, `[locations.<name>]`: a URL and the
   same kinds of identity. The longest URL wins.
3. Local paths get empty credentials.

The server signs in for the identity with its own identity in the cloud: `[identity.aws]`,
`[identity.azure]` or `[identity.gcp]`, else the identity of its environment.

With authorization on, an external table or volume, the `storage_root` of a catalog or schema,
and the credentials of a path need an external location that covers the path, also for admins.
The locations of the settings then serve only `storage.tables_root` and `storage.models_root`.

## AWS S3

With an external location:

```bash
uc credential create --name analytics \
  --aws_iam_role_arn arn:aws:iam::123456789012:role/uc-analytics
uc external_location create --name analytics \
  --url s3://analytics/data --credential_name analytics
```

The answer of `uc credential create` has an `external_id`. In AWS, let the server's identity
assume the role with that external id, in the role's trust policy. The server's identity is
`[identity.aws]`, else the environment: the `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
variables, the role of a Kubernetes service account, an ECS task role, EKS Pod Identity or an
instance role. AWS profiles in `~/.aws` are not read. Set `identity.aws.arn` to its ARN, and the
answers show it as `unity_catalog_iam_arn`.

Or, without an external location, give the role of a bucket in the settings:

```toml
[locations.analytics]
url = "s3://analytics"
region = "us-east-1"
aws_iam_role_arn = "arn:aws:iam::123456789012:role/uc-analytics"
```

For an S3-compatible store, such as MinIO or RustFS, set `identity.aws.endpoint` to its URL.

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
- A managed identity (`--azure_managed_identity`): the server's own Azure identity:
  `[identity.azure]`, else its environment: AKS workload identity, the `AZURE_CLIENT_ID`,
  `AZURE_TENANT_ID` and `AZURE_CLIENT_SECRET` variables, or the managed identity of the VM or App
  Service. With a resource ID (`--azure_managed_identity <resource id>`), the server uses that
  user-assigned managed identity of its VM, through the instance metadata service.

Or, without an external location, give the identity of the container in the settings: a service
principal,

```toml
[locations.lake]
url = "abfss://data@mylake.dfs.core.windows.net"
azure_directory_id = "<tenant id>"
azure_application_id = "<client id>"
azure_client_secret = "<client secret>"
```

or the server's own identity, with no secret:

```toml
[locations.lake]
url = "abfss://data@mylake.dfs.core.windows.net"
azure_managed_identity = true  # or the resource ID of a user-assigned identity
```

On AKS, give the server's pod workload identity with the Helm chart's `serviceAccount.annotations`
(`azure.workload.identity/client-id`) and `server.deployment.extraPodLabels`
(`azure.workload.identity/use: "true"`).

## Google Cloud Storage

The server gets a token of a GCP identity, and Google STS limits the token to the path and the
operation. Give that identity the Storage Object Admin role on the bucket.

With an external location, the credential is a service account or a service account key:

```bash
uc credential create --name models \
  --gcp_service_account unity-catalog@my-project.iam.gserviceaccount.com
uc external_location create --name models \
  --url gs://models/registry --credential_name models
```

- A service account (`--gcp_service_account <email>`): the server's own identity impersonates the
  service account. Give the server's identity the Service Account Token Creator role on the
  service account. The server stores no secret.
- Without an email (`--gcp_service_account`): the server's own identity: the key file of
  `[identity.gcp]`, else Google's Application Default Credentials: the
  `GOOGLE_APPLICATION_CREDENTIALS` file, gcloud's application default credentials, or the
  metadata server, such as GKE Workload Identity or the service account of the VM.
- A service account key (`--gcp_service_account_key <file>`): the JSON key file. The server keeps
  the key in its database, and no answer of the API contains it. Backups contain it.

Or, without an external location, give the identity of the bucket in the settings: a JSON key
file, or a service account that the server's own identity impersonates (`true` for the server's
own identity).

```toml
[locations.models]
url = "gs://models"
gcp_service_account_key = "/etc/uc/gcs-key.json"

[locations.archive]
url = "gs://archive"
gcp_service_account = "unity-catalog@my-project.iam.gserviceaccount.com"
```

On GKE, give the server's pod Workload Identity with the Helm chart's `serviceAccount.annotations`
(`iam.gke.io/gcp-service-account`).

## Dropped data

When you delete a managed object, the server deletes its files 7 days later
(`storage.cleanup_initial_delay`).
