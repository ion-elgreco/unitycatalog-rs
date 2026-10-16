# Kubernetes

The Helm chart `uc-server` runs the server on Kubernetes, with the web UI. The chart is at
`oci://ghcr.io/ion-elgreco/charts/uc-server`; it runs the image
`ghcr.io/ion-elgreco/unitycatalog-rs` of the same version (see [Docker](docker.md)).

## Before you begin

- A Kubernetes cluster, and `kubectl` that connects to it.
- Helm 3.8 or later: the chart is in an OCI registry.

## Install the chart

```sh
helm install uc oci://ghcr.io/ion-elgreco/charts/uc-server \
  --version 0.0.0-dev \
  --namespace uc \
  --create-namespace
kubectl -n uc rollout status deploy/uc-uc-server-server
```

With the default values, the chart runs one replica of the server, with the web UI and without
sign-in. The catalog is in the embedded database, on a volume of 1 GiB.

The names of the chart's objects start with `<release>-uc-server`: with the release `uc`, the
Deployment and the Service are `uc-uc-server-server`.

## Open the server

The Service is a `ClusterIP`. To reach it from your machine:

```sh
kubectl -n uc port-forward svc/uc-uc-server-server 8080:8080
```

Then open <http://localhost:8080> for the web UI. The API is at
`http://localhost:8080/api/2.1/unity-catalog`, and the [CLI](cli/index.md) uses
`http://localhost:8080` when you give no `--server`:

```sh
uc catalog create --name main
```

The image also has `uc`:

```sh
kubectl -n uc exec deploy/uc-uc-server-server -- uc catalog list
```

