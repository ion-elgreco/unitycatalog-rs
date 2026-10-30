"""
The classes of the `recipients` API.
"""

from .sharing import RecipientProfile
from _typeshed import SupportsGetItem
from asyncio import Future
from typing import final
from unitycatalog._enums import AuthenticationType

__all__ = ["ListRecipientsResponse", "RecipientClient", "RecipientInfo"]

@final
class ListRecipientsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items(self, /) -> list[RecipientInfo]:
        """
        The recipients.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """

@final
class RecipientClient:
    """
    The requests on one `RecipientInfo`.
    """
    def create_recipient_profile(self, /) -> RecipientProfile:
        """
        Replace the bearer token of a recipient and return a Delta Sharing profile with the new token. The
        old token stops working.

        Returns:
            A Delta Sharing profile file (`profile.json`): what a Delta Sharing client needs to connect
            to
            the server as a recipient.
        """
    def create_recipient_profile_async(self, /) -> Future[RecipientProfile]:
        """
        Like `create_recipient_profile`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def delete(self, /) -> None:
        """
        Delete a recipient.
        """
    def delete_async(self, /) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> RecipientInfo:
        """
        Get a recipient. The response does not hold the bearer token.

        Returns:
            A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares
            with
            a bearer token. The server keeps a hash of the token only.
        """
    def get_async(self, /) -> Future[RecipientInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def rotate_recipient_token(self, /) -> RecipientInfo:
        """
        Replace the bearer token of a recipient. The old token stops working. The response holds the new
        token.

        Returns:
            A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares
            with
            a bearer token. The server keeps a hash of the token only.
        """
    def rotate_recipient_token_async(self, /) -> Future[RecipientInfo]:
        """
        Like `rotate_recipient_token`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self,
        /,
        *,
        comment: str | None = None,
        add_shares: SupportsGetItem[int, str] | None = None,
        remove_shares: SupportsGetItem[int, str] | None = None,
    ) -> RecipientInfo:
        """
        Change a recipient: its comment and the shares it can read.

        Args:
            comment: User-provided free-form text description.
            add_shares: Names of the shares to give the recipient access to.
            remove_shares: Names of the shares to take the recipient's access away from.

        Returns:
            A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares
            with
            a bearer token. The server keeps a hash of the token only.
        """
    def update_async(
        self,
        /,
        *,
        comment: str | None = None,
        add_shares: SupportsGetItem[int, str] | None = None,
        remove_shares: SupportsGetItem[int, str] | None = None,
    ) -> Future[RecipientInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class RecipientInfo:
    """
    A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares with
    a bearer token. The server keeps a hash of the token only.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def authentication_type(self, /) -> AuthenticationType | None:
        """
        How the recipient signs in.
        """
    @property
    def bearer_token(self, /) -> str | None:
        """
        The bearer token. Set only in the responses of `CreateRecipient` and `RotateRecipientToken`.
        """
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the recipient was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the recipient.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the recipient.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the recipient.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the recipient.
        """
    @property
    def shares(self, /) -> list[str]:
        """
        Names of the shares that the recipient can read.
        """
    @property
    def token_prefix(self, /) -> str:
        """
        The first 8 characters of the bearer token, for display only.
        """
