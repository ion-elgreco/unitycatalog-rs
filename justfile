set shell := ["bash", "-uc"]

# The Python of the stub generator: the venv of the uv workspace
# (`just venv`), with ruff.
python := justfile_directory() / ".venv/bin/python"

# The cargo profile of the builds and tests: dev, or PROFILE. CI sets
# PROFILE=ci, [profile.ci] of Cargo.toml.
profile := env("PROFILE", "dev")

# The uc-server of that profile, which the Python tests start.
uc_server := justfile_directory() / "target" / (if profile == "dev" { "debug" } else { profile }) / (if os() == "windows" { "uc-server.exe" } else { "uc-server" })

# The image that `just docker` builds.
image := "unitycatalog-rs:dev"

# The upstream Java server that `just test-upstream` runs: its latest release.
upstream_image := "unitycatalog/unitycatalog:v0.7.0"

# mike, which publishes a version of the docs site: zensical's fork of it
# (squidfunk/mike), at a fixed commit.
mike := "git+https://github.com/squidfunk/mike.git@2d4ad799442f4592db8ad53b179bfb33db8c69ac"

_default:
    just --list

# Make the venv of the uv workspace (.venv) with the dev tools and the engines of the integration tests, as uv.lock pins them (UV_NO_GROUP=integrations leaves the engines out); it builds none of the packages
venv:
    uv sync --no-install-workspace --all-packages

# Build the web UI (crates/uc-ui) into crates/uc-ui/dist (release), and compress the wasm once (brotli at quality 11, and gzip): the servers send those files as they are
wasm: (_ui "release")
    brotli -q 11 -k crates/uc-ui/dist/assets/uc_ui_bg.wasm
    gzip -9 -k -n crates/uc-ui/dist/assets/uc_ui_bg.wasm

# Build the web UI in the dev profile (or PROFILE): faster, with panic messages and symbols; serve it with `UC_UI_DIR=crates/uc-ui/dist`
wasm-dev: (_ui profile)

# The files of crates/uc-ui/public, with the wasm and its JS loader in assets/
_ui profile:
    cargo build -p uc-ui --target wasm32-unknown-unknown --profile {{ profile }}
    rm -rf crates/uc-ui/dist && cp -R crates/uc-ui/public crates/uc-ui/dist
    wasm-bindgen target/wasm32-unknown-unknown/{{ if profile == "dev" { "debug" } else { profile } }}/uc_ui.wasm --out-dir crates/uc-ui/dist/assets --target web --no-typescript

# Build the four Python packages and install them into .venv: maturin builds the client, uc and uc-server (dev profile, or PROFILE), and cargo shows its progress; the web UI comes from just wasm
develop: develop-client wasm
    cd python/cli && uv run --no-sync maturin develop --profile {{ profile }}
    cd python/server && uv run --no-sync maturin develop --profile {{ profile }}
    uv pip install -q --reinstall --python .venv ./python/ui

# Build only the Python client and install it into .venv, for its tests
develop-client: venv
    cd python/client && uv run --no-sync maturin develop --profile {{ profile }}

# Build everything (just develop), then run `uc dev` from .venv on the port, with its data in .uc-demo; the first run puts in the dummy data of examples/seed_demo.py (delete .uc-demo for new data); Ctrl+C stops it
demo port="8080": develop
    #!/usr/bin/env bash
    set -euo pipefail
    url="http://127.0.0.1:{{ port }}"
    # A server on the port would get the data.
    if curl -s -o /dev/null "$url/"; then
        echo "Port {{ port }} is in use. Choose another one, such as: just demo 8081" >&2
        exit 1
    fi
    uv run --no-sync uc dev --port {{ port }} --dir .uc-demo &
    until curl -fs -o /dev/null "$url/"; do
        # uc dev stopped, and said why.
        kill -0 $! 2>/dev/null || exit 1
        sleep 0.2
    done
    # Only into an empty server.
    uv run --no-sync examples/seed_demo.py "$url" .uc-demo/storage
    # Ctrl+C reaches uc dev too: wait until it has stopped.
    trap '' INT
    wait

