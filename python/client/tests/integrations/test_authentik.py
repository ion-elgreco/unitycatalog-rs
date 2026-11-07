"""authentik, an identity provider, pushes its users and groups to the server
over SCIM 2.0: its SCIM provider sends each change in authentik to the
server's SCIM API, with the token of the server's admin.

The provider belongs to an application whose users are the members of one
group, and syncs that group and those users with authentik's default
property mappings. After a full sync, the first tests check what arrived;
each of the others changes one user in authentik and waits until the server
has the change.

Skipped without authentik: `just test-authentik` starts it in Docker
(authentik/compose.yaml) and sets UC_TEST_AUTHENTIK_URL and
UC_TEST_AUTHENTIK_TOKEN, an API token of authentik's admin."""

import json
import os
import socket
import socketserver
import sys
import threading
import time
import typing
import urllib.error
import urllib.parse
import urllib.request
import uuid
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from dataclasses import dataclass

import pytest

AUTHENTIK_URL = os.environ.get("UC_TEST_AUTHENTIK_URL", "").rstrip("/")
if not AUTHENTIK_URL:
    pytest.skip("needs authentik: UC_TEST_AUTHENTIK_URL", allow_module_level=True)

SCIM = "/api/1.0/unity-control/scim2"
# authentik's default property mappings of SCIM users and groups.
USER_MAPPING = "goauthentik.io/providers/scim/user"
GROUP_MAPPING = "goauthentik.io/providers/scim/group"
# The bounds of the waits, in seconds: the first full sync, and the push of
# one change, which authentik sends when it saves the user or the group.
FULL_SYNC = 120
CHANGE = 30
# The messages of every task of authentik, which say nothing of the sync.
ROUTINE = {
    "Task has been queued",
    "Task is being processed",
    "Task finished processing without errors",
}


