---
hide:
  - toc
---

# unitycatalog-rs

A Rust reimplementation of [Unity Catalog OSS](https://github.com/unitycatalog/unitycatalog),
the open catalog for data and AI. It has upstream's REST APIs (Unity Catalog, Delta and Iceberg
REST), a web UI, the `uc` CLI and a Python client. It keeps the catalog in an embedded database
(Turso) or in PostgreSQL.

```bash
pip install 'unitycatalog-rs[all]'
uc dev
```

[Quickstart](get-started/quickstart.md){ .md-button .md-button--primary }
[Install](get-started/install.md){ .md-button }

<div class="grid cards" markdown>

-   :material-server-outline:{ .lg .middle } **Server**

    ---

    Run `uc-server` on your machine, in Docker or on Kubernetes. Keep the catalog in PostgreSQL,
    sign users in, and give clients credentials for S3, ADLS and GCS.

    [:material-arrow-right: Run the server](server/index.md)

-   :material-console:{ .lg .middle } **Clients**

    ---

    The `uc` CLI and its terminal UI, the Python client, the web UI, and the REST APIs for
    engines such as Polars, deltalake and PyIceberg.

    [:material-arrow-right: Use the clients](clients/cli.md)

-   :material-book-open-variant:{ .lg .middle } **Reference**

    ---

    Each operation of the HTTP APIs, each class of the Python client, each command of `uc`, each
    setting and each value of the Helm chart.

    [:material-arrow-right: Look it up](reference/index.md)

-   :material-scale-balance:{ .lg .middle } **Upstream**

    ---

    Clients of upstream's server use this server as they are. The README compares the two
    servers, feature by feature.

    [:material-arrow-right: Compare](https://github.com/ion-elgreco/unitycatalog-rs#readme)

</div>
