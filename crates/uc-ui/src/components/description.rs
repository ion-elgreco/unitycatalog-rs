//! An object's comment, as the console's `DescriptionCard`, with its
//! "Edit" button and dialog.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::card::Card;
use super::field::TextArea;
use super::icons::Pencil;
use crate::api;
use crate::components::{Modal, use_notify};

/// The comment, else "No description provided.".
#[component]
pub fn Description(#[prop(into)] comment: Signal<String>) -> impl IntoView {
    view! {
        <Show
            when=move || comment.with(|c| !c.is_empty())
            fallback=|| view! { <p class="uc-empty">"No description provided."</p> }
        >
            <p class="uc-prewrap">{move || comment.get()}</p>
        </Show>
    }
}

#[component]
pub fn DescriptionCard<F, Fut>(
    #[prop(into)] comment: Signal<String>,
    /// What is described, such as "catalog".
    noun: &'static str,
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
        let value = text.get_untracked();
        busy.set(true);
        spawn_local(async move {
            match save(value).await {
                Ok(()) => {
                    notify.success("Description updated");
                    open.set(false);
                }
                Err(e) => notify.failed(&e, &format!("Failed to update {noun}")),
            }
            busy.set(false);
        });
    };
    view! {
        <Card
            title="Description"
            actions=view! {
                <button type="button" class="uc-btn uc-btn--outline uc-btn--sm" on:click=edit>
                    <Pencil/>
                    "Edit"
                </button>
            }
                .into_any()
        >
            <Description comment/>
        </Card>
        <Modal
            open
            title="Edit description"
            ok_text="Save"
            busy
            on_cancel=move || open.set(false)
            on_ok
        >
            <TextArea id="uc-description" label="Description" value=text/>
        </Modal>
    }
}
