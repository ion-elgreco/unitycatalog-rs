# Sign-in and authorization

By default, the server has no sign-in, and every client can do everything. To turn sign-in on:

```toml
[auth]
enabled = true
signing_key_file = "/etc/uc/signing-key.pem"
admin_principals = ["alice@example.com"]

[[auth.users]]
email = "alice@example.com"
name = "Alice"
```

Then each API request needs the token of an active user. The client sends it in the
`Authorization: Bearer <token>` header or in the `UC_TOKEN` cookie. The server checks the user's
privileges. The web UI, its sign-in, `/docs`, `/healthz` and `/version` need no token.

To set up sign-in:

1. Make a [signing key](#the-signing-key).
2. Name the [first users](#the-first-users) and admins.
3. Add your [identity providers](#identity-providers).
4. Optional: set up [sign-in to the web UI](#sign-in-to-the-web-ui).
5. Grant [privileges](#privileges).

[Settings](../reference/settings.md#auth) lists every key of `[auth]` and `[[auth.issuers]]`.

## The signing key

The server signs its own tokens with an RSA key. To make a key file:

```bash
(umask 077 && uc-server keygen > signing-key.pem)
```

Set `signing_key_file` to this file, and keep the file secret.

Without `signing_key_file`, the server uses a key that it makes in memory at each start. Use this
only for a local development server or for tests in CI: all sign-ins end when the server stops,
and replicas refuse each other's tokens.

### A new signing key

The file can hold several keys, newest first. The first key signs new tokens. Each key still
checks the tokens that it signed. Personal access tokens do not use the key.

To change the key and keep the sign-ins:

1. Put a new key first in the file, and restart the server:

    ```bash
    (umask 077 && { uc-server keygen; cat signing-key.pem; } > new.pem) && mv new.pem signing-key.pem
    ```

2. Wait for `access_token_timeout` (default `24h`), until the old key's tokens have expired.
   Remove the old key from the file, and restart the server again.

A server refuses the tokens of a key that is not in its file. So with several servers that share
sign-ins, do step 1 in two rounds: put the new key last in the file and restart each server, then
move it first and restart each server again.

To end all sign-ins at once, replace the file with a file that has only a new key.

## The first users

At each start, the server makes the users of `[[auth.users]]` and `admin_principals` that are
missing. It grants the missing `privileges` of `[[auth.users]]` on the metastore. It never changes
or deletes a user that exists. The users of `admin_principals` are admins.

```toml
[[auth.users]]
email = "bob@example.com"
name = "Bob"
privileges = ["CREATE CATALOG"]
```

With an identity provider, the first admin signs in and adds the other users. Before that, or
without a provider, use `uc-server token`. It prints a token of the server's own `admin` user,
valid for one hour. Run it where the server runs, with the server's settings and
`signing_key_file`:

```bash
export UC_CLI_TOKEN=$(uc-server token)
uc user create --name Bob --email bob@example.com
```

## Users and groups

- A user's email is its name in grants and as an owner. A user without an email, such as an
  application ([Who a token names](#who-a-token-names)), uses its `userName`.
- A privilege granted to a group applies to each member. A group's name is its name in grants,
  and it cannot be the email of a user.
- When you delete a user, the server disables the user and removes its grants.
- Only admins can add, change or delete users and groups.

### Manage users by hand

Use `uc`, the Python client, or the "Users" and "Groups" pages of the web UI:

```bash
uc user create --name Bob --email bob@example.com
uc group create --name analysts
```

Add members to a group on its page in the web UI. To let users in without adding them first, set
`create_users = true` on the provider's block: the server then makes each user at their first
sign-in.

### Sync users from your identity provider (SCIM)

SCIM is the standard API that identity providers use to copy their users and groups to other
services. The server has the SCIM 2.0 API, so a provider such as Entra ID or Okta can make,
change and disable its users and groups in the server. In the provider's provisioning settings,
enter:

- **URL:** `https://uc.example.com/api/1.0/unity-control/scim2`
- **Token:** a personal access token of an admin.

## Tokens

A client signs in with one of these tokens:

- **A provider's token, on each request.** The client sends the token of your identity provider
  with each request, and the server checks it each time. The provider's block needs
  `direct = true`. Use this for service principals, managed identities, service accounts, CI jobs
  and engines such as pyiceberg. The [Python client](../clients/python.md#connect) and
  [`uc`](../clients/cli.md#sign-in) find this identity in the environment themselves.
- **A server token, from the exchange.** The client sends a token of your identity provider to
  `/api/1.0/unity-control/auth/tokens` and gets a token of the server back. Then it sends the
  server's token with each request. `uc auth login` signs in this way. A block with
  `direct = true` accepts its tokens both ways. A block without it accepts them only at the
  exchange.
- **A personal access token.** A signed-in user makes one for scripts:
  `uc token create --name ci`. It does not expire, unless you set `--expires_at`.
- **OAuth client credentials.** The client sends a personal access token as its client secret,
  and the token's id as its client id, to `/api/2.1/unity-catalog/oauth/token`.

## Identity providers

An identity provider signs your users and applications in, such as Microsoft Entra, Google or
Keycloak. Add one `[[auth.issuers]]` block for each provider:

```toml
[[auth.issuers]]
name = "entra"
issuer = "https://login.microsoftonline.com/<tenant>/v2.0"
audiences = ["<the API app's client id>", "https://uc.example.com"]
direct = true
principal_claims = ["preferred_username", "oid"]
client_claims = ["azp", "appid"]
groups_claim = "roles"
create_users = true
display_name = "{app_displayname}"
```

A token is a set of signed fields, called claims. The block tells the server which tokens to
accept, and how to read their claims:

| Key | What it does |
| --- | --- |
| `name` | A short name for the provider. It is the prefix of the users that it names without an email, such as `entra:7f3e…`. |
| `kind` | `oidc` (default) for an OpenID Connect provider, or `aws` for [AWS](#aws). |
| `issuer` | The provider's `iss` claim, exactly as its tokens write it. `*` matches one part of a host name, such as `https://*.dev.example.com`. |
| `audiences` | The `aud` values to accept. Use the server's URL, and any other value that the provider puts in `aud`. `["*"]` accepts every value, but not with `direct`. |
| `direct` | Accept the tokens on each request, not only at the exchange. |
| `principal_claims` | The claims that name the user. The first claim that the token has wins. |
| `client_claims` | The claims that hold an application's client id. |
| `groups_claim` | The claim that lists the user's groups ([Groups](#groups)). |
| `create_users` | Make a missing user at its first request. |
| `display_name` | The display name of a new user, with claims in braces. |

Clients read the blocks at `GET /.well-known/oauth-protected-resource` (RFC 9728), so they know
which audience to ask the provider for. The Python client and `uc` use the audience that
`UC_AUDIENCE` names, else the audience that is the server's URL as they call it. They use no other
audience, so a server cannot make a client's identity sign for another service.

Several blocks can have the same issuer with different audiences. A token goes to the first block
that has its issuer and its audience.

### Who a token names

A principal is the name of a user in grants, owners, `admin_principals` and Cedar's `User::"…"`:
the user's email, or `<name>:<value>`. The server finds the user of a token in three steps:

1. **By a name claim.** The server reads the first claim of `principal_claims` that the token has.
    - An email, such as `alice@example.com`, names the user with that email, in any case. It
      names nobody when the token has `email_verified = false`.
    - With an `issuer` with `*`, an email names a user only when the token has
      `email_verified = true`. Anyone can make an issuer that `*` matches, such as a new Entra
      tenant, and give its users any email. Entra does not send `email_verified`, so name its
      users by a claim that is not an email, such as `oid`.
    - Any other value names the user `<name>:<value>`, such as `entra:7f3e…`.
    - Use only claims that the provider sets. Some providers let users set a claim, such as
      `preferred_username`. Then a user can set it to the email of another user.
2. **As an application.** When step 1 finds no user and the token is an application's own, the
   server looks for the user whose `externalId` is `<name>:<client id>`, such as
   `entra:<client id>`. The client id comes from `client_claims`. An application's own token is an
   Entra access token with `idtyp = app`, a token whose `sub` is its client id (RFC 9068), or an
   AWS identity. To set up a service principal ahead of time, make its user with
   `uc user create --external_id` or `external_id` in `[[auth.users]]`. A person's token never
   matches here: it holds the client id of a shared tool, such as `az`.
3. **As a new user.** When no user is found and the block has `create_users = true`, the server
   makes the user. It names the user as in step 1, sets `externalId` for an application, and
   fills in `display_name`. Else the server refuses the request with "User not allowed".

A disabled user names nobody. No token can name the server's own `admin` user.

The web UI's sign-in (`oidc_issuer`) finds users in the same way, with the block of that issuer
when there is one. So with `create_users`, the first sign-in makes the user.

### Groups

To take the user's groups from the provider's tokens:

1. Set `groups_claim` to the claim that lists the groups.
2. Make each group in the server, named `<name>:<value>`:
   `uc group create --name entra:Writers`.
3. Grant privileges to the group.

A user whose token lists `Writers` then gets the privileges of `entra:Writers`, in addition to
the user's groups in the server. Values without a group in the server do not count. The prefix
keeps each provider to its own groups.

- **Entra:** use `roles`. Entra's `groups` claim holds object ids, but app roles that you assign
  to Entra groups arrive as names.
- **An `issuer` with `*`:** `groups_claim` is not allowed, because each issuer that it matches,
  such as each tenant, would name the same groups.

### Providers

- **Microsoft Entra.** Use the block above. On the API app:
    - Set `accessTokenAcceptedVersion` to 2. The `aud` of a v2 access token is the API app's
      client id. For v1 tokens instead, add a block with
      `issuer = "https://sts.windows.net/<tenant>/"`, `principal_claims = ["upn", "oid"]` and
      `client_claims = ["appid"]`.
    - Set the Application ID URI to the server's URL. Its domain must be verified. Clients then
      ask for the scope `https://uc.example.com/.default`.
    - Add the optional claim `idtyp`, so the server can tell applications from people.
- **Google.** `issuer = "https://accounts.google.com"`, `audiences = ["https://uc.example.com"]`,
  `principal_claims = ["email", "sub"]`. Google's access tokens have no claims that the server
  can read. So clients send an ID token instead, with the server's URL as its audience.
- **Any OpenID Connect provider**, such as FerrisKey, Keycloak, Okta, GitHub Actions, GitLab CI
  or a Kubernetes cluster's issuer. Set `issuer` and `audiences = ["<client id>"]`. The provider
  must publish `<issuer>/.well-known/openid-configuration` and its keys.
- **AWS.** `kind = "aws"`, `audiences = ["<server id>"]`, `accounts = ["<account id>"]`. See
  [AWS](#aws).

### How the server checks a token

- It checks the signature with the provider's keys, and the issuer, audience and times, with
  60 seconds of clock skew.
- It fetches a provider's keys once an hour, and never more than once a minute. When the provider
  does not answer, it keeps the keys it has.
- For an `issuer` with `*`, it fetches keys for at most 20 new issuers a minute. So tokens with
  made-up issuers cannot make it call out for each one.
- It keeps a token from a `direct` block until the token expires, for at most five minutes. A busy
  client then costs one check per token, not one per request. A user disabled in the server can
  still make requests for that time.

### AWS

AWS roles and IAM users have no token that the server can check. So the client signs an
`sts:GetCallerIdentity` request and sends it as the bearer token. The server sends it on to AWS
STS, which checks the signature and returns the caller's ARN. The credentials never leave the
client. Vault and the EKS authenticator use the same method.

```toml
[[auth.issuers]]
name = "aws"
kind = "aws"
audiences = ["https://uc.example.com"]
accounts = ["123456789012"]
create_users = true
```

- **`audiences`** holds the server id: the server's URL as clients call it. List each URL that
  clients use. The client signs the server id into the request, so the server refuses a request
  that was signed for another server. A client signs only the URL that it calls, or the server id
  that `UC_AUDIENCE` names, so another server cannot make it sign for this one.
- **`accounts`** holds the AWS accounts to accept. It is required, because STS answers for every
  account.
- **`issuer`** is optional: an STS endpoint, such as a VPC endpoint. The default is AWS's global
  and regional endpoints. The server sends signed requests to no other host.
- **Users** are named by their ARN. An assumed role is `aws:arn:aws:iam::<account>:role/<name>`,
  so all sessions of a role are one user. The role's path is not part of the name. An IAM user is
  `aws:arn:aws:iam::<account>:user/<name>`.
- **A user made ahead of time** is the role when its `externalId` is `<name>:<the role's ARN>`,
  such as `aws:arn:aws:iam::123456789012:role/etl`.

A signed request is valid for 15 minutes. The server keeps the caller for five minutes, as it does
for tokens. AWS identities are accepted on each request. They have no token exchange and no groups.
When STS limits the server's requests, the server answers with HTTP 500, not "Invalid access
token".

## Sign in to the web UI

`auth.mode` sets how users sign in to the web UI: `none` (the default), `oidc` or `forward`. A user
can sign in when their email names an active user of the server.

### OpenID Connect

```toml
[auth]
mode = "oidc"
public_url = "https://uc.example.com"
oidc_issuer = "https://accounts.google.com"
oidc_client_id = "<client id>"
oidc_client_secret = "<client secret>"   # none for a public client
```

- In the provider, allow the redirect URI `https://uc.example.com/auth/callback`.
- "Log out" ends only the sign-in to the web UI. With `oidc_end_session = true`, it also ends the
  provider's session.

### An auth proxy

An auth proxy, such as oauth2-proxy, Authelia or [Envoy Gateway](#envoy-gateway), signs users in
and sends the user's name in a header:

```toml
[auth]
mode = "forward"
forward_trusted_proxies = ["10.42.0.0/16"]
forward_logout_url = "https://auth.example.com/logout"
```

The server reads the email from `Remote-Email`, else the user from `Remote-User`. It reads these
headers only on connections from `forward_trusted_proxies`.

#### Envoy Gateway

[Envoy Gateway](https://gateway.envoyproxy.io) 1.2 or later can be the auth proxy. It needs two
policies:

- **A SecurityPolicy** on the web UI's route signs users in with OpenID Connect. Its JWT provider
  reads the ID token from its cookie, and puts the token's `email` in `Remote-Email`.
- **A ClientTrafficPolicy** on the Gateway removes `Remote-Email` and `Remote-User` from each
  client request. Envoy does not replace a header that the client sends, so without this policy a
  client could name any user.

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

Give the SecurityPolicy only the web UI's route, for example on a host of its own. `uc`, the
Python client and other API clients send the server's tokens. OpenID Connect would send their
requests to the provider's sign-in page.

## Privileges

A securable is an object that privileges apply to, such as a catalog, schema or table.

- The user who makes a securable owns it, and can grant privileges on it.
- A privilege on a catalog or schema applies to everything in it.
- To read a table, a user needs `USE CATALOG` on its catalog, `USE SCHEMA` on its schema, and
  `SELECT` on the table.
- Admins read everything, make catalogs, credentials and external locations, manage users, and
  grant any privilege.
- External tables and volumes, and the credentials of a path, need an external location that
  covers the path. This applies to admins too.

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

[Cedar](https://www.cedarpolicy.com) is a policy language. With `auth.cedar = true`, Cedar checks
the privileges. Its default policy allows what the grants allow. To add your own rules, list your
policy files in `auth.cedar_policies`:

```toml
[auth]
cedar = true
cedar_policies = ["/etc/uc/policies.cedar"]
```

```cedar
// Only owners and admins change the tables of this catalog.
forbid (
  principal,
  action == Action::"MODIFY",
  resource in Catalog::"<catalog id>"
);
```

Give each policy an `@id("…")` that no other policy in the files has. `granted` is the id of the
default policy. When two policies have the same `@id`, the server does not start, and names the
file.

For each check, the server asks Cedar: can this principal do this action on this resource?

| Part | Cedar | Example |
| --- | --- | --- |
| Principal | `User::"<email>"`: the signed-in user | `User::"alice@example.com"` |
| Action | `Action::"<privilege>"`, with the spaces of the privilege | `Action::"USE CATALOG"` |
| Resource | `<type>::"<id>"`: the securable of the check, by its id, such as the catalog for `USE CATALOG` and the table for `SELECT` | `Catalog::"6f1e…"` |

The actions are the privileges of the table in [Privileges](#privileges). The resources, and where
to find their ids:

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

An id stays the same when you rename the securable. A securable that you drop and make again gets
a new id.

- **Parents:** a table, volume, function or model is `in` its schema, a schema is `in` its
  catalog, and a catalog is `in` the metastore. So `resource in Catalog::"<id>"` matches
  everything in the catalog. External locations and credentials have no parent.
- **Resource attributes:**
    - `owner`: the owner's email, when the resource has an owner.
    - `granted_privileges`: the actions granted on the resource or on a parent, to the user or to
      a group of the user.
- **Groups** are not Cedar entities, so a policy cannot say `principal in Group::"…"`. Their
  grants are in `granted_privileges`.
- **Context:** how and from where the request came.

    | Attribute | Type | Value |
    | --- | --- | --- |
    | `context.auth_method` | `String` | `jwt` (a provider's token), `aws-sigv4`, `trusted-headers` (an auth proxy), `server-token`, or `personal-access-token` |
    | `context.provider` | `String` | The `name` of the `[[auth.issuers]]` block, `forward` behind an auth proxy, or `internal` for the server's tokens |
    | `context.groups` | `Set<String>` | The groups that the token names (`groups_claim`), as `<name>:<value>` |
    | `context.source_ip` | `ipaddr` | The address of the connection, when known |
    | `context.request_time` | `datetime` | When the request came |

    Some requests have no address, so test for `source_ip` with `has`:

    ```cedar
    // Writes only from the office network.
    forbid (principal, action == Action::"MODIFY", resource)
    unless { context has source_ip && context.source_ip.isInRange(ip("10.0.0.0/8")) };
    ```

- **Errors deny.** A policy that fails, such as one that reads `resource.owner` of a resource
  without an owner, denies the request. Test optional attributes with `has` first.
- **Owners and admins:** Cedar decides only whether a user holds a privilege. The checks for an
  owner or an admin do not ask Cedar, so a `forbid` does not stop them.
