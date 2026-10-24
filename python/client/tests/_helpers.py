"""Helpers for the tests; pytest puts this directory on `sys.path`."""

import asyncio
import uuid
from collections.abc import Callable
from typing import Any


def unique(prefix: str) -> str:
    """A new name, so that tests do not share objects on the server."""
    return f"{prefix}_{uuid.uuid4().hex[:8]}"


class Calls:
    """The methods of a client, or of a resource handle such as
    `client.catalog(name)`, in one mode: `sync` calls `name`, `async` awaits
    `name_async` in a new event loop. A method without an `_async` twin sends
    no request, such as `catalog(name)`: the handle that it returns gets the
    same mode."""

    def __init__(self, target: Any, mode: str) -> None:
        self.target = target
        self.mode = mode

    def __getattr__(self, name: str) -> Callable[..., Any]:
        if not hasattr(self.target, f"{name}_async"):
            method = getattr(self.target, name)
            return lambda *args, **kwargs: Calls(method(*args, **kwargs), self.mode)
        if self.mode == "sync":
            return getattr(self.target, name)
        method = getattr(self.target, f"{name}_async")

        def call(*args: Any, **kwargs: Any) -> Any:
            async def run() -> Any:
                return await method(*args, **kwargs)

            return asyncio.run(run())

        return call
