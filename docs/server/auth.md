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
user's email is their name in grants and owners; a user without an email, such as an
application that an identity provider names ([Who a token names](#who-a-token-names)), has its
`userName` there. To delete a user, the server disables the user and removes their grants.

Groups (`uc group`) are SCIM groups. A privilege granted to a group counts for each of its
members: the group's name is its name in grants, and cannot be the email of a user.

## Tokens

A request signs in with one of these tokens:

- **A token of your identity provider.** Name the provider as an `[[auth.issuers]]` block
  ([Identity providers](#identity-providers)). With `direct = true`, the server takes its tokens
  on every request: a service principal, a managed identity, a service account or a CI job sends
  the token that its environment gives it, and engines such as pyiceberg sign in with their own
  `entra` or `google` auth.

    ```toml
    [[auth.issuers]]
    name = "entra"
    issuer = "https://login.microsoftonline.com/<tenant>/v2.0"
    audiences = ["api://uc"]
    direct = true
    principal_claims = ["preferred_username", "oid"]
    client_claims = ["azp", "appid"]
    groups_claim = "roles"
    create_users = true
    display_name = "{app_displayname}"
    ```

- **Token exchange.** The client exchanges a token of the provider for a token of the server, as
  `uc auth login` does. Every `[[auth.issuers]]` block allows this; without `direct = true`, a
  block allows only this.

- **Personal access tokens.** A signed-in user makes them for scripts: `uc token create --name ci`.
  They do not expire, unless `--expires_at` says when.

- **OAuth client credentials.** A client gives a personal access token as its client secret, with
  the token's id as the client id, to `/api/2.1/unity-catalog/oauth/token`.

## Identity providers

Each `[[auth.issuers]]` block is one provider. `GET /.well-known/oauth-protected-resource` lists
the blocks (RFC 9728), so a client can ask the provider for a token with the right audience.

| Key | Meaning | Default |
| --- | --- | --- |
| `name` | A short name: the prefix of the principals that it names without an email. | required |
| `issuer` | The `iss` claim of its tokens; `*` stands for one DNS label. | required |
| `audiences` | The `aud` that its tokens must carry; `["*"]` for any. | required |
| `direct` | Take its tokens on every request, not only at the exchange. | `false` |
| `principal_claims` | The claims that name the caller; the first present wins. | `["email", "sub"]` |
| `client_claims` | The claims with a machine's client id, matched to a user's `externalId`. | `["azp", "client_id"]` |
| `groups_claim` | A claim whose values are names of groups the caller is in. | none |
| `create_users` | Make the user at its first request when it is missing. | `false` |
| `display_name` | The display name of a made user, with claims in braces. | the principal |

### Who a token names

1. The first of `principal_claims` that the token has. A value with `@` is an email and names
   the user with that email, unless the provider says `email_verified = false`. Any other value
   names the user `<name>:<value>`, such as `entra:7f3e…`.
2. Nobody found, and the token is an application's own (an access token with `idtyp = app`, or
   without an email-like claim): the first of `client_claims` whose value is a user's
   `externalId`. This is how a user made ahead of time, with `uc user create --external_id` or
   `[[auth.users]]`, is the service principal. A person's token from `az` or `gcloud` carries
   that tool's client id, so a person is never matched by it.
3. Nobody found: with `create_users`, the server makes the user, named as in step 1, with the
   client id of an application as `externalId` and `display_name` filled from the token. Else
   the request is refused with "User not allowed".

A user that is disabled names nobody, and the server's own `admin` can never be assumed. The web
UI's sign-in (`oidc_issuer`) names its users the same way, with the block of that issuer when
there is one, so a first sign-in makes the user when the block has `create_users`.

Grants, owners, `admin_principals` and Cedar's `User::"…"` take the principal: the email, or
`<name>:<value>`.

### Groups

With `groups_claim`, the names in that claim are groups of the caller for the request, next to
the groups the caller is a member of in the server. A name counts when a group of that name
exists in the server: make the group once, grant to it, and every identity whose token names it
is covered. Entra's `groups` claim holds object ids; its app roles (`roles`), assigned to Entra
groups, arrive as names.

### Providers

| Provider | Block | Notes |
| --- | --- | --- |
| Microsoft Entra | `issuer = "https://login.microsoftonline.com/<tenant>/v2.0"`, `audiences = ["api://<app id uri>"]` or the API app's client id, `principal_claims = ["preferred_username", "oid"]`, `client_claims = ["azp", "appid"]`, `groups_claim = "roles"` | Set `accessTokenAcceptedVersion` to 2 on the API app, or add a block for `https://sts.windows.net/<tenant>/` with `principal_claims = ["upn", "oid"]` and `client_claims = ["appid"]`. Add the `idtyp` optional claim so applications are told from people. A client asks for the scope `api://<app id uri>/.default`. |
| Google | `issuer = "https://accounts.google.com"`, `audiences = ["<your audience>"]`, `principal_claims = ["email", "sub"]` | Google's access tokens are opaque: clients send an ID token with your audience. |
| Any OpenID Connect provider | `issuer`, `audiences = ["<client id>"]` | FerrisKey, Keycloak, Okta, GitHub Actions, GitLab CI, a Kubernetes cluster's issuer. The provider must publish `<issuer>/.well-known/openid-configuration` and its keys. |

The server checks a token's signature with the provider's keys, which it fetches once an hour,
its issuer, audience and times, with 60 seconds of clock skew. A token that a `direct` issuer's
block accepts is kept until it expires, at most five minutes, so a busy client costs one check
per token, not per request; a user disabled in the server keeps working that long.

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
