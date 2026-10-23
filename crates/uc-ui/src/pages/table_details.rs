//! A table's page, as the console's `TableDetails`: its description and
//! columns, its details, its credentials and grants; "Delete".

use leptos::prelude::*;
use uc_api::{TableInfo, UpdateTable};
use uc_common::{SecurableType, three_part_full_name, wire_name};

use crate::api;
use crate::auth::use_manages;
use crate::components::{
    ColumnsCard, CredentialsCard, DeleteButton, DeleteModal, DescriptionCard, DetailsTab,
    EntityHeader, Loaded, MetaRow, PermissionsCard, Tabs, Vendable, properties,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

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
        async move { api::client().get_table(&full_name, false, false).await }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let owner = field(info, |t: &TableInfo| t.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |t: &TableInfo| t.comment.clone().unwrap_or_default());
    let badges = field(info, |t: &TableInfo| {
        [
            t.table_type.as_ref().map(wire_name),
            t.data_source_format.as_ref().map(wire_name),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    });
    let table_id = field(info, |t: &TableInfo| t.table_id.clone());

    let save_comment = move |comment: String| {
        let full_name = name.get_value();
        async move {
            let request = UpdateTable {
                comment: Some(comment),
                columns: Vec::new(),
            };
            let updated = api::client().update_table(&full_name, &request).await?;
            cache.set(Key::Table(full_name), updated);
            Ok(())
        }
    };
    let refresh = Callback::new(move |_| cache.changed(Key::Table(name.get_value())));

    view! {
        <EntityHeader
            icon=Kind::Tables.icon()
            name=full_name.clone()
            badges
            on_refresh=refresh
            actions=view! { <DeleteButton open=delete_open when=manages/> }.into_any()
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
                                properties=field(info, |t: &TableInfo| properties(t.properties.as_ref()))
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
                async move { api::client().delete_table(&full_name).await }
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
        MetaRow::new("Type", or_dash(t.table_type.as_ref().map(wire_name))),
        MetaRow::new(
            "Format",
            or_dash(t.data_source_format.as_ref().map(wire_name)),
        ),
        MetaRow::new("Created at", format_timestamp(t.created_at)),
        MetaRow::new("Updated at", format_timestamp(t.updated_at)),
        MetaRow::mono("Table ID", or_dash(t.table_id.clone())),
        MetaRow::mono("Storage location", or_dash(t.storage_location.clone())),
        MetaRow::new("Comment", or_dash(t.comment.clone())),
    ]
}
