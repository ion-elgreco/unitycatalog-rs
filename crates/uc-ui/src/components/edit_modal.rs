//! "Edit": a new name and comment, as the console's `EditMetadataAction`.
//! The button and its dialog. After a rename, the UI goes to the object's
//! new page.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use super::field::{TextArea, TextField};
use super::icons::Pencil;
use crate::api;
use crate::components::{Modal, use_notify};

/// What changes: a new name, a new comment, or both.
pub struct Changes {
    pub new_name: Option<String>,
    pub comment: Option<String>,
}

#[component]
pub fn EditMetadata<F, Fut>(
    #[prop(into)] name: Signal<String>,
    #[prop(into)] comment: Signal<String>,
    /// What is edited, such as "catalog".
    noun: &'static str,
    /// Save the changes; the page to go to after a rename.
    save: F,
) -> impl IntoView
where
    F: Fn(Changes) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = api::Result<Option<String>>> + 'static,
{
    let open = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let new_comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let navigate = use_navigate();
    let edit = move |_| {
        new_name.set(name.get_untracked());
        new_comment.set(comment.get_untracked());
        open.set(true);
    };
    let on_ok = move || {
        let renamed = new_name.get_untracked().trim().to_string();
        if renamed.is_empty() {
            notify.error("Name is required");
            return;
        }
        let changes = Changes {
            new_name: (renamed != name.get_untracked()).then_some(renamed),
            comment: {
                let text = new_comment.get_untracked();
                (text != comment.get_untracked()).then_some(text)
            },
        };
        if changes.new_name.is_none() && changes.comment.is_none() {
            open.set(false);
            return;
        }
        let save = save.clone();
        let navigate = navigate.clone();
        busy.set(true);
        spawn_local(async move {
            match save(changes).await {
                Ok(page) => {
                    notify.success(format!("Updated the {noun}"));
                    if busy.try_get_untracked().is_some() {
                        open.set(false);
                        if let Some(page) = page {
                            navigate(&page, Default::default());
                        }
                    }
                }
                Err(e) => notify.failed(&e, &format!("Failed to update {noun}")),
            }
            busy.set(false);
        });
    };
    view! {
        <button type="button" class="uc-btn uc-btn--outline uc-btn--sm" on:click=edit>
            <Pencil/>
            "Edit"
        </button>
        <Modal
            open
            title="Edit metadata"
            ok_text="Save"
            busy
            on_cancel=move || open.set(false)
            on_ok
        >
            <TextField id="uc-edit-name" label="Name" value=new_name required=true/>
            <TextArea id="uc-edit-comment" label="Comment" value=new_comment/>
        </Modal>
    }
}
