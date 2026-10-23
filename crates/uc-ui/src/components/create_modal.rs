//! "Create …" with a name, a comment and, for catalogs and schemas, a
//! storage root, as the console's `EntityFormDialog`. After the call, a
//! toast, the list that gained the object loads again, and the UI goes to
//! the object's page. Cancel empties the form; a failed call keeps it, so
//! it can be fixed.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::components::{Modal, use_notify};
use crate::load::{Key, use_cache};

/// What the form holds.
pub struct NewObject {
    pub name: String,
    pub comment: Option<String>,
    /// The storage root, when the form has the field.
    pub location: Option<String>,
}

#[component]
pub fn CreateModal<F, Fut>(
    open: RwSignal<bool>,
    /// What is created, such as "catalog".
    noun: &'static str,
    /// The label of a storage field, such as "Storage root"; none for no
    /// such field.
    #[prop(optional)]
    location: Option<&'static str>,
    /// Create the object; the page to go to then.
    create: F,
    /// The list that gains the object.
    list: Key,
) -> impl IntoView
where
    F: Fn(NewObject) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = api::Result<String>> + 'static,
{
    let name = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let root = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let navigate = use_navigate();
    let close = move || {
        open.set(false);
        name.set(String::new());
        comment.set(String::new());
        root.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let text = comment.get_untracked();
        let location = root.get_untracked().trim().to_string();
        let form = NewObject {
            name: new_name.clone(),
            comment: (!text.is_empty()).then_some(text),
            location: (!location.is_empty()).then_some(location),
        };
        let create = create.clone();
        let list = list.clone();
        let navigate = navigate.clone();
        busy.set(true);
        spawn_local(async move {
            match create(form).await {
                Ok(page) => {
                    notify.success(format!("Created {noun} {new_name}"));
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
            <div class="uc-field">
                <label class="uc-label" for="uc-create-name">
                    <span class="req">"*"</span>
                    "Name"
                </label>
                <input
                    id="uc-create-name"
                    class="uc-input"
                    type="text"
                    required
                    prop:value=move || name.get()
                    on:input=move |e| name.set(event_target_value(&e))
                />
            </div>
            <div class="uc-field">
                <label class="uc-label" for="uc-create-comment">"Comment"</label>
                <textarea
                    id="uc-create-comment"
                    class="uc-textarea"
                    prop:value=move || comment.get()
                    on:input=move |e| comment.set(event_target_value(&e))
                />
            </div>
            {location
                .map(|label| {
                    view! {
                        <div class="uc-field">
                            <label class="uc-label" for="uc-create-location">{label}</label>
                            <input
                                id="uc-create-location"
                                class="uc-input"
                                type="text"
                                placeholder="s3://bucket/path"
                                prop:value=move || root.get()
                                on:input=move |e| root.set(event_target_value(&e))
                            />
                        </div>
                    }
                })}
        </Modal>
    }
}
