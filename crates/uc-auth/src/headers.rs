use std::net::IpAddr;

use async_trait::async_trait;
use axum::http::HeaderMap;
use ipnet::IpNet;

use crate::{Identity, RequestContext, Resolution, Resolver};

/// An auth proxy in front of the service (such as oauth2-proxy or
/// Authelia) signs users in and names the user in its headers on every
/// request.
///
/// The headers count only on a connection from one of the proxy's
/// addresses, as anyone else could send them; the address is the
/// connection's, never `X-Forwarded-For`. The first of the headers that the
/// request has names the caller, so the proxy must set or remove each of
/// them: a header that it lets through from the client names any user.
/// That header names nobody when the request has it more than once, as a
/// proxy that adds its value to the client's would make it, or when its
/// value has a `,`, which is two values in one line, or when its value is
/// not UTF-8. The headers after it are not read. Requests from elsewhere, or
/// without the headers, pass to
/// the next resolver.
///
/// Some proxies name users with a `,`, such as the subject of a client
/// certificate (`CN=alice,OU=eng`). [`with_commas`](Self::with_commas)
/// takes such values.
#[derive(Debug, Clone)]
pub struct TrustedHeaders {
    provider: String,
    proxies: Vec<IpNet>,
    headers: Vec<String>,
    commas: bool,
}

impl TrustedHeaders {
    /// The proxy `provider` at `proxies`, which names the user in the first
    /// of `headers` that it sends.
    pub fn new(
        provider: impl Into<String>,
        proxies: impl IntoIterator<Item = IpNet>,
        headers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            provider: provider.into(),
            proxies: proxies.into_iter().collect(),
            headers: headers.into_iter().map(Into::into).collect(),
            commas: false,
        }
    }

    /// A value with a `,` is one name. Only for a proxy that replaces the
    /// client's header: one that adds its value to the client's in one line
    /// would name a user that the client chose a part of.
    pub fn with_commas(mut self, commas: bool) -> Self {
        self.commas = commas;
        self
    }

    /// Whether `peer`, the address of a connection, is one of the proxy's.
    /// An IPv4 peer of a dual-stack listener shows as `::ffff:a.b.c.d`.
    pub fn trusts(&self, peer: IpAddr) -> bool {
        let v4 = match peer {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4),
            IpAddr::V4(_) => None,
        };
        self.proxies
            .iter()
            .any(|net| net.contains(&peer) || v4.is_some_and(|v4| net.contains(&v4)))
    }

    /// The user the proxy names in `headers`, whatever their address: the
    /// value of the first of the proxy's headers that is there and not
    /// blank. `None` when that header is there more than once, in several
    /// lines or as one value with a `,` (unless
    /// [`with_commas`](Self::with_commas)), or when its value is not UTF-8.
    pub fn user<'a>(&self, headers: &'a HeaderMap) -> Option<&'a str> {
        for header in &self.headers {
            let mut values = headers.get_all(header.as_str()).iter();
            let Some(first) = values.next() else {
                continue;
            };
            let Ok(value) = std::str::from_utf8(first.as_bytes()).map(str::trim) else {
                tracing::warn!(%header, "the auth proxy's header is not UTF-8; it names nobody");
                return None;
            };
            if values.next().is_some() || (!self.commas && value.contains(',')) {
                tracing::warn!(%header, "the auth proxy's header has more than one value; it names nobody");
                return None;
            }
            if !value.is_empty() {
                return Some(value);
            }
        }
        None
    }
}

