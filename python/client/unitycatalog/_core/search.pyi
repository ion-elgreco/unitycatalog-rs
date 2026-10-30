"""
The classes of the `search` API.
"""

from typing import final

__all__ = ["SearchObjectsResponse", "SearchResult"]

@final
class SearchObjectsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
    @property
    def results(self, /) -> list[SearchResult]:
        """
        The results: first the names equal to the query, then the names that start with it, the
        names that hold it, and the objects whose comment holds its words. Only the objects that the
        caller may read.
        """

@final
class SearchResult:
    """
    A catalog, schema, table, volume, function or registered model whose name or comment holds the
    words of a search.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def catalog_name(self, /) -> str:
        """
        Name of the catalog of the object, or of the catalog itself.
        """
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the object was created, in epoch milliseconds.
        """
    @property
    def full_name(self, /) -> str:
        """
        Full name of the object: `<catalog>`, `<catalog>.<schema>` or `<catalog>.<schema>.<name>`.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the object.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the object, relative to its parent.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the object.
        """
    @property
    def schema_name(self, /) -> str | None:
        """
        Name of the schema of the object, or of the schema itself. Absent for a catalog.
        """
    @property
    def securable_type(self, /) -> str:
        """
        Type of the object: `catalog`, `schema`, `table`, `volume`, `function` or `registered_model`.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the object was last changed, in epoch milliseconds.
        """
