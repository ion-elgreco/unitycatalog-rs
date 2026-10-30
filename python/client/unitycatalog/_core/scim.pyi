"""
The classes of the `scim` API.
"""

from typing import Any, final

__all__ = [
    "ListScimGroupsResponse",
    "ListScimResourceTypesResponse",
    "ListScimSchemasResponse",
    "ListScimUsersResponse",
    "ScimAuthenticationScheme",
    "ScimBulkConfig",
    "ScimBulkOp",
    "ScimBulkOpResult",
    "ScimBulkResponse",
    "ScimEmail",
    "ScimFilterConfig",
    "ScimGroup",
    "ScimGroupMember",
    "ScimMeta",
    "ScimPatchOperation",
    "ScimPhoto",
    "ScimResourceType",
    "ScimRole",
    "ScimSchema",
    "ScimSchemaAttribute",
    "ScimServiceProviderConfig",
    "ScimSupported",
    "ScimUser",
]

@final
class ListScimGroupsResponse:
    """
    A page of SCIM groups.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items_per_page(self, /) -> int:
        """
        Number of groups in the page.
        """
    @property
    def resources(self, /) -> list[ScimGroup]:
        """
        The groups.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
        """
    @property
    def start_index(self, /) -> int:
        """
        Index of the first group of the page, from 1.
        """
    @property
    def total_results(self, /) -> int:
        """
        Number of groups in the page.
        """

@final
class ListScimResourceTypesResponse:
    """
    The SCIM resource types of the server, User and Group, as a SCIM list (RFC 7644, section 4).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items_per_page(self, /) -> int:
        """
        Number of resource types.
        """
    @property
    def resources(self, /) -> list[ScimResourceType]:
        """
        The resource types.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
        """
    @property
    def start_index(self, /) -> int:
        """
        Index of the first resource type: 1.
        """
    @property
    def total_results(self, /) -> int:
        """
        Number of resource types.
        """

@final
class ListScimSchemasResponse:
    """
    The SCIM schemas of the server, User and Group, as a SCIM list (RFC 7644, section 4).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items_per_page(self, /) -> int:
        """
        Number of schemas.
        """
    @property
    def resources(self, /) -> list[ScimSchema]:
        """
        The schemas.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
        """
    @property
    def start_index(self, /) -> int:
        """
        Index of the first schema: 1.
        """
    @property
    def total_results(self, /) -> int:
        """
        Number of schemas.
        """

@final
class ListScimUsersResponse:
    """
    A page of SCIM users.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def items_per_page(self, /) -> int:
        """
        Number of users in the page.
        """
    @property
    def meta(self, /) -> ScimMeta | None:
        """
        Time of the list.
        """
    @property
    def resources(self, /) -> list[ScimUser]:
        """
        The users.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
        """
    @property
    def start_index(self, /) -> int:
        """
        Index of the first user of the page, from 1.
        """
    @property
    def total_results(self, /) -> int:
        """
        Number of users in the page.
        """

@final
class ScimAuthenticationScheme:
    """
    A way to authenticate to the SCIM endpoints.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def description(self, /) -> str:
        """
        Description of the scheme.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the scheme.
        """
    @property
    def primary(self, /) -> bool:
        """
        Whether this is the preferred scheme.
        """
    @property
    def type(self, /) -> str:
        """
        Kind of the scheme: `oauthbearertoken`.
        """

@final
class ScimBulkConfig:
    """
    How the server supports SCIM bulk requests.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def max_operations(self, /) -> int:
        """
        The maximum number of operations in a bulk request.
        """
    @property
    def max_payload_size(self, /) -> int:
        """
        The maximum size of a bulk request, in bytes.
        """
    @property
    def supported(self, /) -> bool:
        """
        Whether bulk requests are supported.
        """

@final
class ScimBulkOp:
    """
    One operation of a SCIM bulk request (RFC 7644, section 3.7).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        method: str,
        path: str,
        *,
        bulk_id: str | None = None,
        data: Any | None = None,
    ) -> ScimBulkOp: ...
    def __repr__(self, /) -> str: ...
    @property
    def bulk_id(self, /) -> str | None:
        """
        Identifier of the operation, which the result repeats.
        """
    @property
    def data(self, /) -> dict[str, Any] | None:
        """
        The body of the operation: a `ScimUser` or `ScimGroup` for `POST`, a `ScimPatchOp` (without
        `id`) for `PATCH`.
        """
    @property
    def method(self, /) -> str:
        """
        The HTTP method of the operation: `POST`, `PATCH` or `DELETE`.
        """
    @property
    def path(self, /) -> str:
        """
        The resource of the operation: `/Users` or `/Groups` for `POST`, `/Users/<id>` or
        `/Groups/<id>` for `PATCH` and `DELETE`.
        """

@final
class ScimBulkOpResult:
    """
    The result of one operation of a SCIM bulk request.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def bulk_id(self, /) -> str | None:
        """
        The `bulk_id` of the operation.
        """
    @property
    def location(self, /) -> str | None:
        """
        Path of the resource that the operation created or changed, such as `/scim2/Users/<id>`.
        """
    @property
    def method(self, /) -> str:
        """
        The HTTP method of the operation.
        """
    @property
    def response(self, /) -> dict[str, Any] | None:
        """
        The answer of the operation: the user or group, or an error with `status` and `detail`.
        """
    @property
    def status(self, /) -> str:
        """
        The HTTP status of the operation, such as `201`.
        """