# As just demo, with sign-in and S3, in Docker: FerrisKey, an OpenID Connect provider (on idp_port, new at each run), and RustFS, an S3-compatible store (on s3_port), hold the users and the data; alice owns the dummy data, bob reads sales.eu; data in .uc-demo-auth and RustFS's volume (delete .uc-demo-auth for new data); Ctrl+C stops all
demo-auth port="8080" idp_port="8090" s3_port="9000": develop
    #!/usr/bin/env bash
    set -euo pipefail
    # The provider sends the browser back to this URL.
    url="http://localhost:{{ port }}"
    idp="http://localhost:{{ idp_port }}"
    s3="http://localhost:{{ s3_port }}"
    # A server on the port would get the data.
    if curl -s -o /dev/null "$url/"; then
        echo "Port {{ port }} is in use. Choose another one, such as: just demo-auth 8081" >&2
        exit 1
    fi
    compose=(docker compose -f examples/demo-auth/compose.yaml)
    # RustFS's console is on the port after its S3 API.
    export FERRISKEY_PORT={{ idp_port }} RUSTFS_PORT={{ s3_port }} RUSTFS_CONSOLE_PORT=$(({{ s3_port }} + 1))
    # New data in uc dev: in RustFS too.
    if [ ! -d .uc-demo-auth ]; then
        "${compose[@]}" down --volumes
    fi
    trap '"${compose[@]}" down' EXIT
    "${compose[@]}" up -d --force-recreate
    for ready in "$idp/api/realms/master/.well-known/openid-configuration" "$s3/health/ready"; do
        curl -fs -o /dev/null --retry 60 --retry-all-errors --retry-delay 1 "$ready"
    done
    # FerrisKey's realm, client and users, RustFS's bucket, and the settings of
    # uc dev and uc; `uc auth login` listens on the port after FerrisKey's.
    uv run --no-sync examples/demo-auth/setup.py "$idp" "$s3" "$url" $(({{ idp_port }} + 1)) .uc-demo-auth/uc.toml
    uv run --no-sync uc --config .uc-demo-auth/uc.toml dev --port {{ port }} --dir .uc-demo-auth &
    uc=$!
    # On an error, uc dev stops too.
    trap 'kill $uc 2>/dev/null || true; wait; "${compose[@]}" down' EXIT
    until curl -fs -o /dev/null "$url/"; do
        # uc dev stopped, and said why.
        kill -0 $uc 2>/dev/null || exit 1
        sleep 0.2
    done
    # Only into an empty server.
    uv run --no-sync examples/seed_demo_auth.py .uc-demo-auth
    echo "Sign in as alice (password alice) or bob (password bob). Log out signs out of FerrisKey too."
    echo "FerrisKey's console: $idp, as admin (password admin). RustFS's console: http://localhost:$RUSTFS_CONSOLE_PORT/rustfs/console/, as uc-demo (password uc-demo-secret)."
    echo "uc signs in with: uv run uc --config .uc-demo-auth/uc.toml auth login"
    # Ctrl+C reaches uc dev too: wait until it has stopped, then stop the services.
    trap '' INT
    wait

# Run the Python client's tests and the integration tests (polars, deltalake, obstore and pyiceberg; S3 from moto) against a uc-server of this repository; on free-threaded Python, an import that turns the GIL back on fails them
test:
    cargo build -p uc-server --profile {{ profile }}
    cd python/client && UC_TEST_SERVER_BIN='{{ uc_server }}' PYTHONWARNINGS='error:The global interpreter lock (GIL) has been enabled:RuntimeWarning' uv run --no-sync pytest tests

# Run the Python client's tests that are marked `upstream` against the upstream Java server, in Docker (upstream_image); needs the client in .venv (just develop-client)
test-upstream:
    #!/usr/bin/env bash
    # The image starts the server with authorization off: the tests need no
    # token.
    set -euo pipefail
    id=$(docker run -d -p 127.0.0.1:18183:8080 {{ upstream_image }})
    trap 'status=$?; [ $status -eq 0 ] || docker logs "$id"; docker rm -f -v "$id" > /dev/null; exit $status' EXIT
    url=http://127.0.0.1:18183
    curl -fs --retry 60 --retry-all-errors --retry-delay 1 "$url/api/2.1/unity-catalog/catalogs" > /dev/null
    cd python/client && UC_TEST_SERVER_URL="$url" uv run --no-sync pytest tests

