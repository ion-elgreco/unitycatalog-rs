//! What the pages and the sidebar load from the server, kept as upstream's
//! react-query keeps it, but never fresh: a value can change elsewhere, such
//! as the tokens when `uc` makes one.
//!
//! - A view that opens shows the value from the cache at once, if there is
//!   one, and loads it again. Upstream keeps a value fresh for five minutes
//!   (`QUERY_STALE_TIME`), and in that time shows it with no call.
//! - The values with a view load again when the browser tab is shown again,
//!   when the window gets the focus and when the network comes back.
//! - Creates, edits and deletes change the values they touch
//!   ([`Cache::changed`], [`Cache::set`], [`Cache::forget`]): the views
//!   that show them load them again at once.
//! - A load that fails without an answer, or with 408, 429 or 5xx, is
//!   tried 3 more times, after 1, 2 and 4 s, as react-query's default
//!   retry. Other failures show at once: a 404 does not pass.
//! - One load runs per value, for all its views. While a value loads again,
//!   and after a failed load, its views keep what they show.
//!
//! A load runs in a task that no page owns. Its answer goes into the cache,
//! and from there into the signals of the views; a view's signal is
//! disposed with its page, and setting it then does nothing.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::kind::Kind;

/// A value that a view loads from the server.
#[derive(Clone, Debug, PartialEq)]
pub enum Load<T> {
    Loading,
    Ready(T),
    /// The server's message, else what failed.
    Failed(String),
}

impl<T> Load<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            _ => None,
        }
    }
}

/// A part of a loaded value, as a signal: `U::default()` while the value
/// is not ready.
pub fn field<T, U>(
    state: RwSignal<Load<T>>,
    part: impl Fn(&T) -> U + Send + Sync + 'static,
) -> Signal<U>
where
    T: Send + Sync + 'static,
    U: Default + Clone + Send + Sync + 'static,
{
    Signal::derive(move || state.with(|s| s.ready().map(&part).unwrap_or_default()))
}

/// What the UI loads: upstream's query keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Catalogs,
    Catalog(String),
    /// The schemas of a catalog.
    Schemas(String),
    /// A schema, by its full name.
    Schema(String),
    /// The objects of one kind in `catalog.schema`.
    Entries(String, String, Kind),
    Table(String),
    Volume(String),
    Function(String),
    Model(String),
    /// The versions of a registered model, by its full name.
    Versions(String),
    Version(String, i64),
    /// The signed-in user's access tokens.
    Tokens,
}

impl Key {
    /// What the UI says when the load fails and the server's answer has no
    /// message: upstream's `errorMessage`.
    fn failure(&self) -> &'static str {
        match self {
            Key::Catalogs => "Failed to fetch catalogs",
            Key::Catalog(_) => "Failed to fetch catalog",
            Key::Schemas(_) => "Failed to list schemas",
            Key::Schema(_) => "Failed to fetch schema",
            Key::Entries(_, _, Kind::Tables) => "Failed to list tables",
            Key::Entries(_, _, Kind::Volumes) => "Failed to list volumes",
            Key::Entries(_, _, Kind::Functions) => "Failed to list functions",
            Key::Entries(_, _, Kind::Models) => "Failed to list models",
            Key::Table(_) => "Failed to fetch table",
            Key::Volume(_) => "Failed to fetch volume",
            Key::Function(_) => "Failed to fetch function",
            Key::Model(_) => "Failed to fetch model",
            Key::Versions(_) => "Failed to list model versions",
            Key::Version(..) => "Failed to fetch model version",
            Key::Tokens => "Failed to list access tokens",
        }
    }
}

/// The waits before the 3 more tries of a failed load.
const RETRY_WAITS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

/// A loaded value, whatever its type: each key has one.
type Value = Arc<dyn Any + Send + Sync>;

struct Entry {
    /// The last answer.
    value: ArcRwSignal<Load<Value>>,
    /// The load that runs, by its number.
    running: Option<u64>,
    /// The views of the value check it again when this fires.
    check: ArcTrigger,
}

impl Entry {
    fn new() -> Self {
        Self {
            value: ArcRwSignal::new(Load::Loading),
            running: None,
            check: ArcTrigger::new(),
        }
    }
}

/// The loaded values. Its storage belongs to the app, so tasks may keep a
/// copy and use it after their page is gone.
#[derive(Clone, Copy)]
pub struct Cache {
    entries: StoredValue<HashMap<Key, Entry>>,
    loads: StoredValue<u64>,
}

impl Cache {
    /// `key` gained or lost an item, or changed: its views load it again.
    pub fn changed(&self, key: Key) {
        let check = self.with_entry(&key, |entry| {
            // An answer that the change may have outdated is dropped.
            entry.running = None;
            entry.check.clone()
        });
        check.notify();
    }

