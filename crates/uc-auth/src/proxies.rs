//! The client's address behind proxies.

use std::net::{IpAddr, SocketAddr};

use axum::http::{HeaderMap, request::Parts};
use ipnet::IpNet;

use crate::RequestContext;

const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// The address of the client of a request, where it is not the address of
/// the connection: a host behind proxies puts it into the request's
/// extensions ([`TrustedProxies::mark`]) before the request is
/// authenticated. [`RequestContext::client`] reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientAddress(pub IpAddr);

/// The proxies in front of the service that add the address of their client
/// to `X-Forwarded-For`, such as an ingress.
///
/// The header counts only on a connection from one of the proxies, as
/// anyone else could send it. The client is then the last address of the
/// header that is not a proxy's: the proxies add at the end, so the
/// addresses before it are the client's to choose. A proxy that does not
/// add to the header lets the client choose its address.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies(Vec<IpNet>);

impl TrustedProxies {
    pub fn new(proxies: impl IntoIterator<Item = IpNet>) -> Self {
        Self(proxies.into_iter().collect())
    }

    /// Whether `address` is one of the proxies'. An IPv4 peer of a
    /// dual-stack listener shows as `::ffff:a.b.c.d`.
    pub fn trusts(&self, address: IpAddr) -> bool {
        let address = unmapped(address);
        self.0.iter().any(|net| net.contains(&address))
    }

    /// The client of a request with `headers` on a connection from `peer`:
    /// `peer` itself, unless it is a proxy that names another address.
    pub fn client(&self, peer: IpAddr, headers: &HeaderMap) -> IpAddr {
        let peer = unmapped(peer);
        if !self.trusts(peer) {
            return peer;
        }
        let lines = headers.get_all(X_FORWARDED_FOR).iter();
        let forwarded: Vec<&str> = lines
            .filter_map(|line| line.to_str().ok())
            .flat_map(|line| line.split(','))
            .collect();
        // The address that the last proxy saw, then the one before it, up
        // to the first that is no proxy. A value that is no address ends
        // the chain: what is before it cannot be trusted.
        let mut client = peer;
        for value in forwarded.into_iter().rev() {
            let Some(address) = address(value.trim()) else {
                break;
            };
            client = address;
            if !self.trusts(address) {
                break;
            }
        }
        client
    }

    /// Put the [`ClientAddress`] into the extensions of a request, when it
    /// is known.
    pub fn mark(&self, parts: &mut Parts) {
        if self.0.is_empty() {
            return;
        }
        let Some(peer) = RequestContext::new(parts, None).peer() else {
            return;
        };
        let client = self.client(peer, &parts.headers);
        parts.extensions.insert(ClientAddress(client));
    }
}

fn unmapped(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(address, IpAddr::V4),
        IpAddr::V4(_) => address,
    }
}

/// An address of `X-Forwarded-For`, which some proxies write with a port.
fn address(value: &str) -> Option<IpAddr> {
    let plain = value.parse::<IpAddr>().ok();
    let with_port = || value.parse::<SocketAddr>().ok().map(|socket| socket.ip());
    plain.or_else(with_port).map(unmapped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(peer: &str, forwarded: &[&str]) -> String {
        let proxies = TrustedProxies::new(["10.0.0.0/8".parse().unwrap()]);
        let mut headers = HeaderMap::new();
        for line in forwarded {
            headers.append(X_FORWARDED_FOR, line.parse().unwrap());
        }
        proxies.client(peer.parse().unwrap(), &headers).to_string()
    }

    #[test]
    fn the_client_is_the_last_address_that_is_no_proxy() {
        assert_eq!(client("10.0.0.1", &["192.0.2.7"]), "192.0.2.7");
        // The addresses before it are the client's to choose.
        assert_eq!(
            client("10.0.0.1", &["198.51.100.1, 192.0.2.7"]),
            "192.0.2.7"
        );
        assert_eq!(
            client("10.0.0.1", &["198.51.100.1", "192.0.2.7, 10.0.0.2"]),
            "192.0.2.7"
        );
        assert_eq!(
            client("::ffff:10.0.0.1", &["[2001:db8::1]:443"]),
            "2001:db8::1"
        );
        assert_eq!(client("10.0.0.1", &["192.0.2.7:1234"]), "192.0.2.7");
    }

    #[test]
    fn the_header_counts_only_from_a_proxy() {
        assert_eq!(client("192.0.2.9", &["192.0.2.7"]), "192.0.2.9");
        assert_eq!(client("10.0.0.1", &[]), "10.0.0.1");
        // Proxies only: the first of them.
        assert_eq!(client("10.0.0.1", &["10.0.0.3, 10.0.0.2"]), "10.0.0.3");
        // No address: what is before it does not count.
        assert_eq!(client("10.0.0.1", &["192.0.2.7, unknown"]), "10.0.0.1");
        assert_eq!(
            client("10.0.0.1", &["192.0.2.7, unknown, 10.0.0.2"]),
            "10.0.0.2"
        );
    }

    #[test]
    fn a_request_is_marked_with_its_client() {
        use axum::extract::ConnectInfo;
        let proxies = TrustedProxies::new(["10.0.0.0/8".parse().unwrap()]);
        let request = axum::http::Request::builder()
            .header(X_FORWARDED_FOR, "192.0.2.7")
            .body(())
            .unwrap();
        let (mut parts, ()) = request.into_parts();
        proxies.mark(&mut parts);
        TrustedProxies::default().mark(&mut parts);
        assert_eq!(RequestContext::new(&parts, None).client(), None);
        let peer: SocketAddr = "10.0.0.1:5000".parse().unwrap();
        parts.extensions.insert(ConnectInfo(peer));
        assert_eq!(RequestContext::new(&parts, None).client(), peer.ip().into());
        proxies.mark(&mut parts);
        let client = RequestContext::new(&parts, None).client();
        assert_eq!(client, Some("192.0.2.7".parse().unwrap()));
    }
}