# Run the Spark tests (python/client/tests/integrations/test_spark.py) with PySpark `spark`, 4.1.x or 4.2.x, against a uc-server of this repository; needs Java 17 or 21 and the client in .venv (just develop-client). Spark downloads its jars into ~/.ivy2.5.2
test-spark spark="4.2.0":
    cargo build -p uc-server --profile {{ profile }}
    cd python/client && UC_TEST_SERVER_BIN='{{ uc_server }}' uv run --no-sync --with 'pyspark=={{ spark }}' pytest tests/integrations/test_spark.py

# Build the wheels of the four Python packages, and run `uc dev`, `uc web` and the client from them, as `pip install 'unitycatalog-rs[all]'` installs them
test-packages: wasm-dev
    #!/usr/bin/env bash
    # Binaries of the dev profile (or PROFILE), which build faster. Everything
    # goes to target/packages-test.
    set -euo pipefail
    out=target/packages-test
    rm -rf "$out"
    MATURIN_PEP517_ARGS="--profile {{ profile }}" uv build --wheel --all-packages -o "$out/wheels"
    # Each command runs in a new environment with `unitycatalog-rs[all]` from
    # the wheels. No cache: it keeps the files of an earlier wheel of the
    # same name and version, and installs those.
    run=(uv run -q --isolated --no-project --no-cache --no-index --find-links "$out/wheels" --with 'unitycatalog-rs[all]')
    "${run[@]}" uc dev --port 18180 --dir "$out/dev" --no_browser &
    "${run[@]}" uc web --port 18181 --server http://127.0.0.1:18180 --no_browser &
    trap 'kill $(jobs -p) 2>/dev/null; wait' EXIT
    # Both serve the web UI, once they answer.
    for port in 18180 18181; do
        curl -fs --retry 60 --retry-connrefused --retry-delay 1 -H 'Accept: text/html' "http://127.0.0.1:$port/" | grep -q '<html'
    done
    # The client puts in the demo data through uc web, which sends the API
    # calls on to uc dev.
    "${run[@]}" examples/seed_demo.py http://127.0.0.1:18181 "$out/storage"
    curl -fsS http://127.0.0.1:18180/api/2.1/unity-catalog/catalogs | grep -q '"unity"'
    echo "The packages work: uc dev, uc web, uc-server, the web UI and the client."

# Build the image of deploy/docker/Dockerfile (unitycatalog-rs:dev) for this machine's architecture, as the release builds it; needs cargo-zigbuild and zig
docker profile="release":
    #!/usr/bin/env bash
    # The build context is the tree of a Linux release tarball, in
    # target/docker. The binaries are built with cargo zigbuild against
    # glibc 2.28, as in the release. The dev profile builds faster; without
    # debug info, its binaries are a third of the size.
    set -euo pipefail
    target={{ arch() }}-unknown-linux-gnu
    {{ just_executable() }} {{ if profile == "dev" { "wasm-dev" } else { "wasm" } }}
    cargo zigbuild --profile {{ profile }} --config profile.dev.debug=0 \
        --target "$target.2.28" -p uc-server -p uc-cli
    bin="target/$target/{{ if profile == "dev" { "debug" } else { profile } }}"
    context=target/docker
    rm -rf "$context"
    mkdir -p "$context/bin" "$context/share/unitycatalog-rs"
    cp "$bin/uc-server" "$bin/uc" "$context/bin/"
    cp -R crates/uc-ui/dist "$context/share/unitycatalog-rs/ui"
    docker build -f deploy/docker/Dockerfile -t {{ image }} \
        --build-arg UC_REVISION="$(git rev-parse HEAD)" "$context"

