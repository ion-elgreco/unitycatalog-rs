"""The surface of the client: every method of the generated `uc-client`, sync
and async, enums that list the values that the extension takes, and stubs
that match the extension."""

import asyncio
import enum
import importlib
import inspect
import pickle
import re
import subprocess
import sys
import typing
from pathlib import Path

import pytest

import unitycatalog
from unitycatalog import (
    BadRequestError,
    Client,
    ConflictError,
    NotFoundError,
    PermissionDeniedError,
    RateLimitError,
    ServerError,
    UnauthenticatedError,
    UnityCatalogError,
    _enums,
)
from unitycatalog.credentials import AwsIamRoleRequest
from unitycatalog.delta import DeltaProtocol, DeltaStructField, DeltaStructType
from unitycatalog.functions import CreateFunction, FunctionParameterInfo
from unitycatalog.permissions import PermissionsChange
from unitycatalog.tables import ColumnInfo

REPO_ROOT = Path(__file__).resolve().parents[3]
UC_CLIENT = REPO_ROOT / "crates" / "uc-client" / "src" / "codegen"

# A client of a closed port: arguments are checked before a request.
OFFLINE = Client("http://127.0.0.1:9/")
# Not a value of any enum; `Any`, so that type checkers take it.
BOGUS: typing.Any = "BOGUS"

# A method of the generated Rust client, and the type that it returns.
RUST_METHOD = re.compile(r"pub fn (\w+)[^{]*?->\s*([\w:]+)", re.DOTALL)
# The Rust methods that are not requests: constructors, their options and
# accessors.
NOT_REQUESTS = {
    "new",
    "new_unauthenticated",
    "new_with_token",
    "with_user_agent",
    "name",
    "full_name",
}


def _rust_methods(path: Path) -> dict[str, str]:
    """The methods of a generated Rust client, with the type that each
    returns: a builder for a request, a client for a resource handle."""
    return {
        name: returns.rsplit("::", 1)[-1]
        for name, returns in RUST_METHOD.findall(path.read_text())
        if name not in NOT_REQUESTS and not name.endswith("_client")
    }


def _resource_classes() -> dict[str, type]:
    """The resource handles of the extension, such as `CatalogClient`, by
    name."""
    classes = {}
    for name in [n for n in dir(unitycatalog._core) if not n.startswith("_")]:
        module = getattr(unitycatalog._core, name)
        if inspect.ismodule(module):
            for class_name, cls in vars(module).items():
                if class_name.endswith("Client") and inspect.isclass(cls):
                    classes[class_name] = cls
    return classes


def test_each_api_package_has_a_public_module() -> None:
    """`unitycatalog.tables` and so on export the classes of their submodule of
    the extension."""
    for name in [n for n in dir(unitycatalog._core) if not n.startswith("_")]:
        core = getattr(unitycatalog._core, name)
        if not inspect.ismodule(core):
            continue
        public = importlib.import_module(f"unitycatalog.{name}")
        classes = {n for n, value in vars(core).items() if inspect.isclass(value)}
        assert classes <= set(public.__all__), name
        for class_name in classes:
            assert getattr(public, class_name) is getattr(core, class_name)


def _requests(cls: type) -> set[str]:
    """The methods of `cls` that send a request: those with an `_async`
    twin."""
    names = {name for name in dir(cls) if not name.startswith("_")}
    return {name for name in names if f"{name}_async" in names}


def test_every_uc_client_method_is_exposed() -> None:
    rust = _rust_methods(UC_CLIENT / "client.rs")
    requests = {name for name, returns in rust.items() if returns.endswith("Builder")}
    handles = {name: returns for name, returns in rust.items() if name not in requests}
    assert {"create_catalog", "list_catalogs", "get_backup"} <= requests
    assert handles["catalog"] == "CatalogClient"
    assert all(returns.endswith("Client") for returns in handles.values())
    assert _requests(Client) == requests

    # A handle sends no request: it has no `_async` twin.
    resources = _resource_classes()
    for name, returns in handles.items():
        assert not hasattr(Client, f"{name}_async")
        assert type(getattr(OFFLINE, name)("x")) is resources[returns]
    resource_files = sorted(UC_CLIENT.glob("*/resource.rs"))
    assert len(resource_files) == len(handles)
    for path in resource_files:
        [struct] = re.findall(r"pub struct (\w+)", path.read_text())
        assert _requests(resources[struct]) == set(_rust_methods(path)), struct