@final
class ScimBulkResponse:
    """
    The answer to a SCIM bulk request: one result per operation that ran, in order.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def operations(self, /) -> list[ScimBulkOpResult]:
        """
        The results of the operations.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:BulkResponse`.
        """

@final
class ScimEmail:
    """
    An email address of a user.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        value: str,
        *,
        primary: bool | None = None,
        email_type: str | None = None,
    ) -> ScimEmail: ...
    def __repr__(self, /) -> str: ...
    @property
    def email_type(self, /) -> str | None:
        """
        Kind of the address, such as `work`. The server does not keep it.
        """
    @property
    def primary(self, /) -> bool | None:
        """
        Whether this is the primary email address of the user.
        """
    @property
    def value(self, /) -> str:
        """
        The email address.
        """

@final
class ScimFilterConfig:
    """
    How the server supports SCIM filters.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def max_results(self, /) -> int:
        """
        The maximum number of resources in an answer.
        """
    @property
    def supported(self, /) -> bool:
        """
        Whether filters are supported.
        """

@final
class ScimGroup:
    """
    A SCIM 2.0 group of users.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def display_name(self, /) -> str:
        """
        Name of the group.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the group. The server assigns it.
        """
    @property
    def members(self, /) -> list[ScimGroupMember]:
        """
        Members of the group.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Group`. Requests
        send the key even when empty; the server fills it in.
        """

@final
class ScimGroupMember:
    """
    A member of a SCIM group.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        value: str,
        *,
        display: str | None = None,
        member_type: str | None = None,
    ) -> ScimGroupMember: ...
    def __repr__(self, /) -> str: ...
    @property
    def display(self, /) -> str | None:
        """
        A display string, which clients set to the name of the user.
        """
    @property
    def member_type(self, /) -> str | None:
        """
        The SCIM resource type of the member: `User`.
        """
    @property
    def value(self, /) -> str:
        """
        The `id` of the user.
        """

@final
class ScimMeta:
    """
    The SCIM `meta` of a resource: its type and when it was created and last changed.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls,
        /,
        *,
        resource_type: str | None = None,
        created: str | None = None,
        last_modified: str | None = None,
        location: str | None = None,
    ) -> ScimMeta: ...
    def __repr__(self, /) -> str: ...
    @property
    def created(self, /) -> str | None:
        """
        Time at which the resource was created, in RFC 3339.
        """
    @property
    def last_modified(self, /) -> str | None:
        """
        Time at which the resource was last changed, in RFC 3339.
        """
    @property
    def location(self, /) -> str | None:
        """
        Path of the resource, such as `/scim2/ResourceTypes/User`. Only the discovery resources have
        it.
        """
    @property
    def resource_type(self, /) -> str:
        """
        The SCIM resource type, such as `User`.
        """

@final
class ScimPatchOperation:
    """
    One operation of a SCIM PATCH request (RFC 7644, section 3.5.2).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(
        cls, /, op: str, *, path: str | None = None, value: Any | None = None
    ) -> ScimPatchOperation: ...
    def __repr__(self, /) -> str: ...
    @property
    def op(self, /) -> str:
        """
        The operation: `add`, `remove` or `replace`, in any case.
        """
    @property
    def path(self, /) -> str | None:
        """
        SCIM attribute path, such as `active`, `displayName`, `members` or
        `members[value eq "<id>"]`. Absent for a `replace` whose `value` holds the attributes.
        """
    @property
    def value(self, /) -> Any:
        """
        The new value. Any JSON: a boolean or string for `active`, a string for `displayName`, an
        array of members for `members`, an object of attributes for a `replace` without `path`.
        """

@final
class ScimPhoto:
    """
    A picture of a user.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, value: str, *, photo_type: str | None = None) -> ScimPhoto: ...
    def __repr__(self, /) -> str: ...
    @property
    def photo_type(self, /) -> str | None:
        """
        Kind of the picture, such as `photo`. The server does not keep it.
        """
    @property
    def value(self, /) -> str:
        """
        URL of the picture. Empty when the user has no picture.
        """

@final
class ScimResourceType:
    """
    A SCIM resource type (RFC 7643, section 6).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def description(self, /) -> str:
        """
        Description of the resource type.
        """
    @property
    def endpoint(self, /) -> str:
        """
        Path of the resources, relative to `/scim2`, such as `/Users`.
        """
    @property
    def id(self, /) -> str:
        """
        Identifier of the resource type, such as `User`.
        """
    @property
    def meta(self, /) -> ScimMeta | None:
        """
        The type and path of the resource type.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the resource type, such as `User`.
        """
    @property
    def schema(self, /) -> str:
        """
        URI of the schema of the resources, such as `urn:ietf:params:scim:schemas:core:2.0:User`.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:ResourceType`.
        """