def request(
    url: str, token: str, method: str = "GET", body: object = None
) -> tuple[int, typing.Any]:
    """The status and answer of a request with a bearer token: JSON, else
    text, or None if empty."""
    data = None if body is None else json.dumps(body).encode()
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
    req = urllib.request.Request(url, data, headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as answer:
            status, raw = answer.status, answer.read()
    except urllib.error.HTTPError as e:
        status, raw = e.code, e.read()
    if not raw.strip():
        return status, None
    try:
        return status, json.loads(raw)
    except ValueError:
        return status, raw.decode(errors="replace")


@dataclass(frozen=True)
class Authentik:
    """authentik's REST API, as its admin."""

    url: str
    token: str

    def __call__(self, method: str, path: str, body: object = None) -> typing.Any:
        """The answer to a request to `/api/v3/<path>`, which must succeed."""
        status, answer = request(f"{self.url}/api/v3/{path}", self.token, method, body)
        assert status < 300, f"authentik: {method} {path} answered {status}: {answer}"
        return answer

    def mappings(self) -> dict[str, str]:
        """The ids of the SCIM property mappings that authentik manages, by
        their managed names."""
        found = self("GET", "propertymappings/provider/scim/")["results"]
        return {mapping["managed"]: mapping["pk"] for mapping in found}

    def sync_log(self, provider: str) -> str:
        """The latest tasks of the provider of that name, with their messages:
        their ids hold it."""
        query = urllib.parse.urlencode({"search": provider, "page_size": 20})
        lines = []
        for task in self("GET", f"tasks/tasks/?{query}")["results"]:
            lines.append(
                f"{task['actor_name']}: {task['aggregated_status']}, "
                f"{task['retries']} retries"
            )
            for log in task["logs"]:
                if log["event"] not in ROUTINE:
                    lines.append(
                        f"  {log['log_level']}: {log['event']} {log['attributes']}"
                    )
        return "\n".join(lines)


@dataclass(frozen=True)
class Person:
    username: str
    name: str
    email: str

    def as_sent(self, uid: str) -> dict[str, typing.Any]:
        """The fields of the user as authentik's default mapping sends them, in
        the form of `fields`, with authentik's id of the user. Each name has
        two words: authentik sends a family name of " " for one word."""
        given, family = self.name.split(" ")
        return {
            "userName": self.username,
            "emails": [(self.email, True)],
            "name": {"formatted": self.name, "givenName": given, "familyName": family},
            "displayName": self.name,
            "active": True,
            "externalId": uid,
        }


@dataclass(frozen=True)
class Account:
    """A user in authentik, and its id at the server."""

    person: Person
    pk: int
    uid: str
    """authentik's id of the user, which it sends as `externalId`."""
    id: str


@dataclass(frozen=True)
class Pushed:
    """What the provider pushed in its first full sync."""

    provider: str
    """The provider's name."""
    group: str
    """authentik's id of the group."""
    group_name: str
    group_id: str
    """The group's id at the server."""
    accounts: dict[str, Account]


def people(tag: str) -> dict[str, Person]:
    """The members of the application's group: alice signs in with a user name
    that is not her email, bob with his email."""
    return {
        "alice": Person(
            f"alice-{tag}", "Alice Liddell", f"alice.liddell-{tag}@example.com"
        ),
        "bob": Person(
            f"bob-{tag}@example.com", "Bob Builder", f"bob-{tag}@example.com"
        ),
        "carol": Person(f"carol-{tag}", "Carol Danvers", f"carol-{tag}@example.com"),
        "dave": Person(f"dave-{tag}", "Dave Bowman", f"dave-{tag}@example.com"),
    }


def pipe(source: socket.socket, target: socket.socket) -> None:
    """Copies `source` to `target` until `source` ends, then ends `target`."""
    try:
        while data := source.recv(65536):
            target.sendall(data)
        target.shutdown(socket.SHUT_WR)
    except OSError:
        pass


@contextmanager
def relay(port: int) -> Iterator[int]:
    """A port on all interfaces of this machine whose connections go on to
    127.0.0.1:`port`."""

    class Handler(socketserver.BaseRequestHandler):
        def handle(self) -> None:
            with socket.create_connection(("127.0.0.1", port)) as upstream:
                back = threading.Thread(target=pipe, args=(upstream, self.request))
                back.start()
                pipe(self.request, upstream)
                back.join()

    with socketserver.ThreadingTCPServer(("0.0.0.0", 0), Handler) as tcp:
        tcp.daemon_threads = True
        threading.Thread(target=tcp.serve_forever, daemon=True).start()
        try:
            yield tcp.server_address[1]
        finally:
            tcp.shutdown()


def wait_until[T](
    what: str,
    check: Callable[[], tuple[bool, T]],
    authentik: Authentik,
    provider: str,
    timeout: float = CHANGE,
) -> T:
    """The last value of `check()`, once it says done. After `timeout` the test
    fails with that value and authentik's log of the tasks of the provider
    `provider` (its name)."""
    deadline = time.monotonic() + timeout
    while True:
        done, seen = check()
        if done:
            return seen
        if time.monotonic() > deadline:
            pytest.fail(
                f"{what}: not within {timeout:.0f} s. The server has: {seen}\n"
                f"authentik's tasks of the provider:\n{authentik.sync_log(provider)}"
            )
        time.sleep(0.5)


def scim(server: typing.Any, path: str) -> tuple[int, typing.Any]:
    """The status and answer of a GET of the server's SCIM API, as its admin."""
    return request(f"{server.address}{SCIM}{path}", server.token)


@pytest.fixture(scope="module")
def authentik() -> Authentik:
    """authentik, once it has its default SCIM mappings: its worker makes them
    from blueprints after it starts."""
    authentik = Authentik(AUTHENTIK_URL, os.environ["UC_TEST_AUTHENTIK_TOKEN"])
    deadline = time.monotonic() + FULL_SYNC
    while not {USER_MAPPING, GROUP_MAPPING} <= authentik.mappings().keys():
        if time.monotonic() > deadline:
            pytest.fail(f"authentik has no default SCIM mappings after {FULL_SYNC} s")
        time.sleep(1)
    return authentik


@pytest.fixture(scope="module")
def scim_url(server: typing.Any) -> Iterator[str]:
    """The server's SCIM API as authentik's worker reaches it, at
    host.docker.internal. Docker Desktop sends that name to the host's
    loopback, where the server listens. On Linux it is the gateway of
    Docker's bridge, so a relay there passes the connections on."""
    port = urllib.parse.urlsplit(server.address).port
    assert port
    if sys.platform != "linux":
        yield f"http://host.docker.internal:{port}{SCIM}"
        return
    with relay(port) as relay_port:
        yield f"http://host.docker.internal:{relay_port}{SCIM}"


@pytest.fixture(scope="module")
def pushed(server: typing.Any, scim_url: str, authentik: Authentik) -> Pushed:
    """A SCIM provider for the server, of a new application whose users are
    the members of a new group, after its first full sync."""
    tag = uuid.uuid4().hex[:8]
    group_name = f"uc-readers-{tag}"
    group = authentik("POST", "core/groups/", {"name": group_name})["pk"]
    users = {
        key: authentik(
            "POST",
            "core/users/",
            {
                "username": person.username,
                "name": person.name,
                "email": person.email,
                "groups": [group],
            },
        )
        for key, person in people(tag).items()
    }
    slug = f"uc-{tag}"
    application = authentik(
        "POST", "core/applications/", {"name": f"Unity Catalog {tag}", "slug": slug}
    )
    # Before the provider: an application without bindings is open to all
    # users, and a sync then would push them all.
    authentik(
        "POST",
        "policies/bindings/",
        {"target": application["pk"], "group": group, "order": 0},
    )
    mappings = authentik.mappings()
    name = f"uc-{tag}"
    provider = authentik(
        "POST",
        "providers/scim/",
        {
            "name": name,
            "url": scim_url,
            "token": server.token,
            "property_mappings": [mappings[USER_MAPPING]],
            "property_mappings_group": [mappings[GROUP_MAPPING]],
            # Only this group: authentik has groups of its own.
            "group_filters": [group],
        },
    )["pk"]
    authentik(
        "PATCH", f"core/applications/{slug}/", {"backchannel_providers": [provider]}
    )
    # authentik runs a full sync every 4 hours, and when it saves a provider,
    # which had no application then.
    schedules = authentik(
        "GET",
        f"tasks/schedules/?rel_obj_id={provider}"
        "&actor_name=authentik.providers.scim.tasks.scim_sync",
    )["results"]
    assert len(schedules) == 1, schedules
    authentik("POST", f"tasks/schedules/{schedules[0]['id']}/send/")

    def synced() -> tuple[bool, tuple[dict[int, str], dict[str, str]]]:
        user_ids = {
            row["user"]: row["scim_id"]
            for row in authentik(
                "GET", f"providers/scim_users/?provider__id={provider}"
            )["results"]
        }
        group_ids = {
            row["group"]: row["scim_id"]
            for row in authentik(
                "GET", f"providers/scim_groups/?provider__id={provider}"
            )["results"]
        }
        done = pks <= user_ids.keys() and group in group_ids
        return done, (user_ids, group_ids)

    pks = {user["pk"] for user in users.values()}
    user_ids, group_ids = wait_until(
        "authentik pushed the users and the group",
        synced,
        authentik,
        name,
        FULL_SYNC,
    )
    assert user_ids.keys() == pks, "authentik pushed users outside the group"
    accounts = {
        key: Account(people(tag)[key], user["pk"], user["uid"], user_ids[user["pk"]])
        for key, user in users.items()
    }
    return Pushed(name, group, group_name, group_ids[group], accounts)


def user_at_server(server: typing.Any, account: Account) -> dict[str, typing.Any]:
    status, user = scim(server, f"/Users/{account.id}")
    assert status == 200, f"GET /Users/{account.id} answered {status}: {user}"
    return user


def fields(user: dict[str, typing.Any]) -> dict[str, typing.Any]:
    """The fields of a SCIM user that authentik sets; of the emails, the
    address and whether it is the primary one."""
    emails = [
        (email["value"], email.get("primary")) for email in user.get("emails") or []
    ]
    return {
        "userName": user.get("userName"),
        "emails": emails,
        "name": user.get("name"),
        "displayName": user.get("displayName"),
        "active": user.get("active"),
        "externalId": user.get("externalId"),
    }


def members(server: typing.Any, pushed: Pushed) -> tuple[int, typing.Any, set[str]]:
    status, group = scim(server, f"/Groups/{pushed.group_id}")
    ids = {member["value"] for member in (group or {}).get("members") or []}
    return status, group, ids


def test_the_users_arrive_as_authentik_sends_them(server, pushed: Pushed) -> None:
    for account in pushed.accounts.values():
        user = user_at_server(server, account)
        assert fields(user) == account.person.as_sent(account.uid), user


def test_a_user_is_found_by_user_name(server, pushed: Pushed) -> None:
    """authentik looks a user up by its user name when the server answers a
    create with 409."""
    account = pushed.accounts["alice"]
    query = urllib.parse.urlencode(
        {"filter": f'userName eq "{account.person.username}"'}
    )
    status, found = scim(server, f"/Users?{query}")
    assert status == 200, found
    assert [user["id"] for user in found["Resources"]] == [account.id], found


def test_the_group_arrives_with_its_members(server, pushed: Pushed, authentik) -> None:
    everyone = {account.id for account in pushed.accounts.values()}
    status, group, _ = wait_until(
        "the group has its members",
        lambda: ((answer := members(server, pushed))[2] == everyone, answer),
        authentik,
        pushed.provider,
    )
    assert status == 200, group
    assert group["displayName"] == pushed.group_name, group


def test_a_new_name_and_email_arrive(server, pushed: Pushed, authentik) -> None:
    """authentik sends the whole user again, with PUT."""
    account = pushed.accounts["alice"]
    tag = account.person.username.removeprefix("alice-")
    person = Person(
        account.person.username, "Alice Hargreaves", f"alice.h-{tag}@example.com"
    )
    authentik(
        "PATCH",
        f"core/users/{account.pk}/",
        {"name": person.name, "email": person.email},
    )
    user = wait_until(
        "the server has alice's new display name",
        lambda: (
            (user := user_at_server(server, account)).get("displayName") == person.name,
            user,
        ),
        authentik,
        pushed.provider,
    )
    assert fields(user) == person.as_sent(account.uid), user


def test_a_member_removed_from_the_group_leaves_it(
    server, pushed: Pushed, authentik
) -> None:
    """authentik sends PATCH with a `remove` of `members[value eq "<id>"]`."""
    bob = pushed.accounts["bob"]
    authentik("POST", f"core/groups/{pushed.group}/remove_user/", {"pk": bob.pk})
    status, group, ids = wait_until(
        "bob left the group",
        lambda: (bob.id not in (answer := members(server, pushed))[2], answer),
        authentik,
        pushed.provider,
    )
    assert status == 200, group
    assert ids == {a.id for key, a in pushed.accounts.items() if key != "bob"}, group
    # authentik deletes bob at the server only in its next full sync.
    assert user_at_server(server, bob).get("active") is True


def test_a_deactivated_user_is_inactive(server, pushed: Pushed, authentik) -> None:
    """authentik sends the whole user with `active` false, with PUT."""
    carol = pushed.accounts["carol"]
    authentik("PATCH", f"core/users/{carol.pk}/", {"is_active": False})
    wait_until(
        "carol is inactive",
        lambda: ((user := user_at_server(server, carol)).get("active") is False, user),
        authentik,
        pushed.provider,
    )


def test_a_deleted_user_is_gone(server, pushed: Pushed, authentik) -> None:
    dave = pushed.accounts["dave"]
    authentik("DELETE", f"core/users/{dave.pk}/")
    wait_until(
        "dave is gone (GET answers 404)",
        lambda: ((answer := scim(server, f"/Users/{dave.id}"))[0] == 404, answer),
        authentik,
        pushed.provider,
    )
