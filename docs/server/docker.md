# Docker

The image `ghcr.io/ion-elgreco/unitycatalog-rs` has the server, the web UI and `uc`.

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

Open <http://localhost:8080> for the web UI. The catalog is in the volume `uc-data`.

## Settings

Give settings as `UC_<GROUP>_<KEY>` environment variables. [Settings](../reference/settings.md)
lists them.

```sh
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  -e UC_DB_URL='postgresql://uc@postgres.example.com:5432/uc' \
  -e UC_DB_PASSWORD='<password>' \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
```

## Sign-in

Make a signing key, start the server with sign-in, and get a token of its `admin` user:

```sh
docker run --rm -v uc-data:/data ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev \
  keygen --output /data/signing-key.pem
docker run -d --name uc -p 8080:8080 -v uc-data:/data \
  -e UC_AUTH_ENABLED=true -e UC_AUTH_SIGNING_KEY_FILE=/data/signing-key.pem \
  ghcr.io/ion-elgreco/unitycatalog-rs:0.0.0-dev
export UC_CLI_TOKEN=$(docker exec uc uc-server token)
```

See [Sign-in and authorization](auth.md).

## Docker Compose

`compose.yaml` in the repository runs the same server:

```sh
docker compose up
```
