"""The exception of the client."""


class UcError(Exception):
    """An error from the Unity Catalog server, or a request that got no answer.

    Attributes:
        code: The upstream error code, such as ``"CATALOG_NOT_FOUND"``.
            ``"INTERNAL"`` when the request failed before an answer, for
            example when the server is not reachable.
        message: The error message.
    """

    code: str
    message: str

    def __init__(self, code: str, message: str) -> None:
        super().__init__(code, message)
        self.code = code
        self.message = message

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"