#[async_trait]
impl<S: Send + Sync> Resolver<S> for TrustedHeaders {
    async fn resolve(&self, cx: &RequestContext<'_>, _state: &S) -> Resolution {
        if !cx.peer().is_some_and(|peer| self.trusts(peer)) {
            return Resolution::Pass;
        }
        match self.user(cx.headers()) {
            Some(user) => Resolution::Resolved(Identity::new(
                &self.provider,
                Identity::TRUSTED_HEADERS,
                user,
            )),
            None => Resolution::Pass,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::extract::ConnectInfo;
    use axum::http::Request;

    use super::*;

    #[tokio::test]
    async fn only_the_proxies_name_users() {
        let proxy = TrustedHeaders::new(
            "proxy",
            [
                "10.0.0.0/8".parse().unwrap(),
                "192.0.2.7/32".parse().unwrap(),
            ],
            ["Remote-Email", "Remote-User"],
        );
        assert!(proxy.trusts("10.1.2.3".parse().unwrap()));
        assert!(proxy.trusts("::ffff:192.0.2.7".parse().unwrap()));
        assert!(!proxy.trusts("192.0.2.8".parse().unwrap()));

        let resolve = |peer: Option<&str>, headers: &[(&str, &str)]| {
            let mut request = Request::builder();
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            let (mut parts, ()) = request.body(()).unwrap().into_parts();
            if let Some(peer) = peer {
                let peer: SocketAddr = peer.parse().unwrap();
                parts.extensions.insert(ConnectInfo(peer));
            }
            let proxy = proxy.clone();
            async move {
                let cx = RequestContext::new(&parts, None);
                match Resolver::<()>::resolve(&proxy, &cx, &()).await {
                    Resolution::Resolved(identity) => Some(identity.subject),
                    _ => None,
                }
            }
        };
        let both = [
            ("Remote-User", "alice"),
            ("Remote-Email", "alice@example.com"),
        ];
        assert_eq!(
            resolve(Some("10.0.0.1:1"), &both).await.as_deref(),
            Some("alice@example.com")
        );
        let user = [("Remote-User", "alice"), ("Remote-Email", " ")];
        assert_eq!(
            resolve(Some("10.0.0.1:1"), &user).await.as_deref(),
            Some("alice")
        );
        assert_eq!(resolve(Some("10.0.0.1:1"), &[]).await, None);
        // The client's value, and the proxy's added to it.
        let twice = [("Remote-User", "admin"), ("Remote-User", "alice")];
        assert_eq!(resolve(Some("10.0.0.1:1"), &twice).await, None);
        let joined = [("Remote-User", "admin, alice")];
        assert_eq!(resolve(Some("10.0.0.1:1"), &joined).await, None);
        // A proxy whose names have a `,`: one line is one name, two are not.
        let named = proxy.clone().with_commas(true);
        let mut dn = HeaderMap::new();
        dn.insert("Remote-User", "CN=alice,OU=eng".parse().unwrap());
        assert_eq!(named.user(&dn), Some("CN=alice,OU=eng"));
        assert_eq!(proxy.user(&dn), None);
        dn.append("Remote-User", "admin".parse().unwrap());
        assert_eq!(named.user(&dn), None);
        // A value that is not ASCII names the user. One that cannot be read
        // names nobody, and the next header is not read.
        let utf8 = [("Remote-Email", "jos\u{e9}@example.com"), both[0]];
        assert_eq!(
            resolve(Some("10.0.0.1:1"), &utf8).await.as_deref(),
            Some("jos\u{e9}@example.com")
        );
        let mut latin1 = HeaderMap::new();
        let value = axum::http::HeaderValue::from_bytes(b"jos\xe9@example.com").unwrap();
        latin1.insert("Remote-Email", value);
        latin1.insert("Remote-User", "alice".parse().unwrap());
        assert_eq!(proxy.user(&latin1), None);
        let blank_twice = [("Remote-Email", ""), ("Remote-Email", ""), both[0]];
        assert_eq!(resolve(Some("10.0.0.1:1"), &blank_twice).await, None);
        // The header that names the user decides; those after it are not
        // read.
        let after = [("Remote-Email", "alice@example.com"), joined[0]];
        assert_eq!(
            resolve(Some("10.0.0.1:1"), &after).await.as_deref(),
            Some("alice@example.com")
        );
        let after = [both[1], twice[0], twice[1]];
        assert_eq!(
            resolve(Some("10.0.0.1:1"), &after).await.as_deref(),
            Some("alice@example.com")
        );
        assert_eq!(resolve(Some("192.0.2.8:1"), &both).await, None);
        assert_eq!(resolve(None, &both).await, None);
    }
}
