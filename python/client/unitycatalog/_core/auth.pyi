"""
The classes of the `auth` API.
"""

from typing import final

__all__ = ["AuthMode", "OAuthTokenExchangeInfo", "TokenResponse"]

@final
class AuthMode:
    """
    How the web UI signs in, and where its "Log out" goes.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def logout_url(self, /) -> str | None:
        """
        Where "Log out" goes. Absent: the web UI hides "Log out".
        """
    @property
    def mode(self, /) -> str:
        """
        How the web UI signs in: `oidc` (with the OpenID Connect provider of the server), `forward`
        (through an auth proxy in front of the server), `none` (no sign-in in the browser) or `off`
        (authorization is off, and no one signs in). `uc web` answers `token`: the browser uses the
        token of the `uc` commands.
        """

@final
class OAuthTokenExchangeInfo:
    """
    An OAuth 2.0 token exchange response (RFC 8693): a Unity Catalog access token.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def access_token(self, /) -> str:
        """
        The Unity Catalog access token.
        """
    @property
    def expires_in(self, /) -> int:
        """
        Lifetime of the access token, in seconds.
        """
    @property
    def issued_token_type(self, /) -> str:
        """
        The type of the issued token: `urn:ietf:params:oauth:token-type:access_token`.
        """
    @property
    def token_type(self, /) -> str:
        """
        How to send the token: `Bearer`.
        """

@final
class TokenResponse:
    """
    An OAuth 2.0 token response (RFC 6749, section 5.1). The access token is the personal access
    token itself.
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str: ...
    @property
    def access_token(self, /) -> str:
        """
        The access token: the personal access token of the request.
        """
    @property
    def expires_in(self, /) -> int:
        """
        Seconds until the token expires: the time until the token of `client_id` expires, else 3600.
        """
    @property
    def scope(self, /) -> str | None:
        """
        The scope of the request.
        """
    @property
    def token_type(self, /) -> str:
        """
        How to send the token: `Bearer`.
        """
