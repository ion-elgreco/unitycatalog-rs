# Install

unitycatalog-rs has four parts:

| Part | What it does |
| --- | --- |
| [`uc-server`](../server/index.md) | Serves the REST APIs, and the web UI at `/`. |
| [`uc`](../clients/cli.md) | Runs upstream's `bin/uc` commands, a [terminal UI](../clients/tui.md), and `uc dev`. |
| [Python client](../clients/python.md) | Calls the APIs from Python, with or without `asyncio`. |
| [Web UI](../clients/web-ui.md) | Shows and changes the catalog in a browser. |

## pip

The package `unitycatalog-rs` is the Python client. Its extras add the other parts, of the same
version:

| Install | Adds |
| --- | --- |
| `pip install unitycatalog-rs` | The Python client only. |
| `pip install 'unitycatalog-rs[cli]'` | `uc`. |
| `pip install 'unitycatalog-rs[server]'` | `uc-server` and `uc`. |
| `pip install 'unitycatalog-rs[ui]'` | The web UI and `uc`. |
| `pip install 'unitycatalog-rs[all]'` | All of them. |

The wheels are for Python 3.12 and later, and free-threaded Python 3.14t, on Linux (x86_64 and
aarch64, glibc and musl), macOS (x86_64 and arm64) and Windows (x86_64).

## Docker

The image `ghcr.io/ion-elgreco/unitycatalog-rs` has `uc-server`, `uc` and the web UI:

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

See [Docker](../server/docker.md).

## Helm

The Helm chart runs the image on Kubernetes, with the catalog in PostgreSQL. Name your PostgreSQL
server in `values.yaml`, as [Kubernetes](../server/kubernetes.md) shows:

```sh
helm install uc oci://ghcr.io/ion-elgreco/charts/uc-server \
  --version 0.0.0-dev --namespace uc --create-namespace -f values.yaml
```