# Build the image with dev binaries, run it as the Helm chart does (read-only root filesystem, no capabilities) and check the web UI and uc in it
test-docker: (docker "dev")
    #!/usr/bin/env bash
    # The other checks of the server are in `cargo test`; this one checks the
    # image. /data is the image's volume. Turso writes temporary files to
    # /tmp: a tmpfs here, an emptyDir in the chart.
    set -euo pipefail
    id=$(docker run -d --read-only --cap-drop ALL --security-opt no-new-privileges \
        --tmpfs /tmp -p 127.0.0.1:18182:8080 {{ image }})
    trap 'status=$?; [ $status -eq 0 ] || docker logs "$id"; docker rm -f -v "$id" > /dev/null; exit $status' EXIT
    # The web UI, once the server answers; then uc writes to the database.
    curl -fs --retry 30 --retry-all-errors --retry-delay 1 -H 'Accept: text/html' http://127.0.0.1:18182/ | grep -q '<html'
    docker exec "$id" uc catalog create --name docker_test > /dev/null
    echo "The image works: uc-server, the web UI and uc, on a read-only root filesystem."

# Run the Rust tests of the workspace; with UC_TEST_POSTGRES_URL, on PostgreSQL
test-rust:
    cargo test --profile {{ profile }} --workspace --no-fail-fast

# Lint the Helm chart and run its unit tests (requires `helm plugin install https://github.com/helm-unittest/helm-unittest`)
test-helm:
    helm lint deploy/helm/uc-server
    helm unittest deploy/helm/uc-server

# Run the Python client's linter and formatter, and fix what they can
pre-commit:
    cd python/client && uv run --no-sync ruff check --fix .
    cd python/client && uv run --no-sync ruff format .

# Run the Python client's linter, formatter and type checker, and check uv.lock, in CI
pre-commit-check:
    uv lock --check
    cd python/client && uv run --no-sync ruff check .
    cd python/client && uv run --no-sync ruff format --check --diff .
    cd python/client && uv run --no-sync mypy .

# Generate the Rust client (crates/uc-client) and the Python bindings (python/client) from the protos in proto/; needs protoc
generate:
    cargo run -q --profile {{ profile }} -p uc-codegen

# Write the stubs of the Python client (python/client/unitycatalog/_core) from the Rust code, and format them with ruff
generate-stubs:
    #!/usr/bin/env bash
    # pyo3-stub-gen's generator runs Python, so it is built without the
    # `extension-module` feature. PYTHONHOME points it to Python's standard
    # library, which uv's builds do not find when embedded; LD_LIBRARY_PATH
    # to Python's shared library, on Linux. CI fails when the stubs differ
    # from the Rust code.
    set -euo pipefail
    if [ ! -x "{{ python }}" ]; then
        echo "No Python at {{ python }}. Make the venv: just venv" >&2
        exit 1
    fi
    home="$("{{ python }}" -c 'import sys; print(sys.base_prefix)')"
    PYO3_PYTHON="{{ python }}" PYTHONHOME="$home" \
        LD_LIBRARY_PATH="$home/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
        cargo run -q --profile {{ profile }} -p uc-py --no-default-features --features stub-gen --bin stub_gen
    "{{ python }}" -m ruff format --quiet python/client/unitycatalog/_core

# Write the references of the docs site, which docs-build and docs-serve include: the OpenAPI document of the HTTP APIs (docs/reference/openapi.json) from the protos in proto/, through a descriptor set that protoc writes with their comments; and the Python client's pages (docs/reference/python/) from the package's stubs
docs-reference:
    mkdir -p target/docs
    protoc --include_imports --include_source_info -I proto --descriptor_set_out=target/docs/api.binpb $(find proto/unitycatalog -name '*.proto' | sort)
    uv run --no-project --with protobuf --with googleapis-common-protos python docs/gen_openapi.py target/docs/api.binpb
    uv run --no-project --with griffe python docs/gen_python_reference.py

# Build the docs site (docs/, mkdocs.yml) into site/ with zensical; a warning, such as a broken link, fails the build
docs-build: docs-reference
    uvx zensical build --strict

# Serve the docs site on http://localhost:8000, and build it again after each change (the reference: `just docs-reference`)
docs-serve: docs-reference
    uvx zensical serve

# Publish the docs as `version`, and as `latest`, which the site's root opens: mike commits them to the gh-pages branch; push="" only commits
docs-deploy version push="--push": docs-reference
    uvx --from "{{ mike }}" --with zensical mike deploy {{ push }} --update-aliases {{ version }} latest
    uvx --from "{{ mike }}" --with zensical mike set-default {{ push }} latest