def test_each_method_has_an_async_twin_with_its_parameters() -> None:
    for cls in [Client, *_resource_classes().values()]:
        for name in _requests(cls):
            method = inspect.signature(getattr(cls, name))
            twin = inspect.signature(getattr(cls, f"{name}_async"))
            assert method.parameters == twin.parameters, f"{cls.__name__}.{name}"


def test_stubs_match_the_extension() -> None:
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy.stubtest",
            "unitycatalog._core",
            "--allowlist",
            "tests/stubtest_allowlist.txt",
        ],
        check=False,
        capture_output=True,
        text=True,
        cwd=REPO_ROOT / "python" / "client",
    )
    assert result.returncode == 0, result.stdout + result.stderr


# The enums that only answers hold; test_client checks those.
RESULT_ENUMS = {"DeltaMaintenanceOperation", "ModelVersionStatus"}


def _bad_value(name: str) -> typing.Callable[[], object]:
    """A call that gives `BOGUS` as a value of the enum `name`."""
    role = AwsIamRoleRequest("arn:aws:iam::123456789012:role/uc")
    columns = DeltaStructType([DeltaStructField("id", "long", False, {})])
    calls: dict[str, typing.Callable[[], object]] = {
        "AuthenticationType": lambda: OFFLINE.create_recipient(
            "r", authentication_type=BOGUS
        ),
        "ColumnTypeName": lambda: ColumnInfo("id", "{}", type_name=BOGUS),
        "CredentialPurpose": lambda: OFFLINE.create_credential(
            "c", role, purpose=BOGUS
        ),
        "DataSourceFormat": lambda: OFFLINE.create_table(
            "t", "c", "s", "MANAGED", data_source_format=BOGUS
        ),
        "DeltaCredentialOperation": lambda: OFFLINE.get_delta_table_credentials(
            "c", "s", "t", BOGUS
        ),
        "DeltaTableType": lambda: OFFLINE.create_delta_table(
            "c", "s", "t", "file:///t", BOGUS, columns, DeltaProtocol(), 0
        ),
        "FunctionParameterMode": lambda: FunctionParameterInfo(
            "x", "int", "INT", parameter_mode=BOGUS
        ),
        "FunctionParameterType": lambda: FunctionParameterInfo(
            "x", "int", "INT", parameter_type=BOGUS
        ),
        "ModelVersionOperation": (
            lambda: OFFLINE.generate_temporary_model_version_credentials(
                "c", "s", "m", 1, BOGUS
            )
        ),
        "ParameterStyle": lambda: CreateFunction(
            "f", "c", "s", "SQL", BOGUS, "DEFINER"
        ),
        "PathOperation": lambda: OFFLINE.generate_temporary_path_credentials(
            "s3://bucket/p", BOGUS
        ),
        "RoutineBody": lambda: CreateFunction("f", "c", "s", BOGUS, "S", "DEFINER"),
        "SecurityType": lambda: CreateFunction("f", "c", "s", "SQL", "S", BOGUS),
        "SqlDataAccess": lambda: CreateFunction(
            "f", "c", "s", "SQL", "S", "DEFINER", sql_data_access=BOGUS
        ),
        "TableOperation": lambda: OFFLINE.generate_temporary_table_credentials(
            "t", BOGUS
        ),
        "TableType": lambda: OFFLINE.create_table("t", "c", "s", BOGUS),
        "VolumeOperation": lambda: OFFLINE.generate_temporary_volume_credentials(
            "v", BOGUS
        ),
        "VolumeType": lambda: OFFLINE.create_volume("c", "s", "v", BOGUS),
    }
    return calls[name]


ENUMS = sorted(
    name
    for name, value in vars(_enums).items()
    if inspect.isclass(value) and issubclass(value, enum.StrEnum)
)


