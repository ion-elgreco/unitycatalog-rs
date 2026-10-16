//! Function detail page. Mirrors `pages/FunctionDetails.tsx`.

use leptos::prelude::*;
use uc_api::FunctionInfo;
use uc_common::three_part_full_name;

use crate::api;
use crate::components::icons::FunctionOutlined;
use crate::components::{
    ActionDropdown, CodeBox, DeleteModal, Loaded, MenuIcon, MenuItem, MetadataList, MetadataRow,
};
use crate::kind::Kind;
use crate::layout::{Breadcrumb, DetailsLayout};
use crate::load::{Key, field, use_load};
use crate::util::{format_timestamp, route_params};

#[component]
pub fn FunctionDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "function"]);
    move || {
        let [catalog, schema, function] = params.get();
        view! { <FunctionPage catalog schema function/> }
    }
}

#[component]
fn FunctionPage(catalog: String, schema: String, function: String) -> impl IntoView {
    let full_name = three_part_full_name(&catalog, &schema, &function);
    let info = use_load(Key::Function(full_name.clone()), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::client().get_function(&full_name).await }
        }
    });
    let delete_open = RwSignal::new(false);

    let modals = view! {
        <DeleteModal
            open=delete_open
            noun="function"
            question="Are you sure you want to delete the function"
            name=function.clone()
            label=format!("{function} function")
            delete={
                let full_name = full_name.clone();
                move |_| {
                    let full_name = full_name.clone();
                    async move { api::client().delete_function(&full_name).await }
                }
            }
            key=Key::Function(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Functions)
            then=Kind::Functions.tab_href(&catalog, &schema)
        />
    };

    view! {
        <DetailsLayout
            title=view! {
                <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                    <h3 class="uc-title-3">
                        <FunctionOutlined/>
                        " "
                        {function.clone()}
                    </h3>
                    <Show when=move || info.with(|i| i.ready().is_some())>
                        <ActionDropdown items=vec![
                            MenuItem {
                                label: "Delete Function",
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
                Breadcrumb::current(function.clone()),
            ]
        >
            <Loaded state=info>
                <div class="uc-flex-col uc-gap-md">
                    <div>
                        <h5 class="uc-title-5">"Description"</h5>
                        <div class="uc-text-secondary uc-pre-wrap">
                            {field(info, |f: &FunctionInfo| f.comment.clone().unwrap_or_default())}
                        </div>
                    </div>
                    <CodeBox definition=field(info, |f: &FunctionInfo| {
                        f.routine_definition.clone().unwrap_or_default()
                    })/>
                </div>
                <aside>
                    <MetadataList title="Function details" rows=field(info, function_rows)/>
                </aside>
            </Loaded>
        </DetailsLayout>
        {modals}
    }
}

fn function_rows(f: &FunctionInfo) -> Vec<MetadataRow> {
    vec![
        MetadataRow {
            label: "Created at",
            value: format_timestamp(f.created_at),
            code: false,
        },
        MetadataRow {
            label: "Updated at",
            value: format_timestamp(f.updated_at),
            code: false,
        },
        MetadataRow {
            label: "External language",
            value: f.external_language.clone().unwrap_or_default(),
            code: true,
        },
    ]
}
