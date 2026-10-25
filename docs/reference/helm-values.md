# Helm chart values

The values of the Helm chart `oci://ghcr.io/ion-elgreco/charts/uc-server`.
[Kubernetes](../server/kubernetes.md) shows them per task. The chart's `values.yaml` has a comment
for each value:

```sh
helm show values oci://ghcr.io/ion-elgreco/charts/uc-server --version 0.0.0-dev
```

The chart sets the server's settings as `UC_*` environment variables. For the
[settings](settings.md) without a value here, use `server.deployment.env` or `server.config.toml`.

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
| `storage.credentials.adls` | `[]` | Per account: `storageAccountName`, and `credentialsSecretName` or `managedIdentity: true` with an optional `managedIdentityId`. |
| `storage.credentials.gcs` | `[]` | Per bucket: `bucketPath`, `credentialsSecretName`. |
| `ui.enabled` | `true` | Serve the web UI at `/`. |
| `ingress.enabled`, `host`, `ingressClassName`, `annotations`, `tlsSecretName` | off | An Ingress to the server. |
| `httpRoute.enabled`, `host`, `parentRefs`, `annotations` | off | An HTTPRoute to the server; `parentRefs` is required. |
| `serviceAccount.create`, `name`, `annotations`, `automount` | off | The pod's ServiceAccount. |
| `nameOverride`, `fullnameOverride` | `""` | Other names for the chart's objects. |