To publish the server, see [Expose the server](#expose-the-server).

## Give values

Put your values in a file, and give it to `helm install` or `helm upgrade`:

```sh
helm upgrade --install uc oci://ghcr.io/ion-elgreco/charts/uc-server \
  --version 0.0.0-dev \
  --namespace uc \
  -f values.yaml
```

The sections below show the values for each task. [All values](#all-values) lists them, and
`helm show values oci://ghcr.io/ion-elgreco/charts/uc-server --version 0.0.0-dev` prints the
chart's `values.yaml`, with a comment for each value.

## Database

`server.db.type` selects the database:

- `file` (the default): the embedded Turso database, in a file on a PersistentVolumeClaim. Only
  one replica can use the file: the chart refuses `server.deployment.replicaCount` above 1.
- `postgresql`: a PostgreSQL server. Several replicas can share it.

```yaml
server:
  db:
    type: postgresql
    postgresqlConfig:
      host: postgres.example.com
      port: 5432
      dbName: ucdb
      username: uc
      passwordSecretName: uc-db          # a Secret with the key `password`
      ssl:
        enabled: true
        mode: verify-full                # require, verify-ca or verify-full
        rootCertConfigMapName: postgres-ca   # a ConfigMap with ca.crt
  deployment:
    replicaCount: 2
```

```sh
kubectl -n uc create secret generic uc-db --from-literal=password='<password>'
```

Without `ssl.enabled`, the server uses TLS when PostgreSQL offers it, and does not check the
certificate. With `ssl.enabled`, the server checks the certificate against `ssl.rootCert` (the
PEM text), the ConfigMap of `ssl.rootCertConfigMapName`, or else the system's root certificates.
`postgresqlConfig.extraParams` adds parameters to the connection URL, such as
`application_name`.

For the `file` database, `server.db.fileConfig.persistence` sets the volume: `size`,
`storageClassName` and `accessModes`. To use a claim that you made, set `pvcName` and
`createPVC: false`. With `enabled: false`, the database is in an `emptyDir`, and the catalog is
lost with the pod.

`helm uninstall` keeps the PersistentVolumeClaim. Delete it yourself to delete the catalog.

See [Database](server/database.md) for more about the two databases.

## Sign-in and authorization

With `auth.enabled: true`, each request needs the token of an active user, and the server checks
privileges. [Sign-in and authorization](server/auth.md) tells how this works.

### Users

The chart writes the users of `auth.users` into the server's `uc.toml` (`[[auth.users]]`). At each
start, the server makes the users that are missing, and grants their missing privileges on the
metastore. It never changes or deletes a user that exists. A change of `auth.users` restarts the
pods.

```yaml
auth:
  enabled: true
  users:
    - email: alice@example.com
      name: Alice
    - email: bob@example.com
      name: Bob
      privileges: ["CREATE CATALOG"]     # on the metastore
```

A user who creates a catalog owns it, and can grant privileges on it to other users. To make a
user an admin, who can read everything and grant any privilege, name their email in
`UC_AUTH_ADMIN_PRINCIPALS`. The server makes these users too:

```yaml
server:
  deployment:
    env:
      UC_AUTH_ADMIN_PRINCIPALS: alice@example.com
```

With an identity provider, the admin then signs in to the web UI or with `uc auth login`. Without
one, or when no admin can sign in, `uc-server token` in a pod prints a token of the server's
`admin` user that expires after one hour:

```sh
export UC_CLI_TOKEN=$(kubectl -n uc exec deploy/uc-uc-server-server -- uc-server token)
```

### Sign in to the web UI with OpenID Connect

The server signs users in with an OpenID Connect provider, such as Google, Okta, Keycloak or
Microsoft Entra ID. The browser holds no settings of the provider.

```yaml
auth:
  enabled: true
  publicUrl: https://uc.example.com      # the URL of the server, as browsers see it
  oidc:
    issuer: https://accounts.google.com
    clientSecretName: uc-oauth           # a Secret with clientId and clientSecret
```

```sh
kubectl -n uc create secret generic uc-oauth \
  --from-literal=clientId='<client id>' \
  --from-literal=clientSecret='<client secret>'
```

In the provider, allow the redirect URI `https://uc.example.com/auth/callback` for the client.
For a public client, leave out `clientSecret`. `auth.oidc.scopes` sets the scopes (default
`openid email profile`). A user signs in when their email names an active user of the server.
With `auth.oidc.endSession: true`, "Log out" ends the provider's session too: then also allow the
post-logout redirect URI `https://uc.example.com/`. It is off by default, because other apps may
share the provider's sign-in ([Sign-in and authorization](server/auth.md#openid-connect)).

### Sign in through an auth proxy

Or an auth proxy in front of the server, such as oauth2-proxy or Authelia, signs users in and
names the user in its headers:

```yaml
auth:
  enabled: true
  forward:
    trustedProxies: 10.42.0.0/16         # the proxies' addresses: CIDRs or addresses
    emailHeader: Remote-Email
    userHeader: Remote-User
    logoutUrl: https://auth.example.com/logout
```

The server takes the headers only from the addresses of `trustedProxies`. You cannot set
`auth.oidc` and `auth.forward` together.

### Tokens for the CLI and the clients

The CLI and other clients send a token. They get one in two ways:

- They exchange a token of your identity provider for a token of the server (`uc auth login`).
  Set the issuers and the audiences that the server accepts:

    ```yaml
    auth:
      allowedIssuers: https://accounts.google.com
      audiences: <client id>
    ```

    Both take several values with commas. In an issuer, `*` matches one DNS label; `*` alone
    as the audience accepts every audience.

- A signed-in user makes a personal access token: `uc token create --name <name>`.

`auth.accessTokenTimeout` (default `24h`) is the life of the server's tokens, and
`auth.cookieTimeout` (default `5d`) the life of the web UI's sign-in.

### The signing key

At install, the chart makes the Secret `<release>-uc-server-server-jwt-key`, with an RSA key that
signs the server's tokens. Upgrades and `helm uninstall` keep it. The replicas share it, as a
read-only volume (`auth.signing_key_file`). The server writes no file there. To use a key of your
own, give a Secret with `private_key.pem`, such as one of `uc-server keygen`:

```yaml
server:
  jwtKeypairSecret:
    name: my-uc-key
    create: false
```

## Cloud storage

The server gives clients temporary credentials for the data of tables, volumes and models. It
gets them with the credentials that you give per bucket or storage account:

```yaml
storage:
  tableStorageRoot: s3://analytics/managed
  modelStorageRoot: s3://analytics/models
  credentials:
    s3:
      - bucketPath: s3://analytics
        region: us-east-1
        awsRoleArn: arn:aws:iam::123456789012:role/uc
        credentialsSecretName: uc-s3     # accessKey and secretKey
    adls:
      - storageAccountName: myaccount
        credentialsSecretName: uc-adls   # tenantId, clientId and clientSecret
    gcs:
      - bucketPath: gs://models
        credentialsSecretName: uc-gcs    # jsonKey: a service account's JSON key
```

`tableStorageRoot` and `modelStorageRoot` are the roots of managed tables and models that have no
catalog or schema root. The server assumes the role of `awsRoleArn` to get credentials for the
bucket. Each S3 entry needs `region` and `awsRoleArn`. Without `credentialsSecretName`, the AWS
SDK's default chain finds the identity that assumes the role, such as the role of the pod's
service account. For that, set `serviceAccount.create: true` and its `annotations`.
[Cloud storage](server/cloud-storage.md) tells how the server vends credentials, also through
external locations.

## Expose the server

The server answers the API, the sign-in and the web UI on one port. An Ingress:

```yaml
ingress:
  enabled: true
  host: uc.example.com
  ingressClassName: nginx
  tlsSecretName: uc-tls
```

Or an HTTPRoute of the Gateway API:

```yaml
httpRoute:
  enabled: true
  host: uc.example.com
  parentRefs:
    - name: public-gateway
      namespace: gateway
```

Set `auth.publicUrl` to the same URL when you use OpenID Connect.

## Other settings

The chart sets the server's settings as `UC_*` environment variables. For the settings that have
no value in the chart, use `server.deployment.env`, or a `uc.toml` in `server.config.toml`. The
variables win over the file. [Settings](server/settings.md) lists every setting.

```yaml
server:
  deployment:
    env:
      UC_TABLES_ICEBERG_WRITES: "false"
  config:
    toml: |
      [server]
      rate_limit_per_sec = 50
```

## Probes and metrics

The server answers `/livez`, `/readyz` and `/metrics` on its second port,
`server.deployment.obsPort` (default 8081), without sign-in. The chart's startup, liveness and
readiness probes use it, and the Service has it as the port `obs`. To scrape the metrics with
Prometheus, point a scrape job or a ServiceMonitor at that port. See
[Operations](server/operations.md).

## Security

The pod runs as the image's non-root user (65532), with a read-only root filesystem, no Linux
capabilities and the `RuntimeDefault` seccomp profile. The server writes only to the database
volume and to `/tmp`, an `emptyDir`. It logs to stdout. To change this, set
`server.deployment.podSecurityContext` and `server.deployment.securityContext`.

## Upgrade and uninstall

```sh
helm upgrade uc oci://ghcr.io/ion-elgreco/charts/uc-server \
  --version 0.0.0-dev \
  --namespace uc \
  -f values.yaml
helm uninstall uc --namespace uc
```

The server updates the database schema when it starts. `helm uninstall` keeps the database's
PersistentVolumeClaim and the signing key's Secret.

## All values

| Value | Default | Description |
| --- | --- | --- |
| `server.enabled` | `true` | Install the server. |
| `server.db.type` | `file` | `file` (embedded Turso, one replica) or `postgresql`. |
| `server.db.fileConfig.persistence.enabled` | `true` | Keep the database on a volume; else an `emptyDir`. |
| `server.db.fileConfig.persistence.pvcName` | `""` | The claim's name; default `<release>-uc-server-server-db`. |
| `server.db.fileConfig.persistence.createPVC` | `true` | Make the claim. |
| `server.db.fileConfig.persistence.accessModes` | `["ReadWriteOnce"]` | The claim's access modes. |
| `server.db.fileConfig.persistence.size` | `1Gi` | The claim's size. |
| `server.db.fileConfig.persistence.storageClassName` | `""` | The claim's storage class. |
| `server.db.postgresqlConfig.host` | `""` | The PostgreSQL host (required for `postgresql`). |
| `server.db.postgresqlConfig.port` | `5432` | Its port. |
| `server.db.postgresqlConfig.dbName` | `ucdb` | The database. |
| `server.db.postgresqlConfig.username` | `postgres` | The user. |
| `server.db.postgresqlConfig.passwordSecretName` | `""` | A Secret with the password; no password without it. |
| `server.db.postgresqlConfig.passwordSecretKey` | `password` | The key of the password in that Secret. |
| `server.db.postgresqlConfig.ssl.enabled` | `false` | Use `ssl.mode`; without it, TLS when offered, unchecked. |
| `server.db.postgresqlConfig.ssl.mode` | `verify-full` | libpq's `sslmode`: `require`, `verify-ca` or `verify-full`. |
| `server.db.postgresqlConfig.ssl.rootCert` | `""` | The PEM of the root certificate. |
| `server.db.postgresqlConfig.ssl.rootCertConfigMapName` | `""` | Or a ConfigMap with it. |
| `server.db.postgresqlConfig.ssl.rootCertConfigMapKey` | `ca.crt` | Its key in the ConfigMap. |
| `server.db.postgresqlConfig.extraParams` | `{}` | More parameters of the connection URL. |
| `server.config.toml` | `""` | A `uc.toml` for other settings; the variables win over it. |
| `server.service.type` | `ClusterIP` | The Service's type. |
| `server.service.port` | `8080` | The Service's port for the API and the web UI. |
| `server.deployment.replicaCount` | `1` | Replicas; above 1 needs `postgresql`. |
| `server.deployment.port` | `8080` | The container's port for the API and the web UI. |
| `server.deployment.obsPort` | `8081` | The port of the probes and the metrics. |
| `server.deployment.image.repository` | `ghcr.io/ion-elgreco/unitycatalog-rs` | The image. |
| `server.deployment.image.tag` | `""` | Its tag; default the chart's `appVersion`. |
| `server.deployment.image.pullPolicy` | `IfNotPresent` | The pull policy. |
| `server.deployment.imagePullSecrets` | `[]` | Pull secrets. |
| `server.deployment.env` | `{}` | More environment variables, such as `UC_*` settings. |
| `server.deployment.resources` | `{}` | The container's resources. |
| `server.deployment.podSecurityContext` | user 65532, `RuntimeDefault` | The pod's security context. |
| `server.deployment.securityContext` | read-only root, no capabilities | The container's security context. |
| `server.deployment.extraVolumes`, `extraVolumeMounts` | `[]` | More volumes and mounts. |
| `server.deployment.extraInitContainers`, `extraContainers` | `[]` | More containers. |
| `server.deployment.extraPodAnnotations`, `extraPodLabels` | `{}` | More annotations and labels on the pod. |
| `server.deployment.nodeSelector`, `tolerations`, `affinity` | empty | Scheduling. |
| `server.deployment.updateStrategy` | `{}` | The Deployment's strategy. |
| `server.deployment.enableStartupProbe`, `startupProbe` | `true`, `{}` | The startup probe, and its spec instead of the default. |
| `server.deployment.enableLivenessProbe`, `livenessProbe` | `true`, `{}` | The liveness probe, the same way. |
| `server.deployment.enableReadinessProbe`, `readinessProbe` | `true`, `{}` | The readiness probe, the same way. |
| `server.jwtKeypairSecret.name` | `""` | The Secret of the signing key; default `<release>-uc-server-server-jwt-key`. |
| `server.jwtKeypairSecret.create` | `true` | Make the Secret at install when it does not exist. |
| `server.logLevel` | `info` | `RUST_LOG`. |
| `auth.enabled` | `false` | Sign-in and privilege checks. |
| `auth.users` | `[]` | Users that the server makes at start when they are missing: `email`, `name`, `externalId`, `privileges` (on the metastore). |
| `auth.allowedIssuers` | `https://accounts.google.com` | The issuers of tokens that the server exchanges. |
| `auth.audiences` | `""` | The audiences of those tokens; `*` alone accepts all. |
| `auth.oidc.issuer` | `""` | The OpenID Connect provider of the web UI's sign-in. |
| `auth.oidc.clientSecretName` | `""` | A Secret with `clientId` and `clientSecret`. |
| `auth.oidc.scopes` | `openid email profile` | The scopes that the server asks for. |
| `auth.oidc.endSession` | `false` | "Log out" ends the provider's session too. |
| `auth.publicUrl` | `""` | The server's URL, for the provider's redirects after sign-in and "Log out". |
| `auth.forward.trustedProxies` | `""` | The auth proxies' CIDRs or addresses. |
| `auth.forward.emailHeader` | `Remote-Email` | The header with the user's email. |
| `auth.forward.userHeader` | `Remote-User` | The header with the user, without the email header. |
| `auth.forward.logoutUrl` | `""` | The proxy's sign-out, for the web UI's "Log out". |
| `auth.cookieTimeout` | `5d` | The life of the web UI's sign-in. |
| `auth.accessTokenTimeout` | `24h` | The life of the server's tokens. |
| `storage.tableStorageRoot` | `""` | The root of managed tables without a catalog or schema root. |
| `storage.modelStorageRoot` | `""` | The same for models. |
| `storage.credentials.s3` | `[]` | Per bucket: `bucketPath`, `region`, `awsRoleArn`, `credentialsSecretName`. |
| `storage.credentials.adls` | `[]` | Per account: `storageAccountName`, `credentialsSecretName`. |
| `storage.credentials.gcs` | `[]` | Per bucket: `bucketPath`, `credentialsSecretName`. |
| `ui.enabled` | `true` | Serve the web UI at `/`. |
| `ingress.enabled`, `host`, `ingressClassName`, `annotations`, `tlsSecretName` | off | An Ingress to the server. |
| `httpRoute.enabled`, `host`, `parentRefs`, `annotations` | off | An HTTPRoute to the server; `parentRefs` is required. |
| `serviceAccount.create`, `name`, `annotations`, `automount` | off | The pod's ServiceAccount. |
| `nameOverride`, `fullnameOverride` | `""` | Other names for the chart's objects. |
