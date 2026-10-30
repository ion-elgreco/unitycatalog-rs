"""
The classes of the `permissions` API.
"""

from _typeshed import SupportsGetItem
from typing import final

__all__ = ["PermissionsChange", "PermissionsList", "PrivilegeAssignment"]

@final
class PermissionsChange:
    """
    A change to the privileges of one principal.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        principal: str,
        *,
        add: SupportsGetItem[int, str] | None = None,
        remove: SupportsGetItem[int, str] | None = None,
    ) -> PermissionsChange: ...
    def __repr__(self, /) -> str: ...
    @property
    def add(self, /) -> list[str]:
        """
        The privileges to grant. The values are those of `PrivilegeAssignment.privileges`.
        """
    @property
    def principal(self, /) -> str:
        """
        The principal whose privileges change: the email address of a user.
        """
    @property
    def remove(self, /) -> list[str]:
        """
        The privileges to revoke. The values are those of `PrivilegeAssignment.privileges`.
        """

@final
class PermissionsList:
    """
    The privileges on a securable, per principal.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def privilege_assignments(self, /) -> list[PrivilegeAssignment]:
        """
        The privileges of each principal.
        """

@final
class PrivilegeAssignment:
    """
    The privileges of one principal on a securable.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def principal(self, /) -> str:
        """
        The principal: the email address of a user.
        """
    @property
    def privileges(self, /) -> list[str]:
        """
        The privileges of the principal. Values: `CREATE CATALOG`, `USE CATALOG`, `CREATE SCHEMA`,
        `USE SCHEMA`, `CREATE TABLE`, `SELECT`, `MODIFY`, `CREATE FUNCTION`, `EXECUTE`,
        `CREATE VOLUME`, `READ VOLUME`, `CREATE MODEL`, `CREATE EXTERNAL LOCATION`, `READ FILES`,
        `WRITE FILES`, `CREATE EXTERNAL TABLE`, `CREATE EXTERNAL VOLUME`, `CREATE MANAGED STORAGE`,
        `CREATE STORAGE CREDENTIAL`.
        """
