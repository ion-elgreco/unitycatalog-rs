//! Limits per client and per fixed time.

use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// The fewest windows at which those that are over are removed.
const MIN_SWEEP: usize = 1024;
/// The most windows that are kept. A new key then takes the place of
/// another one, whose window starts anew: whoever has that many keys has a
/// new key for each request too.
const MAX_WINDOWS: usize = 100_000;

/// Who a limit counts for: the address of the client
/// ([`RequestContext::client`](crate::RequestContext::client)), an IPv6 one
/// by its /64, which is one subscriber's. Requests without an address count
/// together.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Client(Option<IpAddr>);

impl Client {
    pub(crate) fn of(peer: Option<IpAddr>) -> Self {
        Self(peer.map(|peer| match crate::proxies::unmapped(peer) {
            IpAddr::V6(v6) => {
                let [a, b, c, d, ..] = v6.segments();
                IpAddr::V6([a, b, c, d, 0, 0, 0, 0].into())
            }
            v4 => v4,
        }))
    }
}

/// A `T` per key, which a new one replaces `length` after it was made, or
/// `length` after it was last read or changed ([`Windows::since_last`]).
pub(crate) struct Windows<K, T> {
    length: Duration,
    /// Whether `length` counts from the last [`with`](Self::with), not from
    /// the first.
    since_last: bool,
    inner: std::sync::Mutex<Inner<K, T>>,
}

struct Inner<K, T> {
    windows: HashMap<K, (Instant, T)>,
    /// The number of windows at which those that are over are removed.
    sweep_at: usize,
    /// When those that are over were last removed.
    swept: Option<Instant>,
}

impl<K: Eq + Hash + Clone, T: Default> Windows<K, T> {
    pub(crate) fn new(length: Duration) -> Self {
        Self {
            length,
            since_last: false,
            inner: std::sync::Mutex::new(Inner {
                windows: HashMap::new(),
                sweep_at: MIN_SWEEP,
                swept: None,
            }),
        }
    }

    /// A `T` per key that is kept while [`with`](Self::with) reads or
    /// changes it, and for `length` after the last time: for what counts
    /// in a row, which the start of a new window must not end.
    pub(crate) fn since_last(length: Duration) -> Self {
        Self {
            since_last: true,
            ..Self::new(length)
        }
    }

    /// Read or change the current window of `key`, with its start.
    pub(crate) fn with<R>(&self, key: &K, f: impl FnOnce(&mut T, Instant) -> R) -> R {
        self.with_at(Instant::now(), key, f)
    }

    /// Read the current window of `key`, if it has one. No window is made.
    pub(crate) fn read<R>(&self, key: &K, f: impl FnOnce(&T) -> R) -> Option<R> {
        let now = Instant::now();
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let (start, value) = inner.windows.get(key)?;
        (now.saturating_duration_since(*start) < self.length).then(|| f(value))
    }

    /// As [`with`](Self::with), at the time `now`.
    fn with_at<R>(&self, now: Instant, key: &K, f: impl FnOnce(&mut T, Instant) -> R) -> R {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let inner = &mut *inner;
        let length = self.length;
        let over = |start: &Instant| now.saturating_duration_since(*start) >= length;
        // The windows that are over are removed when a new key makes the
        // map twice as large as it was after the last time, and once per
        // `length` while it has [`MIN_SWEEP`] or more, so the windows of a
        // burst do not stay until the next one.
        let full = inner.windows.len() >= inner.sweep_at && !inner.windows.contains_key(key);
        let swept = *inner.swept.get_or_insert(now);
        let due = inner.windows.len() >= MIN_SWEEP && over(&swept);
        if full || due {
            inner.windows.retain(|_, (start, _)| !over(start));
            inner.sweep_at = (2 * inner.windows.len()).max(MIN_SWEEP);
            inner.windows.shrink_to(inner.sweep_at);
            inner.swept = Some(now);
        }
        if inner.windows.len() >= MAX_WINDOWS && !inner.windows.contains_key(key) {
            let other = inner.windows.keys().next().cloned();
            other.and_then(|other| inner.windows.remove(&other));
        }
        let (start, value) = match inner.windows.get_mut(key) {
            Some(window) => window,
            None => (inner.windows.entry(key.clone())).or_insert_with(|| (now, T::default())),
        };
        if over(start) {
            *start = now;
            *value = T::default();
        }
        let started = *start;
        if self.since_last {
            *start = now;
        }
        f(value, started)
    }
}

/// Says once per `length` that something is due, such as a line of the log
/// that many requests would write.
pub(crate) struct Every {
    length: Duration,
    last: std::sync::Mutex<Option<Instant>>,
}

impl Every {
    pub(crate) fn new(length: Duration) -> Self {
        Self {
            length,
            last: std::sync::Mutex::new(None),
        }
    }

    /// Whether `length` went by since this was last `true`.
    pub(crate) fn due(&self) -> bool {
        self.due_at(Instant::now())
    }

    fn due_at(&self, now: Instant) -> bool {
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        let due = last.is_none_or(|at| now.saturating_duration_since(at) >= self.length);
        if due {
            *last = Some(now);
        }
        due
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

    #[test]
    fn a_window_since_the_last_time_is_kept_while_it_is_used() {
        let length = Duration::from_secs(60);
        let windows: Windows<u8, u32> = Windows::since_last(length);
        let start = Instant::now();
        let count = |at: Instant| {
            windows.with_at(at, &0, |count, _| {
                *count += 1;
                *count
            })
        };
        let step = length - Duration::from_millis(1);
        assert_eq!(
            (count(start), count(start + step), count(start + 2 * step)),
            (1, 2, 3)
        );
        assert_eq!(count(start + 2 * step + length), 1);
    }

    #[test]
    fn the_windows_are_kept_up_to_a_limit() {
        let windows: Windows<usize, u32> = Windows::new(Duration::from_secs(60));
        let start = Instant::now();
        for key in 0..MAX_WINDOWS + 10 {
            windows.with_at(start, &key, |count, _| *count += 1);
        }
        let kept = windows.inner.lock().unwrap().windows.len();
        assert_eq!(kept, MAX_WINDOWS);
        // A key that has a window keeps it.
        let last = MAX_WINDOWS + 9;
        assert_eq!(windows.with_at(start, &last, |count, _| *count), 1);
    }

    #[test]
    fn something_is_due_once_per_length() {
        let length = Duration::from_secs(60);
        let every = Every::new(length);
        let start = Instant::now();
        assert!(every.due_at(start));
        assert!(!every.due_at(start));
        assert!(!every.due_at(start + length - Duration::from_millis(1)));
        assert!(every.due_at(start + length));
        assert!(!every.due_at(start + length));
    }

    #[test]
    fn the_windows_of_a_burst_are_removed_when_they_are_over() {
        let length = Duration::from_secs(60);
        let windows: Windows<usize, u32> = Windows::new(length);
        let start = Instant::now();
        for key in 0..4 * MIN_SWEEP - 100 {
            windows.with_at(start, &key, |_, _| ());
        }
        // No more keys arrive than before the burst.
        windows.with_at(start + length, &0, |_, _| ());
        let kept = windows.inner.lock().unwrap().windows.len();
        assert_eq!(kept, 1);
    }
}
