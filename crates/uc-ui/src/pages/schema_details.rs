//! A schema's page, as the console's `SchemaDetails`: its description, its
//! tables, volumes, functions or models behind a picker, its details, its
//! grants; "Edit" (its name, comment and properties), "New" and "Delete".
//! The picker is the URL's `?tab=`, so that the tree's groups open it.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};
use uc_client::models::schemas::v1::SchemaInfo;
use uc_common::{SecurableType, schema_full_name};

use crate::api;
use crate::auth::use_manages;
use crate::components::edit_modal::Changes;
use crate::components::icons::Plus;
use crate::components::{
    Card, CreateObjectModal, DeleteButton, DeleteModal, DescriptionCard, DetailsTab, EditMetadata,
    Empty, EntityHeader, Loaded, MetaRow, ObjectRow, PermissionsCard, Tabs, loaded_list,
    properties,
};
use crate::contents::Holder;
use crate::kind::{Kind, schema_icon};
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{
    catalog_href, format_timestamp, or_dash, pretty_json, route_params, schema_href,
};

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
    let names = StoredValue::new((catalog.clone(), schema.clone(), full_name.clone()));
    let info = use_load(Key::Schema(full_name.clone()), move || {
        let (_, _, full_name) = names.get_value();
        async move { api::client().schema(full_name).get().await }
    });
    let cache = use_cache();
    let query = use_query_map();
    let kind = Memo::new(move |_| {
        query
            .with(|query| query.get("tab"))
            .and_then(|tab| Kind::from_tab(&tab))
            .unwrap_or(Kind::Tables)
    });
    let delete_open = RwSignal::new(false);
    let create_open = RwSignal::new(false);
    let new_kind = RwSignal::new(Kind::Tables);
    let owner = field(info, |s: &SchemaInfo| s.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |s: &SchemaInfo| s.comment.clone().unwrap_or_default());
    let props = field(info, |s: &SchemaInfo| properties(&s.properties));

    let save_metadata = move |changes: Changes| {
        let (catalog, _, full_name) = names.get_value();
        async move {
            let updated = api::client()
                .schema(&full_name)
                .update()
                .with_comment(changes.comment)
                .with_properties(changes.properties.unwrap_or_default())
                .with_new_name(changes.new_name.clone())
                .await?;
            let page = changes
                .new_name
                .as_deref()
                .map(|new_name| schema_href(&catalog, new_name));
            let renamed = changes.new_name.map(|new_name| {
                (
                    Key::Schema(schema_full_name(&catalog, &new_name)),
                    Key::Schemas(catalog.clone()),
                )
            });
            cache.edited(Key::Schema(full_name), updated, renamed);
            Ok(page)
        }
    };
    let save_comment = move |comment: String| {
        let saving = save_metadata(Changes {
            new_name: None,
            comment: Some(comment),
            properties: None,
        });
        async move { saving.await.map(|_| ()) }
    };
    let refresh = Callback::new(move |_| {
        let (catalog, schema, full_name) = names.get_value();
        cache.changed(Key::Schema(full_name));
        cache.changed(Key::Entries(catalog, schema, kind.get_untracked()));
    });
    let new_object = move |_| {
        new_kind.set(kind.get_untracked());
        create_open.set(true);
    };

    view! {
        <EntityHeader
            icon=schema_icon()
            name=full_name.clone()
            badges=vec!["Schema".to_string()]
            on_refresh=refresh
            actions=view! {
                <EditMetadata name=schema.clone() comment properties=props save=save_metadata/>
                <button type="button" class="uc-btn uc-btn--sm" on:click=new_object>
                    <Plus/>
                    "New"
                </button>
                <DeleteButton open=delete_open when=manages/>
            }
                .into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || {
                        let (catalog, schema, _) = names.get_value();
                        view! {
                            <DescriptionCard comment noun="schema" save=save_comment/>
                            <Card actions=view! {
                                <KindPicker kind catalog=catalog.clone() schema=schema.clone()/>
                                <button type="button" class="uc-btn uc-btn--outline uc-btn--sm" on:click=new_object>
                                    <Plus/>
                                    "New"
                                </button>
                            }
                                .into_any()>
                                {move || {
                                    let (catalog, schema, _) = names.get_value();
                                    view! { <Entries kind=kind.get() catalog schema/> }
                                }}
                            </Card>
                        }
                    }
                    details=move || {
                        view! {
                            <DetailsTab
                                rows=field(info, schema_rows)
                                properties=props
                                json=field(info, pretty_json)
                            />
                        }
                    }
                    permissions=move || {
                        let (_, _, full_name) = names.get_value();
                        view! { <PermissionsCard kind=SecurableType::Schema full_name owner/> }
                    }
                />
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="schema"
            name=schema.clone()
            delete=move |force| {
                let (_, _, full_name) = names.get_value();
                async move { api::client().schema(full_name).delete().with_force(force).await }
            }
            key=Key::Schema(full_name.clone())
            list=Key::Schemas(catalog.clone())
            then=catalog_href(&catalog)
            holder=Holder::Schema(catalog.clone(), schema.clone())
        />
        <CreateObjectModal open=create_open kind=new_kind catalog=catalog.clone() schema=schema.clone()/>
    }
}

