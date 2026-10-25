//! "Delete" in a detail page's actions, as the console's
//! `OwnerDeleteAction`: a dialog that asks for the object's name, then the
//! call. After it, a toast, the list that lost the object loads again, and
//! the UI goes to the parent page.
//!
//! The server deletes a catalog, schema or model only when it holds
//! nothing, unless the call sets `force`; the console never sets it. Here,
//! for an admin (see [`crate::auth::Session::is_admin`]), the dialog counts
//! what the object holds when it opens. An object that holds something
//! gets a box that sets `force`: the delete waits for it, and asks again,
//! in a second dialog over the first. An object that holds nothing gets
//! neither.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use super::icons::Trash2;
use crate::api;
use crate::auth::use_auth;
use crate::components::{Modal, use_notify};
use crate::contents::{Contents, Holder};
use crate::load::{Key, Load, use_cache};

#[component]
pub fn DeleteModal<F, Fut>(
    open: RwSignal<bool>,
    /// What is deleted, such as "catalog".
    noun: &'static str,
    /// The name to type, as the console asks for it: the object's own.
    name: String,
    /// Delete the object; with `force`, also what it holds.
    delete: F,
    /// The object, which the UI forgets.
    key: Key,
    /// The list that loses the object.
    #[prop(optional)]
    list: Option<Key>,
    /// The page to go to afterwards.
    then: String,
    /// Why the object cannot be deleted now: the modal says so, and OK is
    /// off, unless the box of `force` is on.
    #[prop(into, optional)]
    blocked: Signal<Option<&'static str>>,
    /// What the object can hold, for the box of `force`, which admins
    /// get; none for objects that hold nothing.
    #[prop(optional)]
    holder: Option<Holder>,
) -> impl IntoView
where
    F: Fn(bool) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = api::Result<()>> + 'static,
{
    let busy = RwSignal::new(false);
    let forced = RwSignal::new(false);
    // The name as typed: the delete waits for the whole of it.
    let typed = RwSignal::new(String::new());
    // What goes with the object: counted when the dialog opens. A newer
    // count wins.
    let counted: RwSignal<Option<Load<Contents>>> = RwSignal::new(None);
    let counts = StoredValue::new(0u64);
    // The second step: "Are you sure?".
    let confirming = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let auth = use_auth();
    let navigate = use_navigate();
    let holder = StoredValue::new(holder);
    let expected = StoredValue::new(name.clone());
    let can_force =
        move || holder.with_value(Option::is_some) && auth.session.with(|s| s.is_admin());
    let close = move || {
        open.set(false);
        confirming.set(false);
        forced.set(false);
        counted.set(None);
        typed.set(String::new());
    };
    let count = move || {
        if counted.get_untracked().is_some() || !untrack(can_force) {
            return;
        }
        let Some(holder) = holder.get_value() else {
            return;
        };
        counted.set(Some(Load::Loading));
        counts.update_value(|n| *n += 1);
        let this = counts.get_value();
        spawn_local(async move {
            let answer = holder.count().await;
            let load = match answer {
                Ok(contents) => Load::Ready(contents),
                Err(e) => Load::Failed(api::failure(&e, "Failed to count")),
            };
            if counts.try_get_value() == Some(this) {
                counted.set(Some(load));
            }
        });
    };
    let run = move |force: bool| {
        let delete = delete.clone();
        let key = key.clone();
        let list = list.clone();
        let then = then.clone();
        let navigate = navigate.clone();
        let lists = match force {
            true => holder.with_value(|h| h.as_ref().map(Holder::lists).unwrap_or_default()),
            false => Vec::new(),
        };
        busy.set(true);
        spawn_local(async move {
            match delete(force).await {
                Ok(()) => {
                    notify.success(format!("{noun} deleted"));
                    if let Some(list) = list {
                        cache.changed(list);
                    }
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        close();
                        navigate(&then, Default::default());
                    }
                    cache.forget(key);
                    for key in lists {
                        cache.forget(key);
                    }
                }
                Err(e) => notify.failed(&e, &format!("Failed to delete {noun}")),
            }
            busy.set(false);
        });
    };
    Effect::new(move |_| {
        if open.get() {
            count();
        }
    });
    // The first step: without the box, the delete; with it, the second
    // step.
    let on_ok = {
        let run = run.clone();
        move || match forced.get_untracked() && untrack(can_force) {
            true => confirming.set(true),
            false => run(false),
        }
    };
    let counting = move || matches!(counted.get(), Some(Load::Loading));
    // What the count found: something, which the delete takes only with
    // the box on, or nothing.
    let holds = move || {
        counted
            .with(|c| matches!(c, Some(Load::Ready(contents)) if *contents != Contents::default()))
    };
    let empty = move || {
        counted
            .with(|c| matches!(c, Some(Load::Ready(contents)) if *contents == Contents::default()))
    };
    let named = move || typed.with(|typed| expected.with_value(|name| typed == name));
    // The second step, by what the count found.
    let summary = {
        let name = name.clone();
        move || {
            let what = holder.with_value(|h| h.as_ref().map(Holder::noun).unwrap_or("it"));
            let data = holder.with_value(|h| h.as_ref().map(Holder::data_note).unwrap_or_default());
            let held = match counted.get() {
                Some(Load::Ready(contents)) => match contents.describe() {
                    Some(text) => format!(" and all it holds: {text}"),
                    None => String::new(),
                },
                _ => " and all it holds".to_string(),
            };
            format!("This deletes {what} {name}{held}. {data}")
        }
    };
    let prompt = format!("Enter {name} to confirm");
    view! {
        <Modal
            open
            title=format!("Delete {noun}?")
            ok_text="Delete permanently"
            busy_text="Deleting…"
            danger=true
            busy=Signal::derive(move || busy.get() && !confirming.get())
            disabled=Signal::derive(move || {
                !named()
                    || counting()
                    || (holds() && !forced.get())
                    || (blocked.get().is_some() && !forced.get() && !empty())
            })
            on_cancel=move || close()
            on_ok
        >
            {
                let name = name.clone();
                move || match (blocked.get(), forced.get() || empty()) {
                    (Some(reason), false) => view! { <p class="uc-muted">{reason}</p> }.into_any(),
                    _ => {
                        view! {
                            <p class="uc-muted">
                                "This cannot be undone. Enter "
                                <strong>{name.clone()}</strong>
                                " to confirm."
                            </p>
                        }
                            .into_any()
                    }
                }
            }
            <input
                class="uc-input"
                type="text"
                aria-label=prompt.clone()
                autocomplete="off"
                prop:value=move || typed.get()
                on:input=move |e| typed.set(event_target_value(&e))
            />
            {move || can_force().then(|| view! { <ForceChoice counted holder forced/> })}
        </Modal>
        <Modal
            open=confirming
            title="Are you sure?"
            ok_text="Delete everything"
            busy_text="Deleting…"
            danger=true
            busy
            on_cancel=move || confirming.set(false)
            on_ok=move || run(true)
        >
            <p>{summary.clone()}</p>
            <p class="uc-strong">"This cannot be undone."</p>
        </Modal>
    }
}

