"""
The classes of the `tokens` API.
"""

from asyncio import Future
from typing import final

__all__ = ["ListPatsResponse", "PatInfo", "TokenClient"]

@final
class ListPatsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
    @property
    def tokens(self, /) -> list[PatInfo]:
        """
        The tokens.
        """

@final
class PatInfo:
    """
    A personal access token of a user. The server keeps a hash of the token only: the token itself
    is in the response of `CreateToken` and nowhere else.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def created_at(self, /) -> int:
        """
        Time at which the token was created, in epoch milliseconds.
        """
    @property
    def expires_at(self, /) -> int | None:
        """
        Time at which the token expires, in epoch milliseconds. Absent or 0: the token does not
        expire.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the token.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the token.
        """
    @property
    def owner(self, /) -> str:
        """
        Username of the owner of the token.
        """
    @property
    def token_prefix(self, /) -> str:
        """
        The first 8 characters of the token, for display only.
        """
    @property
    def token_value(self, /) -> str | None:
        """
        The token. Set only in the response of `CreateToken`.
        """

@final
class TokenClient:
    """
    The requests on one `PatInfo`.
    """
    def delete(self, /) -> None:
        """
        Revoke a personal access token. Only its owner or an admin can revoke it.
        """
    def delete_async(self, /) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