@final
class ScimRole:
    """
    A SCIM role of a user.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, *, value: str | None = None) -> ScimRole: ...
    def __repr__(self, /) -> str: ...
    @property
    def value(self, /) -> str:
        """
        The role, such as `admin`.
        """

@final
class ScimSchema:
    """
    A SCIM schema (RFC 7643, section 7).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def attributes(self, /) -> list[ScimSchemaAttribute]:
        """
        The attributes of the schema.
        """
    @property
    def description(self, /) -> str:
        """
        Description of the schema.
        """
    @property
    def id(self, /) -> str:
        """
        URI of the schema, such as `urn:ietf:params:scim:schemas:core:2.0:User`.
        """
    @property
    def meta(self, /) -> ScimMeta | None:
        """
        The type and path of the schema.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the schema, such as `User`.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Schema`.
        """

@final
class ScimSchemaAttribute:
    """
    An attribute of a SCIM schema (RFC 7643, section 7).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def multi_valued(self, /) -> bool | None:
        """
        Whether the attribute holds a list of values. Absent: a single value.
        """
    @property
    def name(self, /) -> str:
        """
        Name of the attribute, such as `userName`.
        """
    @property
    def required(self, /) -> bool:
        """
        Whether a resource must have the attribute.
        """
    @property
    def sub_attributes(self, /) -> list[ScimSchemaAttribute]:
        """
        The attributes of a `complex` attribute.
        """
    @property
    def type(self, /) -> str:
        """
        Data type of the attribute: `string`, `boolean` or `complex`.
        """
    @property
    def uniqueness(self, /) -> str | None:
        """
        How the server keeps values unique: `server`. Absent: not unique.
        """

@final
class ScimServiceProviderConfig:
    """
    The SCIM features that the server supports (RFC 7643, section 5).
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def authentication_schemes(self, /) -> list[ScimAuthenticationScheme]:
        """
        The ways to authenticate.
        """
    @property
    def bulk(self, /) -> ScimBulkConfig | None:
        """
        Bulk requests.
        """
    @property
    def change_password(self, /) -> ScimSupported | None:
        """
        Password changes.
        """
    @property
    def etag(self, /) -> ScimSupported | None:
        """
        ETags.
        """
    @property
    def filter(self, /) -> ScimFilterConfig | None:
        """
        Filters.
        """
    @property
    def meta(self, /) -> ScimMeta | None:
        """
        The type and path of the resource.
        """
    @property
    def patch(self, /) -> ScimSupported | None:
        """
        PATCH requests.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the resource:
        `urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig`.
        """
    @property
    def sort(self, /) -> ScimSupported | None:
        """
        Sorting.
        """

@final
class ScimSupported:
    """
    Whether the server supports a SCIM feature.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def supported(self, /) -> bool:
        """
        Whether the feature is supported.
        """

@final
class ScimUser:
    """
    A SCIM 2.0 user. The email of the user is its user name and its one primary email.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def active(self, /) -> bool | None:
        """
        Whether the user can sign in. A new user is always enabled; absent in a replace request:
        unchanged.
        """
    @property
    def display_name(self, /) -> str | None:
        """
        Name of the user.
        """
    @property
    def emails(self, /) -> list[ScimEmail]:
        """
        Email addresses of the user. A new user needs one with `primary` set; a replace request
        does not change the email.
        """
    @property
    def external_id(self, /) -> str | None:
        """
        The identity the user has at the client's identity provider. The token exchange resolves an
        OAuth client (`azp`, `client_id`) to the user by it.
        """
    @property
    def id(self, /) -> str:
        """
        Unique identifier of the user.
        """
    @property
    def meta(self, /) -> ScimMeta | None:
        """
        When the user was created and last changed.
        """
    @property
    def photos(self, /) -> list[ScimPhoto]:
        """
        Pictures of the user. The server keeps the first; a replace request does not change it.
        """
    @property
    def roles(self, /) -> list[ScimRole]:
        """
        Roles of the user. Only `/scim2/Me` sends them: the role `admin` for a caller that can do
        what the metastore admin does. The server ignores roles in requests.
        """
    @property
    def schemas(self, /) -> list[str]:
        """
        SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:User`.
        """
    @property
    def user_name(self, /) -> str:
        """
        The email of the user. Ignored in requests: the server takes the primary email.
        """