/// Tables, Volumes, Functions, Models: a click sets the URL's `?tab=` in
/// place, without a new history entry.
#[component]
fn KindPicker(kind: Memo<Kind>, catalog: String, schema: String) -> impl IntoView {
    let navigate = use_navigate();
    view! {
        <div class="uc-segmented" role="tablist" aria-label="Objects">
            {Kind::ALL
                .into_iter()
                .map(|this| {
                    let href = this.tab_href(&catalog, &schema);
                    let navigate = navigate.clone();
                    let in_place = NavigateOptions {
                        replace: true,
                        scroll: false,
                        ..Default::default()
                    };
                    view! {
                        <button
                            type="button"
                            role="tab"
                            aria-selected=move || (kind.get() == this).to_string()
                            class:is-active=move || kind.get() == this
                            on:click=move |_| navigate(&href, in_place.clone())
                        >
                            {this.icon()}
                            {this.label()}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// The objects of one kind, as rows that link to their pages.
#[component]
fn Entries(kind: Kind, catalog: String, schema: String) -> impl IntoView {
    let names = StoredValue::new((catalog, schema));
    let entries = use_load(
        {
            let (catalog, schema) = names.get_value();
            Key::Entries(catalog, schema, kind)
        },
        move || {
            let (catalog, schema) = names.get_value();
            async move { kind.entries(&catalog, &schema).await }
        },
    );
    move || {
        let empty = Empty::new(
            format!("No {}.", kind.label().to_lowercase()),
            kind.empty_hint(),
        );
        loaded_list(entries.get(), empty, |list| {
            let (catalog, schema) = names.get_value();
            view! {
                <div class="uc-rows">
                    {list
                        .into_iter()
                        .map(|entry| {
                            view! {
                                <ObjectRow
                                    icon=kind.icon()
                                    name=entry.name.clone()
                                    href=kind.href(&catalog, &schema, &entry.name)
                                    comment=None
                                />
                            }
                        })
                        .collect_view()}
                </div>
            }
            .into_any()
        })
    }
}

fn schema_rows(s: &SchemaInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Catalog", or_dash(s.catalog_name.clone())),
        MetaRow::new("Owner", or_dash(s.owner.clone())),
        MetaRow::new("Created at", format_timestamp(s.created_at)),
        MetaRow::new("Updated at", format_timestamp(s.updated_at)),
        MetaRow::mono("Schema ID", or_dash(s.schema_id.clone())),
        MetaRow::mono("Storage root", or_dash(s.storage_root.clone())),
        MetaRow::new("Comment", or_dash(s.comment.clone())),
    ]
}
