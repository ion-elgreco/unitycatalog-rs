"""The services of `just demo-auth` (compose.yaml). FerrisKey gets the realm
`unitycatalog`, with a client for uc-server and uc, and the users alice and
bob, whose passwords are their names; RustFS gets the bucket `uc-demo`. Then
the settings that use them, in a uc.toml for `uc dev` and `uc`:

    uv run --no-sync examples/demo-auth/setup.py http://localhost:8090 \\
        http://localhost:9000 http://localhost:8080 8091 .uc-demo-auth/uc.toml

The arguments are FerrisKey, RustFS, uc-server, the port of `uc auth login`
and the file. FerrisKey must be new: the realm must not exist yet.
"""

import argparse
import json
import subprocess
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

REALM = "unitycatalog"
CLIENT_ID = "unitycatalog"
# The user names, first names and last names. A user's email is
# <name>@example.com.
USERS = [("alice", "Alice", "Smith"), ("bob", "Bob", "Jones")]
# The bucket, and the root user of RustFS (compose.yaml).
BUCKET = "uc-demo"
S3_ACCESS_KEY = "uc-demo"
S3_SECRET_KEY = "uc-demo-secret"


def call(
    method: str,
    url: str,
    token: str | None = None,
    body: Any = None,
    form: dict[str, str] | None = None,
) -> Any:
    """A request to FerrisKey's API, with a JSON body or a form; its JSON
    answer."""
    headers = {}
    data = None
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if body is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(body).encode()
    if form is not None:
        data = urllib.parse.urlencode(form).encode()
    request = urllib.request.Request(url, data, headers, method=method)
    try:
        with urllib.request.urlopen(request) as response:
            return json.load(response)
    except urllib.error.HTTPError as e:
        raise SystemExit(f"{method} {url}: HTTP {e.code} {e.read().decode()}")


def set_up_ferriskey(ferriskey: str, server: str, cli_port: int) -> str:
    """The realm, its client and its users; the client's secret."""
    api = f"{ferriskey}/api"
    realm = f"{api}/realms/{REALM}"
    # The admin that FerrisKey makes at its first start.
    admin = call(
        "POST",
        f"{api}/realms/master/protocol/openid-connect/token",
        form={
            "grant_type": "password",
            "client_id": "admin-cli",
            "username": "admin",
            "password": "admin",
        },
    )["access_token"]
    call(
        "POST", f"{api}/realms", admin, {"name": REALM, "display_name": "Unity Catalog"}
    )
    # Any password: the users' passwords are their names.
    call(
        "PUT",
        f"{realm}/password-policy",
        admin,
        {
            "min_length": 1,
            "require_uppercase": False,
            "require_lowercase": False,
            "require_number": False,
            "require_special": False,
            "min_entropy_bits": 0,
            "forbid_common": False,
        },
    )
    client = call(
        "POST",
        f"{realm}/clients",
        admin,
        {
            "client_id": CLIENT_ID,
            "name": "Unity Catalog",
            "client_type": "confidential",
            "protocol": "openid-connect",
            "enabled": True,
            # examples/seed_demo_auth.py signs alice in with her password.
            "direct_access_grants_enabled": True,
            # uc-server sends a PKCE challenge; `uc auth login` sends none.
            "require_pkce": False,
        },
    )
    # The sign-in of uc-server, and that of `uc auth login`.
    for uri in [f"{server}/auth/callback", f"http://localhost:{cli_port}"]:
        call(
            "POST",
            f"{realm}/clients/{client['id']}/redirects",
            admin,
            {"value": uri, "enabled": True},
        )
    # The web UI's "Log out" ends the FerrisKey session too (oidc_end_session
    # below), which then returns the browser to the web UI.
    call(
        "POST",
        f"{realm}/clients/{client['id']}/post-logout-redirects",
        admin,
        {"value": f"{server}/", "enabled": True},
    )
    for name, first, last in USERS:
        user = call(
            "POST",
            f"{realm}/users",
            admin,
            {
                "username": name,
                "email": f"{name}@example.com",
                "firstname": first,
                "lastname": last,
                "email_verified": True,
            },
        )["data"]
        call(
            "PUT",
            f"{realm}/users/{user['id']}/reset-password",
            admin,
            {"credential_type": "password", "value": name, "temporary": False},
        )
    return client["client_secret"]