def test_the_package_exports_each_enum() -> None:
    assert set(ENUMS) <= set(unitycatalog.__all__)
    assert all(getattr(unitycatalog, name) is getattr(_enums, name) for name in ENUMS)


@pytest.mark.parametrize("name", [n for n in ENUMS if n not in RESULT_ENUMS])
def test_enums_list_the_values_that_the_extension_takes(name: str) -> None:
    with pytest.raises(ValueError, match=f"invalid {name}: ") as error:
        _bad_value(name)()
    expected = str(error.value).split("expected one of: ", 1)[1]
    members = set(getattr(_enums, name))
    assert set(expected.split(", ")) == {member.value for member in members}
    # A member and its name are the same value.
    member = next(iter(members))
    assert member == member.name == str(member)


def test_wrong_arguments_raise_before_a_request() -> None:
    with pytest.raises(TypeError):
        OFFLINE.catalog(1)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        OFFLINE.list_catalogs(1)  # type: ignore[misc]
    with pytest.raises(TypeError):
        OFFLINE.restore_backup(1, [])
    with pytest.raises(TypeError):
        OFFLINE.restore_backup(1, {"catalog": "not a list"})
    with pytest.raises(TypeError, match="not a JSON value"):
        OFFLINE.report_iceberg_metrics(
            "c", "n", "t", "commit-report", metrics={"x": object()}
        )
    # The fields that the API requires are required arguments.
    with pytest.raises(TypeError, match="type_json"):
        ColumnInfo("id")  # type: ignore[call-arg]
    with pytest.raises(TypeError, match="nullable"):
        DeltaStructField("id", "long")  # type: ignore[call-arg]


def test_a_bad_base_url_raises_uc_error() -> None:
    with pytest.raises(UnityCatalogError) as error:
        Client("not a url")
    assert type(error.value) is UnityCatalogError
    assert (error.value.code, error.value.status) == (None, None)
    assert error.value.message.startswith("invalid URL: ")


def test_async_methods_need_a_running_event_loop() -> None:
    with pytest.raises(RuntimeError, match="no running event loop"):
        OFFLINE.list_catalogs_async()

    async def main() -> None:
        future = OFFLINE.list_catalogs_async()
        assert isinstance(future, asyncio.Future)
        with pytest.raises(UnityCatalogError):
            await future

    asyncio.run(main())


def test_value_classes_are_frozen_and_compare_by_value() -> None:
    column = ColumnInfo("id", "{}", type_name="LONG", position=0)
    assert column == ColumnInfo("id", "{}", type_name="LONG", position=0)
    assert column != ColumnInfo("id", "{}", type_name="INT", position=0)
    assert repr(column) == (
        "ColumnInfo(name='id', type_json='{}', "
        "type_name=<ColumnTypeName.LONG: 'LONG'>, position=0)"
    )
    with pytest.raises(AttributeError):
        column.name = "other"  # type: ignore[misc]
    assert column.type_name is _enums.ColumnTypeName.LONG
    change = PermissionsChange("alice", add=["USE CATALOG"])
    assert (change.add, change.remove) == (["USE CATALOG"], [])


@pytest.mark.parametrize(
    ("error", "status"),
    [
        (BadRequestError, 400),
        (UnauthenticatedError, 401),
        (PermissionDeniedError, 403),
        (NotFoundError, 404),
        (ConflictError, 409),
        (RateLimitError, 429),
        (ServerError, 500),
    ],
)
def test_errors_pickle(error: type[UnityCatalogError], status: int) -> None:
    raised = error("Catalog not found: x", "CATALOG_NOT_FOUND", status)
    assert isinstance(raised, UnityCatalogError) and isinstance(raised, Exception)
    copy = pickle.loads(pickle.dumps(raised))
    assert type(copy) is error
    assert (copy.code, copy.message, copy.status, copy.details) == (
        "CATALOG_NOT_FOUND",
        "Catalog not found: x",
        status,
        [],
    )
    assert str(copy) == "CATALOG_NOT_FOUND: Catalog not found: x"


def test_client_repr_leaves_out_the_token() -> None:
    client = Client("http://localhost:8080/", token="secret")
    assert "secret" not in repr(client)
