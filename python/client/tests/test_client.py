"""Each method of the client against a uc-server, as `name` and `name_async`."""

import asyncio
import json
import time

import pytest
from _helpers import Calls, unique

from unitycatalog import (
    BadRequestError,
    Client,
    ConflictError,
    DeltaMaintenanceOperation,
    ModelVersionStatus,
    NotFoundError,
    ServerError,
    UnauthenticatedError,
    UnityCatalogError,
)
from unitycatalog.credentials import (
    AwsIamRoleRequest,
    AzureManagedIdentity,
    AzureServicePrincipalRequest,
    GcpServiceAccount,
    GcpServiceAccountKeyRequest,
)
from unitycatalog.delta import DeltaCommitInfo
from unitycatalog.functions import (
    CreateFunction,
    FunctionParameterInfo,
    FunctionParameterInfos,
)
from unitycatalog.iceberg import (
    IcebergSchema,
    IcebergStructField,
    IcebergTableIdentifier,
)
from unitycatalog.permissions import PermissionsChange
from unitycatalog.scim import (
    ScimEmail,
    ScimGroupMember,
    ScimPatchOperation,
    ScimUser,
)
from unitycatalog.shares import SharedDataObject
from unitycatalog.tables import (
    ColumnInfo,
    Dependency,
    DependencyList,
    TableDependency,
    UpdateColumn,
)


@pytest.mark.upstream
def test_catalogs(uc: Calls, storage_root: str) -> None:
    name = unique("catalog")
    created = uc.create_catalog(
        name,
        comment="primary",
        properties={"team": "data"},
        storage_root=storage_root,
    )
    assert (created.name, created.comment) == (name, "primary")
    assert created.properties == {"team": "data"}
    assert created.id and created.created_at and created.storage_location
    assert uc.catalog(name).get() == created
    assert name in [c.name for c in uc.list_catalogs()]
    assert len(uc.list_catalogs(max_results=1)) == 1

    updated = uc.catalog(name).update(comment="edited", new_name=f"{name}_2")
    assert (updated.name, updated.comment, updated.id) == (
        f"{name}_2",
        "edited",
        created.id,
    )
    assert uc.catalog(f"{name}_2").delete() is None
    with pytest.raises(NotFoundError) as error:
        uc.catalog(f"{name}_2").get()
    assert (error.value.code, error.value.status) == ("CATALOG_NOT_FOUND", 404)


@pytest.mark.upstream
def test_schemas(uc: Calls, catalog: str) -> None:
    created = uc.create_schema(
        "s", catalog, comment="raw data", properties={"layer": "bronze"}
    )
    assert (created.full_name, created.comment) == (f"{catalog}.s", "raw data")
    assert created.properties == {"layer": "bronze"}
    assert uc.schema(f"{catalog}.s").get() == created
    assert [s.name for s in uc.list_schemas(catalog)] == ["s"]

    updated = uc.schema(f"{catalog}.s").update(comment="edited", new_name="s2")
    assert (updated.full_name, updated.comment) == (f"{catalog}.s2", "edited")
    uc.schema(f"{catalog}.s2").delete()
    assert uc.list_schemas(catalog) == []


