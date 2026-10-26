"""polars and obstore create files in Unity Catalog volumes: the server keeps
the volume and its storage location, and the engines write the files there."""

import pytest
from _helpers import unique

from unitycatalog import BadRequestError, Client

pl = pytest.importorskip("polars")
obstore = pytest.importorskip("obstore")


def test_polars_writes_data_files_into_a_managed_volume(
    server, schema: tuple[str, str]
) -> None:
    catalog, schema_name = schema
    client = Client(server.address)
    volume = client.create_volume(catalog, schema_name, "exports", "MANAGED")
    assert volume.volume_id and volume.storage_location
    # Engines ask for credentials to reach the storage of the volume. A local
    # folder needs none: the answer has only its URL.
    credentials = client.generate_temporary_volume_credentials(
        volume.volume_id, "WRITE_VOLUME"
    )
    assert credentials.url == volume.storage_location

    cities = pl.DataFrame(
        {"city": ["Oslo", "Rome"], "population": [717_710, 2_748_109]}
    )
    cities.write_parquet(f"{volume.storage_location}/cities/part-0.parquet")
    cities.write_csv(f"{volume.storage_location}/cities.csv")
    assert pl.read_parquet(f"{volume.storage_location}/cities/*.parquet").equals(cities)
    assert pl.read_csv(f"{volume.storage_location}/cities.csv").equals(cities)


def test_obstore_puts_lists_and_deletes_files_in_an_external_volume(
    server, schema: tuple[str, str], tmp_path_factory: pytest.TempPathFactory
) -> None:
    catalog, schema_name = schema
    folder = tmp_path_factory.mktemp("landing")
    volume = Client(server.address).create_volume(
        catalog, schema_name, "landing", "EXTERNAL", storage_location=folder.as_uri()
    )
    store = obstore.store.from_url(volume.storage_location)
    obstore.put(store, "2026-10-16/events.json", b'{"event": "signup"}\n')
    obstore.put(store, "README.md", b"# Landing\n")
    assert sorted(meta["path"] for meta in obstore.list(store).collect()) == [
        "2026-10-16/events.json",
        "README.md",
    ]
    assert bytes(obstore.get(store, "README.md").bytes()) == b"# Landing\n"

    obstore.delete(store, "README.md")
    # The folder of the volume holds the file that is left.
    assert [path.name for path in folder.rglob("*") if path.is_file()] == [
        "events.json"
    ]


def test_files_api_pages_through_a_volume_on_s3(s3_server) -> None:
    client = Client(s3_server.address)
    catalog = unique("catalog")
    client.create_catalog(catalog, storage_root=f"s3://uc-test/{catalog}")
    client.create_schema("default", catalog)
    client.create_volume(catalog, "default", "landing", "MANAGED")
    root = f"/Volumes/{catalog}/default/landing"
    for name in ["a.csv", "b.csv", "c.csv", "sub/d.csv"]:
        client.upload_file(f"{root}/{name}", name.encode())

    # The server lists a page of S3 per request: its token is S3's.
    names: list[str] = []
    token = None
    while True:
        page = client.list_directory_contents(root, page_size=2, page_token=token)
        assert len(page.contents) <= 2
        names += [entry.name for entry in page.contents]
        token = page.next_page_token
        if not token:
            break
    assert sorted(names) == ["a.csv", "b.csv", "c.csv", "sub"]
    assert client.download_file(f"{root}/sub/d.csv").contents == b"sub/d.csv"

    # In object storage, a directory exists only through its files.
    client.create_directory(f"{root}/empty")
    assert client.get_directory_metadata(f"{root}/empty") is False
    with pytest.raises(BadRequestError):
        client.delete_directory(f"{root}/sub")
    client.delete_file(f"{root}/sub/d.csv")
    assert client.get_directory_metadata(f"{root}/sub") is False
    client.delete_directory(f"{root}/sub")

    # `recursive` deletes a directory and what it holds, in batches.
    for name in ["x/1.csv", "x/2.csv", "x/y/3.csv"]:
        client.upload_file(f"{root}/{name}", b"1")
    client.delete_directory(f"{root}/x", recursive=True)
    assert client.get_directory_metadata(f"{root}/x") is False
    left = client.list_directory_contents(root).contents
    assert sorted(entry.name for entry in left) == ["a.csv", "b.csv", "c.csv"]
