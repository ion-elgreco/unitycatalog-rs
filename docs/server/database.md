# Database

The server keeps the catalog in a database: an embedded Turso file, or PostgreSQL. The data of
tables, volumes and models is not in it: it is in your [storage](cloud-storage.md).

## The embedded database

By default, the catalog is in the file `uc.db` in `server.data_dir` (`.uc`). Turso is compatible
with SQLite, and needs no other service. Only one server can use the file at a time.

| `db.url` | Database |
| --- | --- |
| none | `uc.db` in `server.data_dir`. |
| `file:<path>` | The file `<path>`. |
| `memory` | In memory, for tests and `uc dev`: the catalog is lost when the server stops. |
| `jdbc:h2:file:<path>`, `jdbc:h2:mem:<name>` | Upstream's H2 URLs: the Turso file `<path>.db`, or memory. |

To copy the catalog, stop the server and copy `uc.db`, or use
[`uc admin backup`](operations.md#back-up-and-restore) while it runs.

## PostgreSQL

Several servers can share a PostgreSQL database:

```toml
[db]
url = "postgresql://uc@db.example.com:5432/uc?sslmode=verify-full"
password = "<password>"   # or UC_DB_PASSWORD
```

- The URL is libpq's, with its `sslmode`, `sslrootcert`, `connect_timeout` and
  `application_name`. Upstream's pgJDBC URLs (`jdbc:postgresql://…`) work too.
- Servers that share a database must share their keys too: give them all the same
  `auth.signing_key_file` (see [The signing key](auth.md#the-signing-key)) and
  `secrets.key_file` (see [The secrets key](#the-secrets-key)).
- The server updates the schema of the database when it starts. Do not start an older version on
  a newer database.

The server does not read the databases of upstream's server. To move a catalog, make its objects
again with the API, `uc` or the Python client.

## The secrets key

Credentials with an Azure service principal or a GCP service account key have a secret. The server
seals each secret with the P-256 key of `secrets.key_file` before it writes the secret to the
database, so the database and its backups hold no secret in plain text. The seal is HPKE
(RFC 9180), bound to the credential: a sealed secret copied to another credential does not open.
Without the setting, the server makes a new key in memory at each start, as it does for the
signing key: the secrets of the credentials made then cannot be opened after the server stops.
`uc-server keygen --secrets` makes a key:

```bash
(umask 077 && uc-server keygen --secrets > secrets-key.pem)
```

`openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256` makes one too.

```toml
[secrets]
key_file = "/etc/uc/secrets-key.pem"
```

Keep the key secret, and keep a copy of it: without the key, the server cannot open the secrets,
and each such credential needs its secret again. A server that restores a backup needs the key
too.

After the server opens a secret, it keeps it in memory for `secrets.cache_ttl` (`10m`), and at
most `secrets.cache_capacity` (`500`) secrets. The cache keeps the sign-ins of Azure service
principals too. A changed credential never gets its old secret from the cache. With
`cache_capacity = 0`, the server keeps no secret in memory between requests.

### A new secrets key

The file can hold several keys, newest first: the first key seals, and each key opens the secrets
that it sealed. At each start, the server seals again with the first key the secrets of the older
keys. To change the key of one server:

1. Put a new key first in the file, and restart the server:

    ```bash
    (umask 077 && { uc-server keygen --secrets; cat secrets-key.pem; } > new.pem) && mv new.pem secrets-key.pem
    ```

2. When the log says that the first key seals every secret, remove the old key from the file, and
   restart the server again.

Servers that share a database cannot open a secret of a key that is not in their file. Thus, give
them the new key in two steps: first put it last in the file and restart each server, then move
it first and restart each server again.
