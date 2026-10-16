"""Each method of the client against a uc-server, as `name` and `name_async`."""

import asyncio
import time
import typing
from pathlib import Path

import pytest
from _helpers import Calls, unique

from unitycatalog import (
    Client,
    ColumnInfo,
    Dependency,
    FunctionParameterInfo,
    PermissionsChange,
    UcError,
)
from unitycatalog.types import ModelVersionStatus


def test_catalogs(uc: Calls, tmp_path: Path) -> None:
    name = unique("catalog")
    created = uc.create_catalog(
        name,
        comment="primary",
        properties={"team": "data"},
        storage_root=tmp_path.as_uri(),
    )
    assert (created.name, created.comment) == (name, "primary")
    assert created.properties == {"team": "data"}
    assert created.id and created.created_at and created.storage_location
    assert uc.get_catalog(name) == created
    assert name in [c.name for c in uc.list_catalogs()]

    updated = uc.update_catalog(name, comment="edited", new_name=f"{name}_2")
    assert (updated.name, updated.comment, updated.id) == (
        f"{name}_2",
        "edited",
        created.id,
    )
    assert uc.delete_catalog(f"{name}_2") is None
    with pytest.raises(UcError) as error:
        uc.get_catalog(f"{name}_2")
    assert error.value.code == "CATALOG_NOT_FOUND"


def test_schemas(uc: Calls, catalog: str) -> None:
    created = uc.create_schema(
        "s", catalog, comment="raw data", properties={"layer": "bronze"}
    )
    assert (created.full_name, created.comment) == (f"{catalog}.s", "raw data")
    assert created.properties == {"layer": "bronze"}
    assert uc.get_schema(f"{catalog}.s") == created
    assert [s.name for s in uc.list_schemas(catalog)] == ["s"]

    updated = uc.update_schema(f"{catalog}.s", comment="edited", new_name="s2")
    assert (updated.full_name, updated.comment) == (f"{catalog}.s2", "edited")
    uc.delete_schema(f"{catalog}.s2")
    assert uc.list_schemas(catalog) == []


