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
/// them: a header that it lets through from the client names any user. A
/// header that the request has more than once names nobody, as a proxy that
/// adds its value to the client's would make it; nor does a value with a
/// `,`, which is two values in one line. Requests from elsewhere, or
/// without the headers, pass to the next resolver.
#[derive(Debug, Clone)]
pub struct TrustedHeaders {
    provider: String,
    proxies: Vec<IpNet>,
    headers: Vec<String>,
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
        }
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

    /// The user the proxy names in `headers`, whatever their address:
    /// `None` when one of the proxy's headers is there more than once, in
    /// several lines or as one value with a `,`.
    pub fn user<'a>(&self, headers: &'a HeaderMap) -> Option<&'a str> {
        let twice = |name: &&String| {
            let mut values = headers.get_all(name.as_str()).iter();
            let first = values.next();
            values.next().is_some() || first.is_some_and(|v| v.as_bytes().contains(&b','))
        };
        if let Some(header) = self.headers.iter().find(twice) {
            tracing::warn!(%header, "the auth proxy's header has more than one value; it names nobody");
            return None;
        }
        self.headers.iter().find_map(|name| {
            (headers.get(name)?.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
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
        assert_eq!(resolve(Some("192.0.2.8:1"), &both).await, None);
        assert_eq!(resolve(None, &both).await, None);
    }
}
