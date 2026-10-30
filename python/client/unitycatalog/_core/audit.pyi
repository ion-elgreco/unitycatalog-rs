"""
The classes of the `audit` API.
"""

from typing import final

__all__ = ["AuditEvent", "ListAuditEventsResponse"]

@final
class AuditEvent:
    """
    One request that could change the catalog: a `POST`, `PATCH`, `PUT` or `DELETE`.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the event.
        """
    @property
    def method(self, /) -> str:
        """
        HTTP method of the request, such as `POST` or `DELETE`.
        """
    @property
    def path(self, /) -> str:
        """
        Path of the request, with the query string.
        """
    @property
    def principal(self, /) -> str:
        """
        Username of the principal that sent the request, or `anonymous`.
        """
    @property
    def status(self, /) -> int:
        """
        HTTP status code of the response.
        """
    @property
    def timestamp(self, /) -> int:
        """
        Time at which the response was sent, in epoch milliseconds.
        """

@final
class ListAuditEventsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def events(self, /) -> list[AuditEvent]:
        """
        The events, newest first.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
