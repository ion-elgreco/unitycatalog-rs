//! The client's address behind proxies.

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};

use axum::http::{HeaderMap, request::Parts};
use ipnet::{IpNet, Ipv4Net};

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
///
/// Each address of the proxies can name any client: whoever connects from
/// one of them writes the header. A network that holds more than the
/// proxies, such as all the pods of a cluster, lets each of its hosts choose
/// its address, so name the proxies only, or let only them reach the
/// service.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies(Vec<IpNet>);

impl TrustedProxies {
    /// A network of IPv4-mapped addresses (`::ffff:10.0.0.0/104`) is its
    /// IPv4 network, as the addresses are compared in that form.
    pub fn new(proxies: impl IntoIterator<Item = IpNet>) -> Self {
        Self(proxies.into_iter().map(unmapped_net).collect())
    }

    /// Whether `address` is one of the proxies'. An IPv4 peer of a
    /// dual-stack listener shows as `::ffff:a.b.c.d`.
    pub fn trusts(&self, address: IpAddr) -> bool {
        let address = unmapped(address);
        self.0.iter().any(|net| net.contains(&address))
    }

    /// The client of a request with `headers` on a connection from `peer`:
    /// `peer` itself, unless it is a proxy that names another address.
    #[cfg(test)]
    fn client(&self, peer: IpAddr, headers: &HeaderMap) -> IpAddr {
        let peer = unmapped(peer);
        match self.trusts(peer) {
            true => self.forwarded(peer, headers),
            false => peer,
        }
    }

    /// The client that `headers` name on a connection from the proxy `peer`.
    fn forwarded(&self, peer: IpAddr, headers: &HeaderMap) -> IpAddr {
        // A line that is no text is one value that is no address.
        let lines = headers.get_all(X_FORWARDED_FOR).iter().rev();
        let forwarded = lines.flat_map(|line| line.to_str().unwrap_or_default().rsplit(','));
        // The address that the last proxy saw, then the one before it, up
        // to the first that is no proxy. A value that is no address ends
        // the chain: what is before it cannot be trusted.
        let mut client = peer;
        for value in forwarded {
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

    /// Put the [`ClientAddress`] into the extensions of a request from one
    /// of the proxies. The client of any other request is the address of
    /// its connection. Such a request with `X-Forwarded-For` may come from a
    /// proxy that is not named: the log says so once.
    pub fn mark(&self, parts: &mut Parts) {
        static LOGGED: AtomicBool = AtomicBool::new(false);
        let Some(peer) = RequestContext::new(parts, None).peer() else {
            return;
        };
        if self.trusts(peer) {
            let client = self.forwarded(peer, &parts.headers);
            parts.extensions.insert(ClientAddress(client));
        } else if !LOGGED.load(Ordering::Relaxed)
            && parts.headers.contains_key(X_FORWARDED_FOR)
            && !LOGGED.swap(true, Ordering::Relaxed)
        {
            tracing::warn!(
                %peer,
                "a request with X-Forwarded-For came from an address that is no trusted proxy; \
                 if it is a proxy, all its callers count as one client for the limits per client"
            );
        }
    }
}

/// The IPv4 address of `::ffff:a.b.c.d`, which is how an IPv4 peer of a
/// dual-stack listener shows. Any other address as it is.
pub(crate) fn unmapped(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(address, IpAddr::V4),
        IpAddr::V4(_) => address,
    }
}

/// The IPv4 network of a network of IPv4-mapped addresses. Any other
/// network as it is.
fn unmapped_net(net: IpNet) -> IpNet {
    let IpNet::V6(v6) = net else {
        return net;
    };
    let v4 = (v6.network().to_ipv4_mapped())
        .zip(v6.prefix_len().checked_sub(96))
        .and_then(|(address, prefix)| Ipv4Net::new(address, prefix).ok());
    v4.map_or(net, IpNet::V4)
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
    fn a_network_of_mapped_addresses_is_its_ipv4_network() {
        let proxies = TrustedProxies::new([
            "::ffff:10.42.0.0/112".parse().unwrap(),
            "::ffff:192.0.2.7/128".parse().unwrap(),
            "2001:db8::/32".parse().unwrap(),
        ]);
        for peer in ["10.42.0.5", "::ffff:10.42.0.5", "192.0.2.7", "2001:db8::1"] {
            assert!(proxies.trusts(peer.parse().unwrap()), "{peer}");
        }
        for peer in ["10.43.0.5", "192.0.2.8", "2001:db9::1"] {
            assert!(!proxies.trusts(peer.parse().unwrap()), "{peer}");
        }
    }

    #[test]
    fn a_line_that_is_no_text_ends_the_chain() {
        let proxies = TrustedProxies::new(["10.0.0.0/8".parse().unwrap()]);
        let no_text = axum::http::HeaderValue::from_bytes(b"\xff").unwrap();
        let peer: IpAddr = "10.0.0.1".parse().unwrap();
        // What a client sent, and what the proxy added.
        let mut headers = HeaderMap::new();
        headers.append(X_FORWARDED_FOR, "192.0.2.7".parse().unwrap());
        headers.append(X_FORWARDED_FOR, no_text.clone());
        assert_eq!(proxies.forwarded(peer, &headers), peer);
        headers.append(X_FORWARDED_FOR, "192.0.2.8".parse().unwrap());
        assert_eq!(proxies.forwarded(peer, &headers).to_string(), "192.0.2.8");
        let mut headers = HeaderMap::new();
        headers.append(X_FORWARDED_FOR, "192.0.2.7".parse().unwrap());
        headers.append(X_FORWARDED_FOR, no_text);
        headers.append(X_FORWARDED_FOR, "10.0.0.2".parse().unwrap());
        assert_eq!(proxies.forwarded(peer, &headers).to_string(), "10.0.0.2");
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
