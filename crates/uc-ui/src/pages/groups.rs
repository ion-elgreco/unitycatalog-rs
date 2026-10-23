//! The groups, from "Manage" in the sidebar, for admins: each one's name
//! and the count of its members, and "Create group". A group's page has
//! its members.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{ScimGroup, schema_uri};

use crate::api;
use crate::components::icons::Plus;
use crate::components::{AdminOnly, Empty, Modal, TextField, use_notify};
use crate::layout::{Column, ListLayout};
use crate::load::{Key, use_cache, use_load};
use crate::util::Section;

#[component]
pub fn Groups() -> impl IntoView {
    view! {
        <AdminOnly title="Groups">
            <GroupList/>
        </AdminOnly>
    }
}

#[component]
fn GroupList() -> impl IntoView {
    let groups = use_load(Key::Groups, api::groups);
    let create_open = RwSignal::new(false);
    let columns = vec![
        Column {
            title: "Name",
            width: "60%",
            render: Box::new(|g: &ScimGroup| {
                view! {
                    <a class="uc-btn uc-btn--link" href=Section::Groups.item_href(&g.display_name)>
                        {g.display_name.clone()}
                    </a>
                }
                .into_any()
            }),
            search_key: Some(Box::new(|g: &ScimGroup| g.display_name.clone())),
        },
        Column {
            title: "Members",
            width: "40%",
            render: Box::new(|g: &ScimGroup| {
                let count = g.members.as_ref().map_or(0, Vec::len);
                view! { <span class="uc-muted">{count}</span> }.into_any()
            }),
            search_key: None,
        },
    ];
    view! {
        <div class="uc-page">
            <div class="uc-between">
                <h1 class="uc-page__title">"Groups"</h1>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create group"
                </button>
            </div>
            <p class="uc-muted">
                "A group names users together: a grant to the group is a grant to each of its members."
            </p>
            <ListLayout
                data=groups
                columns
                empty=Empty::new(
                    "No groups.",
                    "Click Create group to make one, then add its members on its page.",
                )
            />
            <CreateGroup open=create_open/>
        </div>
    }
}

#[component]
fn CreateGroup(open: RwSignal<bool>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let close = move || {
        open.set(false);
        name.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let group = ScimGroup {
            schemas: vec![schema_uri::GROUP.to_string()],
            display_name: new_name,
            ..Default::default()
        };
        busy.set(true);
        spawn_local(async move {
            match api::client().create_group(&group).await {
                Ok(_) => {
                    notify.success("Create group complete");
                    cache.changed(Key::Groups);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        close();
                    }
                }
                Err(e) => notify.failed(&e, "Create group failed"),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title="Create group"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <TextField id="uc-group-name" label="Name" value=name required=true/>
        </Modal>
    }
}
