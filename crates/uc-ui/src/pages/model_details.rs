//! A registered model's page, as upstream's `pages/ModelDetails.tsx`: its
//! description, its versions, its details and the action menu.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use uc_api::{ModelVersionInfo, ModelVersionStatus, RegisteredModelInfo, UpdateRegisteredModel};
use uc_common::three_part_full_name;

use crate::api;
use crate::components::icons::{
    CheckCircleOutlined, CloseCircleOutlined, DeploymentUnitOutlined, MinusCircleOutlined,
};
use crate::components::{
    ActionDropdown, DeleteModal, Description, Loaded, MenuIcon, MenuItem, MetadataList,
    MetadataRow, Tooltip,
};
use crate::contents::Holder;
use crate::kind::Kind;
use crate::layout::{Breadcrumb, Column, DetailsLayout, ListLayout};
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, route_params};

#[component]
pub fn ModelDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "model"]);
    move || {
        let [catalog, schema, model] = params.get();
        view! { <ModelPage catalog schema model/> }
    }
}

#[component]
fn ModelPage(catalog: String, schema: String, model: String) -> impl IntoView {
    let full_name = three_part_full_name(&catalog, &schema, &model);
    let info = use_load(Key::Model(full_name.clone()), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::client().get_registered_model(&full_name).await }
        }
    });
    let versions = use_load(Key::Versions(full_name.clone()), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::model_versions(&full_name).await }
        }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let navigate = use_navigate();
    let on_row_click = Callback::new({
        let page = format!("/models/{catalog}/{schema}/{model}");
        move |version: ModelVersionInfo| {
            let number = version.version.unwrap_or_default();
            navigate(&format!("{page}/versions/{number}"), Default::default());
        }
    });
    let save = {
        let full_name = full_name.clone();
        move |comment| {
            let full_name = full_name.clone();
            async move {
                let request = UpdateRegisteredModel {
                    comment: Some(comment),
                    new_name: None,
                };
                let updated = api::client()
                    .update_registered_model(&full_name, &request)
                    .await?;
                cache.set(Key::Model(full_name), updated);
                Ok(())
            }
        }
    };

    let modals = view! {
        <DeleteModal
            open=delete_open
            noun="model"
            question="Are you sure you want to delete model"
            name=model.clone()
            label=format!("{model} model")
            delete={
                let full_name = full_name.clone();
                move |force| {
                    let full_name = full_name.clone();
                    async move { api::client().delete_registered_model(&full_name, force).await }
                }
            }
            key=Key::Model(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Models)
            then=Kind::Models.tab_href(&catalog, &schema)
            holder=Holder::Model(full_name.clone())
            // Upstream's `hasExistingVersions`.
            blocked=Signal::derive(move || {
                versions
                    .with(|v| v.ready().is_some_and(|v| !v.is_empty()))
                    .then_some(
                        "Registered models cannot be deleted with existing model versions. \
                         Delete model versions and try again.",
                    )
            })
        />
    };

    view! {
        <DetailsLayout
            title=view! {
                <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                    <h3 class="uc-title-3">
                        <DeploymentUnitOutlined/>
                        " "
                        {model.clone()}
                    </h3>
                    <Show when=move || info.with(|i| i.ready().is_some())>
                        <ActionDropdown items=vec![
                            MenuItem {
                                label: "Delete Model",
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
                Breadcrumb::current(model.clone()),
            ]
        >
            <Loaded state=info>
                <div class="uc-flex-col uc-gap-xl">
                    <Description
                        comment=field(info, |m: &RegisteredModelInfo| m.comment.clone().unwrap_or_default())
                        noun="model"
                        hint="Model description"
                        label=format!("{model} model")
                        save=save.clone()
                    />
                    <ListLayout
                        title=view! { <h5 class="uc-title-5">"Versions"</h5> }.into_any()
                        data=versions
                        columns=version_columns()
                        show_search=false
                        on_row_click
                    />
                </div>
                <aside>
                    <MetadataList title="Model details" rows=field(info, model_rows)/>
                </aside>
            </Loaded>
        </DetailsLayout>
        {modals}
    }
}

fn version_columns() -> Vec<Column<ModelVersionInfo>> {
    vec![
        Column {
            title: "Status",
            width: "10%",
            center: true,
            render: Box::new(|v: &ModelVersionInfo| status(v.status.as_ref())),
            search_key: None,
        },
        Column {
            title: "Name",
            width: "60%",
            center: false,
            render: Box::new(|v: &ModelVersionInfo| {
                let label = v
                    .version
                    .map(|n| format!("Version {n}"))
                    .unwrap_or_default();
                view! { <span>{label}</span> }.into_any()
            }),
            search_key: None,
        },
        Column {
            title: "Time registered",
            width: "40%",
            center: false,
            render: Box::new(|v: &ModelVersionInfo| {
                view! { <span>{format_timestamp(v.created_at)}</span> }.into_any()
            }),
            search_key: None,
        },
    ]
}

/// Upstream's `ModelVersionStatusDisplay`: an icon in green, gray or red,
/// with the status in a tooltip.
fn status(status: Option<&ModelVersionStatus>) -> AnyView {
    match status {
        Some(ModelVersionStatus::Ready) => view! {
            <Tooltip text="READY">
                <span class="uc-status-icon uc-status-icon--ready">
                    <CheckCircleOutlined/>
                </span>
            </Tooltip>
        }
        .into_any(),
        Some(ModelVersionStatus::PendingRegistration) => view! {
            <Tooltip text="PENDING REGISTRATION">
                <span class="uc-status-icon uc-status-icon--pending">
                    <MinusCircleOutlined/>
                </span>
            </Tooltip>
        }
        .into_any(),
        Some(ModelVersionStatus::FailedRegistration) => view! {
            <Tooltip text="FAILED REGISTRATION">
                <span class="uc-status-icon uc-status-icon--failed">
                    <CloseCircleOutlined/>
                </span>
            </Tooltip>
        }
        .into_any(),
        _ => view! { <span class="uc-text-secondary">"UNKNOWN"</span> }.into_any(),
    }
}

/// Upstream's `MODEL_METADATA`, which the model version page shows too.
pub fn model_rows(m: &RegisteredModelInfo) -> Vec<MetadataRow> {
    vec![
        MetadataRow {
            label: "Created at",
            value: format_timestamp(m.created_at),
            code: false,
        },
        MetadataRow {
            label: "Created by",
            value: m.created_by.clone().unwrap_or_default(),
            code: false,
        },
        MetadataRow {
            label: "Updated at",
            value: format_timestamp(m.updated_at),
            code: false,
        },
        MetadataRow {
            label: "Updated by",
            value: m.updated_by.clone().unwrap_or_default(),
            code: false,
        },
    ]
}
