//! A volume's page, as the console's `VolumeDetails`: its description, its
//! details, its credentials and grants; "Edit" and "Delete".

use leptos::prelude::*;
use uc_api::{UpdateVolumeRequestContent, VolumeInfo};
use uc_common::{SecurableType, three_part_full_name, wire_name};

use crate::api;
use crate::auth::use_auth;
use crate::components::edit_modal::Changes;
use crate::components::icons::{HardDrive, Trash2};
use crate::components::{
    Card, CredentialsCard, DeleteModal, DescriptionCard, EditMetadata, EntityHeader, Loaded,
    MetaGrid, MetaRow, PermissionsCard, RawJsonCard, Tab, Tabs, Vendable,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

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
        async move { api::client().get_volume(&full_name).await }
    });
    let cache = use_cache();
    let auth = use_auth();
    let tab = RwSignal::new(Tab::Overview);
    let delete_open = RwSignal::new(false);
    let owner = field(info, |v: &VolumeInfo| v.owner.clone());
    let manages = Memo::new(move |_| {
        auth.session
            .with(|s| owner.with(|o| s.manages(o.as_deref())))
    });
    let comment = field(info, |v: &VolumeInfo| v.comment.clone().unwrap_or_default());
    let badges = field(info, |v: &VolumeInfo| {
        let mut badges: Vec<String> = v.volume_type.as_ref().map(wire_name).into_iter().collect();
        badges.push("VOLUME".into());
        badges
    });
    let volume_id = field(info, |v: &VolumeInfo| v.volume_id.clone());

    let save_comment = move |comment: String| {
        let (_, _, full_name) = names.get_value();
        async move {
            let request = UpdateVolumeRequestContent {
                comment: Some(comment),
                new_name: None,
            };
            let updated = api::client().update_volume(&full_name, &request).await?;
            cache.set(Key::Volume(full_name), updated);
            Ok(())
        }
    };
    let save_metadata = move |changes: Changes| {
        let (catalog, schema, full_name) = names.get_value();
        async move {
            let request = UpdateVolumeRequestContent {
                comment: changes.comment,
                new_name: changes.new_name.clone(),
            };
            let updated = api::client().update_volume(&full_name, &request).await?;
            match changes.new_name {
                Some(new_name) => {
                    cache.forget(Key::Volume(full_name));
                    cache.changed(Key::Entries(catalog.clone(), schema.clone(), Kind::Volumes));
                    cache.set(
                        Key::Volume(three_part_full_name(&catalog, &schema, &new_name)),
                        updated,
                    );
                    Ok(Some(Kind::Volumes.href(&catalog, &schema, &new_name)))
                }
                None => {
                    cache.set(Key::Volume(full_name), updated);
                    Ok(None)
                }
            }
        }
    };
    let refresh = Callback::new(move |_| {
        let (_, _, full_name) = names.get_value();
        cache.changed(Key::Volume(full_name));
    });

    let actions = view! {
        <EditMetadata name=volume.clone() comment noun="volume" save=save_metadata/>
        <Show when=move || manages.get()>
            <button
                type="button"
                class="uc-btn uc-btn--destructive uc-btn--sm"
                on:click=move |_| delete_open.set(true)
            >
                <Trash2/>
                "Delete"
            </button>
        </Show>
    }
    .into_any();

    let content = move || match tab.get() {
        Tab::Overview => {
            view! { <DescriptionCard comment noun="volume" save=save_comment/> }.into_any()
        }
        Tab::Details => view! {
            <Card>
                <MetaGrid rows=field(info, volume_rows)/>
            </Card>
            <RawJsonCard json=field(info, pretty_json)/>
        }
        .into_any(),
        Tab::Permissions => {
            let (_, _, full_name) = names.get_value();
            view! {
                <CredentialsCard kind=Vendable::Volume id=volume_id/>
                <PermissionsCard kind=SecurableType::Volume full_name owner/>
            }
            .into_any()
        }
    };

    view! {
        <EntityHeader
            icon=view! { <HardDrive class="uc-icon--volume"/> }.into_any()
            name=full_name.clone()
            badges
            on_refresh=refresh
            actions
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs tab/>
                {content}
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="volume"
            question="Are you sure you want to delete the volume"
            name=volume.clone()
            label=format!("{volume} volume")
            delete=move |_| {
                let (_, _, full_name) = names.get_value();
                async move { api::client().delete_volume(&full_name).await }
            }
            key=Key::Volume(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Volumes)
            then=Kind::Volumes.tab_href(&catalog, &schema)
        />
    }
}

fn volume_rows(v: &VolumeInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Name", or_dash(v.name.clone())),
        MetaRow::new("Owner", or_dash(v.owner.clone())),
        MetaRow::new("Type", or_dash(v.volume_type.as_ref().map(wire_name))),
        MetaRow::new("Created at", format_timestamp(v.created_at)),
        MetaRow::new("Created by", or_dash(v.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(v.updated_at)),
        MetaRow::new("Updated by", or_dash(v.updated_by.clone())),
        MetaRow::mono("Volume ID", or_dash(v.volume_id.clone())),
        MetaRow::mono("Storage location", or_dash(v.storage_location.clone())),
    ]
}