def create_bucket(rustfs: str) -> None:
    """The bucket; RustFS also answers 200 when it is there. curl signs the
    request (AWS Signature Version 4), which Python's library does not."""
    subprocess.run(
        [
            "curl",
            "-fsS",
            "-o",
            "/dev/null",
            "--aws-sigv4",
            "aws:amz:us-east-1:s3",
            "--user",
            f"{S3_ACCESS_KEY}:{S3_SECRET_KEY}",
            "-X",
            "PUT",
            f"{rustfs}/{BUCKET}",
        ],
        check=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Set up the services of the demo.")
    parser.add_argument("ferriskey", help="FerrisKey, such as http://localhost:8090")
    parser.add_argument("rustfs", help="RustFS, such as http://localhost:9000")
    parser.add_argument("server", help="uc-server, such as http://localhost:8080")
    parser.add_argument(
        "cli_port", type=int, help="the port that `uc auth login` listens on"
    )
    parser.add_argument("settings", type=Path, help="the uc.toml to write")
    args = parser.parse_args()
    ferriskey = args.ferriskey.rstrip("/")
    rustfs = args.rustfs.rstrip("/")
    server = args.server.rstrip("/")

    secret = set_up_ferriskey(ferriskey, server, args.cli_port)
    create_bucket(rustfs)
    provider = call(
        "GET", f"{ferriskey}/api/realms/{REALM}/.well-known/openid-configuration"
    )
    issuer = provider["issuer"]
    # A TOML string, as JSON writes it.
    q = json.dumps
    users = "\n".join(
        f"[[auth.users]]\nemail = {q(f'{name}@example.com')}\nname = {q(f'{first} {last}')}\n"
        for name, first, last in USERS
    )
    args.settings.parent.mkdir(parents=True, exist_ok=True)
    args.settings.write_text(f"""\
# The settings of `just demo-auth`, from examples/demo-auth/setup.py: users
# sign in with FerrisKey, and the data is in RustFS. Each run writes them
# again, with the client secret of a new FerrisKey.

[auth]
enabled = true
mode = "oidc"
public_url = {q(server)}
oidc_issuer = {q(issuer)}
oidc_client_id = {q(CLIENT_ID)}
oidc_client_secret = {q(secret)}
# "Log out" in the web UI ends the FerrisKey session too.
oidc_end_session = true
# The token exchange, which `uc auth login` uses.
allowed_issuers = [{q(issuer)}]
audiences = [{q(CLIENT_ID)}]
admin_principals = ["alice@example.com"]

# The users, which the server makes at start when they are missing.
{users}
# The server's identity at RustFS, which calls AssumeRole for the
# credentials of each request. uc reads the endpoint too.
[identity.aws]
endpoint = {q(rustfs)}
access_key = {q(S3_ACCESS_KEY)}
secret_key = {q(S3_SECRET_KEY)}

# The managed data of catalogs without a storage root, such as those that the
# web UI makes.
[storage]
tables_root = "s3://{BUCKET}/managed"
models_root = "s3://{BUCKET}/managed"

[cli]
server = {q(server)}
oauth_authorization_url = {q(provider["authorization_endpoint"])}
oauth_token_url = {q(provider["token_endpoint"])}
oauth_client_id = {q(CLIENT_ID)}
oauth_client_secret = {q(secret)}
oauth_redirect_port = {args.cli_port}
""")
    print(
        f"FerrisKey has the realm {REALM}, its client and the users alice and bob; "
        f"RustFS has the bucket {BUCKET}."
    )


if __name__ == "__main__":
    main()
