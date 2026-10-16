//! "Create …" with a name and a comment, as upstream's `Create*Modal`.
//! After the call, a toast, the list that gained the object loads again,
//! and the UI goes to the object's page. Cancel empties the form, as
//! antd's `destroyOnClose`; a failed call keeps it, so it can be fixed.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::components::{Modal, use_notify};
use crate::load::{Key, use_cache};

#[component]
pub fn CreateModal<F, Fut>(
    open: RwSignal<bool>,
    /// What is created, such as "catalog".
    noun: &'static str,
    /// A line above the fields.
    #[prop(optional)]
    hint: Option<&'static str>,
    /// Create the object `name`, with a comment; the page to go to then.
    create: F,
    /// The list that gains the object.
    list: Key,
) -> impl IntoView
where
    F: Fn(String, Option<String>) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = api::Result<String>> + 'static,
{
    let name = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let navigate = use_navigate();
    let close = move || {
        open.set(false);
        name.set(String::new());
        comment.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let text = comment.get_untracked();
        let create = create.clone();
        let list = list.clone();
        let navigate = navigate.clone();
        busy.set(true);
        spawn_local(async move {
            match create(new_name.clone(), (!text.is_empty()).then_some(text)).await {
                Ok(page) => {
                    notify.success(format!("{new_name} {noun} successfully created"));
                    cache.changed(list);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        close();
                        navigate(&page, Default::default());
                    }
                }
                Err(e) => notify.failed(&e, &format!("Failed to create {noun}")),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title=format!("Create {noun}")
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            {hint.map(|hint| view! { <p class="uc-text-secondary">{hint}</p> })}
            <div class="uc-form-item">
                <label>
                    <span class="req">"*"</span>
                    "Name"
                </label>
                <input
                    class="uc-input"
                    type="text"
                    prop:value=move || name.get()
                    on:input=move |e| name.set(event_target_value(&e))
                />
            </div>
            <div class="uc-form-item">
                <label>"Comment"</label>
                <textarea
                    class="uc-textarea"
                    prop:value=move || comment.get()
                    on:input=move |e| comment.set(event_target_value(&e))
                />
            </div>
        </Modal>
    }
}
