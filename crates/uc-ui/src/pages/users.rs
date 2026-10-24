//! The users, from "Manage" in the sidebar, for admins: each one's name,
//! email, state and creation; "Create user"; and "Delete", which
//! deactivates a user and removes their grants, as the server does. Users
//! also come from the identity provider, at their first sign-in.

use js_sys::Date;
use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::scim::v1::{ScimEmail, ScimUser};
use uc_common::EpochMillis;

use crate::api;
use crate::components::icons::{Plus, Trash2};
use crate::components::{AdminOnly, Empty, Modal, TextField, use_notify};
use crate::layout::{Column, ListLayout};
use crate::load::{Key, use_cache, use_load};
use crate::util::{format_timestamp, or_dash};

#[component]
pub fn Users() -> impl IntoView {
    view! {
        <AdminOnly title="Users">
            <UserList/>
        </AdminOnly>
    }
}

/// When the user was made, from SCIM's `meta.created`.
fn created(user: &ScimUser) -> String {
    let created = user
        .meta
        .as_option()
        .and_then(|meta| meta.created.as_deref());
    let ms = created.map(Date::parse).filter(|ms| !ms.is_nan());
    format_timestamp(ms.map_or(0, |ms| ms as EpochMillis))
}

/// Whether the user can sign in: a user without `active` is.
fn is_active(user: &ScimUser) -> bool {
    user.active != Some(false)
}

#[component]
fn UserList() -> impl IntoView {
    let users = use_load(Key::Users, api::users);
    let create_open = RwSignal::new(false);
    // The user of the delete question.
    let deleting: RwSignal<Option<ScimUser>> = RwSignal::new(None);
    let columns = vec![
        Column {
            title: "Name",
            width: "26%",
            render: Box::new(|u: &ScimUser| {
                view! { <span class="uc-strong">{or_dash(u.display_name.clone())}</span> }
                    .into_any()
            }),
            search_key: Some(Box::new(|u: &ScimUser| {
                u.display_name.clone().unwrap_or_default()
            })),
        },
        Column {
            title: "Email",
            width: "30%",
            render: Box::new(|u: &ScimUser| {
                view! { <span>{u.user_name.clone()}</span> }.into_any()
            }),
            search_key: Some(Box::new(|u: &ScimUser| u.user_name.clone())),
        },
        Column {
            title: "State",
            width: "12%",
            render: Box::new(|u: &ScimUser| match is_active(u) {
                true => {
                    view! { <span class="uc-badge uc-badge--success">"Active"</span> }.into_any()
                }
                false => view! { <span class="uc-badge uc-badge--destructive">"Inactive"</span> }
                    .into_any(),
            }),
            search_key: None,
        },
        Column {
            title: "Created",
            width: "20%",
            render: Box::new(|u: &ScimUser| {
                view! { <span class="uc-muted uc-nowrap">{created(u)}</span> }.into_any()
            }),
            search_key: None,
        },
        Column {
            title: "",
            width: "12%",
            render: Box::new(move |u: &ScimUser| {
                if !is_active(u) {
                    return ().into_any();
                }
                let user = u.clone();
                view! {
                    <div class="uc-right" style="display:flex;justify-content:flex-end">
                        <button
                            type="button"
                            class="uc-btn uc-btn--ghost uc-btn--sm uc-danger"
                            on:click=move |_| deleting.set(Some(user.clone()))
                        >
                            <Trash2/>
                            "Delete"
                        </button>
                    </div>
                }
                .into_any()
            }),
            search_key: None,
        },
    ];
    view! {
        <div class="uc-page">
            <div class="uc-between">
                <h1 class="uc-page__title">"Users"</h1>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create user"
                </button>
            </div>
            <p class="uc-muted">
                "The users of the server. A user's email is the principal of their grants and tokens. Users come from the identity provider at their first sign-in, or from this page."
            </p>
            <ListLayout
                data=users
                columns
                empty=Empty::new("No users.", "Click Create user to add one by email.")
            />
            <CreateUser open=create_open/>
            <DeleteUser deleting/>
        </div>
    }
}

#[component]
fn CreateUser(open: RwSignal<bool>) -> impl IntoView {
    let email = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let close = move || {
        open.set(false);
        email.set(String::new());
        name.set(String::new());
    };
    let on_ok = move || {
        let address = email.get_untracked().trim().to_string();
        if address.is_empty() {
            notify.error("Email is required");
            return;
        }
        let display = name.get_untracked().trim().to_string();
        // The server names the user by its primary email.
        let create = api::client()
            .create_user()
            .with_display_name((!display.is_empty()).then_some(display))
            .with_emails([ScimEmail {
                value: address,
                primary: Some(true),
                ..Default::default()
            }]);
        busy.set(true);
        spawn_local(async move {
            match create.await {
                Ok(_) => {
                    notify.success("Create user complete");
                    cache.changed(Key::Users);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        close();
                    }
                }
                Err(e) => notify.failed(&e, "Create user failed"),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title="Create user"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <TextField
                id="uc-user-email"
                label="Email"
                value=email
                required=true
                placeholder="ada@example.com"
                autocomplete="off"
            />
            <TextField id="uc-user-name" label="Name" value=name/>
        </Modal>
    }
}

/// The delete question for the user in `deleting`.
#[component]
fn DeleteUser(deleting: RwSignal<Option<ScimUser>>) -> impl IntoView {
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let on_ok = move || {
        let Some(user) = deleting.get_untracked() else {
            return;
        };
        let ScimUser {
            id,
            user_name: email,
            ..
        } = user;
        busy.set(true);
        spawn_local(async move {
            match api::client().delete_user(id).await {
                Ok(()) => {
                    notify.success(format!("Deleted the user {email}"));
                    cache.changed(Key::Users);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        deleting.set(None);
                    }
                }
                Err(e) => notify.failed(&e, "Failed to delete user"),
            }
            busy.set(false);
        });
    };
    let email = move || {
        deleting
            .try_with(|u| u.as_ref().map(|u| u.user_name.clone()))
            .flatten()
            .unwrap_or_default()
    };
    view! {
        <Modal
            open=Signal::derive(move || deleting.try_with(Option::is_some).unwrap_or(false))
            title="Delete user"
            ok_text="Delete"
            busy_text="Deleting…"
            danger=true
            busy
            on_cancel=move || deleting.set(None)
            on_ok
        >
            <p>
                "This deactivates "
                <span class="uc-strong">{email}</span>
                " and removes all their grants. They can no longer sign in, and their access tokens stop working."
            </p>
        </Modal>
    }
}
