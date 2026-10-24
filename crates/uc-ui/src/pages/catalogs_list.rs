//! The home page, as upstream's `pages/CatalogsList.tsx`: the list of
//! catalogs, where a click on a row opens its page, and "Create Catalog".

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use uc_client::models::catalogs::v1::CatalogInfo;

use crate::api;
use crate::components::CreateModal;
use crate::layout::{Column, ListLayout};
use crate::load::{Key, use_load};
use crate::util::format_timestamp;

#[component]
pub fn CatalogsList() -> impl IntoView {
    let catalogs = use_load(Key::Catalogs, api::catalogs);
    let create_open = RwSignal::new(false);
    let navigate = use_navigate();

    let columns = vec![
        Column {
            title: "Name",
            width: "60%",
            center: false,
            render: Box::new(|c: &CatalogInfo| view! { <span>{c.name.clone()}</span> }.into_any()),
            search_key: Some(Box::new(|c: &CatalogInfo| c.name.clone())),
        },
        Column {
            title: "Created At",
            width: "40%",
            center: false,
            render: Box::new(|c: &CatalogInfo| {
                view! { <span>{format_timestamp(c.created_at)}</span> }.into_any()
            }),
            search_key: None,
        },
    ];

    let on_row_click = Callback::new(move |catalog: CatalogInfo| {
        navigate(&format!("/data/{}", catalog.name), Default::default());
    });

    view! {
        <div class="uc-flex-col uc-gap-md uc-grow">
            <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                <h2 class="uc-title-2">"Catalogs"</h2>
                <button class="uc-btn uc-btn--primary" on:click=move |_| create_open.set(true)>
                    "Create Catalog"
                </button>
            </div>
            <ListLayout data=catalogs columns on_row_click/>
            <CreateModal
                open=create_open
                noun="catalog"
                hint="Create catalog description"
                create=|name, comment| async move {
                    let catalog = api::client().create_catalog(name).with_comment(comment).await?;
                    Ok(format!("/data/{}", catalog.name))
                }
                list=Key::Catalogs
            />
        </div>
    }
}
