//! A credential's page: its description and the external locations bound
//! to it, its details, its grants; "Edit" (its name, its role and its
//! comment) and "Delete".

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uc_client::models::credentials::v1::{AwsIamRoleRequest, CredentialInfo};
use uc_common::SecurableType;

use super::credentials::role_arn;
use crate::api;
use crate::auth::use_manages;
use crate::components::{
    Card, DeleteButton, DeleteModal, DescriptionCard, DetailsTab, Empty, EntityHeader, Loaded,
    MetaRow, Modal, ObjectRow, PermissionsCard, Tabs, TextField, loaded_list, use_notify,
};
use crate::contents::Holder;
use crate::load::{Key, Load, field, use_cache, use_load};
use crate::util::{Section, enum_name, format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn CredentialDetails() -> impl IntoView {
    let params = route_params(["name"]);
    move || {
        let [name] = params.get();
        view! { <CredentialPage name/> }
    }
}

#[component]
fn CredentialPage(name: String) -> impl IntoView {
    let key = StoredValue::new(name.clone());
    let info = use_load(Key::Credential(name.clone()), move || {
        let name = key.get_value();
        async move { api::client().credential(name).get().await }
    });
    let locations = use_load(Key::ExternalLocations, api::external_locations);
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let owner = field(info, |c: &CredentialInfo| c.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |c: &CredentialInfo| {
        c.comment.clone().unwrap_or_default()
    });
    let role = field(info, |c: &CredentialInfo| role_arn(c).unwrap_or_default());
    let save_comment = move |comment: String| {
        let name = key.get_value();
        async move {
            let updated = api::client()
                .credential(&name)
                .update()
                .with_comment(comment)
                .await?;
            cache.set(Key::Credential(name), updated);
            Ok(())
        }
    };
    let reload = vec![Key::Credential(name.clone()), Key::ExternalLocations];
    // The external locations bound to it, among the ones the user may see.
    let bound = Signal::derive(move || {
        locations.with(|locations| match locations {
            Load::Ready(list) => Load::Ready(
                list.iter()
                    .filter(|l| key.with_value(|name| l.credential_name.as_deref() == Some(name)))
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            Load::Loading => Load::Loading,
            Load::Failed(message) => Load::Failed(message.clone()),
        })
    });
    let bound_title = move || {
        format!(
            "External locations ({})",
            bound.with(|b| b.ready().map_or(0, Vec::len))
        )
    };
    let location_rows = move || {
        let empty = Empty::new(
            "No external locations.",
            "Make one on the External locations page and pick this credential.",
        );
        loaded_list(bound.get(), empty, |list| {
            view! {
                <div class="uc-rows">
                    {list
                        .into_iter()
                        .map(|l| {
                            view! {
                                <ObjectRow
                                    icon=Section::ExternalLocations.icon()
                                    name=l.name.clone()
                                    href=Section::ExternalLocations.item_href(&l.name)
                                    comment=Some(l.url)
                                />
                            }
                        })
                        .collect_view()}
                </div>
            }
            .into_any()
        })
    };

    view! {
        <EntityHeader
            icon=Section::Credentials.icon()
            name=name.clone()
            badges=vec!["Credential".to_string()]
            reload
            actions=view! {
                <EditCredential name=name.clone() role comment/>
                <DeleteButton open=delete_open when=manages/>
            }
                .into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || {
                        view! {
                            <DescriptionCard comment noun="credential" save=save_comment/>
                            <Card title=bound_title>{location_rows}</Card>
                        }
                    }
                    details=move || {
                        view! {
                            <DetailsTab rows=field(info, credential_rows) json=field(info, pretty_json)/>
                        }
                    }
                    permissions=move || {
                        view! {
                            <PermissionsCard kind=SecurableType::Credential full_name=key.get_value() owner/>
                        }
                    }
                />
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="credential"
            name=name.clone()
            delete=move |force| {
                let name = key.get_value();
                async move { api::client().credential(name).delete().with_force(force).await }
            }
            key=Key::Credential(name.clone())
            list=Key::Credentials
            then=Section::Credentials.href().to_string()
            holder=Holder::Credential(name.clone())
        />
    }
}

/// "Edit": a new name, role and comment.
#[component]
fn EditCredential(
    name: String,
    #[prop(into)] role: Signal<String>,
    #[prop(into)] comment: Signal<String>,
) -> impl IntoView {
    let key = StoredValue::new(name);
    let open = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let new_role = RwSignal::new(String::new());
    let new_comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let navigate = use_navigate();
    let edit = move |_| {
        new_name.set(key.get_value());
        new_role.set(role.get_untracked());
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
        let arn = new_role.get_untracked().trim().to_string();
        let text = new_comment.get_untracked();
        let comment_change = (text != comment.get_untracked()).then_some(text);
        // The API sets a role; it does not unset one.
        let role_change =
            (!arn.is_empty() && arn != role.get_untracked()).then_some(AwsIamRoleRequest {
                role_arn: arn,
                ..Default::default()
            });
        let name_change = (renamed != name).then_some(renamed);
        // Nothing to save: the dialog stays, as the console's.
        if comment_change.is_none() && role_change.is_none() && name_change.is_none() {
            return;
        }
        let update = api::client()
            .credential(&name)
            .update()
            .with_comment(comment_change)
            .with_aws_iam_role(role_change)
            .with_new_name(name_change.clone());
        let navigate = navigate.clone();
        busy.set(true);
        spawn_local(async move {
            match update.await {
                Ok(updated) => {
                    notify.success("Edit credential complete");
                    let page = name_change
                        .as_deref()
                        .map(|new_name| Section::Credentials.item_href(new_name));
                    let renamed =
                        name_change.map(|new_name| (Key::Credential(new_name), Key::Credentials));
                    cache.edited(Key::Credential(name), updated, renamed);
                    if busy.try_get_untracked().is_some() {
                        open.set(false);
                        if let Some(page) = page {
                            navigate(&page, Default::default());
                        }
                    }
                }
                Err(e) => notify.failed(&e, "Edit credential failed"),
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
            title="Edit credential"
            ok_text="Save"
            busy
            on_cancel=move || open.set(false)
            on_ok
        >
            <TextField id="uc-edit-name" label="Name" value=new_name required=true/>
            <TextField
                id="uc-edit-role"
                label="AWS IAM role ARN"
                value=new_role
                placeholder="arn:aws:iam::123456789012:role/unity-catalog"
            />
            <TextField id="uc-edit-comment" label="Comment" value=new_comment/>
        </Modal>
    }
}

fn credential_rows(c: &CredentialInfo) -> Vec<MetaRow> {
    let role = c.aws_iam_role.as_option();
    vec![
        MetaRow::new("Owner", or_dash(c.owner.clone())),
        MetaRow::new("Purpose", or_dash(c.purpose.map(enum_name))),
        MetaRow::mono(
            "AWS IAM role ARN",
            or_dash(role.map(|r| r.role_arn.clone())),
        ),
        MetaRow::mono(
            "Unity Catalog IAM ARN",
            or_dash(role.and_then(|r| r.unity_catalog_iam_arn.clone())),
        ),
        MetaRow::mono("External ID", or_dash(role.map(|r| r.external_id.clone()))),
        MetaRow::new("Created at", format_timestamp(c.created_at)),
        MetaRow::new("Created by", or_dash(c.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(c.updated_at)),
        MetaRow::new("Updated by", or_dash(c.updated_by.clone())),
        MetaRow::mono("Credential ID", or_dash(c.id.clone())),
        MetaRow::new("Comment", or_dash(c.comment.clone())),
    ]
}
