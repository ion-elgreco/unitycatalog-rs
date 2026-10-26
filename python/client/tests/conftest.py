"""Fixtures: a uc-server with an empty catalog in memory, and clients of it.

The server is `UC_TEST_SERVER_BIN`, or the debug build of this repository,
which cargo builds first. It gets no settings but its own: an empty `uc.toml`
and the `UC_*` variables here, with a signing key and a secrets key of
`uc-server keygen`, so that `uc-server token` gives the admin's token and
credentials can keep secrets.

With `UC_TEST_SERVER_URL` the tests use that server instead, such as the
upstream Java server (`just test-upstream`). Then only the tests marked
`upstream` run: those of the API that both servers have.

The tests in integrations/ use engines (polars, deltalake, obstore and
pyiceberg) from the `integrations` dependency group. A module whose engine
is not installed is skipped. For S3 they have `s3_server`, a second server that vends
credentials for a bucket of moto, a local S3 and STS. For GCS they have `gcs_server`, which
vends credentials for a bucket of floci-gcp, a local GCP emulator
(`UC_TEST_FLOCI_GCP_URL`); without it, they are skipped.
"""

import base64
import json
import os
import socket
import subprocess
import sys
import time
import typing
import urllib.error
import urllib.request
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path

import pytest
from _helpers import Calls, unique

from unitycatalog import Client, UnityCatalogError

REPO_ROOT = Path(__file__).resolve().parents[3]
SERVER_URL = os.environ.get("UC_TEST_SERVER_URL")


@dataclass(frozen=True)
class Server:
    address: str
    """The address of the server, such as `http://127.0.0.1:8080`: `Client`
    takes it, and engines add the path of the API to it."""
    url: str
    """The base URL of the Unity Catalog REST API."""
    token: str
    """A token of the server's `admin`, from `uc-server token`."""


def _server_binary() -> str:
    if binary := os.environ.get("UC_TEST_SERVER_BIN"):
        return str(Path(binary).resolve())
    subprocess.run(
        ["cargo", "build", "--quiet", "-p", "uc-server"], cwd=REPO_ROOT, check=True
    )
    name = "uc-server.exe" if os.name == "nt" else "uc-server"
    return str(REPO_ROOT / "target" / "debug" / name)


def _output(
    command: list[str], env: dict[str, str] | None = None, cwd: Path | None = None
) -> str:
    """The standard output of `command`, which must succeed."""
    done = subprocess.run(
        command, env=env, cwd=cwd, check=True, capture_output=True, text=True
    )
    return done.stdout


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _request(url: str, method: str = "GET") -> None:
    with urllib.request.urlopen(urllib.request.Request(url, method=method)):
        pass


