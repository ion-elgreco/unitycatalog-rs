# Sign-in and authorization

By default the server has no sign-in: every client can do everything. With `auth.enabled`
(`UC_AUTH_ENABLED=true`), each request needs a token of an active user, and the server checks the
user's privileges, as upstream's server does.

## Without sign-in

With `auth.enabled = false`, the server answers every request and checks no privileges. A
request with a valid token of the server names its caller, for example as the owner of what it
creates. Other requests have no caller.

Even then, two things need a signed-in caller:

- Personal access tokens: a user makes and lists their own.
- `/admin/backup`, `/admin/restore` and `/admin/audit`: an admin, such as the `admin` user of
  [`uc-server token`](#the-admin-user).

## Turn on sign-in and authorization

```toml
[auth]
enabled = true
```

Then:

- Each request to the APIs needs a token of the server, in the header `Authorization: Bearer
  <token>` or in the `UC_TOKEN` cookie. The requests that sign in need none.
- The token must name a user that exists and is active.
- The server checks the privileges of the user on each request. A denial is HTTP 403.

Paths outside `/api/2.1/unity-catalog` and `/api/1.0/unity-control` need no token: the web UI,
its sign-in, the API docs, `/healthz` and `/version`.

## The signing key

The server signs its tokens with an RSA key. For a server that keeps its sign-ins after a restart,
and for replicas, give it the key in a PEM file:

```toml
[auth]
signing_key_file = "/etc/uc/signing-key.pem"
```

The server only reads the file. `uc-server keygen` prints a new key, and `--output <file>` writes
it to a new file that only its owner reads:

```bash
(umask 077 && uc-server keygen > signing-key.pem)
```

Without `auth.signing_key_file`, the server makes a new key in memory at each start, as for a
server on your own machine. Its tokens and the web UI's sign-ins then end when it stops, and
replicas refuse each other's tokens. With `auth.enabled`, the server logs a warning.

The key signs every token: keep it secret. A new key ends the tokens of the old one. Personal
access tokens stay valid: the server keeps them, it does not sign them.

## The first users

Name the first users in `uc.toml`. At each start, the server makes the users that are missing, and
grants their missing privileges on the metastore:

```toml
[auth]
enabled = true
admin_principals = ["alice@example.com"]

[[auth.users]]
email = "alice@example.com"
name = "Alice"

[[auth.users]]
email = "bob@example.com"
name = "Bob"
privileges = ["CREATE CATALOG"]
```

- `email` names the user in grants and owners, and `name` is the display name.
- `external_id` is the id of an OAuth client, for a machine client that signs in with its own
  access tokens (see [Token exchange](#token-exchange)).
- `privileges` are privileges on the metastore.
- The users of `auth.admin_principals` are admins. The server makes them too, when
  `[[auth.users]]` does not have them.

The server never changes or deletes a user that exists, and gives a disabled user no privileges. A
privilege of the list that someone revokes comes back at the next start: remove it from the list
too. An entry with the `external_id` of another user stops the start. `[[auth.users]]` is only in
`uc.toml`: no variable sets it.

With an identity provider, the first admin then signs in to the web UI or with `uc auth login`, and
adds other users.

## The admin user

When the server starts with `auth.enabled`, it makes the user `admin` if it is not there, and
makes it the owner of the metastore if the metastore has no owner. Only the tokens of
`uc-server token` name `admin`: no identity provider or auth proxy signs it in. Use it when no
other admin can sign in, such as before the first users exist:

```bash
export UC_CLI_TOKEN=$(uc-server token)
uc user create --name Alice --email alice@example.com
```

`uc-server token` signs a token of `admin` with the key of `auth.signing_key_file`. The token
expires after one hour; `--ttl` sets another time, at most `auth.access_token_timeout`. Run it
with the settings of the server, where the server runs, such as with `docker exec` or
`kubectl exec`. Without `auth.signing_key_file`, the key exists only in the memory of the running
server, so `uc-server token` cannot sign a token.

## Users and groups

Users are SCIM 2.0 users: `uc user`, the Python client, or `/api/1.0/unity-control/scim2/Users`.
A user's email is their name in grants and owners.

```bash
uc user create --name Alice --email alice@example.com
uc user list --filter 'userName eq "alice@example.com"'
```

When you delete a user, the server removes the user's grants and disables the user: the user
stays, and its email cannot be used again. Groups (`uc group`) are SCIM groups too; privileges go
to users, not to groups.

The owner of the metastore and the users of `auth.admin_principals` (their emails) are admins:
see [Privileges](#privileges).

## Tokens

The server accepts only its own tokens: tokens that it signed, and personal access tokens. A
client gets one in one of these ways.

### Token exchange

The client exchanges a token of your identity provider (an ID token or an access token) for an
access token of the server (RFC 8693). `uc auth login` does this. Set the providers and the
audiences that the server accepts:

```toml
[auth]
enabled = true
allowed_issuers = ["https://accounts.google.com"]
audiences = ["<client id>"]
```

An issuer is exact, or has `*` for one DNS label. `audiences = ["*"]` accepts every audience.
The server finds the provider's keys with OpenID Connect discovery. The token must name an active
user: its `email` claim, else its `sub` claim, is the user's email. An `email` that the provider
marks as not verified (`email_verified` is false) names no user. An access token of a machine
client can also name the user whose `externalId` is the client's id (`azp` or `client_id`): make
that user with `uc user create --name etl --email etl@example.com --external_id <client id>`.

```bash
curl -X POST http://localhost:8080/api/1.0/unity-control/auth/tokens \
  -d grant_type=urn:ietf:params:oauth:grant-type:token-exchange \
  -d requested_token_type=urn:ietf:params:oauth:token-type:access_token \
  -d subject_token_type=urn:ietf:params:oauth:token-type:id_token \
  -d subject_token="$ID_TOKEN"
```

The answer has `access_token`, which is valid for `auth.access_token_timeout` (default `24h`).
With `?ext=cookie`, the server also sets it as the `UC_TOKEN` cookie.

### Personal access tokens

A signed-in user makes tokens for scripts and services:

```bash
uc token create --name ci
uc token create --name nightly --expires_at 1798761600000
uc token list
uc token delete --id <id>
```

`uc token create` prints the token once. `--expires_at` is the time when the token expires, in
epoch milliseconds. A token without it does not expire. The user or an admin can delete it.

### OAuth client credentials

Clients that use the OAuth client credentials flow give a personal access token as the client
secret, and its id as the client id:

```bash
curl -X POST http://localhost:8080/api/2.1/unity-catalog/oauth/token \
  -u '<token id>:<token>' -d grant_type=client_credentials
```

The answer's `access_token` is the personal access token.

## Sign in to the web UI

The server signs users in to the web UI. The browser holds no settings of the identity provider.
`auth.mode` selects how:

| `auth.mode` | How users sign in |
| --- | --- |
| `none` (the default) | No one signs in to the web UI. With authorization on, its login page says so. |
| `oidc` | With an OpenID Connect provider. |
| `forward` | Through an auth proxy in front of the server. |

With `oidc` and `forward`, a user signs in when their email names an active user of the server.
Then the server sets the `UC_TOKEN` cookie, valid for `auth.cookie_timeout` (default `5d`). "Log
out" in the web UI goes to `/auth/logout`.

### OpenID Connect

```toml
[auth]
enabled = true
mode = "oidc"
public_url = "https://uc.example.com"
oidc_issuer = "https://accounts.google.com"
oidc_client_id = "<client id>"
oidc_client_secret = "<client secret>"   # none for a public client
```

1. In the provider, make an OAuth client, and allow the redirect URI
   `<public_url>/auth/callback`, such as `https://uc.example.com/auth/callback`.
2. The login page's "Sign in" button goes to `/auth/login`. The server sends the browser to the
   provider, with PKCE.
3. The provider sends the browser back to `/auth/callback`. The server checks the ID token, finds
   the user by the token's email, sets the `UC_TOKEN` cookie and returns to the page.
4. "Log out" deletes the cookie. The provider's session stays, so the next sign-in may need no
   password.

With `oidc_end_session = true`, "Log out" ends the provider's session too, and the next sign-in
asks for the password again. The server sends the browser to the `end_session_endpoint` of the
provider's discovery document, with `client_id` and `post_logout_redirect_uri`. The provider then
returns the browser to `<public_url>/`: allow that post-logout redirect URI in the provider, such
as `https://uc.example.com/`. A provider without that endpoint, such as Google, keeps its session.
The setting is off by default: when other apps share the provider's sign-in, such as a company's
Okta or Entra ID, ending its session signs the user out of those apps too.

`oidc_scopes` sets the scopes (default `openid email profile`). With an `http://` public URL, the
cookies are not `Secure`, as browsers drop secure cookies on plain HTTP: use it only on your own
machine.

When the sign-in fails, the server logs why and sends the browser to `/login?error=<code>`. The
web UI shows "Login failed" and the code: `provider`, `expired`, `token`, `user` (no active user
has the email) or `server`.

### An auth proxy

An auth proxy, such as oauth2-proxy or Authelia, signs users in and names the user in its headers
on each request:

```toml
[auth]
enabled = true
mode = "forward"
forward_trusted_proxies = ["10.42.0.0/16"]
forward_email_header = "Remote-Email"    # the default
forward_user_header = "Remote-User"      # the default
forward_logout_url = "https://auth.example.com/logout"
```

The server reads the user's email from `forward_email_header`, else the user from
`forward_user_header`. It takes the headers only from the addresses of `forward_trusted_proxies`:
the address of the connection, never `X-Forwarded-For`. The proxy must replace these headers on
the requests that it sends on. Other requests, such as API clients that call the server directly,
sign in with tokens. A proxy user named `admin` does not sign in: that is the server's own user.
Without `forward_logout_url`, the web UI shows no "Log out": the proxy would sign the user in
again.

## Privileges

The server checks privileges as upstream's server does.

- The user who makes a securable owns it. The owner can do everything with it, and grant
  privileges on it. Ownership is not inherited.
- A privilege that is granted on a catalog or schema counts for what it holds: `SELECT` on a
  schema lets the user read all its tables.
- Most actions need privileges on the parents too. To read a table, a user needs `USE CATALOG` on
  its catalog, `USE SCHEMA` on its schema, and `SELECT` on the table.
- Admins, the owner of the metastore and the users of `auth.admin_principals`, own the metastore.
  They read every securable, make catalogs, storage credentials and external locations, delete
  catalogs, manage users, and grant any privilege on any securable. For other actions, such as a
  new comment on a catalog of another user, they need the grant or the ownership, as other users.
- Lists show only what the user can read.

The securables are the `metastore`, `catalog`, `schema`, `table`, `volume`, `function`,
`registered_model`, `external_location` and `credential`.

| Privilege | Lets the user |
| --- | --- |
| `CREATE CATALOG` | Make catalogs (on the metastore). |
| `USE CATALOG` | Use a catalog and what it holds. |
| `CREATE SCHEMA` | Make schemas in a catalog. |
| `USE SCHEMA` | Use a schema and what it holds. |
| `CREATE TABLE` | Make tables in a schema. |
| `SELECT` | Read a table. |
| `MODIFY` | Change the data of a table. |
| `CREATE FUNCTION` | Make functions in a schema. |
| `EXECUTE` | Run a function; read a registered model and its versions. |
| `CREATE VOLUME` | Make volumes in a schema. |
| `READ VOLUME` | Read a volume and its files. |
| `CREATE MODEL` | Make registered models in a schema. |
| `CREATE EXTERNAL LOCATION` | Make external locations (on the metastore, and on the storage credential that they use). |
| `READ FILES`, `WRITE FILES` | Read and write files under an external location. |
| `CREATE EXTERNAL TABLE`, `CREATE EXTERNAL VOLUME` | Make tables and volumes under an external location. |
| `CREATE MANAGED STORAGE` | Use a path under an external location as the storage root of a catalog or schema. |
| `CREATE STORAGE CREDENTIAL` | Make storage credentials (on the metastore). |

Grant and revoke one privilege at a time with `uc`, or several with the API or the Python
client:

```bash
uc permission create --securable_type catalog --name main \
  --principal bob@example.com --privilege "USE CATALOG"
uc permission create --securable_type schema --name main.default \
  --principal bob@example.com --privilege "USE SCHEMA"
uc permission create --securable_type table --name main.default.events \
  --principal bob@example.com --privilege SELECT
uc permission get --securable_type table --name main.default.events
uc permission delete --securable_type table --name main.default.events \
  --principal bob@example.com --privilege SELECT
```

Grants stay with a securable when it gets a new name, and go when it is deleted.

## Cedar policies

With `auth.cedar = true`, the [Cedar](https://www.cedarpolicy.com) policy engine decides the
checks of granted privileges. Its default policy allows what the grants allow, as without Cedar.
`auth.cedar_policies` adds your own policy files. Owners and admins do not go through Cedar.

In the policies, each privilege is an action, such as `Action::"SELECT"` or
`Action::"USE CATALOG"`, and the user is `User::"<email>"`. Each securable is an entity of its
type with its id, such as `Table::"<id>"`, with its schema, catalog and metastore as its parents:
a policy can say `resource in Catalog::"<id>"`. A securable has the attributes
`granted_privileges` (what is granted on it or a parent) and `owner`.

```cedar
// Grants of MODIFY do not count in this catalog:
// only owners and admins change its tables.
forbid (
  principal,
  action == Action::"MODIFY",
  resource in Catalog::"<catalog id>"
);
```
