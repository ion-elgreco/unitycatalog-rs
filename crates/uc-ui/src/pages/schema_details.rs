//! A schema's page, as upstream's `pages/SchemaDetails.tsx`: its
//! description, its tables, volumes, functions or models in tabs, its
//! details and the action menu. The tabs sit beside the list's search box,
//! as upstream's radio buttons. The tab is the URL's `?tab=`, so that the
//! sidebar's "Tables" … "Models" links open it.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};
use uc_api::{CreateRegisteredModel, SchemaInfo, UpdateSchema};
use uc_common::schema_full_name;

use crate::api;
use crate::components::icons::DatabaseOutlined;
use crate::components::{
    ActionDropdown, CreateModal, DeleteModal, Description, Loaded, MenuIcon, MenuItem,
    MetadataList, MetadataRow,
};
use crate::contents::Holder;
use crate::kind::{Entry, Kind};
use crate::layout::{Breadcrumb, Column, DetailsLayout, ListLayout};
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, route_params};

#[component]
pub fn SchemaDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema"]);
    move || {
        let [catalog, schema] = params.get();
        view! { <SchemaPage catalog schema/> }
    }
}

#[component]
fn SchemaPage(catalog: String, schema: String) -> impl IntoView {
    let full_name = schema_full_name(&catalog, &schema);
    let info = use_load(Key::Schema(full_name.clone()), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::client().get_schema(&full_name).await }
        }
    });
    let cache = use_cache();
    let query = use_query_map();
    let tab = Memo::new(move |_| {
        query
            .with(|query| query.get("tab"))
            .and_then(|tab| Kind::from_tab(&tab))
            .unwrap_or(Kind::Tables)
    });
    let delete_open = RwSignal::new(false);
    let create_model_open = RwSignal::new(false);
    let save = {
        let full_name = full_name.clone();
        move |comment| {
            let full_name = full_name.clone();
            async move {
                let request = UpdateSchema {
                    comment: Some(comment),
                    properties: None,
                    new_name: None,
                };
                let updated = api::client().update_schema(&full_name, &request).await?;
                cache.set(Key::Schema(full_name), updated);
                Ok(())
            }
        }
    };

    let modals = view! {
        <DeleteModal
            open=delete_open
            noun="schema"
            question="Are you sure you want to delete the schema"
            name=schema.clone()
            label=format!("{schema} schema")
            delete={
                let full_name = full_name.clone();
                move |force| {
                    let full_name = full_name.clone();
                    async move { api::client().delete_schema(&full_name, force).await }
                }
            }
            key=Key::Schema(full_name.clone())
            list=Key::Schemas(catalog.clone())
            then=format!("/data/{catalog}")
            holder=Holder::Schema(catalog.clone(), schema.clone())
        />
        <CreateModal
            open=create_model_open
            noun="registered model"
            create={
                let catalog = catalog.clone();
                let schema = schema.clone();
                move |name, comment| {
                    let request = CreateRegisteredModel {
                        name,
                        catalog_name: catalog.clone(),
                        schema_name: schema.clone(),
                        comment,
                    };
                    let page = Kind::Models.tab_href(&catalog, &schema);
                    async move {
                        api::client().create_registered_model(&request).await?;
                        Ok(page)
                    }
                }
            }
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Models)
        />
    };

    view! {
        <DetailsLayout
            title=view! {
                <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                    <h3 class="uc-title-3">
                        <DatabaseOutlined/>
                        " "
                        {full_name.clone()}
                    </h3>
                    <Show when=move || info.with(|i| i.ready().is_some())>
                        <ActionDropdown items=vec![
                            MenuItem {
                                label: "Create Registered Model",
                                icon: MenuIcon::Model,
                                on_click: Callback::new(move |_| create_model_open.set(true)),
                            },
                            MenuItem {
                                label: "Delete Schema",
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
                Breadcrumb::current(schema.clone()),
            ]
        >
            <Loaded state=info>
                <div class="uc-flex-col uc-gap-md">
                    <Description
                        comment=field(info, |s: &SchemaInfo| s.comment.clone().unwrap_or_default())
                        noun="schema"
                        hint="Schema description"
                        label=format!("{schema} schema")
                        save=save.clone()
                    />
                    {
                        let catalog = catalog.clone();
                        let schema = schema.clone();
                        move || {
                            view! { <KindTab kind=tab.get() catalog=catalog.clone() schema=schema.clone()/> }
                        }
                    }
                </div>
                <aside>
                    <MetadataList title="Schema details" rows=field(info, schema_rows)/>
                </aside>
            </Loaded>
        </DetailsLayout>
        {modals}
    }
}

/// The tabs, as upstream's radio buttons. A click sets the URL's `?tab=`
/// in place, without a new history entry.
#[component]
fn TabPicker(tab: Kind, catalog: String, schema: String) -> impl IntoView {
    let navigate = use_navigate();
    view! {
        <div class="uc-radio-group" role="radiogroup" aria-label="Objects">
            {Kind::ALL
                .into_iter()
                .map(|kind| {
                    let href = kind.tab_href(&catalog, &schema);
                    let navigate = navigate.clone();
                    let in_place = NavigateOptions {
                        replace: true,
                        scroll: false,
                        ..Default::default()
                    };
                    view! {
                        <button
                            type="button"
                            role="radio"
                            aria-checked=(tab == kind).to_string()
                            class:is-checked={tab == kind}
                            on:click=move |_| navigate(&href, in_place.clone())
                        >
                            {kind.label()}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// The tables, volumes, functions or models of the schema, with the tabs
/// beside the search box.
#[component]
fn KindTab(kind: Kind, catalog: String, schema: String) -> impl IntoView {
    let entries = use_load(Key::Entries(catalog.clone(), schema.clone(), kind), {
        let catalog = catalog.clone();
        let schema = schema.clone();
        move || {
            let catalog = catalog.clone();
            let schema = schema.clone();
            async move { kind.entries(&catalog, &schema).await }
        }
    });
    let navigate = use_navigate();
    let filters = view! { <TabPicker tab=kind catalog=catalog.clone() schema=schema.clone()/> };
    let on_row_click = Callback::new(move |entry: Entry| {
        navigate(
            &kind.href(&catalog, &schema, &entry.name),
            Default::default(),
        );
    });
    let columns = vec![
        Column {
            title: "Name",
            width: "60%",
            center: false,
            render: Box::new(move |e: &Entry| {
                view! {
                    <span class="uc-name">
                        {kind.icon()}
                        " "
                        {e.name.clone()}
                    </span>
                }
                .into_any()
            }),
            search_key: Some(Box::new(|e: &Entry| e.name.clone())),
        },
        Column {
            title: "Created At",
            width: "40%",
            center: false,
            render: Box::new(|e: &Entry| {
                view! { <span>{format_timestamp(e.created_at)}</span> }.into_any()
            }),
            search_key: None,
        },
    ];
    view! {
        <ListLayout
            title=view! { <h4 class="uc-title-4">{kind.label()}</h4> }.into_any()
            data=entries
            columns
            on_row_click
            filters=filters.into_any()
        />
    }
}

fn schema_rows(s: &SchemaInfo) -> Vec<MetadataRow> {
    vec![
        MetadataRow {
            label: "Created at",
            value: format_timestamp(s.created_at),
            code: false,
        },
        MetadataRow {
            label: "Updated at",
            value: format_timestamp(s.updated_at),
            code: false,
        },
    ]
}
