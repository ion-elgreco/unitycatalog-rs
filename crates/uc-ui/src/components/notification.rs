//! Toasts, as upstream's `utils/NotificationContext` shows them with antd's
//! `notification`: top right, an icon, a close button, 4.5 s. Their timers
//! stop while the pointer is on them. More than 3 toasts stack, the newest
//! on top, until the pointer is on them.

use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::icons::{CheckCircleFilled, CloseCircleFilled, CloseOutlined};
use crate::api;

/// How long a toast shows: antd's `duration`.
const DURATION: Duration = Duration::from_millis(4500);
const TICK: Duration = Duration::from_millis(100);
/// More toasts than this stack: antd's `stack.threshold`.
const STACK_FROM: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Success,
    Error,
}

#[derive(Clone)]
struct Toast {
    id: u64,
    message: String,
    /// A second line, as antd's `description`.
    description: Option<String>,
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
        self.show(message.into(), None, Level::Success);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.show(message.into(), None, Level::Error);
    }

    /// An error with a second line.
    pub fn error_with(&self, message: impl Into<String>, description: impl Into<String>) {
        self.show(message.into(), Some(description.into()), Level::Error);
    }

    /// A failed call: the server's message, else `action`'s failure.
    pub fn failed(&self, e: &api::Error, action: &str) {
        self.error(api::failure(e, action));
    }

    fn show(&self, message: String, description: Option<String>, level: Level) {
        self.next_id.update_value(|id| *id += 1);
        let id = self.next_id.get_value();
        self.toasts.update(|toasts| {
            toasts.push(Toast {
                id,
                message,
                description,
                level,
            })
        });
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
    let stacked =
        Memo::new(move |_| notify.toasts.with(Vec::len) > STACK_FROM && !notify.hovered.get());
    view! {
        <div
            class="uc-toasts"
            class:uc-toasts--stacked=stacked
            on:mouseenter=move |_| notify.hovered.set(true)
            on:mouseleave=move |_| notify.hovered.set(false)
        >
            <For
                each=move || notify.toasts.get()
                key=|t| t.id
                children=move |t| {
                    let id = t.id;
                    // 0 for the newest toast.
                    let place = Memo::new(move |_| {
                        notify
                            .toasts
                            .with(|toasts| toasts.iter().rev().position(|t| t.id == id))
                            .unwrap_or_default()
                    });
                    let style = move || stacked.get().then(|| stack_style(place.get()));
                    let (class, icon) = match t.level {
                        Level::Success => (
                            "uc-toast__icon uc-toast__icon--success",
                            view! { <CheckCircleFilled/> }.into_any(),
                        ),
                        Level::Error => (
                            "uc-toast__icon uc-toast__icon--error",
                            view! { <CloseCircleFilled/> }.into_any(),
                        ),
                    };
                    view! {
                        <div class="uc-toast" role="alert" style=style>
                            <span class=class>{icon}</span>
                            <div class="uc-toast__message">{t.message}</div>
                            {t
                                .description
                                .map(|text| view! { <div class="uc-toast__description">{text}</div> })}
                            <button
                                type="button"
                                class="uc-toast__close"
                                aria-label="Close"
                                on:click=move |_| notify.close(id)
                            >
                                <CloseOutlined/>
                            </button>
                        </div>
                    }
                }
            />
        </div>
    }
}

/// A stacked toast, `place` toasts below the newest: antd moves each 8 px
/// down and narrows it, and hides all but the newest 3.
fn stack_style(place: usize) -> String {
    let width = 384.0;
    let narrower = 16.0 * place.min(STACK_FROM) as f64;
    let opacity = if place < STACK_FROM { 1 } else { 0 };
    format!(
        "transform: translateY({}px) scaleX({}); opacity: {opacity}; z-index: {};",
        8 * place,
        (width - narrower) / width,
        100 - place.min(99)
    )
}
