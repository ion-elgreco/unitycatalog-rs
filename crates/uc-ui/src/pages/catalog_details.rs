//! A catalog's page, as the console's `CatalogDetails`: its description and
//! schemas, its details, its grants; "Edit" (its name, comment and
//! properties), "Create schema" and "Delete".

use leptos::prelude::*;
use uc_client::models::catalogs::v1::CatalogInfo;
use uc_common::SecurableType;

use crate::api;
use crate::auth::use_manages;
use crate::components::edit_modal::Changes;
use crate::components::icons::Plus;
use crate::components::{
    Card, CreateModal, DeleteButton, DeleteModal, DescriptionCard, DetailsTab, EditMetadata, Empty,
    EntityHeader, Loaded, MetaRow, NewObject, ObjectRow, PermissionsCard, ProtectButton, Tabs,
    counted, loaded_list, properties, use_protection,
};
use crate::contents::Holder;
use crate::kind::{catalog_icon, schema_icon};
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{
    catalog_href, format_timestamp, or_dash, pretty_json, route_params, schema_href,
};

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
        async move { api::client().catalog(catalog).get().await }
    });
    let schemas = use_load(Key::Schemas(catalog.clone()), move || {
        let catalog = name.get_value();
        async move { api::schemas(&catalog).await }
    });
    let cache = use_cache();
    let delete_open = RwSignal::new(false);
    let create_open = RwSignal::new(false);
    let owner = field(info, |c: &CatalogInfo| c.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |c: &CatalogInfo| {
        c.comment.clone().unwrap_or_default()
    });
    let props = field(info, |c: &CatalogInfo| properties(&c.properties));

    let save_metadata = move |changes: Changes| {
        let catalog = name.get_value();
        async move {
            let updated = api::client()
                .catalog(&catalog)
                .update()
                .with_comment(changes.comment)
                .with_properties(changes.properties.unwrap_or_default())
                .with_new_name(changes.new_name.clone())
                .await?;
            let page = changes.new_name.as_deref().map(catalog_href);
            let renamed = changes
                .new_name
                .map(|new_name| (Key::Catalog(new_name), Key::Catalogs));
            cache.edited(Key::Catalog(catalog), updated, renamed);
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
    let protected = use_protection(SecurableType::Catalog, catalog.clone());
    let reload = vec![
        Key::Catalog(catalog.clone()),
        Key::Schemas(catalog.clone()),
        Key::Protection(SecurableType::Catalog, catalog.clone()),
    ];

    let schema_rows = move || {
        let empty = Empty::new(
            "No schemas.",
            "A schema groups the tables, volumes, functions and models of a catalog. Click Create schema to make one.",
        );
        loaded_list(schemas.get(), empty, |list| {
            let catalog = name.get_value();
            view! {
                <div class="uc-rows">
                    {list
                        .into_iter()
                        .map(|s| {
                            let schema = s.name;
                            view! {
                                <ObjectRow
                                    icon=schema_icon()
                                    name=schema.clone()
                                    href=schema_href(&catalog, &schema)
                                    comment=s.comment
                                />
                            }
                        })
                        .collect_view()}
                </div>
            }
            .into_any()
        })
    };

    view! {
        <EntityHeader
            icon=catalog_icon()
            name=catalog.clone()
            badges=vec!["Catalog".to_string()]
            reload
            protected
            actions=view! {
                <EditMetadata name=catalog.clone() comment properties=props save=save_metadata/>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create schema"
                </button>
                <ProtectButton kind=SecurableType::Catalog full_name=catalog.clone() protected when=manages/>
                <DeleteButton open=delete_open when=manages protected/>
            }
                .into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || {
                        view! {
                            <DescriptionCard comment noun="catalog" save=save_comment/>
                            <Card title=counted("Schemas", schemas)>{schema_rows}</Card>
                        }
                    }
                    details=move || {
                        view! {
                            <DetailsTab
                                rows=field(info, catalog_rows)
                                properties=props
                                json=field(info, pretty_json)
                            />
                        }
                    }
                    permissions=move || {
                        view! { <PermissionsCard kind=SecurableType::Catalog full_name=name.get_value() owner/> }
                    }
                />
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="catalog"
            name=catalog.clone()
            delete=move |force| {
                let catalog = name.get_value();
                async move { api::client().catalog(catalog).delete().with_force(force).await }
            }
            key=Key::Catalog(catalog.clone())
            list=Key::Catalogs
            then="/".to_string()
            holder=Holder::Catalog(catalog.clone())
        />
        <CreateModal
            open=create_open
            noun="schema"
            create=move |form: NewObject| {
                let catalog = name.get_value();
                async move {
                    api::client()
                        .create_schema(form.name, catalog)
                        .with_comment(form.comment)
                        .with_storage_root(form.location)
                        .await?;
                    Ok(())
                }
            }
            list=Key::Schemas(catalog.clone())
        />
    }
}

fn catalog_rows(c: &CatalogInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Owner", or_dash(c.owner.clone())),
        MetaRow::new("Created at", format_timestamp(c.created_at)),
        MetaRow::new("Updated at", format_timestamp(c.updated_at)),
        MetaRow::mono("Catalog ID", or_dash(c.id.clone())),
        MetaRow::mono("Storage root", or_dash(c.storage_root.clone())),
        MetaRow::new("Comment", or_dash(c.comment.clone())),
    ]
}
