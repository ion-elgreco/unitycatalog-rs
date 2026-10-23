//! A schema's page, as the console's `SchemaDetails`: its description, its
//! tables, volumes, functions or models behind a picker, its details, its
//! grants; "Edit", "New" and "Delete". The picker is the URL's `?tab=`, so
//! that the tree's groups open it.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};
use uc_api::{SchemaInfo, UpdateSchema};
use uc_common::{SecurableType, schema_full_name};

use crate::api;
use crate::auth::use_auth;
use crate::components::edit_modal::Changes;
use crate::components::icons::{Database, Plus, Trash2};
use crate::components::{
    Card, CreateObjectModal, DeleteModal, DescriptionCard, EditMetadata, EntityHeader, LoadError,
    Loaded, LoadingBlock, MetaGrid, MetaRow, PermissionsCard, PropertiesCard, RawJsonCard, Tab,
    Tabs, properties,
};
use crate::contents::Holder;
use crate::kind::{Kind, schema_href};
use crate::load::{Key, Load, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

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
        async move { api::client().get_schema(&full_name).await }
    });
    let cache = use_cache();
    let auth = use_auth();
    let tab = RwSignal::new(Tab::Overview);
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
    let manages = Memo::new(move |_| {
        auth.session
            .with(|s| owner.with(|o| s.manages(o.as_deref())))
    });
    let comment = field(info, |s: &SchemaInfo| s.comment.clone().unwrap_or_default());

    let save_comment = move |comment: String| {
        let (_, _, full_name) = names.get_value();
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
    };
    let save_metadata = move |changes: Changes| {
        let (catalog, _, full_name) = names.get_value();
        async move {
            let request = UpdateSchema {
                comment: changes.comment,
                properties: None,
                new_name: changes.new_name.clone(),
            };
            let updated = api::client().update_schema(&full_name, &request).await?;
            match changes.new_name {
                Some(new_name) => {
                    cache.forget(Key::Schema(full_name));
                    cache.changed(Key::Schemas(catalog.clone()));
                    cache.set(Key::Schema(schema_full_name(&catalog, &new_name)), updated);
                    Ok(Some(schema_href(&catalog, &new_name)))
                }
                None => {
                    cache.set(Key::Schema(full_name), updated);
                    Ok(None)
                }
            }
        }
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

    let actions = view! {
        <EditMetadata name=schema.clone() comment noun="schema" save=save_metadata/>
        <button type="button" class="uc-btn uc-btn--sm" on:click=new_object>
            <Plus/>
            "New"
        </button>
        <Show when=move || manages.get()>
            <button
                type="button"
                class="uc-btn uc-btn--destructive uc-btn--sm"
                on:click=move |_| delete_open.set(true)
            >
                <Trash2/>
                "Delete"
            </button>
        </Show>
    }
    .into_any();

    let content = move || {
        match tab.get() {
        Tab::Overview => {
            let (catalog, schema, _) = names.get_value();
            view! {
                <DescriptionCard comment noun="schema" save=save_comment/>
                <Card
                    title="Objects"
                    actions=view! {
                        <KindPicker kind catalog=catalog.clone() schema=schema.clone()/>
                        <button type="button" class="uc-btn uc-btn--outline uc-btn--sm" on:click=new_object>
                            <Plus/>
                            "New"
                        </button>
                    }
                        .into_any()
                >
                    {move || {
                        let (catalog, schema, _) = names.get_value();
                        view! { <Entries kind=kind.get() catalog schema/> }
                    }}
                </Card>
            }
            .into_any()
        }
        Tab::Details => view! {
            <Card>
                <MetaGrid rows=field(info, schema_rows)/>
            </Card>
            <PropertiesCard properties=field(info, |s: &SchemaInfo| properties(s.properties.as_ref()))/>
            <RawJsonCard json=field(info, pretty_json)/>
        }
        .into_any(),
        Tab::Permissions => {
            let (_, _, full_name) = names.get_value();
            view! { <PermissionsCard kind=SecurableType::Schema full_name owner/> }.into_any()
        }
    }
    };

    view! {
        <EntityHeader
            icon=view! { <Database class="uc-icon--schema"/> }.into_any()
            name=schema.clone()
            badges=vec!["SCHEMA".to_string()]
            on_refresh=refresh
            actions
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs tab/>
                {content}
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="schema"
            question="Are you sure you want to delete the schema"
            name=schema.clone()
            label=format!("{schema} schema")
            delete=move |force| {
                let (_, _, full_name) = names.get_value();
                async move { api::client().delete_schema(&full_name, force).await }
            }
            key=Key::Schema(full_name.clone())
            list=Key::Schemas(catalog.clone())
            then=format!("/catalog/{catalog}")
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
    move || match entries.get() {
        Load::Loading => view! { <LoadingBlock/> }.into_any(),
        Load::Failed(message) => view! { <LoadError message/> }.into_any(),
        Load::Ready(list) if list.is_empty() => {
            view! { <p class="uc-empty">{format!("No {}.", kind.label().to_lowercase())}</p> }
                .into_any()
        }
        Load::Ready(list) => view! {
            <div class="uc-rows">
                {list
                    .into_iter()
                    .map(|entry| {
                        let (catalog, schema) = names.get_value();
                        let href = kind.href(&catalog, &schema, &entry.name);
                        view! {
                            <a class="uc-row" href=href>
                                {kind.icon()}
                                <span class="uc-row__name">{entry.name}</span>
                                {entry
                                    .comment
                                    .map(|c| view! { <span class="uc-row__comment">{c}</span> })}
                            </a>
                        }
                    })
                    .collect_view()}
            </div>
        }
        .into_any(),
    }
}

fn schema_rows(s: &SchemaInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Name", or_dash(s.name.clone())),
        MetaRow::new("Catalog", or_dash(s.catalog_name.clone())),
        MetaRow::new("Owner", or_dash(s.owner.clone())),
        MetaRow::new("Created at", format_timestamp(s.created_at)),
        MetaRow::new("Created by", or_dash(s.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(s.updated_at)),
        MetaRow::new("Updated by", or_dash(s.updated_by.clone())),
        MetaRow::mono("Schema ID", or_dash(s.schema_id.clone())),
        MetaRow::mono("Storage root", or_dash(s.storage_root.clone())),
    ]
}