    /// `key`'s new value, from the answer of an edit.
    pub fn set<T: Send + Sync + 'static>(&self, key: Key, value: T) {
        let signal = self.with_entry(&key, |entry| {
            entry.running = None;
            entry.value.clone()
        });
        signal.set(Load::Ready(Arc::new(value)));
    }

    /// `key` is gone, such as a deleted object: a view of it loads it
    /// again.
    pub fn forget(&self, key: Key) {
        let signal = self.with_entry(&key, |entry| {
            entry.running = None;
            entry.value.clone()
        });
        signal.set(Load::Loading);
    }

    /// Each value with a view loads again.
    fn check_all(&self) {
        let checks: Vec<ArcTrigger> = self
            .entries
            .with_value(|entries| entries.values().map(|e| e.check.clone()).collect());
        for check in checks {
            check.notify();
        }
    }

    /// `f` on `key`'s entry. Signals change after it, outside the lock.
    fn with_entry<R>(&self, key: &Key, f: impl FnOnce(&mut Entry) -> R) -> R {
        self.entries
            .try_update_value(|entries| f(entries.entry(key.clone()).or_insert_with(Entry::new)))
            .expect("the cache lives as long as the app")
    }

    /// Load `key` with `fetch`, unless a load of it runs.
    fn load<T, F, Fut>(&self, key: &Key, fetch: F)
    where
        T: Send + Sync + 'static,
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = api::Result<T>> + 'static,
    {
        self.loads.update_value(|n| *n += 1);
        let number = self.loads.get_value();
        let start = self.with_entry(key, |entry| {
            if entry.running.is_some() {
                return false;
            }
            entry.running = Some(number);
            true
        });
        if !start {
            return;
        }
        let cache = *self;
        let key = key.clone();
        spawn_local(async move {
            let answer = with_retries(fetch).await;
            let current = cache.with_entry(&key, |entry| {
                let current = entry.running == Some(number);
                if current {
                    entry.running = None;
                }
                current.then(|| entry.value.clone())
            });
            let Some(signal) = current else {
                return;
            };
            match answer {
                Ok(value) => signal.set(Load::Ready(Arc::new(value))),
                // Upstream keeps showing the value it has.
                Err(e) => {
                    let message = api::failure(&e, key.failure());
                    if signal.with_untracked(|v| v.ready().is_none()) {
                        signal.set(Load::Failed(message));
                    }
                }
            }
        });
    }
}

/// Keep the loaded values; load the views' values again when the browser
/// tab is shown again, when the window gets the focus and when the network
/// comes back.
pub fn provide_cache() {
    let cache = Cache {
        entries: StoredValue::new(HashMap::new()),
        loads: StoredValue::new(0),
    };
    provide_context(cache);
    // The app's lifetime: the listeners stay.
    let _ = window_event_listener(ev::visibilitychange, move |_| {
        let visible = document().visibility_state() == web_sys::VisibilityState::Visible;
        if visible {
            cache.check_all();
        }
    });
    // Such as after a change in `uc`, in a window beside the browser.
    let _ = window_event_listener(ev::focus, move |_| cache.check_all());
    let _ = window_event_listener(ev::online, move |_| cache.check_all());
}

pub fn use_cache() -> Cache {
    expect_context::<Cache>()
}

/// The value of `key`, which `fetch` loads: from the cache at once, and
/// loaded again as the module doc says. The signal belongs to the view.
pub fn use_load<T, F, Fut>(key: Key, fetch: F) -> RwSignal<Load<T>>
where
    T: Clone + Send + Sync + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: Future<Output = api::Result<T>> + 'static,
{
    let cache = use_cache();
    let (value, check) = cache.with_entry(&key, |entry| (entry.value.clone(), entry.check.clone()));
    let state = RwSignal::new(value.with_untracked(typed::<T>));
    // Each new answer, after the one the view started with.
    Effect::new(move |first: Option<()>| {
        let load = value.with(typed::<T>);
        if first.is_some() {
            state.set(load);
        }
    });
    Effect::new(move |_| {
        check.track();
        cache.load(&key, fetch.clone());
    });
    state
}

/// A cached answer as the type of its key's views.
fn typed<T: Clone + 'static>(load: &Load<Value>) -> Load<T> {
    match load {
        Load::Loading => Load::Loading,
        Load::Ready(value) => match value.downcast_ref::<T>() {
            Some(value) => Load::Ready(value.clone()),
            None => Load::Loading,
        },
        Load::Failed(message) => Load::Failed(message.clone()),
    }
}

/// `fetch`'s answer, after up to 3 more tries for failures that can pass.
async fn with_retries<T, F, Fut>(fetch: F) -> api::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = api::Result<T>>,
{
    let mut answer = fetch().await;
    for wait in RETRY_WAITS {
        match &answer {
            Err(e) if can_pass(e) => {}
            _ => break,
        }
        gloo_timers::future::sleep(wait).await;
        answer = fetch().await;
    }
    answer
}

/// No answer, or a server that was busy or failed.
fn can_pass(e: &api::Error) -> bool {
    match e.status() {
        Some(status) => matches!(status, 408 | 429 | 500..=599),
        None => matches!(e, api::Error::Http(_)),
    }
}
