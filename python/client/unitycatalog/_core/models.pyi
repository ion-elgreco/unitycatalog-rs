"""
The classes of the `models` API.
"""

from asyncio import Future
from typing import final
from unitycatalog._enums import ModelVersionStatus

__all__ = [
    "ListModelVersionsResponse",
    "ListRegisteredModelsResponse",
    "ModelVersionInfo",
    "RegisteredModelClient",
    "RegisteredModelInfo",
]

@final
class ListModelVersionsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def model_versions(self, /) -> list[ModelVersionInfo]:
        """
        The model versions.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """

@final
class ListRegisteredModelsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
    @property
    def registered_models(self, /) -> list[RegisteredModelInfo]:
        """
        The registered models.
        """

@final
class ModelVersionInfo:
    """
    A version of a registered model. The server gives each new version the next number.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def catalog_name(self, /) -> str:
        """
        Name of the parent catalog.
        """
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the model version was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the model version.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the model version.
        """
    @property
    def model_name(self, /) -> str:
        """
        Name of the parent registered model, relative to its schema.
        """
    @property
    def run_id(self, /) -> str | None:
        """
        ID of the run of the ML package that made the model.
        """
    @property
    def schema_name(self, /) -> str:
        """
        Name of the parent schema.
        """
    @property
    def source(self, /) -> str:
        """
        URI of the source files the model version was created from, such as an MLflow run
        artifact path.
        """
    @property
    def status(self, /) -> ModelVersionStatus | None:
        """
        Status of the model version.
        """
    @property
    def storage_location(self, /) -> str:
        """
        Storage location of the files of the model version.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the model version was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the model version.
        """
    @property
    def version(self, /) -> int:
        """
        Number of the model version.
        """

@final
class RegisteredModelClient:
    """
    The requests on one `RegisteredModelInfo`.
    """
    def delete(self, /, *, force: bool | None = None) -> None:
        """
        Delete a registered model. A model with versions needs `force`.

        Args:
            force: Delete the model versions of the registered model too.
        """
    def delete_async(self, /, *, force: bool | None = None) -> Future[None]:
        """
        Like `delete`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def get(self, /) -> RegisteredModelInfo:
        """
        Get a registered model.

        Returns:
            A registered model: a securable in the three-level namespace (catalog.schema.model) that
            holds
            model versions.
        """
    def get_async(self, /) -> Future[RegisteredModelInfo]:
        """
        Like `get`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """
    def update(
        self, /, *, comment: str | None = None, new_name: str | None = None
    ) -> RegisteredModelInfo:
        """
        Change a registered model.

        Args:
            comment: User-provided free-form text description.
            new_name: New name of the registered model, relative to its schema.

        Returns:
            A registered model: a securable in the three-level namespace (catalog.schema.model) that
            holds
            model versions.
        """
    def update_async(
        self, /, *, comment: str | None = None, new_name: str | None = None
    ) -> Future[RegisteredModelInfo]:
        """
        Like `update`, but returns an `asyncio.Future` of the result. Call it in a running event loop.
        """

@final
class RegisteredModelInfo:
    """
    A registered model: a securable in the three-level namespace (catalog.schema.model) that
    holds model versions.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def catalog_name(self, /) -> str:
        """
        Name of the parent catalog.
        """
    @property
    def comment(self, /) -> str | None:
        """
        User-provided free-form text description.
        """
    @property
    def created_at(self, /) -> int:
        """
        Time at which the registered model was created, in epoch milliseconds.
        """
    @property
    def created_by(self, /) -> str | None:
        """
        Username of the creator of the registered model.
        """
    @property
    def full_name(self, /) -> str:
        """
        Three-level name of the registered model: `catalog.schema.model`.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the registered model.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the registered model, relative to its schema.
        """
    @property
    def owner(self, /) -> str | None:
        """
        Username of the owner of the registered model.
        """
    @property
    def schema_name(self, /) -> str:
        """
        Name of the parent schema.
        """
    @property
    def storage_location(self, /) -> str:
        """
        Storage location under which the model versions keep their files.
        """
    @property
    def updated_at(self, /) -> int:
        """
        Time at which the registered model was last changed, in epoch milliseconds.
        """
    @property
    def updated_by(self, /) -> str | None:
        """
        Username of the user who last changed the registered model.
        """