def test_tables_and_views(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    columns = [
        ColumnInfo(
            "id",
            type_text="bigint",
            type_json='{"name":"id","type":"long","nullable":false,"metadata":{}}',
            type_name="LONG",
            position=0,
            nullable=False,
        ),
        ColumnInfo(
            "email",
            type_text="string",
            type_json='{"name":"email","type":"string","nullable":true,"metadata":{}}',
            type_name="STRING",
            position=1,
            nullable=True,
        ),
    ]
    staging = uc.create_staging_table("t", catalog, schema_name)
    assert staging.id and staging.staging_location
    table = uc.create_table(
        "t",
        catalog,
        schema_name,
        "MANAGED",
        columns=columns,
        data_source_format="DELTA",
        storage_location=staging.staging_location,
        comment="users",
        properties={"io.unitycatalog.tableId": staging.id},
    )
    assert (table.table_type, table.data_source_format) == ("MANAGED", "DELTA")
    assert table.columns == columns
    assert table.table_id == staging.id
    full_name = f"{catalog}.{schema_name}.t"
    assert uc.get_table(full_name) == table

    view = uc.create_table(
        "v",
        catalog,
        schema_name,
        "VIEW",
        columns=columns,
        view_definition="select * from t",
        view_dependencies=[Dependency(table_full_name=full_name)],
    )
    assert view.view_definition == "select * from t"
    assert view.view_dependencies == [Dependency(table_full_name=full_name)]
    assert sorted(t.name for t in uc.list_tables(catalog, schema_name)) == ["t", "v"]

    uc.delete_table(f"{catalog}.{schema_name}.v")
    uc.delete_table(full_name)
    assert uc.list_tables(catalog, schema_name) == []


def test_volumes(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    created = uc.create_volume("v", catalog, schema_name, "MANAGED", comment="scratch")
    assert created.full_name == f"{catalog}.{schema_name}.v"
    assert (created.volume_type, created.comment) == ("MANAGED", "scratch")
    assert created.storage_location
    assert uc.get_volume(created.full_name) == created
    assert [v.name for v in uc.list_volumes(catalog, schema_name)] == ["v"]

    updated = uc.update_volume(created.full_name, comment="edited", new_name="v2")
    assert (updated.name, updated.comment) == ("v2", "edited")
    uc.delete_volume(f"{catalog}.{schema_name}.v2")
    assert uc.list_volumes(catalog, schema_name) == []


def test_functions(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    parameters = [
        FunctionParameterInfo("x", "long", "LONG", type_json='"long"', position=0)
    ]
    created = uc.create_function(
        "inc",
        catalog,
        schema_name,
        "SQL",
        input_params=parameters,
        data_type="LONG",
        full_data_type="long",
        routine_definition="x + 1",
        is_deterministic=True,
        sql_data_access="NO_SQL",
        is_null_call=False,
        specific_name="inc",
        comment="adds one",
    )
    assert created.full_name == f"{catalog}.{schema_name}.inc"
    assert created.input_params == parameters
    assert (created.routine_body, created.parameter_style, created.security_type) == (
        "SQL",
        "S",
        "DEFINER",
    )
    assert (created.data_type, created.sql_data_access) == ("LONG", "NO_SQL")
    assert uc.get_function(created.full_name) == created
    assert [f.name for f in uc.list_functions(catalog, schema_name)] == ["inc"]

    uc.delete_function(created.full_name)
    assert uc.list_functions(catalog, schema_name) == []


def test_registered_models_and_versions(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    full_name = f"{catalog}.{schema_name}.m"
    created = uc.create_registered_model("m", catalog, schema_name, comment="first")
    assert (created.full_name, created.comment) == (full_name, "first")
    assert created.storage_location
    assert uc.get_registered_model(full_name) == created
    assert full_name in [m.full_name for m in uc.list_registered_models()]
    in_schema = uc.list_registered_models(catalog_name=catalog, schema_name=schema_name)
    assert [m.full_name for m in in_schema] == [full_name]

    version = uc.create_model_version(
        "m", catalog, schema_name, "s3://bucket/m/v1", run_id="run-1", comment="v1"
    )
    assert (version.version, version.run_id, version.comment) == (1, "run-1", "v1")
    assert version.status in typing.get_args(ModelVersionStatus)
    assert [v.id for v in uc.list_model_versions(full_name)] == [version.id]

    updated = uc.update_registered_model(full_name, comment="edited", new_name="m2")
    assert (updated.name, updated.comment) == ("m2", "edited")
    with pytest.raises(UcError):
        uc.delete_registered_model(f"{catalog}.{schema_name}.m2")
    uc.delete_registered_model(f"{catalog}.{schema_name}.m2", force=True)
    assert full_name not in [m.full_name for m in uc.list_registered_models()]


def test_model_versions(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    full_name = f"{catalog}.{schema_name}.m"
    uc.create_registered_model("m", catalog, schema_name)
    created = uc.create_model_version("m", catalog, schema_name, "s3://bucket/m/v1")
    assert created.status == "PENDING_REGISTRATION"
    assert uc.get_model_version(full_name, 1) == created

    updated = uc.update_model_version(full_name, 1, comment="first")
    assert (updated.id, updated.comment) == (created.id, "first")
    finalized = uc.finalize_model_version(full_name, 1)
    assert (finalized.status, finalized.comment) == ("READY", "first")

    assert uc.delete_model_version(full_name, 1) is None
    assert uc.list_model_versions(full_name) == []
    with pytest.raises(UcError) as error:
        uc.get_model_version(full_name, 1)
    assert error.value.code == "NOT_FOUND"


def test_credentials_and_external_locations(uc: Calls) -> None:
    credential_name = unique("credential")
    role = "arn:aws:iam::123456789012:role/uc"
    credential = uc.create_credential(
        credential_name, purpose="STORAGE", aws_iam_role_arn=role, comment="prod"
    )
    assert (credential.purpose, credential.comment) == ("STORAGE", "prod")
    assert credential.aws_iam_role is not None
    assert credential.aws_iam_role.role_arn == role
    assert credential.aws_iam_role.external_id
    assert uc.get_credential(credential_name) == credential
    assert credential_name in [c.name for c in uc.list_credentials()]

    updated = uc.update_credential(
        credential_name, comment="edited", aws_iam_role_arn=f"{role}2"
    )
    assert updated.comment == "edited"
    assert updated.aws_iam_role is not None
    assert updated.aws_iam_role.role_arn == f"{role}2"
    assert updated.aws_iam_role.external_id != credential.aws_iam_role.external_id

    location_name = unique("location")
    url = f"s3://bucket/{location_name}"
    location = uc.create_external_location(
        location_name, url, credential_name, comment="lake"
    )
    assert (location.url, location.credential_name) == (url, credential_name)
    assert location.credential_id == credential.id
    assert uc.get_external_location(location_name) == location
    assert location_name in [loc.name for loc in uc.list_external_locations()]
    moved = uc.update_external_location(
        location_name, comment="edited", new_name=f"{location_name}_2"
    )
    assert (moved.name, moved.comment) == (f"{location_name}_2", "edited")

    with pytest.raises(UcError) as error:
        uc.delete_credential(credential_name)
    assert error.value.code == "INVALID_ARGUMENT"
    uc.delete_external_location(f"{location_name}_2")
    uc.delete_credential(credential_name)
    assert credential_name not in [c.name for c in uc.list_credentials()]


def test_temporary_credentials(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    staging = uc.create_staging_table("t", catalog, schema_name)
    table = uc.create_table(
        "t",
        catalog,
        schema_name,
        "MANAGED",
        data_source_format="DELTA",
        storage_location=staging.staging_location,
        properties={"io.unitycatalog.tableId": staging.id},
    )
    assert table.table_id and table.storage_location
    # Local storage needs no cloud credentials: only the URL comes back.
    credentials = uc.generate_temporary_table_credentials(table.table_id, "READ")
    assert credentials.url == table.storage_location
    assert credentials.aws_temp_credentials is None
    assert credentials.azure_user_delegation_sas is None
    assert credentials.gcp_oauth_token is None

    volume = uc.create_volume("v", catalog, schema_name, "MANAGED")
    assert volume.volume_id
    credentials = uc.generate_temporary_volume_credentials(
        volume.volume_id, "WRITE_VOLUME"
    )
    assert credentials.url == volume.storage_location

    credentials = uc.generate_temporary_path_credentials(
        table.storage_location, "PATH_READ"
    )
    assert credentials.url == table.storage_location
    assert repr(credentials) == f"TemporaryCredentials(url={table.storage_location!r})"


def test_permissions(uc: Calls, catalog: str) -> None:
    user = f"{unique('user')}@example.com"
    other = f"{unique('user')}@example.com"
    uc.create_user(user)
    uc.create_user(other)
    assert uc.get_permissions("catalog", catalog) == []

    grant = PermissionsChange(user, add=["USE CATALOG", "CREATE SCHEMA"])
    granted = uc.update_permissions("catalog", catalog, [grant])
    assert [(a.principal, sorted(a.privileges)) for a in granted] == [
        (user, ["CREATE SCHEMA", "USE CATALOG"])
    ]
    assert uc.get_permissions("catalog", catalog) == granted
    assert uc.get_permissions("catalog", catalog, principal=user) == granted
    uc.update_permissions(
        "catalog", catalog, [PermissionsChange(other, add=["USE CATALOG"])]
    )
    assert uc.get_permissions("catalog", catalog, principal=user) == granted

    revoke = [
        PermissionsChange(user, remove=["USE CATALOG", "CREATE SCHEMA"]),
        PermissionsChange(other, remove=["USE CATALOG"]),
    ]
    assert uc.update_permissions("catalog", catalog, revoke) == []


def test_metastore_summary(uc: Calls) -> None:
    summary = uc.metastore_summary()
    assert summary.metastore_id
    assert uc.metastore_summary() == summary


def test_personal_access_tokens(admin: Calls, server) -> None:
    created = admin.create_pat(unique("token"))
    assert created.token_value and created.token_prefix and created.owner
    assert created.token_value not in repr(created)
    signed_in = Client(server.url).with_bearer(created.token_value)
    assert signed_in.list_pats()[0].id == created.id

    listed = {p.id: p for p in admin.list_pats()}
    assert listed[created.id].token_value is None
    admin.revoke_pat(created.id)
    assert created.id not in [p.id for p in admin.list_pats()]


def test_scim_users(uc: Calls) -> None:
    email = f"{unique('user')}@example.com"
    created = uc.create_user(email, display_name="Ada", external_id=email)
    assert created.id and created.created
    assert (created.user_name, created.emails) == (email, [email])
    assert (created.display_name, created.external_id) == ("Ada", email)
    assert created.active is True
    assert uc.get_user(created.id) == created
    assert uc.list_users(filter=f'userName eq "{email}"') == [created]
    assert len(uc.list_users(start_index=1, count=1)) == 1

    updated = uc.update_user(created.id, display_name="Ada L")
    assert (updated.display_name, updated.user_name) == ("Ada L", email)
    assert updated.external_id == email
    # As in upstream, a deleted user stays, disabled.
    assert uc.delete_user(created.id) is None
    assert uc.get_user(created.id).active is False
    assert uc.update_user(created.id, active=True).active is True


def test_me(admin: Calls, uc: Calls) -> None:
    # The server's `admin` is a caller, but without authorization no user.
    with pytest.raises(UcError) as error:
        admin.me()
    assert error.value.code == "NOT_FOUND"
    # Without a token the server knows no caller.
    with pytest.raises(UcError):
        uc.me()


def test_scim_groups(uc: Calls) -> None:
    name = unique("group")
    created = uc.create_group(name)
    assert created.id
    assert (created.display_name, created.member_ids) == (name, [])
    assert uc.get_group(created.id) == created
    assert uc.list_groups(filter=f'displayName eq "{name}"') == [created]

    assert uc.delete_group(created.id) is None
    assert created.id not in [g.id for g in uc.list_groups()]
    with pytest.raises(UcError) as error:
        uc.get_group(created.id)
    assert error.value.code == "NOT_FOUND"


def test_backup_and_restore(admin: Calls, uc: Calls, catalog: str) -> None:
    snapshot = admin.backup()
    names = [row.get("name") for row in snapshot["tables"]["catalog"]]
    assert catalog in names
    restored = admin.restore(snapshot)
    assert restored.imported["catalog"] == len(names)
    assert restored.skipped == []
    assert uc.get_catalog(catalog).name == catalog

    # Without a token there is no admin, even with authorization off.
    with pytest.raises(UcError) as error:
        uc.backup()
    assert error.value.code == "UNAUTHENTICATED"


def test_audit_events(admin: Calls, catalog: str) -> None:
    path = "/api/2.1/unity-catalog/catalogs"
    # The server records events in the background.
    deadline = time.monotonic() + 10
    while True:
        events = admin.list_audit_events(
            principal="anonymous", method="POST", path_prefix=path
        )
        if events or time.monotonic() > deadline:
            break
        time.sleep(0.1)
    assert events
    assert {(e.principal, e.method, e.path) for e in events} == {
        ("anonymous", "POST", path)
    }
    assert all(e.id and e.status == 200 and e.timestamp for e in events)
    assert len(admin.list_audit_events(max_results=1)) == 1


def test_iceberg_rest_catalog(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, namespace = schema
    config = uc.iceberg_config(catalog)
    assert config.overrides["prefix"] == f"catalogs/{catalog}"
    assert config.endpoints
    assert uc.list_iceberg_namespaces(catalog) == [[namespace]]
    assert uc.list_iceberg_tables(catalog, namespace) == []

    schema_json = {
        "type": "struct",
        "schema-id": 0,
        "fields": [{"id": 1, "name": "id", "required": True, "type": "long"}],
    }
    uc.raw_json(
        "POST",
        f"iceberg/v1/catalogs/{catalog}/namespaces/{namespace}/tables",
        body={"name": "t", "schema": schema_json},
    )
    [table] = uc.list_iceberg_tables(catalog, namespace)
    assert (table.namespace, table.name) == ([namespace], "t")
    loaded = uc.load_iceberg_table(catalog, namespace, "t")
    assert loaded["metadata-location"]
    assert loaded["metadata"]["format-version"] in (1, 2)
    with pytest.raises(UcError) as error:
        uc.load_iceberg_table(catalog, namespace, "missing")
    assert error.value.code == "NOT_FOUND"


def test_delta_commits(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    staging = uc.create_staging_table("t", catalog, schema_name)
    table = uc.create_table(
        "t",
        catalog,
        schema_name,
        "MANAGED",
        data_source_format="DELTA",
        storage_location=staging.staging_location,
        properties={"io.unitycatalog.tableId": staging.id},
    )
    assert table.table_id and table.storage_location
    assert uc.get_delta_commits(table.table_id, table.storage_location, 0) == {
        "commits": [],
        "latest_table_version": 0,
    }

    now = int(time.time() * 1000)
    commit_info = {
        "version": 1,
        "timestamp": now,
        "file_name": "00000000000000000001.uuid.json",
        "file_size": 10,
        "file_modification_timestamp": now,
    }
    commit = {
        "table_id": table.table_id,
        "table_uri": table.storage_location,
        "commit_info": commit_info,
    }
    assert uc.commit_delta(commit) is None
    commits = uc.get_delta_commits(
        table.table_id, table.storage_location, 1, end_version=1
    )
    assert commits == {"commits": [commit_info], "latest_table_version": 1}

    missing = {**commit, "table_id": "00000000-0000-0000-0000-000000000000"}
    with pytest.raises(UcError) as error:
        uc.commit_delta(missing)
    assert error.value.code == "TABLE_NOT_FOUND"


def test_raw_json(uc: Calls, catalog: str) -> None:
    listed = uc.raw_json("GET", "catalogs")
    assert catalog in [c["name"] for c in listed["catalogs"]]
    assert uc.raw_json("get", f"catalogs/{catalog}")["name"] == catalog
    with pytest.raises(UcError) as error:
        uc.raw_json("GET", f"catalogs/{catalog}_missing")
    assert error.value.code == "CATALOG_NOT_FOUND"
    page = uc.raw_json("GET", "catalogs", query={"max_results": "1"})
    assert len(page["catalogs"]) == 1


def test_post_form(uc: Calls) -> None:
    form = {
        "grant_type": "urn:ietf:params:oauth:grant-type:token-exchange",
        "subject_token": "not-a-token",
    }
    with pytest.raises(UcError):
        uc.post_form("/api/1.0/unity-control/auth/tokens", form)


def test_server_errors(uc: Calls) -> None:
    name = unique("missing")
    with pytest.raises(UcError) as error:
        uc.get_catalog(name)
    assert error.value.code == "CATALOG_NOT_FOUND"
    assert name in error.value.message
    assert str(error.value) == f"CATALOG_NOT_FOUND: {error.value.message}"


def test_async_requests_in_one_event_loop(server, catalog: str) -> None:
    client = Client(server.url)

    async def main() -> list[str | None]:
        requests = [client.get_catalog_async(catalog) for _ in range(20)]
        return [c.name for c in await asyncio.gather(*requests)]

    assert asyncio.run(main()) == [catalog] * 20


def test_unreachable_server(mode: str) -> None:
    client = Calls(Client(f"http://127.0.0.1:{_closed_port()}/"), mode)
    with pytest.raises(UcError) as error:
        client.list_catalogs()
    assert error.value.code == "INTERNAL"


def _closed_port() -> int:
    import socket

    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
