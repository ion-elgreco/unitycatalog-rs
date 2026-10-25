//! Toasts, as the console's `sonner`: bottom right, an icon, 4 s. Their
//! timers stop while the pointer is on them.

use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::icons::{CircleCheck, CircleX};
use crate::api;

/// How long a toast shows.
const DURATION: Duration = Duration::from_millis(4000);
const TICK: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Success,
    Error,
}

#[derive(Clone)]
struct Toast {
    id: u64,
    message: String,
    level: Level,
}

/// The toasts. Its signals belong to the app, so tasks may keep a copy
/// and use it after their page is gone.
#[derive(Clone, Copy)]
pub struct Notify {
    toasts: RwSignal<Vec<Toast>>,
    next_id: StoredValue<u64>,
    /// The pointer is on the toasts.
    hovered: RwSignal<bool>,
}

impl Notify {
    pub fn success(&self, message: impl Into<String>) {
        self.show(message.into(), Level::Success);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.show(message.into(), Level::Error);
    }

    /// A failed call: the server's message, else `action`'s failure.
    pub fn failed(&self, e: &api::Error, action: &str) {
        self.error(api::failure(e, action));
    }

    fn show(&self, message: String, level: Level) {
        self.next_id.update_value(|id| *id += 1);
        let id = self.next_id.get_value();
        self.toasts
            .update(|toasts| toasts.push(Toast { id, message, level }));
        let notify = *self;
        spawn_local(async move {
            let mut shown = Duration::ZERO;
            while shown < DURATION {
                gloo_timers::future::sleep(TICK).await;
                if !notify
                    .toasts
                    .with_untracked(|toasts| toasts.iter().any(|t| t.id == id))
                {
                    return;
                }
                if !notify.hovered.get_untracked() {
                    shown += TICK;
                }
            }
            notify.close(id);
        });
    }

    fn close(&self, id: u64) {
        self.toasts.update(|toasts| toasts.retain(|t| t.id != id));
        // The pointer leaves no toast behind: the next ones count down.
        if self.toasts.with_untracked(Vec::is_empty) {
            self.hovered.set(false);
        }
    }
}

pub fn provide_notifications() {
    provide_context(Notify {
        toasts: RwSignal::new(Vec::new()),
        next_id: StoredValue::new(0),
        hovered: RwSignal::new(false),
    });
}

pub fn use_notify() -> Notify {
    expect_context::<Notify>()
}

#[component]
pub fn Notifications() -> impl IntoView {
    let notify = use_notify();
    view! {
        <div
            class="uc-toasts"
            on:mouseenter=move |_| notify.hovered.set(true)
            on:mouseleave=move |_| notify.hovered.set(false)
        >
            <For
                each=move || notify.toasts.get()
                key=|t| t.id
                children=move |t| {
                    let icon = match t.level {
                        Level::Success => view! {
                            <span class="uc-toast__icon uc-toast__icon--success">
                                <CircleCheck/>
                            </span>
                        }
                        .into_any(),
                        Level::Error => view! {
                            <span class="uc-toast__icon uc-toast__icon--error">
                                <CircleX/>
                            </span>
                        }
                        .into_any(),
                    };
                    view! {
                        <div class="uc-toast" role="alert">
                            {icon}
                            <div class="uc-toast__message">{t.message}</div>
                        </div>
                    }
                }
            />
        </div>
    }
}
