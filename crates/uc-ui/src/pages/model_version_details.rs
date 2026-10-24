//! Model version detail page. Mirrors `pages/ModelVersionDetails.tsx`.

use leptos::prelude::*;
use uc_client::models::models::v1::ModelVersionInfo;
use uc_common::three_part_full_name;

use super::model_details::model_rows;
use crate::api;
use crate::components::icons::DeploymentUnitOutlined;
use crate::components::{
    ActionDropdown, DeleteModal, Description, LoadError, Loaded, MenuIcon, MenuItem, MetadataList,
};
use crate::layout::{Breadcrumb, DetailsLayout};
use crate::load::{Key, Load, field, use_cache, use_load};
use crate::util::{enum_name, format_timestamp, route_params};

#[component]
pub fn ModelVersionDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "model", "version"]);
    move || {
        let [catalog, schema, model, version] = params.get();
        view! { <VersionPage catalog schema model version/> }
    }
}

#[component]
fn VersionPage(catalog: String, schema: String, model: String, version: String) -> impl IntoView {
    let title = view! {
        <h3 class="uc-title-3">
            <DeploymentUnitOutlined/>
            " "
            {format!("{model} version {version}")}
        </h3>
    };
    let breadcrumbs = vec![
        Breadcrumb::link("Catalogs", "/"),
        Breadcrumb::link(catalog.clone(), format!("/data/{catalog}")),
        Breadcrumb::link(schema.clone(), format!("/data/{catalog}/{schema}")),
        Breadcrumb::link(model.clone(), format!("/models/{catalog}/{schema}/{model}")),
        Breadcrumb::current(format!("version {version}")),
    ];
    let Ok(number) = version.parse::<i64>() else {
        return view! {
            <DetailsLayout title=title.into_any() breadcrumbs>
                <LoadError message=format!("\"{version}\" is not a version number.")/>
            </DetailsLayout>
        }
        .into_any();
    };

    let full_name = three_part_full_name(&catalog, &schema, &model);
    let info = use_load(Key::Version(full_name.clone(), number), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::client().get_model_version(full_name, number).await }
        }
    });
    // Upstream's `ModelSidebar`: the registered model's details.
    let model_info = use_load(Key::Model(full_name.clone()), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::client().registered_model(full_name).get().await }
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
                    .update_model_version(&full_name, number, comment)
                    .await?;
                cache.set(Key::Version(full_name, number), updated);
                Ok(())
            }
        }
    };

    let modals = view! {
        <DeleteModal
            open=delete_open
            noun="model version"
            question="Are you sure you want to delete model version"
            name=number.to_string()
            label=format!("Model version {number}")
            delete={
                let full_name = full_name.clone();
                move |_| {
                    let full_name = full_name.clone();
                    async move { api::client().delete_model_version(full_name, number).await }
                }
            }
            key=Key::Version(full_name.clone(), number)
            list=Key::Versions(full_name.clone())
            then=format!("/models/{catalog}/{schema}/{model}")
        />
    };

    view! {
        <DetailsLayout
            title=view! {
                <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                    {title}
                    <Show when=move || info.with(|i| i.ready().is_some())>
                        <ActionDropdown items=vec![
                            MenuItem {
                                label: "Delete Model Version",
                                icon: MenuIcon::Delete,
                                on_click: Callback::new(move |_| delete_open.set(true)),
                            },
                        ]/>
                    </Show>
                </div>
            }
                .into_any()
            breadcrumbs
        >
            <Loaded state=info>
                <div class="uc-flex-col uc-gap-xl">
                    <Description
                        comment=field(info, |v: &ModelVersionInfo| v.comment.clone().unwrap_or_default())
                        noun="model version"
                        hint="Model version description"
                        label=format!("Model version {number}")
                        save=save.clone()
                    />
                    <div>
                        <h5 class="uc-title-5 uc-mb-md">"Version details"</h5>
                        <table class="uc-table uc-table--bordered">
                            <tbody>
                                {move || {
                                    version_rows(info)
                                        .into_iter()
                                        .map(|(label, value)| {
                                            view! {
                                                <tr>
                                                    <th scope="row">{label}</th>
                                                    <td>{value}</td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </tbody>
                        </table>
                    </div>
                </div>
                <aside>
                    <Loaded state=model_info>
                        <MetadataList title="Model details" rows=field(model_info, model_rows)/>
                    </Loaded>
                </aside>
            </Loaded>
        </DetailsLayout>
        {modals}
    }
        .into_any()
}

/// Upstream's version details table: empty values stay, as there.
fn version_rows(info: RwSignal<Load<ModelVersionInfo>>) -> Vec<(&'static str, String)> {
    info.with(|i| {
        let Some(v) = i.ready() else {
            return Vec::new();
        };
        vec![
            ("Created at", format_timestamp(v.created_at)),
            ("Created by", v.created_by.clone().unwrap_or_default()),
            ("Updated at", format_timestamp(v.updated_at)),
            ("Updated by", v.updated_by.clone().unwrap_or_default()),
            ("Status", enum_name(v.status)),
            ("Training run", v.run_id.clone().unwrap_or_default()),
        ]
    })
}
