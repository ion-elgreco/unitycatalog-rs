//! Limits per client and per fixed time.

use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// The fewest windows at which those that are over are removed.
const MIN_SWEEP: usize = 1024;

/// Who a limit counts for: the address of the client
/// ([`RequestContext::client`](crate::RequestContext::client)), an IPv6 one
/// by its /64, which is one subscriber's. Requests without an address count
/// together.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Client(Option<IpAddr>);

impl Client {
    pub(crate) fn of(peer: Option<IpAddr>) -> Self {
        Self(peer.map(|peer| match peer {
            IpAddr::V4(_) => peer,
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => IpAddr::V4(v4),
                None => {
                    let [a, b, c, d, ..] = v6.segments();
                    IpAddr::V6([a, b, c, d, 0, 0, 0, 0].into())
                }
            },
        }))
    }
}

/// A `T` per key, which a new one replaces `length` after it was made.
pub(crate) struct Windows<K, T> {
    length: Duration,
    inner: std::sync::Mutex<Inner<K, T>>,
}

struct Inner<K, T> {
    windows: HashMap<K, (Instant, T)>,
    /// The number of windows at which those that are over are removed.
    sweep_at: usize,
}

impl<K: Eq + Hash + Clone, T: Default> Windows<K, T> {
    pub(crate) fn new(length: Duration) -> Self {
        Self {
            length,
            inner: std::sync::Mutex::new(Inner {
                windows: HashMap::new(),
                sweep_at: MIN_SWEEP,
            }),
        }
    }

    /// Read or change the current window of `key`, with its start.
    pub(crate) fn with<R>(&self, key: &K, f: impl FnOnce(&mut T, Instant) -> R) -> R {
        self.with_at(Instant::now(), key, f)
    }

    /// As [`with`](Self::with), at the time `now`.
    fn with_at<R>(&self, now: Instant, key: &K, f: impl FnOnce(&mut T, Instant) -> R) -> R {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let inner = &mut *inner;
        let length = self.length;
        let over = |start: &Instant| now.saturating_duration_since(*start) >= length;
        if inner.windows.len() >= inner.sweep_at && !inner.windows.contains_key(key) {
            inner.windows.retain(|_, (start, _)| !over(start));
            inner.sweep_at = (2 * inner.windows.len()).max(MIN_SWEEP);
        }
        let (start, value) = match inner.windows.get_mut(key) {
            Some(window) => window,
            None => (inner.windows.entry(key.clone())).or_insert_with(|| (now, T::default())),
        };
        if over(start) {
            *start = now;
            *value = T::default();
        }
        f(value, *start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clients_are_addresses_and_ipv6_networks() {
        let client = |peer: &str| Client::of(Some(peer.parse().unwrap()));
        assert_eq!(client("::ffff:192.0.2.7"), client("192.0.2.7"));
        assert_ne!(client("192.0.2.8"), client("192.0.2.7"));
        assert_eq!(client("2001:db8:1:2:3::4"), client("2001:db8:1:2::9"));
        assert_ne!(client("2001:db8:1:3::4"), client("2001:db8:1:2::9"));
        assert_ne!(client("192.0.2.7"), Client::of(None));
    }

    #[test]
    fn a_window_starts_anew_and_old_ones_are_removed() {
        let length = Duration::from_secs(60);
        let windows: Windows<usize, u32> = Windows::new(length);
        let start = Instant::now();
        let count = |at: Instant, key: usize| {
            windows.with_at(at, &key, |count, _| {
                *count += 1;
                *count
            })
        };
        assert_eq!(
            (count(start, 0), count(start, 0), count(start, 1)),
            (1, 2, 1)
        );
        for key in 2..MIN_SWEEP {
            count(start, key);
        }
        let last = start + length - Duration::from_millis(1);
        assert_eq!(count(last, 0), 3);
        let next = start + length;
        assert_eq!(count(next, 0), 1);
        // A new key at the limit removes those that are over.
        assert_eq!(count(next, MIN_SWEEP), 1);
        let kept = windows.inner.lock().unwrap().windows.len();
        assert_eq!(kept, 2);
    }
}
