"""Helpers for the tests; pytest puts this directory on `sys.path`."""

import asyncio
import uuid
from collections.abc import Callable
from typing import Any

from unitycatalog import Client


def unique(prefix: str) -> str:
    """A new name, so that tests do not share objects on the server."""
    return f"{prefix}_{uuid.uuid4().hex[:8]}"


class Calls:
    """The methods of a client in one mode: `sync` calls `name`, `async`
    awaits `name_async` in a new event loop."""

    def __init__(self, client: Client, mode: str) -> None:
        self.client = client
        self.mode = mode

    def __getattr__(self, name: str) -> Callable[..., Any]:
        if self.mode == "sync":
            return getattr(self.client, name)
        method = getattr(self.client, f"{name}_async")

        def call(*args: Any, **kwargs: Any) -> Any:
            async def run() -> Any:
                return await method(*args, **kwargs)

            return asyncio.run(run())

        return call
