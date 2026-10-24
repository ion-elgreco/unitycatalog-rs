//! A table's page, as upstream's `pages/TableDetails.tsx`: its description,
//! its columns, its details and the action menu.

use leptos::prelude::*;
use uc_client::models::tables::v1::{ColumnInfo, TableInfo};
use uc_common::three_part_full_name;

use crate::api;
use crate::components::icons::TableOutlined;
use crate::components::{
    ActionDropdown, DeleteModal, Loaded, MenuIcon, MenuItem, MetadataList, MetadataRow,
};
use crate::kind::Kind;
use crate::layout::{Breadcrumb, Column, DetailsLayout, ListLayout};
use crate::load::{Key, Load, field, use_load};
use crate::util::{enum_name, format_timestamp, route_params};

#[component]
pub fn TableDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "table"]);
    move || {
        let [catalog, schema, table] = params.get();
        view! { <TablePage catalog schema table/> }
    }
}

#[component]
fn TablePage(catalog: String, schema: String, table: String) -> impl IntoView {
    let full_name = three_part_full_name(&catalog, &schema, &table);
    let info = use_load(Key::Table(full_name.clone()), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::client().table(full_name).get().await }
        }
    });
    let delete_open = RwSignal::new(false);

    let modals = view! {
        <DeleteModal
            open=delete_open
            noun="table"
            question="Are you sure you want to delete the table"
            name=full_name.clone()
            label=format!("{table} table")
            delete={
                let full_name = full_name.clone();
                move |_| {
                    let full_name = full_name.clone();
                    async move { api::client().table(full_name).delete().await }
                }
            }
            key=Key::Table(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Tables)
            then=Kind::Tables.tab_href(&catalog, &schema)
        />
    };

    view! {
        <DetailsLayout
            title=view! {
                <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                    <h3 class="uc-title-3">
                        <TableOutlined/>
                        " "
                        {full_name.clone()}
                    </h3>
                    <Show when=move || info.with(|i| i.ready().is_some())>
                        <ActionDropdown items=vec![
                            MenuItem {
                                label: "Delete Table",
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
                Breadcrumb::current(table.clone()),
            ]
        >
            <Loaded state=info>
                <div class="uc-flex-col uc-gap-md">
                    <div>
                        <h5 class="uc-title-5">"Description"</h5>
                        <div class="uc-text-secondary uc-pre-wrap">
                            {field(info, |t: &TableInfo| t.comment.clone().unwrap_or_default())}
                        </div>
                    </div>
                    <ListLayout
                        title=view! { <h4 class="uc-title-4">"Columns"</h4> }.into_any()
                        data=table_columns(info)
                        columns=column_columns()
                    />
                </div>
                <aside>
                    <MetadataList title="Table details" rows=field(info, table_rows)/>
                </aside>
            </Loaded>
        </DetailsLayout>
        {modals}
    }
}

/// The columns of the loaded table.
fn table_columns(info: RwSignal<Load<TableInfo>>) -> Signal<Load<Vec<ColumnInfo>>> {
    Signal::derive(move || {
        info.with(|i| match i {
            Load::Ready(table) => Load::Ready(table.columns.clone()),
            Load::Loading => Load::Loading,
            Load::Failed(message) => Load::Failed(message.clone()),
        })
    })
}

fn column_columns() -> Vec<Column<ColumnInfo>> {
    vec![
        Column {
            title: "Name",
            width: "60%",
            center: false,
            render: Box::new(|c: &ColumnInfo| view! { <span>{c.name.clone()}</span> }.into_any()),
            search_key: Some(Box::new(|c: &ColumnInfo| c.name.clone())),
        },
        Column {
            title: "Type",
            width: "40%",
            center: false,
            render: Box::new(|c: &ColumnInfo| {
                let type_name = c.type_name.map(enum_name).unwrap_or_default();
                view! { <span>{type_name}</span> }.into_any()
            }),
            search_key: None,
        },
    ]
}

fn table_rows(t: &TableInfo) -> Vec<MetadataRow> {
    vec![
        MetadataRow {
            label: "Created at",
            value: format_timestamp(t.created_at),
            code: false,
        },
        MetadataRow {
            label: "Updated at",
            value: format_timestamp(t.updated_at),
            code: false,
        },
        MetadataRow {
            label: "Data source format",
            value: t.data_source_format.map(enum_name).unwrap_or_default(),
            code: true,
        },
    ]
}
