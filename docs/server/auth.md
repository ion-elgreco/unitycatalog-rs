# Sign-in and authorization

By default, the server has no sign-in, and every client can do everything. Turn sign-in on:

```toml
[auth]
enabled = true
signing_key_file = "/etc/uc/signing-key.pem"
admin_principals = ["alice@example.com"]

[[auth.users]]
email = "alice@example.com"
name = "Alice"
```

Then each request to the APIs needs a token of an active user, in the header
`Authorization: Bearer <token>` or in the `UC_TOKEN` cookie, and the server checks the user's
privileges, as upstream's server does. The web UI, its sign-in, `/docs`, `/healthz` and
`/version` need no token.

## The signing key

The server signs its tokens with an RSA key. `uc-server keygen` makes one:

```bash
(umask 077 && uc-server keygen > signing-key.pem)
```

Without `signing_key_file`, the server makes a new key in memory at each start: its tokens end
when it stops, and replicas refuse each other's tokens. Keep the key secret.

### A new signing key

The file can hold several keys, newest first: the first key signs, and each key checks the tokens
that it signed. Personal access tokens do not use the key. To change the key and keep the
sign-ins:

1. Put a new key first in the file, and restart the server:

    ```bash
    (umask 077 && { uc-server keygen; cat signing-key.pem; } > new.pem) && mv new.pem signing-key.pem
    ```

2. After `access_token_timeout` (default `24h`), the tokens of the old key have expired. Remove
   the old key from the file, and restart the server again.

A server refuses the tokens of a key that is not in its file. Thus, give servers that share
sign-ins the new key in two steps: first put it last in the file and restart each server, then
move it first and restart each server again. To end all sign-ins at once, replace the file with
only a new key.

## The first users

At each start, the server makes the users of `[[auth.users]]` that are missing, with their
`privileges` on the metastore. It never changes or deletes a user that exists. The users of
`admin_principals` are admins: the server makes them too.

With an identity provider, the first admin then signs in and adds other users. Before that, or
without a provider, `uc-server token` prints a token of the server's own `admin` user, valid for
one hour. Run it where the server runs, with its settings and its `signing_key_file`:

```bash
export UC_CLI_TOKEN=$(uc-server token)
uc user create --name Bob --email bob@example.com
```

## Users and groups

Users are SCIM users: `uc user`, the Python client, or `/api/1.0/unity-control/scim2/Users`. A
user's email is their name in grants and owners. To delete a user, the server disables the user
and removes their grants.

Groups (`uc group`) are SCIM groups. A privilege granted to a group counts for each of its
members: the group's name is its name in grants, and cannot be the email of a user.

## Tokens

The server accepts only its own tokens. A client gets one in one of these ways:

- **Token exchange.** The client exchanges a token of your identity provider for a token of the
  server, as `uc auth login` does. Name the providers and audiences that the server accepts:

    ```toml
    [auth]
    allowed_issuers = ["https://accounts.google.com"]
    audiences = ["<client id>"]
    ```

    The token's `email` claim, else its `sub`, names the user. The token of a machine client
    names the user whose `external_id` is the client's id.

- **Personal access tokens.** A signed-in user makes them for scripts: `uc token create --name ci`.
  They do not expire, unless `--expires_at` says when.

- **OAuth client credentials.** A client gives a personal access token as its client secret, with
  the token's id as the client id, to `/api/2.1/unity-catalog/oauth/token`.

## Sign in to the web UI

`auth.mode` selects how users sign in to the web UI: `none` (the default), `oidc` or `forward`.
A user signs in when their email names an active user of the server.

### OpenID Connect

```toml
[auth]
mode = "oidc"
public_url = "https://uc.example.com"
oidc_issuer = "https://accounts.google.com"
oidc_client_id = "<client id>"
oidc_client_secret = "<client secret>"   # none for a public client
```

In the provider, allow the redirect URI `https://uc.example.com/auth/callback`. "Log out" ends
the sign-in to the web UI only; with `oidc_end_session = true`, it also ends the provider's
session.

### An auth proxy

