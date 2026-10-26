//! A table's page, as the console's `TableDetails`: its description and
//! columns, its details, its credentials and grants; "Delete".

use leptos::prelude::*;
use uc_client::models::tables::v1::TableInfo;
use uc_common::{SecurableType, three_part_full_name};

use crate::api;
use crate::auth::use_manages;
use crate::components::{
    ColumnsCard, CredentialsCard, DeleteButton, DeleteModal, DescriptionCard, DetailsTab,
    EntityHeader, Loaded, MetaRow, PermissionsCard, ProtectButton, Tabs, Vendable, properties,
    use_protection,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{enum_name, format_timestamp, or_dash, pretty_json, route_params};

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
    let name = StoredValue::new(full_name.clone());
    let info = use_load(Key::Table(full_name.clone()), move || {
        let full_name = name.get_value();
        async move { api::client().table(full_name).get().await }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let owner = field(info, |t: &TableInfo| t.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |t: &TableInfo| t.comment.clone().unwrap_or_default());
    let badges = field(info, |t: &TableInfo| {
        [
            enum_name(t.table_type),
            t.data_source_format.map(enum_name).unwrap_or_default(),
        ]
        .into_iter()
        .filter(|badge| !badge.is_empty())
        .collect::<Vec<_>>()
    });
    let table_id = field(info, |t: &TableInfo| {
        Some(t.table_id.clone()).filter(|id| !id.is_empty())
    });

    let save_comment = move |comment: String| {
        let full_name = name.get_value();
        async move {
            let updated = api::client()
                .table(&full_name)
                .update()
                .with_comment(comment)
                .await?;
            cache.set(Key::Table(full_name), updated);
            Ok(())
        }
    };
    let protected = use_protection(SecurableType::Table, full_name.clone());
    let reload = vec![
        Key::Table(full_name.clone()),
        Key::Protection(SecurableType::Table, full_name.clone()),
    ];

    view! {
        <EntityHeader
            icon=Kind::Tables.icon()
            name=full_name.clone()
            badges
            reload
            protected
            actions=view! {
                <ProtectButton kind=SecurableType::Table full_name=full_name.clone() protected when=manages/>
                <DeleteButton open=delete_open when=manages protected/>
            }
                .into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || {
                        view! {
                            <DescriptionCard comment noun="table" save=save_comment/>
                            <ColumnsCard info full_name=name.get_value()/>
                        }
                    }
                    details=move || {
                        view! {
                            <DetailsTab
                                rows=field(info, table_rows)
                                properties=field(info, |t: &TableInfo| properties(&t.properties))
                                json=field(info, pretty_json)
                            />
                        }
                    }
                    permissions=move || {
                        view! {
                            <CredentialsCard kind=Vendable::Table id=table_id/>
                            <PermissionsCard kind=SecurableType::Table full_name=name.get_value() owner/>
                        }
                    }
                />
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="table"
            name=table.clone()
            delete=move |_| {
                let full_name = name.get_value();
                async move { api::client().table(full_name).delete().await }
            }
            key=Key::Table(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Tables)
            then=Kind::Tables.tab_href(&catalog, &schema)
        />
    }
}

fn table_rows(t: &TableInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Owner", or_dash(t.owner.clone())),
        MetaRow::new("Type", or_dash(enum_name(t.table_type))),
        MetaRow::new("Format", or_dash(t.data_source_format.map(enum_name))),
        MetaRow::new("Created at", format_timestamp(t.created_at)),
        MetaRow::new("Updated at", format_timestamp(t.updated_at)),
        MetaRow::mono("Table ID", or_dash(t.table_id.clone())),
        MetaRow::mono("Storage location", or_dash(t.storage_location.clone())),
        MetaRow::new("Comment", or_dash(t.comment.clone())),
    ]
}
