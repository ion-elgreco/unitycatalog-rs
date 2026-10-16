# Database

The server keeps the catalog in a database: an embedded Turso file, or PostgreSQL. `db.url`
(`UC_DB_URL`) selects it. The data of tables, volumes and models is not in the database: it is in
your storage, such as S3 or a local folder.

## The embedded database

Without `db.url`, the catalog is in the Turso file `uc.db` in `server.data_dir` (default `.uc`).
Turso is an embedded database that is compatible with SQLite. The server needs no other service.

| `db.url` | Database |
| --- | --- |
| none | The file `uc.db` in `server.data_dir`. |
| `file:<path>` | The file `<path>`. |
| `memory` | In memory: the catalog is lost when the server stops. For tests. |
| `jdbc:h2:file:<path>` | Upstream's URL of an H2 file: the Turso file `<path>.db`. The server does not read H2's files. |
| `jdbc:h2:mem:<name>` | Upstream's URL of an H2 database in memory: in memory. |

Only one server can use a file at one time: Turso locks it. A second server stops with the error
`File is locked by another process`.

Turso writes temporary files to `/tmp`, or to the folder of `TURSO_TMPDIR` or `SQLITE_TMPDIR`. On
a read-only root filesystem, give `/tmp` a volume.

To copy the catalog, stop the server and copy `uc.db`, or use
[`uc admin backup`](operations.md#back-up-and-restore) while it runs.

## PostgreSQL

Several servers can share a PostgreSQL database: use it for more than one replica.

```toml
[db]
url = "postgresql://uc@db.example.com:5432/uc?sslmode=verify-full"
password = "<password>"   # or UC_DB_PASSWORD
pool_size = 20
```

The URL is libpq's (`postgresql://user:password@host:port/database?…`) or pgJDBC's, as upstream
has it in `hibernate.properties`: `jdbc:postgresql://host:port/database?user=…&password=…`.
`db.user` and `db.password` win over the URL's user and password. Without a user, the server
uses the operating system's user, as libpq does.

| URL parameter | What it does |
| --- | --- |
| `sslmode` | libpq's values: `disable`, `allow`, `prefer` (the default), `require`, `verify-ca`, `verify-full`. |
| `sslrootcert` | The PEM file of the root certificates; else, or with `system`, the system's roots. |
| `connect_timeout` | How long a connection can take, in seconds. |
| `application_name`, `options` | As libpq has them. |
| `currentSchema` | pgJDBC URLs only: the PostgreSQL schema of the catalog's tables. The server makes it when it is not there. Without it, `search_path` decides. |
| `ssl=true` | pgJDBC URLs only: the same as `sslmode=verify-full`. |
| `ApplicationName`, `connectTimeout` | pgJDBC URLs only: `application_name` and `connect_timeout`. |

The server ignores other parameters of a pgJDBC URL. Client certificates (`sslcert`, `sslkey`)
do not work.

| `sslmode` | TLS | Certificate checked | Host name checked |
| --- | --- | --- | --- |
| `disable` | no | | |
| `allow`, `prefer` | when the server offers it | no | no |
| `require` | yes | only with `sslrootcert` | no |
| `verify-ca` | yes | yes | no |
| `verify-full` | yes | yes | yes |

The tests of the server run on PostgreSQL 17.

### Several replicas

Replicas that share a database must also share the signing key of their tokens: a token that one
replica signs must be valid on the others. Give all replicas the same key file, such as a mounted
secret, in `auth.signing_key_file` (`UC_AUTH_SIGNING_KEY_FILE`). The server only reads the file.
Without it, each replica makes its own key in memory, and refuses the tokens of the others. The
[Helm chart](../kubernetes.md) does this for you.

The file is an RSA private key in PEM. `uc-server keygen` prints a new one:

```bash
(umask 077 && uc-server keygen > signing-key.pem)
```

## Schema changes

The server updates the database schema when it starts. On PostgreSQL, one replica at a time does
this, under a lock. A server that finds an applied change that it does not know stops: do not
start an older version on a newer database.

## Move from upstream

The server does not read the databases of upstream's server: H2, PostgreSQL or MySQL. To move a
catalog, make its catalogs, schemas, tables, volumes, functions and models again in this server,
with the API, `uc` or the Python client.
