---
title: HTTP API
template: api-reference.html
---

# HTTP API

The operations of the server's HTTP APIs, with their parameters, request bodies and responses:
the Unity Catalog REST API, the Delta REST API, the Iceberg REST catalog, the control plane and
the sharing protocols (Delta Sharing and Open Sharing). Scalar shows them from the
[OpenAPI document](openapi.json). It comes from the protos in `proto/` that the server's routes
and the clients follow, and for the sharing protocols from the OpenAPI document of the
`opensharing-api` crate, which serves them.
