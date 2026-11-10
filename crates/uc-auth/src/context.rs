use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::{Extensions, HeaderMap, Method, header, request::Parts};

/// What the resolvers see of a request: its method, path, headers and
/// extensions, the address of the connection, and its token.
///
/// No `Debug`: it holds the token.
pub struct RequestContext<'a> {
    parts: &'a Parts,
    token: Option<String>,
}

impl<'a> RequestContext<'a> {
    /// The context of a request with `token`, which the
    /// [`Authenticator`](crate::Authenticator) found in its
    /// `Authorization: Bearer` header or one of its cookies.
    pub fn new(parts: &'a Parts, token: Option<String>) -> Self {
        Self { parts, token }
    }

    pub fn method(&self) -> &Method {
        &self.parts.method
    }

    pub fn path(&self) -> &str {
        self.parts.uri.path()
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    /// The trimmed text of header `name`, unless it is missing or blank.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.parts
            .headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// What other layers put into the request, such as a TLS client
    /// certificate.
    pub fn extensions(&self) -> &Extensions {
        &self.parts.extensions
    }

    /// The address of the connection, from axum's `ConnectInfo`, never from
    /// `X-Forwarded-For`. An IPv4 peer of a dual-stack listener
    /// (`::ffff:a.b.c.d`) is its IPv4 address.
    pub fn peer(&self) -> Option<IpAddr> {
        let peer = self
            .parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()?
            .0
            .ip();
        Some(match peer {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(peer, IpAddr::V4),
            IpAddr::V4(_) => peer,
        })
    }

    /// The request's token, if it has one.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

/// The token of an `Authorization: Bearer` header. The scheme is in any
/// case (RFC 9110).
pub fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_string())
}

/// The value of cookie `name`.
pub fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, value)| *key == name && !value.is_empty())
        .map(|(_, value)| value.to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::Request;

    use super::*;

    #[test]
    fn tokens_cookies_and_peers() {
        let (mut parts, ()) = Request::builder()
            .uri("/a/b?c")
            .header(header::AUTHORIZATION, "Bearer  t1 ")
            .header(header::COOKIE, "x=1; session=t2")
            .header("Remote-User", "  ")
            .body(())
            .unwrap()
            .into_parts();
        assert_eq!(bearer_token(&parts.headers).as_deref(), Some("t1"));
        for (value, token) in [
            ("bearer t3", Some("t3")),
            ("Basic t3", None),
            ("Bearer", None),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::AUTHORIZATION, value.parse().unwrap());
            assert_eq!(bearer_token(&headers).as_deref(), token, "{value}");
        }
        assert_eq!(cookie(&parts.headers, "session").as_deref(), Some("t2"));
        assert_eq!(cookie(&parts.headers, "y"), None);

        parts.extensions.insert(ConnectInfo::<SocketAddr>(
            "[::ffff:10.1.2.3]:80".parse().unwrap(),
        ));
        let cx = RequestContext::new(&parts, Some("t1".into()));
        assert_eq!(cx.path(), "/a/b");
        assert_eq!(cx.header("Remote-User"), None);
        assert_eq!(cx.peer(), Some("10.1.2.3".parse().unwrap()));
        assert_eq!(cx.token(), Some("t1"));
    }
}
