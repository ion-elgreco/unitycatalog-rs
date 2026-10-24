//! The external locations, from "Manage" in the sidebar: each one's name,
//! URL, credential, owner and creation, and "Create external location".
//! The server lists the ones the user may see. Upstream's console has no
//! such page.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::external_locations::v1::ExternalLocationInfo;

use crate::api;
use crate::components::icons::Plus;
use crate::components::{Empty, Modal, TextField, use_notify};
use crate::layout::{Column, ListLayout};
use crate::load::{Key, Load, use_cache, use_load};
use crate::util::{Section, format_timestamp, or_dash};

#[component]
pub fn ExternalLocations() -> impl IntoView {
    let locations = use_load(Key::ExternalLocations, api::external_locations);
    let create_open = RwSignal::new(false);
    let columns = vec![
        Column {
            title: "Name",
            width: "16%",
            render: Box::new(|l: &ExternalLocationInfo| {
                let name = l.name.clone();
                view! {
                    <a class="uc-btn uc-btn--link" href=Section::ExternalLocations.item_href(&name)>
                        {name.clone()}
                    </a>
                }
                .into_any()
            }),
            search_key: Some(Box::new(|l: &ExternalLocationInfo| l.name.clone())),
        },
        Column {
            title: "URL",
            width: "28%",
            render: Box::new(|l: &ExternalLocationInfo| {
                view! { <code>{or_dash(l.url.clone())}</code> }.into_any()
            }),
            search_key: Some(Box::new(|l: &ExternalLocationInfo| l.url.clone())),
        },
        Column {
            title: "Credential",
            width: "14%",
            render: Box::new(|l: &ExternalLocationInfo| match l.credential_name.clone() {
                Some(credential) => view! {
                    <a class="uc-btn uc-btn--link" href=Section::Credentials.item_href(&credential)>
                        {credential.clone()}
                    </a>
                }
                .into_any(),
                None => view! { <span class="uc-muted">"—"</span> }.into_any(),
            }),
            search_key: Some(Box::new(|l: &ExternalLocationInfo| {
                l.credential_name.clone().unwrap_or_default()
            })),
        },
        Column {
            title: "Owner",
            width: "21%",
            render: Box::new(|l: &ExternalLocationInfo| {
                view! { <span class="uc-muted">{or_dash(l.owner.clone())}</span> }.into_any()
            }),
            search_key: Some(Box::new(|l: &ExternalLocationInfo| {
                l.owner.clone().unwrap_or_default()
            })),
        },
        Column {
            title: "Created",
            width: "21%",
            render: Box::new(|l: &ExternalLocationInfo| {
                view! { <span class="uc-muted uc-nowrap">{format_timestamp(l.created_at)}</span> }
                    .into_any()
            }),
            search_key: None,
        },
    ];
    view! {
        <div class="uc-page">
            <div class="uc-between">
                <h1 class="uc-page__title">"External locations"</h1>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create external location"
                </button>
            </div>
            <p class="uc-muted">
                "An external location is a URL in cloud storage, with the credential that reaches it. External tables and volumes, and the storage roots of catalogs and schemas, are under external locations."
            </p>
            <ListLayout
                data=locations
                columns
                empty=Empty::new(
                    "No external locations.",
                    "Click Create external location to make one: a URL, and the credential that reaches it.",
                )
            />
            <CreateLocation open=create_open/>
        </div>
    }
}

/// The credential of an external location: a choice among the ones the
/// user may see.
#[component]
pub fn CredentialSelect(#[prop(into)] id: String, value: RwSignal<String>) -> impl IntoView {
    let credentials = use_load(Key::Credentials, api::credentials);
    let for_id = id.clone();
    view! {
        <div class="uc-field">
            <label class="uc-label" for=for_id>"Credential"</label>
            {move || match credentials.get() {
                Load::Loading => view! { <p class="uc-hint">"Loading the credentials…"</p> }.into_any(),
                Load::Failed(message) => view! { <p class="uc-error">{message}</p> }.into_any(),
                Load::Ready(list) if list.is_empty() => {
                    view! {
                        <p class="uc-hint">"No credentials yet. Make one on the Credentials page first."</p>
                    }
                        .into_any()
                }
                Load::Ready(list) => {
                    view! {
                        <select
                            id=id.clone()
                            class="uc-select"
                            on:change=move |e| value.set(event_target_value(&e))
                        >
                            <option value="" prop:selected=move || value.with(String::is_empty)>
                                "Choose a credential"
                            </option>
                            {list
                                .into_iter()
                                .map(|c| c.name)
                                .map(|name| {
                                    let this = name.clone();
                                    let text = name.clone();
                                    view! {
                                        <option value=name prop:selected=move || value.with(|v| *v == this)>
                                            {text}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

#[component]
fn CreateLocation(open: RwSignal<bool>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let url = RwSignal::new(String::new());
    let credential = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let close = move || {
        open.set(false);
        name.set(String::new());
        url.set(String::new());
        credential.set(String::new());
        comment.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let new_url = url.get_untracked().trim().to_string();
        if new_url.is_empty() {
            notify.error("URL is required");
            return;
        }
        let credential_name = credential.get_untracked();
        if credential_name.is_empty() {
            notify.error("Credential is required");
            return;
        }
        let text = comment.get_untracked();
        let create = api::client()
            .create_external_location(new_name, new_url, credential_name)
            .with_comment((!text.is_empty()).then_some(text));
        busy.set(true);
        spawn_local(async move {
            match create.await {
                Ok(_) => {
                    notify.success("Create external location complete");
                    cache.changed(Key::ExternalLocations);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        close();
                    }
                }
                Err(e) => notify.failed(&e, "Create external location failed"),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title="Create external location"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <TextField id="uc-location-name" label="Name" value=name required=true/>
            <TextField
                id="uc-location-url"
                label="URL"
                value=url
                required=true
                placeholder="s3://bucket/path"
            />
            <CredentialSelect id="uc-location-credential" value=credential/>
            <TextField id="uc-location-comment" label="Comment" value=comment/>
        </Modal>
    }
}
