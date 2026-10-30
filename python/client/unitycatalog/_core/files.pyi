"""
The classes of the `files` API.
"""

from typing import final

__all__ = [
    "DirectoryEntry",
    "DownloadFileResponse",
    "GetFileMetadataResponse",
    "ListDirectoryContentsResponse",
]

@final
class DirectoryEntry:
    """
    A file or directory in a directory.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def file_size(self, /) -> int | None:
        """
        Size of the file, in bytes. Absent for a directory.
        """
    @property
    def is_directory(self, /) -> bool:
        """
        Whether the entry is a directory.
        """
    @property
    def last_modified(self, /) -> int | None:
        """
        Time at which the file was last changed, in epoch milliseconds. Absent for a directory.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the file or directory: the last part of its path.
        """
    @property
    def path(self, /) -> str:
        """
        Absolute path of the file or directory.
        """

@final
class DownloadFileResponse:
    """
    A file and its metadata.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def content_length(self, /) -> int | None:
        """
        Size of the file, in bytes.
        """
    @property
    def content_range(self, /) -> str | None:
        """
        For a `range`, the bytes that the answer holds and the size of the file, such as
        `bytes 0-1023/4096`.
        """
    @property
    def content_type(self, /) -> str | None:
        """
        Media type of the file.
        """
    @property
    def contents(self, /) -> bytes:
        """
        The bytes of the file.
        """
    @property
    def last_modified(self, /) -> str | None:
        """
        Time at which the file was last changed, as an HTTP date.
        """

@final
class GetFileMetadataResponse:
    """
    The metadata of a file. The response has no body.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def content_length(self, /) -> int | None:
        """
        Size of the file, in bytes.
        """
    @property
    def content_type(self, /) -> str | None:
        """
        Media type of the file.
        """
    @property
    def last_modified(self, /) -> str | None:
        """
        Time at which the file was last changed, as an HTTP date.
        """

@final
class ListDirectoryContentsResponse:
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def contents(self, /) -> list[DirectoryEntry]:
        """
        The files and directories of the directory, directories first.
        """
    @property
    def next_page_token(self, /) -> str | None:
        """
        Opaque token of the next page. Absent on the last page.
        """
