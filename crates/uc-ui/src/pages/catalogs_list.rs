//! The home page, as the console's `CatalogsList`: a card per catalog, and
//! "Create catalog".

use leptos::prelude::*;
use uc_api::{CatalogInfo, CreateCatalog};

use crate::api;
use crate::components::icons::{Notebook, Plus};
use crate::components::{CreateModal, LoadError, LoadingBlock, NewObject};
use crate::kind::catalog_href;
use crate::load::{Key, Load, use_load};
use crate::util::format_timestamp;

#[component]
pub fn CatalogsList() -> impl IntoView {
    let catalogs = use_load(Key::Catalogs, api::catalogs);
    let create_open = RwSignal::new(false);
    let cards = move || match catalogs.get() {
        Load::Loading => view! { <LoadingBlock/> }.into_any(),
        Load::Failed(message) => view! { <LoadError message/> }.into_any(),
        Load::Ready(list) if list.is_empty() => {
            view! { <p class="uc-empty">"No catalogs yet."</p> }.into_any()
        }
        Load::Ready(list) => view! {
            <div class="uc-grid">
                {list.into_iter().map(|c| view! { <CatalogCard catalog=c/> }).collect_view()}
            </div>
        }
        .into_any(),
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
                location="Storage root"
                create=|form: NewObject| async move {
                    let request = CreateCatalog {
                        name: form.name,
                        comment: form.comment,
                        properties: None,
                        storage_root: form.location,
                    };
                    let catalog = api::client().create_catalog(&request).await?;
                    Ok(catalog_href(&catalog.name.unwrap_or_default()))
                }
                list=Key::Catalogs
            />
        </div>
    }
}

#[component]
fn CatalogCard(catalog: CatalogInfo) -> impl IntoView {
    let name = catalog.name.unwrap_or_default();
    let href = catalog_href(&name);
    let updated = format_timestamp(catalog.updated_at.or(catalog.created_at));
    view! {
        <a class="uc-card uc-card--link" href=href>
            <span class="uc-card__name">
                <Notebook class="uc-icon--catalog"/>
                <span class="uc-truncate">{name}</span>
            </span>
            {catalog
                .comment
                .filter(|c| !c.is_empty())
                .map(|comment| view! { <p class="uc-truncate">{comment}</p> })}
            <p class="uc-small">{format!("Updated {updated}")}</p>
        </a>
    }
}
