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
when it stops, and replicas refuse each other's tokens. A new key ends the tokens of the old one,
but not personal access tokens. Keep the key secret.

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

A privilege is an action, such as `Action::"SELECT"`, the user is `User::"<email>"`, and a
securable is an entity such as `Table::"<id>"`, in its schema, catalog and metastore. A securable
has the attributes `owner` and `granted_privileges`: what is granted on it or a parent, to the
user or to a group of the user.
