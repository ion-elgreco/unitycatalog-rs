//! "Edit": a new name and comment, as the console's `EditMetadataAction`,
//! and, for a catalog or a schema, its properties. The button and its
//! dialog. After a rename, the UI goes to the object's new page.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uc_common::SecurablePropertiesMap;

use super::field::TextField;
use super::icons::{Plus, X};
use crate::api;
use crate::components::{Modal, use_notify};

/// What changes: a new name, a new comment, new properties, or some of
/// these.
pub struct Changes {
    pub new_name: Option<String>,
    pub comment: Option<String>,
    /// All the properties, when one changed: the server replaces them.
    pub properties: Option<SecurablePropertiesMap>,
}

/// A property, as the form holds it.
#[derive(Clone, Copy)]
struct PropertyRow {
    id: u64,
    key: RwSignal<String>,
    value: RwSignal<String>,
}

#[component]
pub fn EditMetadata<F, Fut>(
    #[prop(into)] name: Signal<String>,
    #[prop(into)] comment: Signal<String>,
    /// The properties, for an object that has them: the dialog edits them
    /// too.
    #[prop(into, optional)]
    properties: Option<Signal<Vec<(String, String)>>>,
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
    let rows: RwSignal<Vec<PropertyRow>> = RwSignal::new(Vec::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let navigate = use_navigate();
    // A row's signals belong to the dialog, not to the click that adds the
    // row.
    let owner = StoredValue::new(Owner::current().expect("a component has an owner"));
    let next_row = StoredValue::new(0u64);
    let new_row = move |key: &str, value: &str| {
        next_row.update_value(|n| *n += 1);
        let id = next_row.get_value();
        owner.with_value(|owner| {
            owner.with(|| PropertyRow {
                id,
                key: RwSignal::new(key.to_string()),
                value: RwSignal::new(value.to_string()),
            })
        })
    };
    let edit = move |_| {
        new_name.set(name.get_untracked());
        new_comment.set(comment.get_untracked());
        if let Some(properties) = properties {
            rows.set(
                properties
                    .get_untracked()
                    .iter()
                    .map(|(key, value)| new_row(key, value))
                    .collect(),
            );
        }
        open.set(true);
    };
    // The properties of the form, when they differ from the object's; an
    // error for a value without a key, or a key twice.
    let changed_properties = move || -> Result<Option<SecurablePropertiesMap>, &'static str> {
        let Some(properties) = properties else {
            return Ok(None);
        };
        let mut map = SecurablePropertiesMap::new();
        for row in rows.get_untracked() {
            let key = row.key.get_untracked().trim().to_string();
            let value = row.value.get_untracked();
            if key.is_empty() {
                if value.trim().is_empty() {
                    continue;
                }
                return Err("Every property needs a key");
            }
            if map.insert(key, value).is_some() {
                return Err("A property key is listed twice");
            }
        }
        let before: SecurablePropertiesMap = properties.get_untracked().into_iter().collect();
        Ok((map != before).then_some(map))
    };
    let on_ok = move || {
        let renamed = new_name.get_untracked().trim().to_string();
        if renamed.is_empty() {
            notify.error("Name is required");
            return;
        }
        let properties = match changed_properties() {
            Ok(properties) => properties,
            Err(message) => {
                notify.error(message);
                return;
            }
        };
        let changes = Changes {
            new_name: (renamed != name.get_untracked()).then_some(renamed),
            comment: {
                let text = new_comment.get_untracked();
                (text != comment.get_untracked()).then_some(text)
            },
            properties,
        };
        // Nothing to save: the dialog stays, as the console's.
        if changes.new_name.is_none() && changes.comment.is_none() && changes.properties.is_none() {
            return;
        }
        let save = save.clone();
        let navigate = navigate.clone();
        busy.set(true);
        spawn_local(async move {
            match save(changes).await {
                Ok(page) => {
                    notify.success("Edit metadata complete");
                    if busy.try_get_untracked().is_some() {
                        open.set(false);
                        if let Some(page) = page {
                            navigate(&page, Default::default());
                        }
                    }
                }
                Err(e) => notify.failed(&e, "Edit metadata failed"),
            }
            busy.set(false);
        });
    };
    view! {
        <button type="button" class="uc-btn uc-btn--outline uc-btn--sm" on:click=edit>
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
            <TextField id="uc-edit-comment" label="Comment" value=new_comment/>
            {properties
                .is_some()
                .then(|| {
                    view! {
                        <div class="uc-field">
                            <span class="uc-label">"Properties"</span>
                            <For
                                each=move || rows.get()
                                key=|row| row.id
                                children=move |row| {
                                    view! {
                                        <div class="uc-property-row">
                                            <TextField
                                                id=format!("uc-edit-key-{}", row.id)
                                                label="Key"
                                                value=row.key
                                            />
                                            <TextField
                                                id=format!("uc-edit-value-{}", row.id)
                                                label="Value"
                                                value=row.value
                                            />
                                            <button
                                                type="button"
                                                class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--sm"
                                                title="Remove property"
                                                aria-label="Remove property"
                                                on:click=move |_| {
                                                    rows.update(|rows| rows.retain(|other| other.id != row.id))
                                                }
                                            >
                                                <X/>
                                            </button>
                                        </div>
                                    }
                                }
                            />
                            <button
                                type="button"
                                class="uc-btn uc-btn--outline uc-btn--sm uc-self-start"
                                on:click=move |_| rows.update(|rows| rows.push(new_row("", "")))
                            >
                                <Plus/>
                                "Add property"
                            </button>
                        </div>
                    }
                })}
        </Modal>
    }
}
