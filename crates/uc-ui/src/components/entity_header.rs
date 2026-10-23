//! The path at the top of the page (`Crumbs`), which the shell draws from
//! the URL, and the head of a detail page, as the console's `EntityHeader`:
//! the icon, the name, its type badges, a button that loads the object
//! again, and the page's actions.

use leptos::prelude::*;

use super::icons::{ChevronRight, Notebook, RefreshCw};
use crate::util::{Route, catalog_href, schema_href};

/// The path: "Catalog > catalog > schema > name", or, outside the
/// catalogs, "Credentials > name". Each part but the last, which is the
/// page shown, is a link; a first part alone, such as "Catalog" on the list
/// of catalogs, keeps its link, as the console's.
#[component]
pub fn Crumbs(route: Route) -> impl IntoView {
    // Each part's label and page.
    let mut parts: Vec<(String, String)> = Vec::new();
    match route.section {
        Some(section) => {
            parts.push((section.title().into(), section.href().into()));
            if let Some(item) = &route.item {
                parts.push((item.clone(), section.item_href(item)));
            }
        }
        None => {
            parts.push(("Catalog".into(), "/".into()));
            if let Some(catalog) = &route.catalog {
                parts.push((catalog.clone(), catalog_href(catalog)));
                if let Some(schema) = &route.schema {
                    parts.push((schema.clone(), schema_href(catalog, schema)));
                    if let Some((kind, name)) = &route.object {
                        let label = match &route.version {
                            Some(version) => format!("{name} · v{version}"),
                            None => name.clone(),
                        };
                        parts.push((label, kind.href(catalog, schema, name)));
                    }
                }
            }
        }
    }
    let last = parts.len() - 1;
    let items = parts
        .into_iter()
        .enumerate()
        .map(|(i, (label, href))| {
            let separator = (i > 0).then(|| {
                view! {
                    <li class="uc-crumbs__sep" role="presentation" aria-hidden="true">
                        <ChevronRight/>
                    </li>
                }
            });
            let icon = (i == 0).then(|| match route.section {
                Some(section) => section.icon(),
                None => view! { <Notebook/> }.into_any(),
            });
            let item = if i == last && last > 0 {
                view! {
                    <li>
                        <span class="uc-crumbs__page" aria-current="page">{icon}{label}</span>
                    </li>
                }
                .into_any()
            } else {
                view! { <li><a href=href>{icon}{label}</a></li> }.into_any()
            };
            view! { {separator}{item} }
        })
        .collect_view();
    view! {
        <nav class="uc-crumbs" aria-label="breadcrumb">
            <ol>{items}</ol>
        </nav>
    }
}

#[component]
pub fn EntityHeader(
    icon: AnyView,
    #[prop(into)] name: String,
    /// Such as "MANAGED" and "DELTA" for a table.
    #[prop(into, optional)]
    badges: Signal<Vec<String>>,
    /// Load the object again.
    #[prop(into, optional)]
    on_refresh: Option<Callback<()>>,
    #[prop(optional)] actions: Option<AnyView>,
) -> impl IntoView {
    view! {
        <div class="uc-entity">
            <div class="uc-entity__row">
                <div class="uc-entity__title">
                    {icon}
                    <h1>{name}</h1>
                    {move || {
                        badges
                            .get()
                            .into_iter()
                            .map(|badge| view! { <span class="uc-badge uc-badge--type">{badge}</span> })
                            .collect_view()
                    }}
                    {on_refresh
                        .map(|on_refresh| {
                            view! {
                                <button
                                    type="button"
                                    class="uc-btn uc-btn--outline uc-btn--icon uc-btn--sm"
                                    title="Reload"
                                    aria-label="Reload"
                                    on:click=move |_| on_refresh.run(())
                                >
                                    <RefreshCw/>
                                </button>
                            }
                        })}
                </div>
                {actions.map(|actions| view! { <div class="uc-entity__actions">{actions}</div> })}
            </div>
        </div>
    }
}
