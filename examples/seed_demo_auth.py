"""Dummy data for `just demo-auth`. The server makes the users alice and bob
from the `[[auth.users]]` of the uc.toml of examples/demo-auth/setup.py, and
alice is an admin. alice signs in, makes an external location for the RustFS
bucket and puts in the dummy data of seed_demo.py under it, so she owns it
all. `uc` writes 15 random rows into each table and a file into each volume,
with the credentials that the server gets from RustFS. Then alice lets bob
read the tables and volumes of sales.eu. It takes the folder of `uc dev`,
with the uc.toml of setup.py:

    uv run --no-sync examples/seed_demo_auth.py .uc-demo-auth

A server that has catalogs gets no dummy data.
"""

import argparse
import json
import os
import subprocess
import tomllib
import urllib.parse
import urllib.request
from pathlib import Path

from seed_demo import seed
from unitycatalog import Client
from unitycatalog.credentials import AwsIamRoleRequest
from unitycatalog.permissions import PermissionsChange

BUCKET = "s3://uc-demo"


def sign_in(client: Client, cli: dict, name: str) -> str:
    """A token of the server for the user `name`: an ID token from the
    provider for the user's password, which is the name at FerrisKey,
    exchanged for it (the token exchange of `uc auth login`)."""
    form = {
        "grant_type": "password",
        "client_id": cli["oauth_client_id"],
        "client_secret": cli["oauth_client_secret"],
        "username": name,
        "password": name,
        "scope": "openid email profile",
    }
    data = urllib.parse.urlencode(form).encode()
    with urllib.request.urlopen(cli["oauth_token_url"], data) as response:
        id_token = json.load(response)["id_token"]
    answer = client.exchange_token(
        "urn:ietf:params:oauth:grant-type:token-exchange",
        "urn:ietf:params:oauth:token-type:access_token",
        id_token,
        "urn:ietf:params:oauth:token-type:id_token",
    )
    return answer.access_token


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Put dummy data into a uc-server with sign-in."
    )
    parser.add_argument(
        "dir", type=Path, help="the folder of `uc dev`, such as .uc-demo-auth"
    )
    args = parser.parse_args()
    settings = args.dir / "uc.toml"
    cli = tomllib.loads(settings.read_text())["cli"]
    token = sign_in(Client(cli["server"]), cli, "alice")
    alice = Client(cli["server"], token=token)
    # An admin lists every catalog.
    existing = sorted(c.name for c in alice.list_catalogs())
    if existing:
        print(
            f"The server has catalogs, so it gets no dummy data: {', '.join(existing)}."
        )
        return
    # With authorization on, a catalog's storage root must be under an
    # external location. Its credential needs a role ARN, but RustFS takes
    # any: the server's [identity.aws] keys call AssumeRole, and the session
    # policy limits each credential to its path and operation.
    alice.create_credential(
        "rustfs",
        aws_iam_role=AwsIamRoleRequest("arn:aws:iam::000000000000:role/uc-demo"),
        purpose="STORAGE",
        comment="The server's root user at RustFS",
    )
    alice.create_external_location(
        "rustfs", BUCKET, "rustfs", comment="The bucket of the demo"
    )
    seed(alice, BUCKET)

    def uc(*command: str) -> None:
        """A `uc` command, as alice."""
        subprocess.run(
            ["uc", "--config", str(settings), *command],
            check=True,
            stdout=subprocess.DEVNULL,
            env={**os.environ, "UC_CLI_TOKEN": token},
        )

    for catalog in alice.list_catalogs():
        c = catalog.name
        for schema in alice.list_schemas(c):
            s = schema.name
            for table in alice.list_tables(c, s):
                uc("table", "write", "--full_name", f"{c}.{s}.{table.name}")
            for volume in alice.list_volumes(c, s):
                uc("volume", "write", "--full_name", f"{c}.{s}.{volume.name}")

    bob = "bob@example.com"
    alice.update_permissions(
        "catalog", "sales", changes=[PermissionsChange(bob, add=["USE CATALOG"])]
    )
    alice.update_permissions(
        "schema",
        "sales.eu",
        changes=[PermissionsChange(bob, add=["USE SCHEMA", "SELECT", "READ VOLUME"])],
    )
    print("alice's dummy data is in RustFS, and bob reads sales.eu.")


if __name__ == "__main__":
    main()
