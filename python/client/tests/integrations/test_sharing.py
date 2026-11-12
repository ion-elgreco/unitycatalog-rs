"""A recipient reads a shared Delta table on S3: the server reads the Delta
log with credentials that it vends to itself, and answers the files as
presigned URLs (Delta Sharing) or gives read-only credentials (Open Sharing)."""

import io
import json
import typing
import urllib.error
import urllib.request

import pytest
from _helpers import unique
from test_tables import create_managed_table

from unitycatalog import Client
from unitycatalog.shares import SharedDataObject

pl = pytest.importorskip("polars")
deltalake = pytest.importorskip("deltalake")


def call(url: str, token: str, body: dict[str, typing.Any] | None = None) -> bytes:
    """The answer of the protocol at `url` for the recipient of `token`: a
    `POST` with `body`, else a `GET`."""
    data = None if body is None else json.dumps(body).encode()
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
    request = urllib.request.Request(url, data=data, headers=headers)
    with urllib.request.urlopen(request) as response:
        return response.read()


def test_a_recipient_reads_a_shared_table_on_s3(s3: str, s3_server) -> None:
    client = Client(s3_server.address)
    catalog = unique("catalog")
    client.create_catalog(catalog, storage_root=f"s3://uc-test/{catalog}")
    client.create_schema("default", catalog)
    staging = client.create_staging_table("orders", catalog, "default")
    assert staging.id
    vended = client.generate_temporary_table_credentials(staging.id, "READ_WRITE")
    aws = vended.aws_temp_credentials
    assert aws
    endpoint = {
        "aws_endpoint_url": s3,
        "aws_allow_http": "true",
        "aws_region": "us-east-1",
    }
    credentials = {
        **endpoint,
        "aws_access_key_id": aws.access_key_id,
        "aws_secret_access_key": aws.secret_access_key,
        "aws_session_token": aws.session_token,
    }
    orders = pl.DataFrame({"order_id": [1, 2], "amount": [9.5, 3.25]})
    deltalake.write_deltalake(
        staging.staging_location, orders, storage_options=credentials
    )
    create_managed_table(client, staging, credentials)

    share, recipient = unique("share"), unique("recipient")
    client.create_share(share)
    shared = SharedDataObject(f"{catalog}.default.orders", "sales", "orders")
    client.share(share).update(add_objects=[shared])
    client.create_recipient(recipient)
    client.recipient(recipient).update(add_shares=[share])
    profile = client.recipient(recipient).create_recipient_profile()
    token = profile.bearer_token

    # Delta Sharing: the files of the table, as URLs that need no credentials.
    table = f"{profile.endpoint}/shares/{share}/schemas/sales/tables/orders"
    lines = [
        json.loads(line) for line in call(f"{table}/query", token, {}).splitlines()
    ]
    assert [next(iter(line)) for line in lines[:2]] == ["protocol", "metaData"]
    files = [line["file"] for line in lines[2:]]
    assert files and all(file["url"].startswith(s3) for file in files)
    assert all("X-Amz-Signature=" in file["url"] for file in files)
    read = [
        pl.read_parquet(io.BytesIO(urllib.request.urlopen(file["url"]).read()))
        for file in files
    ]
    assert pl.concat(read).sort("order_id").equals(orders)
    assert sum(json.loads(file["stats"])["numRecords"] for file in files) == 2

    # Open Sharing: credentials that read the location of the table.
    address = s3_server.address
    table = f"{address}/api/1.0/opensharing/shares/{share}/schemas/sales/tables/orders"
    vended = json.loads(call(f"{table}/temporary-table-credentials", token, {}))
    assert vended["url"] == staging.staging_location
    aws = vended["aws_temp_credentials"]
    read_only = {
        **endpoint,
        "aws_access_key_id": aws["access_key_id"],
        "aws_secret_access_key": aws["secret_access_key"],
        "aws_session_token": aws["session_token"],
    }
    by_credentials = pl.read_delta(vended["url"], storage_options=read_only)
    assert by_credentials.sort("order_id").equals(orders)

    # History is not shared, so an older version is refused.
    with pytest.raises(urllib.error.HTTPError) as refused:
        call(
            f"{profile.endpoint}/shares/{share}/schemas/sales/tables/orders/query",
            token,
            {"version": 0},
        )
    assert refused.value.code == 400
