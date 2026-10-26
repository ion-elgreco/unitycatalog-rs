//! A volume's page, as the console's `VolumeDetails`: its description, its
//! files, its details, its credentials and grants; "Edit" and "Delete".

use leptos::prelude::*;
use uc_client::models::volumes::v1::VolumeInfo;
use uc_common::{SecurableType, three_part_full_name};

use crate::api;
use crate::auth::use_manages;
use crate::components::edit_modal::Changes;
use crate::components::{
    CredentialsCard, DeleteButton, DeleteModal, DescriptionCard, DetailsTab, EditMetadata,
    EntityHeader, FilesCard, Loaded, MetaRow, PermissionsCard, Tabs, Vendable,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{enum_name, format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn VolumeDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "volume"]);
    move || {
        let [catalog, schema, volume] = params.get();
        view! { <VolumePage catalog schema volume/> }
    }
}

#[component]
fn VolumePage(catalog: String, schema: String, volume: String) -> impl IntoView {
    let full_name = three_part_full_name(&catalog, &schema, &volume);
    let names = StoredValue::new((catalog.clone(), schema.clone(), full_name.clone()));
    let info = use_load(Key::Volume(full_name.clone()), move || {
        let (_, _, full_name) = names.get_value();
        async move { api::client().volume(full_name).get().await }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let owner = field(info, |v: &VolumeInfo| v.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |v: &VolumeInfo| v.comment.clone().unwrap_or_default());
    let badges = field(info, |v: &VolumeInfo| {
        let mut badges: Vec<String> = Some(enum_name(v.volume_type))
            .filter(|kind| !kind.is_empty())
            .into_iter()
            .collect();
        badges.push("VOLUME".into());
        badges
    });
    let volume_id = field(info, |v: &VolumeInfo| {
        Some(v.volume_id.clone()).filter(|id| !id.is_empty())
    });

    let save_metadata = move |changes: Changes| {
        let (catalog, schema, full_name) = names.get_value();
        async move {
            let updated = api::client()
                .volume(&full_name)
                .update()
                .with_comment(changes.comment)
                .with_new_name(changes.new_name.clone())
                .await?;
            let page = changes
                .new_name
                .as_deref()
                .map(|new_name| Kind::Volumes.href(&catalog, &schema, new_name));
            let renamed = changes.new_name.map(|new_name| {
                (
                    Key::Volume(three_part_full_name(&catalog, &schema, &new_name)),
                    Key::Entries(catalog.clone(), schema.clone(), Kind::Volumes),
                )
            });
            cache.edited(Key::Volume(full_name), updated, renamed);
            Ok(page)
        }
    };
    let save_comment = move |comment: String| {
        let saving = save_metadata(Changes {
            new_name: None,
            comment: Some(comment),
            properties: None,
        });
        async move { saving.await.map(|_| ()) }
    };
    let reload = vec![Key::Volume(full_name.clone())];
    let files_root = StoredValue::new(format!("/Volumes/{catalog}/{schema}/{volume}"));

    view! {
        <EntityHeader
            icon=Kind::Volumes.icon()
            name=full_name.clone()
            badges
            reload
            actions=view! {
                <EditMetadata name=volume.clone() comment save=save_metadata/>
                <DeleteButton open=delete_open when=manages/>
            }
                .into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || view! { <DescriptionCard comment noun="volume" save=save_comment/> }
                    files=move || view! { <FilesCard root=files_root.get_value() writable=manages/> }
                    details=move || {
                        view! { <DetailsTab rows=field(info, volume_rows) json=field(info, pretty_json)/> }
                    }
                    permissions=move || {
                        let (_, _, full_name) = names.get_value();
                        view! {
                            <CredentialsCard kind=Vendable::Volume id=volume_id/>
                            <PermissionsCard kind=SecurableType::Volume full_name owner/>
                        }
                    }
                />
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="volume"
            name=volume.clone()
            delete=move |_| {
                let (_, _, full_name) = names.get_value();
                async move { api::client().volume(full_name).delete().await }
            }
            key=Key::Volume(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Volumes)
            then=Kind::Volumes.tab_href(&catalog, &schema)
        />
    }
}

fn volume_rows(v: &VolumeInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Owner", or_dash(v.owner.clone())),
        MetaRow::new("Type", or_dash(enum_name(v.volume_type))),
        MetaRow::new("Created at", format_timestamp(v.created_at)),
        MetaRow::new("Updated at", format_timestamp(v.updated_at)),
        MetaRow::mono("Volume ID", or_dash(v.volume_id.clone())),
        MetaRow::mono("Storage location", or_dash(v.storage_location.clone())),
        MetaRow::new("Comment", or_dash(v.comment.clone())),
    ]
}