An auth proxy, such as oauth2-proxy, Authelia or [Envoy Gateway](#envoy-gateway), signs users in
and names the user in a header:

```toml
[auth]
mode = "forward"
forward_trusted_proxies = ["10.42.0.0/16"]
forward_logout_url = "https://auth.example.com/logout"
```

The server reads the email from `Remote-Email`, else the user from `Remote-User`, and only on
connections from `forward_trusted_proxies`.

#### Envoy Gateway

[Envoy Gateway](https://gateway.envoyproxy.io) 1.2 or later can be the auth proxy. A
SecurityPolicy signs users in with OpenID Connect. A JWT provider in the same policy reads the ID
token from its cookie, and puts the token's `email` in `Remote-Email`:

```yaml
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: SecurityPolicy
metadata:
  name: uc-web-ui
spec:
  targetRefs:
    - group: gateway.networking.k8s.io
      kind: HTTPRoute
      name: uc-web-ui              # the route of the web UI's host only
  oidc:
    provider:
      issuer: https://accounts.google.com
    clientID: <client id>
    clientSecret:
      name: uc-oidc                # a Secret with the key client-secret
    redirectURL: https://uc.example.com/oauth2/callback
    logoutPath: /logout
    cookieNames:
      idToken: uc-id-token
  jwt:
    providers:
      - name: id-token
        issuer: https://accounts.google.com
        audiences: ["<client id>"]
        remoteJWKS:
          uri: https://www.googleapis.com/oauth2/v3/certs
        extractFrom:
          cookies: [uc-id-token]
        claimToHeaders:
          - header: Remote-Email
            claim: email
---
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: ClientTrafficPolicy
metadata:
  name: uc
spec:
  targetRefs:
    - group: gateway.networking.k8s.io
      kind: Gateway
      name: <gateway>
  headers:
    earlyRequestHeaders:
      remove: [Remote-Email, Remote-User]
```

```toml
[auth]
mode = "forward"
forward_trusted_proxies = ["10.42.0.0/16"]           # the addresses of the Envoy pods
forward_logout_url = "https://uc.example.com/logout"  # the policy's logoutPath
```

- Envoy adds `Remote-Email`, and does not replace a header of that name that the client sends.
  The ClientTrafficPolicy removes the client's `Remote-Email` and `Remote-User` on every route
  of the Gateway first. Without it, a client could name any user.
- Give the SecurityPolicy only the route of the web UI, for example on a host of its own. `uc`,
  the Python client and other API clients send the server's tokens, and OpenID Connect would
  send their requests to the provider's sign-in page.

## Privileges

- The user who makes a securable owns it, and can grant privileges on it.
- A privilege on a catalog or schema counts for what it holds.
- To read a table, a user needs `USE CATALOG` on its catalog, `USE SCHEMA` on its schema, and
  `SELECT` on the table.
- Admins read everything, make catalogs, credentials and external locations, manage users, and
  grant any privilege.
- External tables and volumes, and the credentials of a path, need an external location that
  covers the path, also for admins.

| Privilege | Lets the user |
| --- | --- |
| `CREATE CATALOG` | Make catalogs (on the metastore). |
| `USE CATALOG`, `USE SCHEMA` | Use a catalog or schema, and what it holds. |
| `CREATE SCHEMA`, `CREATE TABLE`, `CREATE VOLUME`, `CREATE FUNCTION`, `CREATE MODEL` | Make these in a catalog or schema. |
| `SELECT`, `MODIFY` | Read a table, change its data. |
| `READ VOLUME` | Read a volume and its files. |
| `EXECUTE` | Run a function; read a registered model. |
| `CREATE STORAGE CREDENTIAL`, `CREATE EXTERNAL LOCATION` | Make storage credentials and external locations. |
| `READ FILES`, `WRITE FILES` | Read and write files under an external location. |
| `CREATE EXTERNAL TABLE`, `CREATE EXTERNAL VOLUME` | Make tables and volumes under an external location. |
| `CREATE MANAGED STORAGE` | Use a path under an external location as a storage root. |

```bash
uc permission create --securable_type table --name main.default.events \
  --principal bob@example.com --privilege SELECT
uc permission get --securable_type table --name main.default.events
```

## Cedar policies

With `auth.cedar = true`, the [Cedar](https://www.cedarpolicy.com) policy engine checks the
granted privileges. Its default policy allows what the grants allow; `auth.cedar_policies` adds
your own policy files:

```cedar
// Only owners and admins change the tables of this catalog.
forbid (
  principal,
  action == Action::"MODIFY",
  resource in Catalog::"<catalog id>"
);
```

For each check, the server asks Cedar whether a principal may do an action on a resource.

| Part | Cedar | Example |
| --- | --- | --- |
| Principal | `User::"<email>"`: the signed-in user | `User::"alice@example.com"` |
| Action | `Action::"<privilege>"`, as a grant writes it, with its spaces | `Action::"USE CATALOG"` |
| Resource | `<type>::"<id>"`: the securable that the check is on, by its id: `USE CATALOG` on the catalog, `SELECT` on the table | `Catalog::"6f1e…"` |

The actions are the privileges: `CREATE CATALOG`, `USE CATALOG`, `CREATE SCHEMA`, `USE SCHEMA`,
`CREATE TABLE`, `SELECT`, `MODIFY`, `CREATE FUNCTION`, `EXECUTE`, `CREATE VOLUME`, `READ VOLUME`,
`CREATE MODEL`, `CREATE EXTERNAL LOCATION`, `READ FILES`, `WRITE FILES`, `CREATE EXTERNAL TABLE`,
`CREATE EXTERNAL VOLUME`, `CREATE MANAGED STORAGE` and `CREATE STORAGE CREDENTIAL`.

The resource types, and where their ids are:

| Resource | Id | Shown by |
| --- | --- | --- |
| `Metastore::"<id>"` | `metastore_id` | `uc metastore get` |
| `Catalog::"<id>"` | `id` | `uc catalog get` |
| `Schema::"<id>"` | `schema_id` | `uc schema get` |
| `Table::"<id>"` | `table_id` | `uc table get` |
| `Volume::"<id>"` | `volume_id` | `uc volume get` |
| `Function::"<id>"` | `function_id` | `uc function get` |
| `RegisteredModel::"<id>"` | `id` | `uc registered_model get` |
| `ExternalLocation::"<id>"` | `id` | `uc external_location get` |
| `Credential::"<id>"` | `id` | `uc credential get` |

- **Parents:** a table, volume, function or model is `in` its schema, a schema `in` its catalog,
  and a catalog `in` the metastore. So `resource in Catalog::"<id>"` matches all that the catalog
  holds. External locations and credentials have no parent.
- **Attributes of a resource:** `owner`, the owner's email, when it has one; and
  `granted_privileges`, the set of actions granted on it or on a parent, to the user or to a group
  of the user.
- **Groups** are no Cedar entities: a policy cannot say `principal in Group::"…"`. Their grants are
  in `granted_privileges`.
- **Owners and admins:** Cedar decides only whether a user holds a privilege. The checks for an
  owner or an admin do not ask Cedar, so a `forbid` does not stop them.
- An id stays the same when the securable is renamed. A securable that is dropped and made again
  gets a new id.