def _post(url: str, body: dict[str, object]) -> dict[str, typing.Any]:
    """The JSON answer to a POST of `body`; 409 (it exists) gives `{}`."""
    request = urllib.request.Request(
        url, json.dumps(body).encode(), {"Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(request) as answer:
            return json.load(answer)
    except urllib.error.HTTPError as e:
        if e.code == 409:
            return {}
        raise


def _wait_for(
    name: str,
    process: subprocess.Popen[bytes],
    log_path: Path,
    ready: Callable[[], object],
) -> None:
    """Waits until `ready()` raises no error, as `process` answers."""
    deadline = time.monotonic() + 60
    while True:
        try:
            ready()
            return
        except (OSError, UnityCatalogError):
            if process.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(
                    f"{name} did not start:\n{log_path.read_text()}"
                ) from None
            time.sleep(0.1)


@contextmanager
def _running_server(root: Path, settings: dict[str, str]) -> Iterator[Server]:
    """A uc-server with its data in `root`, and `settings` (`UC_*` variables)
    on top of its own."""
    config = root / "uc.toml"
    config.write_text("")
    binary = _server_binary()
    key = root / "signing-key.pem"
    key.write_text(_output([binary, "keygen"]))
    secrets_key = root / "secrets-key.pem"
    secrets_key.write_text(_output([binary, "keygen", "--secrets"]))
    port = _free_port()
    env = {
        **os.environ,
        "UC_CONFIG": str(config),
        "UC_SERVER_BIND": f"127.0.0.1:{port}",
        "UC_SERVER_DATA_DIR": str(root),
        "UC_DB_URL": "memory",
        "UC_LOG_DIR": str(root / "logs"),
        "UC_AUTH_SIGNING_KEY_FILE": str(key),
        "UC_SECRETS_KEY_FILE": str(secrets_key),
        **settings,
    }
    address = f"http://127.0.0.1:{port}"
    url = f"{address}/api/2.1/unity-catalog/"
    log_path = root / "server.log"
    with log_path.open("w") as log:
        process = subprocess.Popen([binary], cwd=root, env=env, stdout=log, stderr=log)
        try:
            ready = Client(address).get_metastore_summary
            _wait_for("uc-server", process, log_path, ready)
            token = _output([binary, "token"], env=env, cwd=root).strip()
            yield Server(address=address, url=url, token=token)
        finally:
            process.terminate()
            process.wait(timeout=10)


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers", "upstream: also runs against the upstream Java server"
    )


def pytest_collection_modifyitems(
    config: pytest.Config, items: list[pytest.Item]
) -> None:
    """With `UC_TEST_SERVER_URL`, only the tests marked `upstream` run."""
    if not SERVER_URL:
        return
    other = [item for item in items if not item.get_closest_marker("upstream")]
    if other:
        config.hook.pytest_deselected(items=other)
        items[:] = [item for item in items if item.get_closest_marker("upstream")]


@pytest.fixture(scope="session")
def server(tmp_path_factory: pytest.TempPathFactory) -> Iterator[Server]:
    if SERVER_URL:
        address = SERVER_URL.rstrip("/")
        url = f"{address}/api/2.1/unity-catalog/"
        yield Server(address=address, url=url, token="")
        return
    with _running_server(tmp_path_factory.mktemp("uc-server"), {}) as server:
        yield server


@pytest.fixture(scope="session")
def s3(tmp_path_factory: pytest.TempPathFactory) -> Iterator[str]:
    """The URL of moto's S3 and STS, with the bucket `uc-test`. moto runs in a
    process of its own, as a cloud store would."""
    pytest.importorskip("moto")
    port = _free_port()
    url = f"http://127.0.0.1:{port}"
    log_path = tmp_path_factory.mktemp("moto") / "moto.log"
    with log_path.open("w") as log:
        process = subprocess.Popen(
            [sys.executable, "-m", "moto.server", "--port", str(port)],
            stdout=log,
            stderr=log,
        )
        try:
            _wait_for("moto", process, log_path, lambda: _request(f"{url}/moto-api/"))
            # moto's S3 takes a request without a signature.
            _request(f"{url}/uc-test", "PUT")
            yield url
        finally:
            process.terminate()
            process.wait(timeout=10)


@pytest.fixture(scope="session")
def s3_server(tmp_path_factory: pytest.TempPathFactory, s3: str) -> Iterator[Server]:
    """A uc-server that vends credentials for the bucket `uc-test` of `s3`: it
    assumes a role with moto's STS (`[identity.aws]`, `[locations.test]`)."""
    settings = {
        "UC_IDENTITY_AWS_ENDPOINT": s3,
        # moto takes any keys.
        "UC_IDENTITY_AWS_ACCESS_KEY": "testing",
        "UC_IDENTITY_AWS_SECRET_KEY": "testing",
        "UC_LOCATIONS_TEST_URL": "s3://uc-test",
        "UC_LOCATIONS_TEST_REGION": "us-east-1",
        "UC_LOCATIONS_TEST_AWS_IAM_ROLE_ARN": "arn:aws:iam::123456789012:role/uc-test",
    }
    with _running_server(tmp_path_factory.mktemp("uc-server-s3"), settings) as server:
        yield server


@dataclass(frozen=True)
class Gcs:
    url: str
    """The URL of floci-gcp: its GCS, OAuth, IAM Credentials and STS APIs."""
    json_key: str
    """The JSON key of a service account, whose token endpoint is floci-gcp's."""


@pytest.fixture(scope="session")
def gcs() -> Gcs:
    """floci-gcp, with the bucket `uc-test`. It checks the access boundaries
    of downscoped tokens, not the signatures of service account keys."""
    url = os.environ.get("UC_TEST_FLOCI_GCP_URL", "").rstrip("/")
    if not url:
        pytest.skip("needs floci-gcp: UC_TEST_FLOCI_GCP_URL")
    _post(f"{url}/storage/v1/b?project=floci-local", {"name": "uc-test"})
    accounts = f"{url}/v1/projects/floci-local/serviceAccounts"
    _post(accounts, {"accountId": "uc-server"})
    email = "uc-server@floci-local.iam.gserviceaccount.com"
    key = _post(f"{accounts}/{email}/keys", {})
    json_key = json.loads(base64.b64decode(key["privateKeyData"]))
    json_key["token_uri"] = f"{url}/token"
    # floci-gcp labels its PKCS #8 key as PKCS #1; Google's keys say PRIVATE KEY.
    json_key["private_key"] = json_key["private_key"].replace("RSA PRIVATE", "PRIVATE")
    return Gcs(url=url, json_key=json.dumps(json_key))


@pytest.fixture(scope="session")
def gcs_server(tmp_path_factory: pytest.TempPathFactory, gcs: Gcs) -> Iterator[Server]:
    """A uc-server that vends credentials for the bucket `uc-test` of `gcs`:
    the service account's token, downscoped at floci-gcp's STS
    (`identity.gcp.endpoint`, `[locations.test]`)."""
    root = tmp_path_factory.mktemp("uc-server-gcs")
    key_file = root / "key.json"
    key_file.write_text(gcs.json_key)
    settings = {
        "UC_IDENTITY_GCP_ENDPOINT": gcs.url,
        "UC_LOCATIONS_TEST_URL": "gs://uc-test",
        "UC_LOCATIONS_TEST_GCP_SERVICE_ACCOUNT_KEY": str(key_file),
    }
    with _running_server(root, settings) as server:
        yield server


@pytest.fixture(params=["sync", "async"])
def mode(request: pytest.FixtureRequest) -> str:
    return request.param


@pytest.fixture
def uc(server: Server, mode: str) -> Calls:
    """An anonymous client, in the mode of the test."""
    return Calls(Client(server.address), mode)


@pytest.fixture
def admin(server: Server, mode: str) -> Calls:
    """A client signed in as the server's `admin`, in the mode of the test."""
    return Calls(Client(server.address, token=server.token), mode)


@pytest.fixture
def storage_root(tmp_path: Path) -> str:
    """A new directory for managed storage, as the server sees it: `tmp_path`,
    or a directory in /tmp of the server of `UC_TEST_SERVER_URL`, which can
    run on another machine or in a container."""
    if SERVER_URL:
        return f"file:///tmp/{unique('storage')}"
    return tmp_path.as_uri()


@pytest.fixture
def catalog(server: Server, storage_root: str) -> Iterator[str]:
    """A new catalog with managed storage in a temporary directory."""
    client = Client(server.address)
    name = unique("catalog")
    client.create_catalog(name, storage_root=storage_root)
    yield name
    client.catalog(name).delete(force=True)


@pytest.fixture
def schema(server: Server, catalog: str) -> tuple[str, str]:
    """`(catalog, schema)`: a new schema in a new catalog."""
    Client(server.address).create_schema("default", catalog)
    return catalog, "default"
