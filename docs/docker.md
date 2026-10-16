# Docker

The image `ghcr.io/ion-elgreco/unitycatalog-rs` has the server, the web UI and the `uc` CLI. It is
built for `linux/amd64` and `linux/arm64`, on distroless: it has no shell and no package manager,
and it runs as a non-root user (65532). Each image has an SBOM and the provenance of its build.

## Run the server

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

Open <http://localhost:8080> for the web UI. The API is at
`http://localhost:8080/api/2.1/unity-catalog`. The server keeps the catalog database in `/data`,
here the volume `uc-data`.

Use the `uc` of the image, or a [`uc` on your machine](cli/index.md):

```sh
docker exec uc uc catalog create --name main
uc catalog list
```

The server logs to stdout: `docker logs -f uc`. `docker stop uc` stops it after it has finished
its requests.

## Give settings

The server reads its settings from `UC_<GROUP>_<KEY>` environment variables, or from a `uc.toml`
that `UC_CONFIG` names. [Settings](server/settings.md) lists every setting.

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  -e UC_AUTH_ENABLED=true \
  -e UC_STORAGE_TABLES_ROOT=s3://analytics/managed \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  -v "$PWD/uc.toml:/etc/uc/uc.toml:ro" -e UC_CONFIG=/etc/uc/uc.toml \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

The arguments after the image go to `uc-server`: `--port`, `--obs-port` and `--config`.

## Sign in as the admin

With `UC_AUTH_ENABLED=true`, each request needs a token. First give the server a signing key, so
that its tokens stay valid after a restart. `uc-server keygen` writes a new key into the volume,
in a file that only the server's user reads:

```sh
docker run --rm -v uc-data:/data ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev \
  keygen --output /data/signing-key.pem
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  -e UC_AUTH_ENABLED=true -e UC_AUTH_SIGNING_KEY_FILE=/data/signing-key.pem \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

Then `uc-server token`, in the container, prints a token of the server's `admin` user. The token
expires after one hour (`--ttl`):

```sh
export UC_CLI_TOKEN=$(docker exec uc uc-server token)
uc user create --name Alice --email alice@example.com
```

Without `UC_AUTH_SIGNING_KEY_FILE`, the server makes a new key in memory at each start: a restart
ends all sign-ins, and no `admin` token exists. [Sign-in and authorization](server/auth.md) tells
how users sign in, and how `[[auth.users]]` makes the first users.

## Use PostgreSQL

Without `UC_DB_URL`, the catalog is in the embedded Turso database, `/data/uc.db`. Only one
server can use that file. For a PostgreSQL server:

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  -e UC_DB_URL='postgresql://uc@postgres.example.com:5432/uc?sslmode=verify-full' \
  -e UC_DB_PASSWORD='<password>' \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

Servers that share a database must use the same signing key (`UC_AUTH_SIGNING_KEY_FILE`): see
[Database](server/database.md).

## Probes and metrics

`--obs-port` turns on `/livez`, `/readyz` and `/metrics` on a second port:

```sh
docker run -d --name uc -p 8080:8080 -p 8081:8081 -v uc-data:/data \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev --obs-port 8081
curl http://localhost:8081/readyz
```

The image has no health check: it has no shell or `curl` to run one. Use these endpoints from
outside the container.

## Run with a read-only root filesystem

The server writes only to `/data` and to `/tmp`, where Turso keeps temporary files. Give `/tmp`
a `tmpfs`:

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  --read-only --tmpfs /tmp --cap-drop ALL --security-opt no-new-privileges \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

## Docker Compose

`compose.yaml` in the repository runs the server with the web UI on
<http://localhost:8080>, with its data in the volume `uc-data`. Its main lines:

```yaml
name: unitycatalog-rs

services:
  server:
    image: ${UC_IMAGE:-ghcr.io/ion-elgreco/unitycatalog-rs:latest}
    ports:
      - "8080:8080"
    volumes:
      - uc-data:/data
    environment:
      RUST_LOG: info
      # UC_AUTH_ENABLED: "true"
      # UC_DB_URL: "postgresql://uc@postgres:5432/uc"
      # UC_DB_PASSWORD: "change-me"

volumes:
  uc-data:
```

```sh
docker compose up
```

## What is in the image

| Path | What it is |
| --- | --- |
| `/usr/local/bin/uc-server` | The server; the image's entrypoint. |
| `/usr/local/bin/uc` | The CLI. |
| `/usr/local/share/unitycatalog-rs/ui` | The web UI, which the server serves at `/`. |
| `/data` | The server's data: the catalog database `uc.db`. A volume. |

| Port | What it serves |
| --- | --- |
| 8080 | The APIs, the sign-in and the web UI. |
| 8081 | `/livez`, `/readyz` and `/metrics`, with `--obs-port 8081`. |

The image sets `UC_SERVER_DATA_DIR=/data`. Its tags are the release versions, such as
`0.0.0-dev`, and `latest`.

## Build the image from a checkout

In a checkout of the repository, `just docker` builds the image `unitycatalog-rs:dev` for your
machine's architecture, as the release builds it. It needs Docker,
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) and [zig](https://ziglang.org).

```sh
just docker
UC_IMAGE=unitycatalog-rs:dev docker compose up
```

`just test-docker` builds the image with a faster dev profile, runs it with a read-only root
filesystem, and checks the web UI and `uc` in it.
