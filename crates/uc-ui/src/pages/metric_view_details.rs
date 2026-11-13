//! A metric view's page, as the console's `MetricViewDetails`: its
//! description, columns and YAML definition, its details, its grants;
//! "Delete". The API keeps a metric view as a table of the type
//! `METRIC_VIEW`.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;
use uc_client::models::tables::v1::TableInfo;
use uc_common::{SecurableType, three_part_full_name};

use crate::api;
use crate::auth::use_manages;
use crate::components::{
    Card, ColumnsCard, DeleteButton, DeleteModal, DescriptionCard, DetailsTab, EntityHeader, Label,
    Loaded, MetaRow, PermissionsCard, Tabs, properties,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::metric_view::{dependencies, is_metric_view};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn MetricViewDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "view"]);
    move || {
        let [catalog, schema, view] = params.get();
        view! { <MetricViewPage catalog schema view/> }
    }
}

#[component]
fn MetricViewPage(catalog: String, schema: String, view: String) -> impl IntoView {
    let full_name = three_part_full_name(&catalog, &schema, &view);
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
    let definition = field(info, |t: &TableInfo| {
        t.view_definition.clone().unwrap_or_default()
    });

    // A table of another type has the table's page.
    let table_page = Kind::Tables.href(&catalog, &schema, &view);
    let navigate = use_navigate();
    Effect::new(move |_| {
        if info.with(|i| i.ready().is_some_and(|t| !is_metric_view(t))) {
            let in_place = NavigateOptions {
                replace: true,
                ..Default::default()
            };
            navigate(&table_page, in_place);
        }
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
    let reload = vec![Key::Table(full_name.clone())];

    view! {
        <EntityHeader
            icon=Kind::MetricViews.icon()
            name=full_name.clone()
            badges=vec![Label::from("Metric view")]
            reload
            actions=view! { <DeleteButton open=delete_open when=manages/> }.into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || {
                        view! {
                            <DescriptionCard comment noun="metric view" save=save_comment/>
                            <ColumnsCard info full_name=name.get_value()/>
                            <Show when=move || definition.with(|d| !d.is_empty())>
                                <Card title="Metric view definition">
                                    <pre class="uc-code uc-code--dark">{move || definition.get()}</pre>
                                </Card>
                            </Show>
                        }
                    }
                    details=move || {
                        view! {
                            <DetailsTab
                                rows=field(info, metric_view_rows)
                                properties=field(info, |t: &TableInfo| properties(&t.properties))
                                json=field(info, pretty_json)
                            />
                        }
                    }
                    permissions=move || {
                        view! { <PermissionsCard kind=SecurableType::Table full_name=name.get_value() owner/> }
                    }
                />
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="metric view"
            name=view.clone()
            delete=move |_| {
                let full_name = name.get_value();
                async move { api::client().table(full_name).delete().await }
            }
            key=Key::Table(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::MetricViews)
            then=Kind::MetricViews.tab_href(&catalog, &schema)
        />
    }
}

fn metric_view_rows(t: &TableInfo) -> Vec<MetaRow> {
    let reads = dependencies(t);
    vec![
        MetaRow::new("Owner", or_dash(t.owner.clone())),
        MetaRow::new("Created at", format_timestamp(t.created_at)),
        MetaRow::new("Updated at", format_timestamp(t.updated_at)),
        MetaRow::mono("Metric view ID", or_dash(t.table_id.clone())),
        MetaRow::mono(
            "Depends on",
            or_dash((!reads.is_empty()).then(|| reads.join(", "))),
        ),
        MetaRow::new("Comment", or_dash(t.comment.clone())),
    ]
}
