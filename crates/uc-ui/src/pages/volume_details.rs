//! Volume detail page. Mirrors `pages/VolumeDetails.tsx`.

use leptos::prelude::*;
use uc_client::models::volumes::v1::VolumeInfo;
use uc_common::three_part_full_name;

use crate::api;
use crate::components::icons::FolderOutlined;
use crate::components::{
    ActionDropdown, DeleteModal, Description, Loaded, MenuIcon, MenuItem, MetadataList, MetadataRow,
};
use crate::kind::Kind;
use crate::layout::{Breadcrumb, DetailsLayout};
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{enum_name, format_timestamp, route_params};

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
    let info = use_load(Key::Volume(full_name.clone()), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::client().volume(full_name).get().await }
        }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let save = {
        let full_name = full_name.clone();
        move |comment| {
            let full_name = full_name.clone();
            async move {
                let updated = api::client()
                    .volume(&full_name)
                    .update()
                    .with_comment(comment)
                    .await?;
                cache.set(Key::Volume(full_name), updated);
                Ok(())
            }
        }
    };

    let modals = view! {
        <DeleteModal
            open=delete_open
            noun="volume"
            question="Are you sure you want to delete the volume"
            name=volume.clone()
            label=format!("{volume} volume")
            delete={
                let full_name = full_name.clone();
                move |_| {
                    let full_name = full_name.clone();
                    async move { api::client().volume(full_name).delete().await }
                }
            }
            key=Key::Volume(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Volumes)
            then=Kind::Volumes.tab_href(&catalog, &schema)
        />
    };

    view! {
        <DetailsLayout
            title=view! {
                <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                    <h3 class="uc-title-3">
                        <FolderOutlined/>
                        " "
                        {full_name.clone()}
                    </h3>
                    <Show when=move || info.with(|i| i.ready().is_some())>
                        <ActionDropdown items=vec![
                            MenuItem {
                                label: "Delete Volume",
                                icon: MenuIcon::Delete,
                                on_click: Callback::new(move |_| delete_open.set(true)),
                            },
                        ]/>
                    </Show>
                </div>
            }
                .into_any()
            breadcrumbs=vec![
                Breadcrumb::link("Catalogs", "/"),
                Breadcrumb::link(catalog.clone(), format!("/data/{catalog}")),
                Breadcrumb::link(schema.clone(), format!("/data/{catalog}/{schema}")),
                Breadcrumb::current(volume.clone()),
            ]
        >
            <Loaded state=info>
                <div>
                    <Description
                        comment=field(info, |v: &VolumeInfo| v.comment.clone().unwrap_or_default())
                        noun="volume"
                        hint="Volume description"
                        label=format!("{volume} volume")
                        save=save.clone()
                    />
                </div>
                <aside>
                    <MetadataList title="Volume details" rows=field(info, volume_rows)/>
                </aside>
            </Loaded>
        </DetailsLayout>
        {modals}
    }
}

fn volume_rows(v: &VolumeInfo) -> Vec<MetadataRow> {
    vec![
        MetadataRow {
            label: "Created at",
            value: format_timestamp(v.created_at),
            code: false,
        },
        MetadataRow {
            label: "Updated at",
            value: format_timestamp(v.updated_at),
            code: false,
        },
        MetadataRow {
            label: "Volume type",
            value: enum_name(v.volume_type),
            code: true,
        },
    ]
}