/// "Delete" in a page's actions, for those who manage the object (`when`):
/// it opens the modal.
#[component]
pub fn DeleteButton(open: RwSignal<bool>, when: Memo<bool>) -> impl IntoView {
    view! {
        <Show when=move || when.get()>
            <button
                type="button"
                class="uc-btn uc-btn--destructive uc-btn--sm"
                on:click=move |_| open.set(true)
            >
                <Trash2/>
                "Delete"
            </button>
        </Show>
    }
}

/// The first step's count, and the box that deletes what it found too.
/// Nothing for an object that holds nothing.
#[component]
fn ForceChoice(
    counted: RwSignal<Option<Load<Contents>>>,
    holder: StoredValue<Option<Holder>>,
    forced: RwSignal<bool>,
) -> impl IntoView {
    let what = holder.with_value(|h| h.as_ref().map(Holder::noun).unwrap_or("it"));
    let label = holder.with_value(|h| h.as_ref().map(Holder::force_label).unwrap_or_default());
    move || {
        let note = match counted.get() {
            None | Some(Load::Loading) => {
                let text = format!("Counting what {what} holds…");
                return view! { <p class="uc-hint">{text}</p> }.into_any();
            }
            Some(Load::Ready(contents)) => match contents.describe() {
                Some(text) => format!("{} holds {text}.", capitalized(what)),
                None => return ().into_any(),
            },
            Some(Load::Failed(message)) => {
                format!("The UI could not count what {what} holds: {message}")
            }
        };
        view! {
            <p class="uc-hint">{note}</p>
            <label class="uc-checkbox">
                <input
                    type="checkbox"
                    prop:checked=move || forced.get()
                    on:change=move |e| forced.set(event_target_checked(&e))
                />
                <span>{label}</span>
            </label>
        }
        .into_any()
    }
}

/// `the catalog` → `The catalog`.
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
