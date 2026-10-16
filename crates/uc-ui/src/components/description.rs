//! An object's description and its "Edit description" modal, as upstream's
//! `DescriptionBox` with an `Edit*DescriptionModal`.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::icons::EditOutlined;
use crate::api;
use crate::components::{Modal, use_notify};

#[component]
pub fn Description<F, Fut>(
    #[prop(into)] comment: Signal<String>,
    /// What is described, such as "catalog".
    noun: &'static str,
    /// The line above the field, such as "Catalog description".
    hint: &'static str,
    /// The subject of the toast, such as "main catalog".
    label: String,
    /// Save a new comment.
    save: F,
) -> impl IntoView
where
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = api::Result<()>> + 'static,
{
    let open = RwSignal::new(false);
    let text = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let edit = move |_| {
        text.set(comment.get_untracked());
        open.set(true);
    };
    let on_ok = move || {
        let save = save.clone();
        let label = label.clone();
        let value = text.get_untracked();
        busy.set(true);
        spawn_local(async move {
            match save(value).await {
                Ok(()) => {
                    notify.success(format!("{label} successfully updated"));
                    open.set(false);
                }
                Err(e) => notify.failed(&e, &format!("Failed to update {noun}")),
            }
            busy.set(false);
        });
    };
    let has_comment = move || comment.with(|c| !c.is_empty());
    view! {
        <div>
            <h5 class="uc-title-5">
                "Description "
                <Show when=has_comment>
                    <button type="button" class="uc-icon-btn" title="Edit description" on:click=edit>
                        <EditOutlined/>
                    </button>
                </Show>
            </h5>
            <div class="uc-text-secondary uc-pre-wrap">{move || comment.get()}</div>
            <Show when=move || !has_comment()>
                <button type="button" class="uc-btn" on:click=edit>
                    "Add description"
                </button>
            </Show>
        </div>
        <Modal
            open
            title="Edit description"
            ok_text="Save"
            busy
            on_cancel=move || open.set(false)
            on_ok
        >
            <p class="uc-text-secondary">{hint}</p>
            <div class="uc-form-item">
                <textarea
                    class="uc-textarea"
                    prop:value=move || text.get()
                    on:input=move |e| text.set(event_target_value(&e))
                />
            </div>
        </Modal>
    }
}
