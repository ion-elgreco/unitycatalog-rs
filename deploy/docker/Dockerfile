# uc-server, the web UI and the uc CLI on distroless: no shell, no package
# manager, a non-root user (65532).
#
# Nothing is compiled here. The build context is the tree of a Linux release
# tarball of the image's architecture, as the release workflow and
# `just docker` make it:
#
#   bin/uc-server, bin/uc      built with cargo zigbuild against glibc 2.28
#   share/unitycatalog-rs/ui   the web UI (`just wasm`)

ARG BUILDER_IMAGE=debian:bookworm-slim
ARG BASE_IMAGE=gcr.io/distroless/cc-debian12:nonroot

# The runtime has no shell, so the files get their owners and modes here. The
# stage runs on the build platform: no emulation for the other architecture.
FROM --platform=$BUILDPLATFORM ${BUILDER_IMAGE} AS prep
COPY bin/uc-server bin/uc /staging/usr/local/bin/
COPY share/unitycatalog-rs/ui/ /staging/usr/local/share/unitycatalog-rs/ui/
# The binaries and the web UI belong to root: the server cannot change them.
# The server writes only in /data: the catalog database.
RUN set -eux; \
    chmod -R u=rwX,go=rX /staging; \
    chmod 0755 /staging/usr/local/bin/uc-server /staging/usr/local/bin/uc; \
    install -d -m 0755 -o 65532 -g 65532 /staging/data

FROM ${BASE_IMAGE}

ARG UC_VERSION=dev
ARG UC_REVISION=unknown
ARG BUILD_DATE=unknown
ARG BASE_IMAGE

LABEL org.opencontainers.image.title="unitycatalog-rs" \
      org.opencontainers.image.description="Unity Catalog server in Rust, with its web UI and the uc CLI" \
      org.opencontainers.image.authors="Ion Koutsouris" \
      org.opencontainers.image.source="https://github.com/ion-elgreco/unitycatalog-rs" \
      org.opencontainers.image.url="https://github.com/ion-elgreco/unitycatalog-rs" \
      org.opencontainers.image.documentation="https://github.com/ion-elgreco/unitycatalog-rs" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${UC_VERSION}" \
      org.opencontainers.image.revision="${UC_REVISION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.base.name="${BASE_IMAGE}"

COPY --from=prep /staging/ /

USER 65532:65532
WORKDIR /data

# The server's state on /data: the catalog database (uc.db). Its settings
# are UC_<GROUP>_<KEY> variables, or a uc.toml that UC_CONFIG names; logs
# go to stdout. The web UI is where uc-server
# and uc find it: share/unitycatalog-rs/ui next to their bin/ folder. On a
# read-only root filesystem, /tmp needs a volume too: Turso writes
# temporary files there.
ENV UC_SERVER_DATA_DIR=/data

# 8080: API and UI. 8081: probes and metrics, with `--obs-port 8081`.
EXPOSE 8080 8081
VOLUME ["/data"]

STOPSIGNAL SIGTERM

ENTRYPOINT ["/usr/local/bin/uc-server"]
