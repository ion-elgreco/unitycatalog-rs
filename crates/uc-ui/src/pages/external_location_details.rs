//! An external location's page: its description, its URL and credential,
//! its details, its temporary credentials and its grants; "Edit" (its
//! name, URL, credential and comment) and "Delete".

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uc_api::{ExternalLocationInfo, UpdateExternalLocation};
use uc_common::SecurableType;

use super::external_locations::CredentialSelect;
use crate::api;
use crate::auth::use_manages;
use crate::components::{
    Card, CredentialsCard, DeleteButton, DeleteModal, DescriptionCard, DetailsTab, EntityHeader,
    Loaded, MetaRow, Modal, PermissionsCard, Tabs, TextField, Vendable, use_notify,
};
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{Section, format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn ExternalLocationDetails() -> impl IntoView {
    let params = route_params(["name"]);
    move || {
        let [name] = params.get();
        view! { <LocationPage name/> }
    }
}

#[component]
fn LocationPage(name: String) -> impl IntoView {
    let key = StoredValue::new(name.clone());
    let info = use_load(Key::ExternalLocation(name.clone()), move || {
        let name = key.get_value();
        async move { api::client().get_external_location(&name).await }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let owner = field(info, |l: &ExternalLocationInfo| l.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |l: &ExternalLocationInfo| {
        l.comment.clone().unwrap_or_default()
    });
    let url = field(info, |l: &ExternalLocationInfo| l.url.clone());
    let credential = field(info, |l: &ExternalLocationInfo| {
        l.credential_name.clone().unwrap_or_default()
    });
    let save_comment = move |comment: String| {
        let name = key.get_value();
        async move {
            let request = UpdateExternalLocation {
                comment: Some(comment),
                ..Default::default()
            };
            let updated = api::client()
                .update_external_location(&name, &request)
                .await?;
            cache.set(Key::ExternalLocation(name), updated);
            Ok(())
        }
    };
    let refresh = Callback::new(move |_| cache.changed(Key::ExternalLocation(key.get_value())));

    view! {
        <EntityHeader
            icon=Section::ExternalLocations.icon()
            name=name.clone()
            badges=vec!["External location".to_string()]
            on_refresh=refresh
            actions=view! {
                <EditLocation name=name.clone() url credential comment/>
                <DeleteButton open=delete_open when=manages/>
            }
                .into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || {
                        view! {
                            <DescriptionCard comment noun="external location" save=save_comment/>
                            <Card title="Storage">
                                <dl class="uc-meta">
                                    <dt>"URL"</dt>
                                    <dd class="uc-mono">{move || or_dash(url.get())}</dd>
                                    <dt>"Credential"</dt>
                                    <dd>
                                        {move || {
                                            let credential = credential.get();
                                            match credential.is_empty() {
                                                true => view! { "—" }.into_any(),
                                                false => {
                                                    view! {
                                                        <a class="uc-btn uc-btn--link" href=Section::Credentials.item_href(&credential)>
                                                            {credential.clone()}
                                                        </a>
                                                    }
                                                        .into_any()
                                                }
                                            }
                                        }}
                                    </dd>
                                </dl>
                            </Card>
                        }
                    }
                    details=move || {
                        view! {
                            <DetailsTab rows=field(info, location_rows) json=field(info, pretty_json)/>
                        }
                    }
                    permissions=move || {
                        view! {
                            <CredentialsCard kind=Vendable::Path id=url/>
                            <PermissionsCard
                                kind=SecurableType::ExternalLocation
                                full_name=key.get_value()
                                owner
                            />
                        }
                    }
                />
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="external location"
            name=name.clone()
            delete=move |force| {
                let name = key.get_value();
                async move { api::client().delete_external_location(&name, force).await }
            }
            key=Key::ExternalLocation(name.clone())
            list=Key::ExternalLocations
            then=Section::ExternalLocations.href().to_string()
        />
    }
}

/// "Edit": a new name, URL, credential and comment.
#[component]
fn EditLocation(
    name: String,
    #[prop(into)] url: Signal<Option<String>>,
    #[prop(into)] credential: Signal<String>,
    #[prop(into)] comment: Signal<String>,
) -> impl IntoView {
    let key = StoredValue::new(name);
    let open = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let new_url = RwSignal::new(String::new());
    let new_credential = RwSignal::new(String::new());
    let new_comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let navigate = use_navigate();
    let edit = move |_| {
        new_name.set(key.get_value());
        new_url.set(url.get_untracked().unwrap_or_default());
        new_credential.set(credential.get_untracked());
        new_comment.set(comment.get_untracked());
        open.set(true);
    };
    let on_ok = move || {
        let name = key.get_value();
        let renamed = new_name.get_untracked().trim().to_string();
        if renamed.is_empty() {
            notify.error("Name is required");
            return;
        }
        let moved = new_url.get_untracked().trim().to_string();
        if moved.is_empty() {
            notify.error("URL is required");
            return;
        }
        let rebound = new_credential.get_untracked();
        if rebound.is_empty() {
            notify.error("Credential is required");
            return;
        }
        let text = new_comment.get_untracked();
        let request = UpdateExternalLocation {
            url: (Some(moved.as_str()) != url.get_untracked().as_deref()).then_some(moved),
            new_name: (renamed != name).then_some(renamed),
            comment: (text != comment.get_untracked()).then_some(text),
            credential_name: (rebound != credential.get_untracked()).then_some(rebound),
            owner: None,
        };
        // Nothing to save: the dialog stays, as the console's.
        if request.url.is_none()
            && request.new_name.is_none()
            && request.comment.is_none()
            && request.credential_name.is_none()
        {
            return;
        }
        let navigate = navigate.clone();
        busy.set(true);
        spawn_local(async move {
            match api::client()
                .update_external_location(&name, &request)
                .await
            {
                Ok(updated) => {
                    notify.success("Edit external location complete");
                    let page = request
                        .new_name
                        .as_deref()
                        .map(|new_name| Section::ExternalLocations.item_href(new_name));
                    let renamed = request
                        .new_name
                        .map(|new_name| (Key::ExternalLocation(new_name), Key::ExternalLocations));
                    cache.edited(Key::ExternalLocation(name), updated, renamed);
                    if busy.try_get_untracked().is_some() {
                        open.set(false);
                        if let Some(page) = page {
                            navigate(&page, Default::default());
                        }
                    }
                }
                Err(e) => notify.failed(&e, "Edit external location failed"),
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
            title="Edit external location"
            ok_text="Save"
            busy
            on_cancel=move || open.set(false)
            on_ok
        >
            <TextField id="uc-edit-name" label="Name" value=new_name required=true/>
            <TextField
                id="uc-edit-url"
                label="URL"
                value=new_url
                required=true
                placeholder="s3://bucket/path"
            />
            <CredentialSelect id="uc-edit-credential" value=new_credential/>
            <TextField id="uc-edit-comment" label="Comment" value=new_comment/>
        </Modal>
    }
}

fn location_rows(l: &ExternalLocationInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::mono("URL", or_dash(l.url.clone())),
        MetaRow::new("Credential", or_dash(l.credential_name.clone())),
        MetaRow::new("Owner", or_dash(l.owner.clone())),
        MetaRow::new("Created at", format_timestamp(l.created_at)),
        MetaRow::new("Created by", or_dash(l.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(l.updated_at)),
        MetaRow::new("Updated by", or_dash(l.updated_by.clone())),
        MetaRow::mono("External location ID", or_dash(l.id.clone())),
        MetaRow::mono("Credential ID", or_dash(l.credential_id.clone())),
        MetaRow::new("Comment", or_dash(l.comment.clone())),
    ]
}
