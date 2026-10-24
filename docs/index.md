# unitycatalog-rs

A Rust reimplementation of [Unity Catalog OSS](https://github.com/unitycatalog/unitycatalog),
the open catalog for data and AI. It has upstream's REST APIs (Unity Catalog, Delta and Iceberg
REST), a web UI, the `uc` CLI and a Python client. It keeps the catalog in an embedded database
(Turso) or in PostgreSQL.

## Try it

```bash
pip install 'unitycatalog-rs[all]'
uc dev
```

`unitycatalog-rs[all]` installs the Python client, the `uc` CLI, the server and the web UI.
`uc dev` runs a server on <http://localhost:8080>, opens the web UI in your browser, and keeps its
data in `.uc`, in the working directory. In a second terminal, use the CLI:

```bash
uc catalog create --name main
uc schema create --catalog main --name default
uc catalog list
```

Or use the Python client:

```python
from unitycatalog import Client

client = Client("http://localhost:8080/")
print([catalog.name for catalog in client.list_catalogs()])
```

Press Ctrl+C to stop `uc dev`.

## The parts

| Part | What it does | Where you get it |
| --- | --- | --- |
| [`uc-server`](server/index.md) | Serves the REST APIs, and the web UI at `/` | The image, the Helm chart, `pip install 'unitycatalog-rs[server]'`, the release tarballs |
| [`uc`](cli/index.md) | Runs upstream's `bin/uc` commands, a terminal UI, and `uc dev` | `pip install 'unitycatalog-rs[cli]'`, the release tarballs, the image |
| [Python client](python-client.md) | Calls the API from Python, with or without `asyncio` | `pip install unitycatalog-rs` |
| [Web UI](ui.md) | Shows and changes the catalog in a browser | The image, `pip install 'unitycatalog-rs[ui]'`, the release tarballs |

## Run it

- [Kubernetes](kubernetes.md): the Helm chart, for production.
- [Docker](docker.md): the image, with `docker run` or Docker Compose.
- [Server](server/index.md): the `uc-server` binary on your own machine, and all its settings.

## Use it

- [CLI](cli/index.md): the `uc` commands.
- [Python client](python-client.md): the `unitycatalog` package.
- [Web UI](ui.md): the pages of the web UI.
- [APIs](server/apis.md): the REST APIs, for Spark, Iceberg clients and your own code.

The README of the [repository](https://github.com/ion-elgreco/unitycatalog-rs) compares this
project with upstream (Java), feature by feature.
