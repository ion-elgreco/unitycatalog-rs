//! A catalog's page, as upstream's `pages/CatalogDetails.tsx`: its
//! description, its schemas and its details, the action menu, and the
//! "Create Schema" button.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use uc_api::{CatalogInfo, CreateSchema, SchemaInfo, UpdateCatalog};

use crate::api;
use crate::components::icons::{DatabaseOutlined, ProductOutlined};
use crate::components::{
    ActionDropdown, CreateModal, DeleteModal, Description, Loaded, MenuIcon, MenuItem,
    MetadataList, MetadataRow,
};
use crate::contents::Holder;
use crate::layout::{Breadcrumb, Column, DetailsLayout, ListLayout};
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, route_params};

#[component]
pub fn CatalogDetails() -> impl IntoView {
    let params = route_params(["catalog"]);
    move || {
        let [catalog] = params.get();
        view! { <CatalogPage catalog/> }
    }
}

#[component]
fn CatalogPage(catalog: String) -> impl IntoView {
    let info = use_load(Key::Catalog(catalog.clone()), {
        let catalog = catalog.clone();
        move || {
            let catalog = catalog.clone();
            async move { api::client().get_catalog(&catalog).await }
        }
    });
    let schemas = use_load(Key::Schemas(catalog.clone()), {
        let catalog = catalog.clone();
        move || {
            let catalog = catalog.clone();
            async move { api::schemas(&catalog).await }
        }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let create_open = RwSignal::new(false);
    let navigate = use_navigate();
    let on_row_click = Callback::new({
        let catalog = catalog.clone();
        move |schema: SchemaInfo| {
            let name = schema.name.unwrap_or_default();
            navigate(&format!("/data/{catalog}/{name}"), Default::default());
        }
    });
    let save = {
        let catalog = catalog.clone();
        move |comment| {
            let catalog = catalog.clone();
            async move {
                let request = UpdateCatalog {
                    comment: Some(comment),
                    properties: None,
                    new_name: None,
                };
                let updated = api::client().update_catalog(&catalog, &request).await?;
                cache.set(Key::Catalog(catalog), updated);
                Ok(())
            }
        }
    };

    let modals = view! {
        <DeleteModal
            open=delete_open
            noun="catalog"
            question="Are you sure you want to delete the catalog"
            name=catalog.clone()
            label=format!("{catalog} catalog")
            delete={
                let catalog = catalog.clone();
                move |force| {
                    let catalog = catalog.clone();
                    async move { api::client().delete_catalog(&catalog, force).await }
                }
            }
            key=Key::Catalog(catalog.clone())
            list=Key::Catalogs
            then="/".to_string()
            holder=Holder::Catalog(catalog.clone())
        />
        <CreateModal
            open=create_open
            noun="schema"
            hint="Create schema description"
            create={
                let catalog = catalog.clone();
                move |name, comment| {
                    let catalog = catalog.clone();
                    async move {
                        let request = CreateSchema {
                            name,
                            catalog_name: catalog.clone(),
                            comment,
                            properties: None,
                            storage_root: None,
                        };
                        let schema = api::client().create_schema(&request).await?;
                        Ok(format!("/data/{catalog}/{}", schema.name.unwrap_or_default()))
                    }
                }
            }
            list=Key::Schemas(catalog.clone())
        />
    };

    view! {
        <DetailsLayout
            title=view! {
                <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                    <h3 class="uc-title-3">
                        <ProductOutlined/>
                        " "
                        {catalog.clone()}
                    </h3>
                    <Show when=move || info.with(|i| i.ready().is_some())>
                        <div class="uc-flex uc-gap-md">
                            <ActionDropdown items=vec![
                                MenuItem {
                                    label: "Delete Catalog",
                                    icon: MenuIcon::Delete,
                                    on_click: Callback::new(move |_| delete_open.set(true)),
                                },
                            ]/>
                            <button
                                class="uc-btn uc-btn--primary"
                                on:click=move |_| create_open.set(true)
                            >
                                "Create Schema"
                            </button>
                        </div>
                    </Show>
                </div>
            }
                .into_any()
            breadcrumbs=vec![Breadcrumb::link("Catalogs", "/"), Breadcrumb::current(catalog.clone())]
        >
            <Loaded state=info>
                <div class="uc-flex-col uc-gap-md">
                    <Description
                        comment=field(info, |c: &CatalogInfo| c.comment.clone().unwrap_or_default())
                        noun="catalog"
                        hint="Catalog description"
                        label=format!("{catalog} catalog")
                        save=save.clone()
                    />
                    <ListLayout
                        title=view! { <h4 class="uc-title-4">"Schemas"</h4> }.into_any()
                        data=schemas
                        columns=schema_columns()
                        on_row_click
                    />
                </div>
                <aside>
                    <MetadataList title="Catalog details" rows=field(info, catalog_rows)/>
                </aside>
            </Loaded>
        </DetailsLayout>
        {modals}
    }
}

fn schema_columns() -> Vec<Column<SchemaInfo>> {
    vec![
        Column {
            title: "Name",
            width: "60%",
            center: false,
            render: Box::new(|s: &SchemaInfo| {
                view! {
                    <span class="uc-name">
                        <DatabaseOutlined/>
                        " "
                        {s.name.clone().unwrap_or_default()}
                    </span>
                }
                .into_any()
            }),
            search_key: Some(Box::new(|s: &SchemaInfo| {
                s.name.clone().unwrap_or_default()
            })),
        },
        Column {
            title: "Created At",
            width: "40%",
            center: false,
            render: Box::new(|s: &SchemaInfo| {
                view! { <span>{format_timestamp(s.created_at)}</span> }.into_any()
            }),
            search_key: None,
        },
    ]
}

fn catalog_rows(c: &CatalogInfo) -> Vec<MetadataRow> {
    vec![
        MetadataRow {
            label: "Created at",
            value: format_timestamp(c.created_at),
            code: false,
        },
        MetadataRow {
            label: "Updated at",
            value: format_timestamp(c.updated_at),
            code: false,
        },
    ]
}