@pytest.mark.upstream
def test_tables_and_views(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    columns = [
        ColumnInfo(
            "id",
            '{"name":"id","type":"long","nullable":false,"metadata":{}}',
            type_text="bigint",
            type_name="LONG",
            position=0,
            nullable=False,
        ),
        ColumnInfo(
            "email",
            '{"name":"email","type":"string","nullable":true,"metadata":{}}',
            type_text="string",
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
    assert uc.table(full_name).get() == table

    dependencies = DependencyList(
        dependencies=[Dependency(table=TableDependency(full_name))]
    )
    view = uc.create_table(
        "v",
        catalog,
        schema_name,
        "VIEW",
        columns=columns,
        view_definition="select * from t",
        view_dependencies=dependencies,
    )
    assert view.view_definition == "select * from t"
    assert view.view_dependencies == dependencies
    assert sorted(t.name for t in uc.list_tables(catalog, schema_name)) == ["t", "v"]

    uc.table(f"{catalog}.{schema_name}.v").delete()
    uc.table(full_name).delete()
    assert uc.list_tables(catalog, schema_name) == []


def test_table_comments(uc: Calls, schema: tuple[str, str]) -> None:
    """Upstream has no table update."""
    catalog, schema_name = schema
    column = ColumnInfo(
        "id",
        '{"name":"id","type":"long","nullable":true,"metadata":{}}',
        type_text="bigint",
        type_name="LONG",
        position=0,
    )
    staging = uc.create_staging_table("t", catalog, schema_name)
    uc.create_table(
        "t",
        catalog,
        schema_name,
        "MANAGED",
        columns=[column],
        data_source_format="DELTA",
        storage_location=staging.staging_location,
        properties={"io.unitycatalog.tableId": staging.id},
    )
    updated = uc.table(f"{catalog}.{schema_name}.t").update(
        comment="users", columns=[UpdateColumn("id", "primary key")]
    )
    assert updated.comment == "users"
    assert updated.columns[0].comment == "primary key"


@pytest.mark.upstream
def test_volumes(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    created = uc.create_volume(catalog, schema_name, "v", "MANAGED", comment="scratch")
    assert created.full_name == f"{catalog}.{schema_name}.v"
    assert (created.volume_type, created.comment) == ("MANAGED", "scratch")
    assert created.storage_location
    assert uc.volume(created.full_name).get() == created
    assert [v.name for v in uc.list_volumes(catalog, schema_name)] == ["v"]

    updated = uc.volume(created.full_name).update(comment="edited", new_name="v2")
    assert (updated.name, updated.comment) == ("v2", "edited")
    uc.volume(f"{catalog}.{schema_name}.v2").delete()
    assert uc.list_volumes(catalog, schema_name) == []


def test_search(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    name = unique("orders")
    for volume in [name, f"{name}_raw"]:
        uc.create_volume(catalog, schema_name, volume, "MANAGED", comment="landing")
    found = uc.search_objects(name)
    assert [r.name for r in found.results] == [name, f"{name}_raw"]
    first = found.results[0]
    assert first.full_name == f"{catalog}.{schema_name}.{name}"
    assert (first.securable_type, first.catalog_name, first.schema_name) == (
        "volume",
        catalog,
        schema_name,
    )
    assert first.comment == "landing"
    assert found.next_page_token is None

    page = uc.search_objects(name, securable_type="volume", max_results=1)
    assert [r.name for r in page.results] == [name]
    following = uc.search_objects(name, max_results=1, page_token=page.next_page_token)
    assert [r.name for r in following.results] == [f"{name}_raw"]
    assert uc.search_objects(name, securable_type="table").results == []


@pytest.mark.upstream
def test_functions(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    parameters = FunctionParameterInfos(
        parameters=[
            FunctionParameterInfo("x", "long", "LONG", type_json='"long"', position=0)
        ]
    )
    created = uc.create_function(
        CreateFunction(
            "inc",
            catalog,
            schema_name,
            "SQL",
            "S",
            "DEFINER",
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
    )
    assert created.full_name == f"{catalog}.{schema_name}.inc"
    assert created.input_params == parameters
    assert (created.routine_body, created.parameter_style, created.security_type) == (
        "SQL",
        "S",
        "DEFINER",
    )
    assert (created.data_type, created.sql_data_access) == ("LONG", "NO_SQL")
    assert uc.function(created.full_name).get() == created
    assert [f.name for f in uc.list_functions(catalog, schema_name)] == ["inc"]

    uc.function(created.full_name).delete()
    assert uc.list_functions(catalog, schema_name) == []


@pytest.mark.upstream
def test_registered_models_and_versions(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    full_name = f"{catalog}.{schema_name}.m"
    created = uc.create_registered_model("m", catalog, schema_name, comment="first")
    assert (created.full_name, created.comment) == (full_name, "first")
    assert created.storage_location
    assert uc.registered_model(full_name).get() == created
    assert full_name in [m.full_name for m in uc.list_registered_models()]
    in_schema = uc.list_registered_models(catalog_name=catalog, schema_name=schema_name)
    assert [m.full_name for m in in_schema] == [full_name]

    version = uc.create_model_version(
        "m", catalog, schema_name, "s3://bucket/m/v1", run_id="run-1", comment="v1"
    )
    assert (version.version, version.run_id, version.comment) == (1, "run-1", "v1")
    assert version.status in set(ModelVersionStatus)
    assert [v.id for v in uc.list_model_versions(full_name)] == [version.id]

    updated = uc.registered_model(full_name).update(comment="edited", new_name="m2")
    assert (updated.name, updated.comment) == ("m2", "edited")
    with pytest.raises(ConflictError):
        uc.registered_model(f"{catalog}.{schema_name}.m2").delete()
    uc.registered_model(f"{catalog}.{schema_name}.m2").delete(force=True)
    assert full_name not in [m.full_name for m in uc.list_registered_models()]


@pytest.mark.upstream
def test_model_versions(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    full_name = f"{catalog}.{schema_name}.m"
    uc.create_registered_model("m", catalog, schema_name)
    created = uc.create_model_version("m", catalog, schema_name, "s3://bucket/m/v1")
    assert created.status is ModelVersionStatus.PENDING_REGISTRATION
    assert uc.get_model_version(full_name, 1) == created

    updated = uc.update_model_version(full_name, 1, "first")
    assert (updated.id, updated.comment) == (created.id, "first")
    finalized = uc.finalize_model_version(full_name, 1)
    assert (finalized.status, finalized.comment) == ("READY", "first")
    # Local storage has no credentials to give.
    with pytest.raises(BadRequestError, match="file based storage location"):
        uc.generate_temporary_model_version_credentials(
            catalog, schema_name, "m", 1, "READ_MODEL_VERSION"
        )

    assert uc.delete_model_version(full_name, 1) is None
    assert uc.list_model_versions(full_name) == []
    with pytest.raises(NotFoundError) as error:
        uc.get_model_version(full_name, 1)
    assert error.value.code == "NOT_FOUND"


@pytest.mark.upstream
def test_credentials_and_external_locations(uc: Calls) -> None:
    credential_name = unique("credential")
    role = "arn:aws:iam::123456789012:role/uc"
    credential = uc.create_credential(
        credential_name,
        aws_iam_role=AwsIamRoleRequest(role),
        purpose="STORAGE",
        comment="prod",
    )
    assert (credential.purpose, credential.comment) == ("STORAGE", "prod")
    assert credential.aws_iam_role is not None
    assert credential.aws_iam_role.role_arn == role
    assert credential.aws_iam_role.external_id
    assert uc.credential(credential_name).get() == credential
    assert credential_name in [c.name for c in uc.list_credentials()]
    assert credential_name in [c.name for c in uc.list_credentials(purpose="STORAGE")]

    updated = uc.credential(credential_name).update(
        comment="edited", aws_iam_role=AwsIamRoleRequest(f"{role}2")
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
    assert uc.external_location(location_name).get() == location
    assert location_name in [loc.name for loc in uc.list_external_locations()]
    moved = uc.external_location(location_name).update(
        comment="edited", new_name=f"{location_name}_2"
    )
    assert (moved.name, moved.comment) == (f"{location_name}_2", "edited")

    with pytest.raises(BadRequestError) as error:
        uc.credential(credential_name).delete()
    assert error.value.code == "INVALID_ARGUMENT"
    uc.external_location(f"{location_name}_2").delete()
    uc.credential(credential_name).delete()
    assert credential_name not in [c.name for c in uc.list_credentials()]


def test_azure_credentials(uc: Calls) -> None:
    name = unique("credential")
    principal = AzureServicePrincipalRequest("tenant", "app", "secret")
    credential = uc.create_credential(name, azure_service_principal=principal)
    assert credential.aws_iam_role is None
    assert credential.azure_service_principal is not None
    assert credential.azure_service_principal.directory_id == "tenant"
    assert credential.azure_service_principal.application_id == "app"
    url = f"abfss://lake@acct.dfs.core.windows.net/{name}"
    uc.create_external_location(name, url, name)

    updated = uc.credential(name).update(azure_managed_identity=AzureManagedIdentity())
    assert updated.azure_service_principal is None
    assert updated.azure_managed_identity == AzureManagedIdentity()
    # The location's storage needs an Azure identity.
    with pytest.raises(BadRequestError):
        uc.credential(name).update(
            aws_iam_role=AwsIamRoleRequest("arn:aws:iam::123456789012:role/uc")
        )
    uc.external_location(name).delete()
    uc.credential(name).delete()


def test_gcp_credentials(uc: Calls) -> None:
    name = unique("credential")
    json_key = json.dumps(
        {
            "type": "service_account",
            "client_email": "uc@p.iam.gserviceaccount.com",
            "private_key_id": "kid",
            "private_key": "PRIVATE",
        }
    )
    credential = uc.create_credential(
        name, gcp_service_account_key=GcpServiceAccountKeyRequest(json_key)
    )
    assert credential.gcp_service_account_key is not None
    assert credential.gcp_service_account_key.email == "uc@p.iam.gserviceaccount.com"
    assert credential.gcp_service_account_key.private_key_id == "kid"
    uc.create_external_location(name, f"gs://bucket/{name}", name)

    email = "data@p.iam.gserviceaccount.com"
    account = GcpServiceAccount(email=email)
    updated = uc.credential(name).update(gcp_service_account=account)
    assert updated.gcp_service_account_key is None
    assert updated.gcp_service_account == account
    uc.external_location(name).delete()
    uc.credential(name).delete()


def test_list_credentials_by_purpose(uc: Calls) -> None:
    # The upstream server takes `purpose`, but lists every credential.
    name = unique("credential")
    role = AwsIamRoleRequest("arn:aws:iam::123456789012:role/uc")
    uc.create_credential(name, aws_iam_role=role, purpose="STORAGE")
    assert name in [c.name for c in uc.list_credentials(purpose="STORAGE")]
    services = uc.list_credentials(purpose="SERVICE_PRINCIPAL")
    assert name not in [c.name for c in services]
    uc.credential(name).delete()


@pytest.mark.upstream
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

    volume = uc.create_volume(catalog, schema_name, "v", "MANAGED")
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
    # The upstream server keeps no grants while authorization is off.
    user = f"{unique('user')}@example.com"
    other = f"{unique('user')}@example.com"
    uc.create_user(emails=[ScimEmail(user, primary=True)])
    uc.create_user(emails=[ScimEmail(other, primary=True)])
    assert uc.get_permissions("catalog", catalog).privilege_assignments == []

    grant = PermissionsChange(user, add=["USE CATALOG", "CREATE SCHEMA"])
    granted = uc.update_permissions("catalog", catalog, changes=[grant])
    assert [
        (a.principal, sorted(a.privileges)) for a in granted.privilege_assignments
    ] == [(user, ["CREATE SCHEMA", "USE CATALOG"])]
    assert uc.get_permissions("catalog", catalog) == granted
    assert uc.get_permissions("catalog", catalog, principal=user) == granted
    uc.update_permissions(
        "catalog", catalog, changes=[PermissionsChange(other, add=["USE CATALOG"])]
    )
    assert uc.get_permissions("catalog", catalog, principal=user) == granted

    revoke = [
        PermissionsChange(user, remove=["USE CATALOG", "CREATE SCHEMA"]),
        PermissionsChange(other, remove=["USE CATALOG"]),
    ]
    revoked = uc.update_permissions("catalog", catalog, changes=revoke)
    assert revoked.privilege_assignments == []
    with pytest.raises(BadRequestError, match="unknown variant `bogus`"):
        uc.get_permissions("bogus", catalog)


@pytest.mark.upstream
def test_metastore_summary(uc: Calls) -> None:
    summary = uc.get_metastore_summary()
    assert summary.metastore_id
    assert uc.get_metastore_summary() == summary


def test_health_and_version(uc: Calls) -> None:
    assert uc.get_health().status == "ok"
    version = uc.get_version()
    assert (version.server, version.api_version) == ("unitycatalog-rs", "2.1")


def test_personal_access_tokens(admin: Calls, uc: Calls, server) -> None:
    created = admin.create_token(unique("token"))
    assert created.token_value and created.token_prefix and created.owner
    # The repr hides the secret; the attribute gives it.
    assert created.token_value not in repr(created)
    assert "token_value='***'" in repr(created)
    signed_in = Client(server.address, token=created.token_value)
    assert signed_in.list_tokens()[0].id == created.id

    listed = {p.id: p for p in admin.list_tokens()}
    assert listed[created.id].token_value is None
    # The OAuth `client_credentials` grant, a form: the token is the access token.
    answer = uc.request_access_token("client_credentials", created.token_value)
    assert (answer.access_token, answer.token_type) == (created.token_value, "Bearer")
    assert created.token_value not in repr(answer)
    admin.token(created.id).delete()
    assert created.id not in [p.id for p in admin.list_tokens()]


@pytest.mark.upstream
def test_token_exchange_posts_a_form(uc: Calls) -> None:
    with pytest.raises(BadRequestError, match="Authorization is disabled"):
        uc.exchange_token(
            "urn:ietf:params:oauth:grant-type:token-exchange",
            "urn:ietf:params:oauth:token-type:access_token",
            "not-a-token",
            "urn:ietf:params:oauth:token-type:id_token",
        )


@pytest.mark.upstream
def test_scim_users(uc: Calls) -> None:
    email = f"{unique('user')}@example.com"
    created = uc.create_user(
        emails=[ScimEmail(email, primary=True)], display_name="Ada", external_id=email
    )
    assert created.id and created.meta and created.meta.created
    assert created.user_name == email
    assert created.emails == [ScimEmail(email, primary=True)]
    assert (created.display_name, created.external_id) == ("Ada", email)
    assert created.active is True
    assert _without_meta(uc.get_user(created.id)) == _without_meta(created)
    [listed] = uc.list_users(filter=f'userName eq "{email}"').resources
    assert _without_meta(listed) == _without_meta(created)
    assert len(uc.list_users(start_index=1, count=1).resources) == 1

    updated = uc.replace_user(created.id, display_name="Ada L")
    assert (updated.display_name, updated.user_name) == ("Ada L", email)
    assert updated.external_id == email
    # As in upstream, a deleted user stays, disabled.
    assert uc.delete_user(created.id) is None
    assert uc.get_user(created.id).active is False
    assert uc.replace_user(created.id, active=True).active is True


def test_patch_scim_user(uc: Calls) -> None:
    # The upstream server has no PATCH for users.
    email = f"{unique('user')}@example.com"
    created = uc.create_user(emails=[ScimEmail(email, primary=True)])
    disable = ScimPatchOperation("replace", path="active", value=False)
    assert uc.patch_user(created.id, [disable]) is None
    assert uc.get_user(created.id).active is False


def _without_meta(user: ScimUser) -> tuple[object, ...]:
    """The fields of `user` but `meta`: the upstream server gives a user who
    was never updated the time of the request as `meta.last_modified`."""
    return (
        user.schemas,
        user.id,
        user.external_id,
        user.user_name,
        user.display_name,
        user.active,
        user.emails,
        user.photos,
        user.roles,
    )


def test_me(admin: Calls, uc: Calls) -> None:
    # The server's `admin` is a caller, but without authorization no user.
    with pytest.raises(NotFoundError) as error:
        admin.get_me()
    assert error.value.code == "NOT_FOUND"
    # Without a token the server knows no caller: a SCIM error, status 500
    # as upstream.
    with pytest.raises(ServerError, match="No user found"):
        uc.get_me()


def test_scim_groups(uc: Calls) -> None:
    name = unique("group")
    created = uc.create_group(name)
    assert created.id
    assert (created.display_name, created.members) == (name, [])
    assert uc.get_group(created.id) == created
    assert uc.list_groups(filter=f'displayName eq "{name}"').resources == [created]

    email = f"{unique('user')}@example.com"
    user = uc.create_user(emails=[ScimEmail(email, primary=True)])
    add = ScimPatchOperation("add", path="members", value=[{"value": user.id}])
    patched = uc.patch_group(created.id, [add])
    assert patched.members == [ScimGroupMember(user.id)]
    replaced = uc.replace_group(created.id, f"{name}_2")
    assert (replaced.display_name, replaced.members) == (f"{name}_2", [])

    assert uc.delete_group(created.id) is None
    assert created.id not in [g.id for g in uc.list_groups().resources]
    with pytest.raises(NotFoundError) as error:
        uc.get_group(created.id)
    assert error.value.code == "NOT_FOUND"


def test_backup_and_restore(admin: Calls, uc: Calls, catalog: str) -> None:
    snapshot = admin.get_backup()
    names = [row.get("name") for row in snapshot.tables["catalog"]]
    assert catalog in names
    restored = admin.restore_backup(snapshot.version, snapshot.tables)
    assert restored.imported["catalog"] == len(names)
    assert restored.skipped == []
    assert uc.catalog(catalog).get().name == catalog

    # Without a token there is no admin, even with authorization off.
    with pytest.raises(UnauthenticatedError) as error:
        uc.get_backup()
    assert error.value.code == "UNAUTHENTICATED"


def test_audit_events(admin: Calls, catalog: str) -> None:
    path = "/api/2.1/unity-catalog/catalogs"
    # The server records events in the background.
    deadline = time.monotonic() + 10
    while True:
        events = admin.list_audit_events(
            principal="anonymous", method="POST", path_prefix=path
        ).events
        if events or time.monotonic() > deadline:
            break
        time.sleep(0.1)
    assert events
    assert {(e.principal, e.method, e.path) for e in events} == {
        ("anonymous", "POST", path)
    }
    assert all(e.id and e.status == 200 and e.timestamp for e in events)
    page = admin.list_audit_events(max_results=1)
    assert len(page.events) == 1 and page.next_page_token
    following = admin.list_audit_events(max_results=1, page_token=page.next_page_token)
    assert following.events and following.events != page.events


@pytest.mark.upstream
def test_iceberg_rest_catalog(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, namespace = schema
    config = uc.get_iceberg_config(catalog)
    assert config.overrides["prefix"] == f"catalogs/{catalog}"
    assert config.endpoints
    assert uc.list_iceberg_namespaces(catalog).namespaces == [[namespace]]
    assert uc.iceberg_namespace_exists(catalog, namespace) is True
    assert uc.iceberg_namespace_exists(catalog, "missing") is False
    assert uc.list_iceberg_tables(catalog, namespace).identifiers == []

    fields = [IcebergStructField(1, "id", True, "long")]
    schema_of_t = IcebergSchema(schema_type="struct", schema_id=0, fields=fields)
    created = uc.create_iceberg_table(catalog, namespace, "t", schema_of_t)
    assert created.metadata and created.metadata.format_version == 2
    [table] = uc.list_iceberg_tables(catalog, namespace).identifiers
    assert table == IcebergTableIdentifier(namespace=[namespace], name="t")
    loaded = uc.load_iceberg_table(catalog, namespace, "t")
    assert loaded.metadata_location == created.metadata_location
    assert loaded.metadata and loaded.metadata.format_version in (1, 2)
    assert uc.iceberg_table_exists(catalog, namespace, "t") is True
    assert uc.iceberg_table_exists(catalog, namespace, "missing") is False
    with pytest.raises(NotFoundError) as error:
        uc.load_iceberg_table(catalog, namespace, "missing")
    assert error.value.code == "NoSuchTableException"

    uc.drop_iceberg_table(catalog, namespace, "t")
    assert uc.iceberg_table_exists(catalog, namespace, "t") is False


@pytest.mark.upstream
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
    commits = uc.get_delta_commits(table.table_id, 0, table_uri=table.storage_location)
    assert (commits.commits, commits.latest_table_version) == ([], 0)

    now = int(time.time() * 1000)
    commit_info = DeltaCommitInfo(1, now, "00000000000000000001.uuid.json", 10, now)
    assert (
        uc.commit_delta(table.table_id, table.storage_location, commit_info=commit_info)
        is None
    )
    commits = uc.get_delta_commits(
        table.table_id, 1, table_uri=table.storage_location, end_version=1
    )
    assert (commits.commits, commits.latest_table_version) == ([commit_info], 1)

    with pytest.raises(NotFoundError) as error:
        uc.commit_delta(
            "00000000-0000-0000-0000-000000000000",
            table.storage_location,
            commit_info=commit_info,
        )
    assert error.value.code == "TABLE_NOT_FOUND"


@pytest.mark.upstream
def test_delta_api(uc: Calls, schema: tuple[str, str]) -> None:
    catalog, schema_name = schema
    config = uc.get_delta_config(catalog, "1.0")
    assert config.protocol_version == "1.0" and config.endpoints
    staging = uc.create_delta_staging_table(catalog, schema_name, "staged")
    assert staging.table_type == "MANAGED"
    assert staging.required_properties["io.unitycatalog.tableId"] == staging.table_id
    assert uc.get_delta_staging_table_credentials(staging.table_id).storage_credentials

    created = uc.create_staging_table("t", catalog, schema_name)
    uc.create_table(
        "t",
        catalog,
        schema_name,
        "MANAGED",
        data_source_format="DELTA",
        storage_location=created.staging_location,
        properties={"io.unitycatalog.tableId": created.id},
    )
    loaded = uc.load_delta_table(catalog, schema_name, "t")
    assert loaded.metadata and loaded.metadata.table_uuid == created.id
    assert set(loaded.allowed_maintenance_operations) <= set(DeltaMaintenanceOperation)
    [credential] = uc.get_delta_table_credentials(
        catalog, schema_name, "t", "READ"
    ).storage_credentials
    assert (credential.prefix, credential.operation) == (
        created.staging_location,
        "READ",
    )

    assert uc.rename_delta_table(catalog, schema_name, "t", "t2") is None
    assert uc.delta_table_exists(catalog, schema_name, "t") is False
    assert uc.delta_table_exists(catalog, schema_name, "t2") is True
    uc.delete_delta_table(catalog, schema_name, "t2")
    assert uc.delta_table_exists(catalog, schema_name, "t2") is False


def test_shares_recipients_and_delta_sharing(
    uc: Calls, schema: tuple[str, str], server, mode: str
) -> None:
    catalog, schema_name = schema
    staging = uc.create_staging_table("orders", catalog, schema_name)
    uc.create_table(
        "orders",
        catalog,
        schema_name,
        "MANAGED",
        data_source_format="DELTA",
        storage_location=staging.staging_location,
        properties={"io.unitycatalog.tableId": staging.id},
    )
    share_name, recipient_name = unique("share"), unique("recipient")
    share = uc.create_share(share_name, comment="for partners")
    assert (share.name, share.objects) == (share_name, [])
    shared = SharedDataObject(f"{catalog}.{schema_name}.orders", "sales", "orders")
    assert uc.share(share_name).update(add_objects=[shared]).objects == [shared]
    assert share_name in [s.name for s in uc.list_shares()]

    recipient = uc.create_recipient(recipient_name, comment="partner")
    assert recipient.authentication_type == "TOKEN" and recipient.bearer_token
    updated = uc.recipient(recipient_name).update(add_shares=[share_name])
    assert (updated.shares, updated.bearer_token) == ([share_name], None)
    assert recipient_name in [r.name for r in uc.list_recipients()]
    profile = uc.recipient(recipient_name).create_recipient_profile()
    assert profile.endpoint.endswith("/delta-sharing")

    # The Delta Sharing protocol, with the token of the recipient.
    reader = Calls(Client(server.address, token=profile.bearer_token), mode)
    assert [s.name for s in reader.sharing_list_shares().items] == [share_name]
    assert [s.name for s in reader.sharing_list_schemas(share_name).items] == ["sales"]
    [table] = reader.sharing_list_tables(share_name, "sales").items
    assert (table.share, table.schema, table.name) == (share_name, "sales", "orders")
    assert reader.sharing_list_all_tables(share_name).items == [table]
    version = reader.sharing_get_table_version(share_name, "sales", "orders")
    assert version.version == 0
    protocol, metadata = reader.sharing_get_table_metadata(
        share_name, "sales", "orders"
    )
    assert protocol.protocol and metadata.metadata
    assert metadata.metadata.id == staging.id

    rotated = uc.recipient(recipient_name).rotate_recipient_token()
    assert rotated.bearer_token and rotated.bearer_token != profile.bearer_token
    uc.recipient(recipient_name).delete()
    uc.share(share_name).delete()
    assert share_name not in [s.name for s in uc.list_shares()]


@pytest.mark.upstream
def test_server_errors(uc: Calls) -> None:
    name = unique("missing")
    with pytest.raises(NotFoundError) as error:
        uc.catalog(name).get()
    assert (error.value.code, error.value.status) == ("CATALOG_NOT_FOUND", 404)
    assert name in error.value.message
    assert str(error.value) == f"CATALOG_NOT_FOUND: {error.value.message}"
    assert error.value.details[0]["reason"] == "CATALOG_NOT_FOUND"


@pytest.mark.upstream
def test_async_requests_in_one_event_loop(server, catalog: str) -> None:
    client = Client(server.address)

    async def main() -> list[str]:
        requests = [client.catalog(catalog).get_async() for _ in range(20)]
        return [c.name for c in await asyncio.gather(*requests)]

    assert asyncio.run(main()) == [catalog] * 20


def test_unreachable_server(mode: str) -> None:
    client = Calls(Client(f"http://127.0.0.1:{_closed_port()}/"), mode)
    with pytest.raises(UnityCatalogError) as error:
        client.list_catalogs()
    # Without an answer the error has no status, and so no subclass.
    assert type(error.value) is UnityCatalogError
    assert (error.value.code, error.value.status) == (None, None)
    assert error.value.message.startswith("request failed: ")


def _closed_port() -> int:
    import socket

    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
