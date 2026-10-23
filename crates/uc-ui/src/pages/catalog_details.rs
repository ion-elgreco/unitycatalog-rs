//! A catalog's page, as the console's `CatalogDetails`: its description and
//! schemas, its details, its grants; "Edit", "Create schema" and "Delete".

use leptos::prelude::*;
use uc_api::{CatalogInfo, CreateSchema, UpdateCatalog};
use uc_common::SecurableType;

use crate::api;
use crate::auth::use_auth;
use crate::components::edit_modal::Changes;
use crate::components::icons::{Database, Notebook, Plus, Trash2};
use crate::components::{
    Card, CreateModal, DeleteModal, DescriptionCard, EditMetadata, EntityHeader, LoadError, Loaded,
    LoadingBlock, MetaGrid, MetaRow, NewObject, PermissionsCard, PropertiesCard, RawJsonCard, Tab,
    Tabs, properties,
};
use crate::contents::Holder;
use crate::kind::{catalog_href, schema_href};
use crate::load::{Key, Load, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

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
    let name = StoredValue::new(catalog.clone());
    let info = use_load(Key::Catalog(catalog.clone()), move || {
        let catalog = name.get_value();
        async move { api::client().get_catalog(&catalog).await }
    });
    let schemas = use_load(Key::Schemas(catalog.clone()), move || {
        let catalog = name.get_value();
        async move { api::schemas(&catalog).await }
    });
    let cache = use_cache();
    let auth = use_auth();
    let tab = RwSignal::new(Tab::Overview);
    let delete_open = RwSignal::new(false);
    let create_open = RwSignal::new(false);
    let owner = field(info, |c: &CatalogInfo| c.owner.clone());
    let manages = Memo::new(move |_| {
        auth.session
            .with(|s| owner.with(|o| s.manages(o.as_deref())))
    });
    let comment = field(info, |c: &CatalogInfo| {
        c.comment.clone().unwrap_or_default()
    });

    let save_comment = move |comment: String| {
        let catalog = name.get_value();
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
    };
    let save_metadata = move |changes: Changes| {
        let catalog = name.get_value();
        async move {
            let request = UpdateCatalog {
                comment: changes.comment,
                properties: None,
                new_name: changes.new_name.clone(),
            };
            let updated = api::client().update_catalog(&catalog, &request).await?;
            match changes.new_name {
                Some(new_name) => {
                    cache.forget(Key::Catalog(catalog));
                    cache.changed(Key::Catalogs);
                    cache.set(Key::Catalog(new_name.clone()), updated);
                    Ok(Some(catalog_href(&new_name)))
                }
                None => {
                    cache.set(Key::Catalog(catalog), updated);
                    Ok(None)
                }
            }
        }
    };
    let refresh = Callback::new(move |_| {
        cache.changed(Key::Catalog(name.get_value()));
        cache.changed(Key::Schemas(name.get_value()));
    });

    let actions = view! {
        <EditMetadata name=catalog.clone() comment noun="catalog" save=save_metadata/>
        <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
            <Plus/>
            "Create schema"
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

    let schema_rows = move || match schemas.get() {
        Load::Loading => view! { <LoadingBlock/> }.into_any(),
        Load::Failed(message) => view! { <LoadError message/> }.into_any(),
        Load::Ready(list) if list.is_empty() => {
            view! { <p class="uc-empty">"No schemas."</p> }.into_any()
        }
        Load::Ready(list) => view! {
            <div class="uc-rows">
                {list
                    .into_iter()
                    .map(|s| {
                        let schema = s.name.unwrap_or_default();
                        let href = schema_href(&name.get_value(), &schema);
                        view! {
                            <a class="uc-row" href=href>
                                <Database class="uc-icon--schema"/>
                                <span class="uc-row__name">{schema}</span>
                                {s
                                    .comment
                                    .filter(|c| !c.is_empty())
                                    .map(|c| view! { <span class="uc-row__comment">{c}</span> })}
                            </a>
                        }
                    })
                    .collect_view()}
            </div>
        }
        .into_any(),
    };
    let schema_count = move || {
        schemas
            .with(|s| s.ready().map(Vec::len))
            .map(|n| n.to_string())
            .unwrap_or_default()
    };

    let content = move || {
        match tab.get() {
        Tab::Overview => view! {
            <DescriptionCard comment noun="catalog" save=save_comment/>
            <Card
                title="Schemas"
                actions=view! { <span class="uc-badge">{schema_count}</span> }.into_any()
            >
                {schema_rows}
            </Card>
        }
        .into_any(),
        Tab::Details => view! {
            <Card>
                <MetaGrid rows=field(info, catalog_rows)/>
            </Card>
            <PropertiesCard properties=field(info, |c: &CatalogInfo| properties(c.properties.as_ref()))/>
            <RawJsonCard json=field(info, pretty_json)/>
        }
        .into_any(),
        Tab::Permissions => view! {
            <PermissionsCard kind=SecurableType::Catalog full_name=name.get_value() owner/>
        }
        .into_any(),
    }
    };

    view! {
        <EntityHeader
            icon=view! { <Notebook class="uc-icon--catalog"/> }.into_any()
            name=catalog.clone()
            badges=vec!["CATALOG".to_string()]
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
            noun="catalog"
            question="Are you sure you want to delete the catalog"
            name=catalog.clone()
            label=format!("{catalog} catalog")
            delete=move |force| {
                let catalog = name.get_value();
                async move { api::client().delete_catalog(&catalog, force).await }
            }
            key=Key::Catalog(catalog.clone())
            list=Key::Catalogs
            then="/".to_string()
            holder=Holder::Catalog(catalog.clone())
        />
        <CreateModal
            open=create_open
            noun="schema"
            location="Storage root"
            create=move |form: NewObject| {
                let catalog = name.get_value();
                async move {
                    let request = CreateSchema {
                        name: form.name,
                        catalog_name: catalog.clone(),
                        comment: form.comment,
                        properties: None,
                        storage_root: form.location,
                    };
                    let schema = api::client().create_schema(&request).await?;
                    Ok(schema_href(&catalog, &schema.name.unwrap_or_default()))
                }
            }
            list=Key::Schemas(catalog.clone())
        />
    }
}

fn catalog_rows(c: &CatalogInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Name", or_dash(c.name.clone())),
        MetaRow::new("Owner", or_dash(c.owner.clone())),
        MetaRow::new("Created at", format_timestamp(c.created_at)),
        MetaRow::new("Created by", or_dash(c.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(c.updated_at)),
        MetaRow::new("Updated by", or_dash(c.updated_by.clone())),
        MetaRow::mono("Catalog ID", or_dash(c.id.clone())),
        MetaRow::mono("Storage root", or_dash(c.storage_root.clone())),
    ]
}
