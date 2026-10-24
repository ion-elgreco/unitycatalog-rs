//! The home page, as the console's `CatalogsList`: a card per catalog, and
//! "Create catalog".

use leptos::prelude::*;
use uc_client::models::catalogs::v1::CatalogInfo;

use crate::api;
use crate::components::icons::Plus;
use crate::components::{CreateModal, Empty, NewObject, loaded_list};
use crate::kind::catalog_icon;
use crate::load::{Key, use_load};
use crate::util::{catalog_href, format_timestamp};

#[component]
pub fn CatalogsList() -> impl IntoView {
    let catalogs = use_load(Key::Catalogs, api::catalogs);
    let create_open = RwSignal::new(false);
    let cards = move || {
        let empty = Empty::new(
            "No catalogs yet.",
            "A catalog is the top of the hierarchy: it holds schemas, which hold tables, volumes, functions and models. Click Create catalog to make the first one.",
        );
        loaded_list(catalogs.get(), empty, |list| {
            view! {
                <div class="uc-grid">
                    {list.into_iter().map(|c| view! { <CatalogCard catalog=c/> }).collect_view()}
                </div>
            }
            .into_any()
        })
    };
    view! {
        <div class="uc-page">
            <div class="uc-between">
                <h1 class="uc-page__title">"Catalogs"</h1>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create catalog"
                </button>
            </div>
            {cards}
            <CreateModal
                open=create_open
                noun="catalog"
                storage_root=true
                create=|form: NewObject| async move {
                    api::client()
                        .create_catalog(form.name)
                        .with_comment(form.comment)
                        .with_storage_root(form.location)
                        .await?;
                    Ok(())
                }
                list=Key::Catalogs
            />
        </div>
    }
}

#[component]
fn CatalogCard(catalog: CatalogInfo) -> impl IntoView {
    let name = catalog.name;
    let href = catalog_href(&name);
    let updated = format_timestamp(match catalog.updated_at {
        0 => catalog.created_at,
        at => at,
    });
    view! {
        <a class="uc-card uc-card--link" href=href>
            <span class="uc-card__name">
                {catalog_icon()}
                <span class="uc-truncate">{name}</span>
            </span>
            {catalog
                .comment
                .filter(|c| !c.is_empty())
                .map(|comment| view! { <p class="uc-card__comment">{comment}</p> })}
            <p class="uc-small">{format!("Updated {updated}")}</p>
        </a>
    }
}
