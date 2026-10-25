# Kubernetes

The Helm chart `oci://ghcr.io/ion-elgreco/charts/uc-server` runs the server and the web UI, with
the catalog in PostgreSQL. It needs Helm 3.8 or later.

```yaml
# values.yaml
server:
  db:
    type: postgresql
    postgresqlConfig:
      host: postgres.example.com
      dbName: ucdb
      username: uc
      passwordSecretName: uc-db              # a Secret with the key `password`
      ssl:
        enabled: true
        rootCertConfigMapName: postgres-ca   # a ConfigMap with ca.crt
  deployment:
    replicaCount: 2
```

```sh
kubectl create namespace uc
kubectl -n uc create secret generic uc-db --from-literal=password='<password>'
helm install uc oci://ghcr.io/ion-elgreco/charts/uc-server \
  --version 0.0.0-dev --namespace uc -f values.yaml
kubectl -n uc port-forward svc/uc-uc-server-server 8080:8080
```

Open <http://localhost:8080> for the web UI. The sections below add values for each task;
[Helm chart values](../reference/helm-values.md) lists them all.

## Sign-in

```yaml
auth:
  enabled: true
  users:                                   # made at each start when missing
    - email: alice@example.com
      name: Alice
  publicUrl: https://uc.example.com
  oidc:                                    # the web UI's sign-in
    issuer: https://accounts.google.com
    clientSecretName: uc-oauth             # a Secret with clientId and clientSecret
  allowedIssuers: https://accounts.google.com   # for `uc auth login`
  audiences: <client id>
server:
  deployment:
    env:
      UC_AUTH_ADMIN_PRINCIPALS: alice@example.com
```

- In the identity provider, allow the redirect URI `https://uc.example.com/auth/callback`.
- For an auth proxy instead of OpenID Connect, set `auth.forward`.
- At install, the chart makes a Secret with the key that signs the server's tokens, and keeps it.
- Before an admin can sign in, get a token of the server's `admin` user:
  `kubectl -n uc exec deploy/uc-uc-server-server -- uc-server token`.

[Sign-in and authorization](auth.md) tells how this works.

## Cloud storage

```yaml
storage:
  tableStorageRoot: s3://analytics/managed
  locations:
    - url: s3://analytics
      region: us-east-1
      awsIamRoleArn: arn:aws:iam::123456789012:role/uc
```

The server's identity assumes the role. Without `storage.identity.aws.credentialsSecretName`, it
comes from the environment, such as the role of the pod's service account
(`serviceAccount.create` and `annotations`). See [Cloud storage](cloud-storage.md).

## Expose the server

```yaml
ingress:
  enabled: true
  host: uc.example.com
  ingressClassName: nginx
  tlsSecretName: uc-tls
```

Or `httpRoute`, for the Gateway API.

## Other settings

For a setting without a value in the chart, use `server.deployment.env`, or a `uc.toml` in
`server.config.toml`:

```yaml
server:
  deployment:
    env:
      UC_TABLES_ICEBERG_WRITES: "false"
```

## Operations

- The probes and the metrics are on the port `obs` (8081). See [Operations](operations.md).
- The pod runs as a non-root user, with a read-only root filesystem and no capabilities.
