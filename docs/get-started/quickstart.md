# Quickstart

Run a server on your machine, make a table, and read it.

## 1. Start a server

```bash
pip install 'unitycatalog-rs[all]'
uc dev
```

This installs the server, the CLI, the Python client and the web UI. `uc dev` runs a server on
<http://localhost:8080>, and opens the web UI in your browser.

## 2. Make a table

In a second terminal:

```bash
uc catalog create --name main
uc schema create --catalog main --name default
uc table create --full_name main.default.events \
  --columns "id INT, name STRING" --table_type MANAGED
uc table write --full_name main.default.events   # 15 random rows
uc table read --full_name main.default.events
```

## 3. Read it from Python

```python
from unitycatalog import Client

client = Client("http://localhost:8080/")
table = client.table("main.default.events").get()
print(table.storage_location, [column.name for column in table.columns])
```

The web UI shows the same table: open `main`, then `default`, then `events`.

## Next steps

- [Run the server](../server/index.md), in [Docker](../server/docker.md) or on
  [Kubernetes](../server/kubernetes.md).
- Learn the [CLI](../clients/cli.md) and the [Python client](../clients/python.md).
