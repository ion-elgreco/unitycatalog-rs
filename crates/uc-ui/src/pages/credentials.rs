//! The credentials, from "Manage" in the sidebar: each one's name, AWS IAM
//! role, owner and creation, and "Create credential". The server lists the
//! ones the user may see: all of them to admins, else their own and the
//! ones they may bind external locations to. Upstream's console has no
//! such page.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{AwsIamRoleRequest, CreateCredentialRequest, CredentialInfo, CredentialPurpose};

use crate::api;
use crate::components::icons::Plus;
use crate::components::{Empty, Modal, TextField, use_notify};
use crate::layout::{Column, ListLayout};
use crate::load::{Key, use_cache, use_load};
use crate::util::{Section, format_timestamp, or_dash};

/// The ARN of a credential's AWS IAM role.
pub fn role_arn(c: &CredentialInfo) -> Option<String> {
    c.aws_iam_role.as_ref()?.role_arn.clone()
}

#[component]
pub fn Credentials() -> impl IntoView {
    let credentials = use_load(Key::Credentials, api::credentials);
    let create_open = RwSignal::new(false);
    let columns = vec![
        Column {
            title: "Name",
            width: "24%",
            render: Box::new(|c: &CredentialInfo| {
                let name = c.name.clone().unwrap_or_default();
                view! {
                    <a class="uc-btn uc-btn--link" href=Section::Credentials.item_href(&name)>
                        {name.clone()}
                    </a>
                }
                .into_any()
            }),
            search_key: Some(Box::new(|c: &CredentialInfo| {
                c.name.clone().unwrap_or_default()
            })),
        },
        Column {
            title: "AWS IAM role",
            width: "36%",
            render: Box::new(|c: &CredentialInfo| {
                view! { <code>{or_dash(role_arn(c))}</code> }.into_any()
            }),
            search_key: Some(Box::new(|c: &CredentialInfo| {
                role_arn(c).unwrap_or_default()
            })),
        },
        Column {
            title: "Owner",
            width: "20%",
            render: Box::new(|c: &CredentialInfo| {
                view! { <span class="uc-muted">{or_dash(c.owner.clone())}</span> }.into_any()
            }),
            search_key: Some(Box::new(|c: &CredentialInfo| {
                c.owner.clone().unwrap_or_default()
            })),
        },
        Column {
            title: "Created",
            width: "20%",
            render: Box::new(|c: &CredentialInfo| {
                view! { <span class="uc-muted uc-nowrap">{format_timestamp(c.created_at)}</span> }
                    .into_any()
            }),
            search_key: None,
        },
    ];
    view! {
        <div class="uc-page">
            <div class="uc-between">
                <h1 class="uc-page__title">"Credentials"</h1>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create credential"
                </button>
            </div>
            <p class="uc-muted">
                "A storage credential holds the cloud identity that the server uses to reach storage: an AWS IAM role that it assumes. Each external location is bound to one."
            </p>
            <ListLayout
                data=credentials
                columns
                empty=Empty::new(
                    "No credentials.",
                    "Click Create credential to make one, then bind external locations to it.",
                )
            />
            <CreateCredential open=create_open/>
        </div>
    }
}

#[component]
fn CreateCredential(open: RwSignal<bool>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let role = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let close = move || {
        open.set(false);
        name.set(String::new());
        role.set(String::new());
        comment.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let arn = role.get_untracked().trim().to_string();
        let text = comment.get_untracked();
        let request = CreateCredentialRequest {
            name: new_name,
            comment: (!text.is_empty()).then_some(text),
            aws_iam_role: (!arn.is_empty()).then_some(AwsIamRoleRequest { role_arn: arn }),
            purpose: Some(CredentialPurpose::Storage),
        };
        busy.set(true);
        spawn_local(async move {
            match api::client().create_credential(&request).await {
                Ok(_) => {
                    notify.success("Create credential complete");
                    cache.changed(Key::Credentials);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        close();
                    }
                }
                Err(e) => notify.failed(&e, "Create credential failed"),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title="Create credential"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <TextField id="uc-credential-name" label="Name" value=name required=true/>
            <TextField
                id="uc-credential-role"
                label="AWS IAM role ARN"
                value=role
                placeholder="arn:aws:iam::123456789012:role/unity-catalog"
            />
            <p class="uc-hint">"The role that the server assumes to reach the storage."</p>
            <TextField id="uc-credential-comment" label="Comment" value=comment/>
        </Modal>
    }
}
