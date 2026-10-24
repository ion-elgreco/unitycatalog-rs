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
| `memory` | In memory, for tests: the catalog is lost when the server stops. |
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
- Servers that share a database must share their signing key too: give them all the same
  `auth.signing_key_file` (see [The signing key](auth.md#the-signing-key)).
- The server updates the schema of the database when it starts. Do not start an older version on
  a newer database.

The server does not read the databases of upstream's server. To move a catalog, make its objects
again with the API, `uc` or the Python client.
