"""polars and obstore create files in Unity Catalog volumes: the server keeps
the volume and its storage location, and the engines write the files there."""

import pytest

from unitycatalog import Client

pl = pytest.importorskip("polars")
obstore = pytest.importorskip("obstore")


def test_polars_writes_data_files_into_a_managed_volume(
    server, schema: tuple[str, str]
) -> None:
    catalog, schema_name = schema
    client = Client(server.url)
    volume = client.create_volume("exports", catalog, schema_name, "MANAGED")
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
    volume = Client(server.url).create_volume(
        "landing", catalog, schema_name, "EXTERNAL", storage_location=folder.as_uri()
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
