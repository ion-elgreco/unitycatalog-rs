"""The surface of the client: every method of `uc-client`, sync and async,
and stubs that match the extension."""

import asyncio
import inspect
import pickle
import re
import subprocess
import sys
import typing
from pathlib import Path

import pytest

import unitycatalog.types
from unitycatalog import (
    Client,
    ColumnInfo,
    FunctionParameterInfo,
    PermissionsChange,
    UcError,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
UC_CLIENT = REPO_ROOT / "crates" / "uc-client" / "src" / "lib.rs"

# A client of a closed port: arguments are checked before a request.
OFFLINE = Client("http://127.0.0.1:9/api/2.1/unity-catalog/")
# Not a value of any enum; `Any`, so that type checkers take it.
BOGUS: typing.Any = "BOGUS"


def _methods() -> set[str]:
    return {name for name in dir(Client) if not name.startswith("_")}


def test_every_uc_client_method_is_exposed() -> None:
    rust = set(re.findall(r"pub async fn (\w+)", UC_CLIENT.read_text()))
    assert len(rust) == 74
    assert rust <= _methods()
    assert {f"{name}_async" for name in rust} <= _methods()


def test_each_method_has_an_async_twin_with_its_parameters() -> None:
    sync = {name for name in _methods() if not name.endswith("_async")}
    assert sync - {"with_bearer"} == {
        name.removesuffix("_async") for name in _methods() if name.endswith("_async")
    }
    for name in sync - {"with_bearer"}:
        method = inspect.signature(getattr(Client, name))
        twin = inspect.signature(getattr(Client, f"{name}_async"))
        assert method.parameters == twin.parameters, name


def test_stubs_match_the_extension() -> None:
    result = subprocess.run(
        [sys.executable, "-m", "mypy.stubtest", "unitycatalog._core"],
        check=False,
        capture_output=True,
        text=True,
        cwd=REPO_ROOT / "python" / "client",
    )
    assert result.returncode == 0, result.stdout + result.stderr


def _bad_value(name: str) -> typing.Callable[[], object]:
    """A call that gives `BOGUS` as a value of the enum `name`."""
    calls: dict[str, typing.Callable[[], object]] = {
        "TableType": lambda: OFFLINE.create_table("t", "c", "s", BOGUS),
        "DataSourceFormat": lambda: OFFLINE.create_table(
            "t", "c", "s", "MANAGED", data_source_format=BOGUS
        ),
        "ColumnTypeName": lambda: ColumnInfo(type_name=BOGUS),
        "VolumeType": lambda: OFFLINE.create_volume("v", "c", "s", BOGUS),
        "RoutineBody": lambda: OFFLINE.create_function("f", "c", "s", BOGUS),
        "ParameterStyle": lambda: OFFLINE.create_function(
            "f", "c", "s", "SQL", parameter_style=BOGUS
        ),
        "SecurityType": lambda: OFFLINE.create_function(
            "f", "c", "s", "SQL", security_type=BOGUS
        ),
        "SqlDataAccess": lambda: OFFLINE.create_function(
            "f", "c", "s", "SQL", sql_data_access=BOGUS
        ),
        "FunctionParameterMode": lambda: FunctionParameterInfo(
            "x", "int", "INT", parameter_mode=BOGUS
        ),
        "FunctionParameterType": lambda: FunctionParameterInfo(
            "x", "int", "INT", parameter_type=BOGUS
        ),
        "CredentialPurpose": lambda: OFFLINE.create_credential("c", purpose=BOGUS),
        "SecurableType": lambda: OFFLINE.get_permissions(BOGUS, "x"),
        "Privilege": lambda: PermissionsChange("p", add=[BOGUS]),
        "TableOperation": lambda: OFFLINE.generate_temporary_table_credentials(
            "t", BOGUS
        ),
        "VolumeOperation": lambda: OFFLINE.generate_temporary_volume_credentials(
            "v", BOGUS
        ),
        "PathOperation": lambda: OFFLINE.generate_temporary_path_credentials(
            "s3://bucket/p", BOGUS
        ),
    }
    return calls[name]


# `ModelVersionStatus` is only in results; test_client checks it.
@pytest.mark.parametrize(
    "name", [n for n in unitycatalog.types.__all__ if n != "ModelVersionStatus"]
)
def test_literal_types_list_the_values_of_the_enums(name: str) -> None:
    with pytest.raises(ValueError, match=f"invalid {name}: ") as error:
        _bad_value(name)()
    expected = str(error.value).split("expected", 1)[1]
    values = re.findall(r"`([^`]*)`", expected)
    assert set(typing.get_args(getattr(unitycatalog.types, name))) == set(values)


def test_wrong_arguments_raise_before_a_request() -> None:
    with pytest.raises(TypeError):
        OFFLINE.get_catalog(1)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="invalid HTTP method"):
        OFFLINE.raw_json("NOT A METHOD", "catalogs")
    with pytest.raises(ValueError, match="Out of range float values"):
        OFFLINE.raw_json("POST", "catalogs", body={"x": float("nan")})
    with pytest.raises(TypeError):
        OFFLINE.restore([])  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="invalid snapshot: "):
        OFFLINE.restore({"version": "one"})
    with pytest.raises(ValueError, match="invalid commit: "):
        OFFLINE.commit_delta({"commit_info": {"version": "one"}})


def test_a_bad_base_url_raises_uc_error() -> None:
    with pytest.raises(UcError) as error:
        Client("not a url")
    assert error.value.code == "INVALID_ARGUMENT"


def test_async_methods_need_a_running_event_loop() -> None:
    with pytest.raises(RuntimeError, match="no running event loop"):
        OFFLINE.list_catalogs_async()

    async def main() -> None:
        future = OFFLINE.list_catalogs_async()
        assert isinstance(future, asyncio.Future)
        with pytest.raises(UcError):
            await future

    asyncio.run(main())


def test_value_classes_are_frozen_and_compare_by_value() -> None:
    column = ColumnInfo("id", type_name="LONG", position=0)
    assert column == ColumnInfo("id", type_name="LONG", position=0)
    assert column != ColumnInfo("id", type_name="INT", position=0)
    assert repr(column) == "ColumnInfo(name='id', type_name='LONG', position=0)"
    with pytest.raises(AttributeError):
        column.name = "other"  # type: ignore[misc]
    change = PermissionsChange("alice", add=["USE CATALOG"])
    assert (change.add, change.remove) == (["USE CATALOG"], [])


def test_uc_error_pickles() -> None:
    error = UcError("NOT_FOUND", "Catalog not found: x")
    copy = pickle.loads(pickle.dumps(error))
    assert (copy.code, copy.message, str(copy)) == (
        "NOT_FOUND",
        "Catalog not found: x",
        "NOT_FOUND: Catalog not found: x",
    )


def test_client_repr_leaves_out_the_token() -> None:
    client = Client("http://localhost:8080/").with_bearer("secret")
    assert repr(client) == 'Client("http://localhost:8080/")'
