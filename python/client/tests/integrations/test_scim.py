"""scim2-tester, a checker of SCIM servers against RFC 7643 and RFC 7644,
runs each of its checks against the server's SCIM API: the discovery
endpoints, and the creation, reading, listing, searching, replacement,
patching and deletion of users and groups."""

import pytest

httpx2 = pytest.importorskip("httpx2")
scim2_tester = pytest.importorskip("scim2_tester")
engines = pytest.importorskip("scim2_client.engines.httpx2")

SCIM = "/api/1.0/unity-control/scim2"


def test_the_scim_api_passes_every_check_of_scim2_tester(server) -> None:
    headers = {"Authorization": f"Bearer {server.token}"} if server.token else {}
    with httpx2.Client(base_url=f"{server.address}{SCIM}", headers=headers) as http:
        results = scim2_tester.check_server(engines.SyncSCIMClient(http))
    failed = [
        f"{r.status.name} {r.title} [{getattr(r, 'resource_type', None)}]: {r.reason}"
        for r in results
        if r.status != scim2_tester.Status.SUCCESS
    ]
    assert results
    assert not failed, "\n".join(failed)
