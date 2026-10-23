//! The path at the top of the page (`CatalogCrumbs`), which the shell
//! draws from the URL, and the head of a detail page, as the console's
//! `EntityHeader`: the icon, the name, its type badges, a button that
//! loads the object again, and the page's actions.

use leptos::prelude::*;

use super::icons::{ChevronRight, Notebook, RefreshCw};
use crate::kind::{catalog_href, schema_href};

/// A crumb between the schema and the current one, such as the model of a
/// version: its label and its page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parent {
    pub label: String,
    pub href: String,
}

/// The path: "Catalog > catalog > schema > name". Each part but the last
/// is a link.
#[component]
pub fn Crumbs(
    #[prop(optional_no_strip)] catalog: Option<String>,
    #[prop(optional_no_strip)] schema: Option<String>,
    #[prop(optional_no_strip)] parent: Option<Parent>,
    #[prop(optional_no_strip)] leaf: Option<String>,
) -> impl IntoView {
    // The label of a part, and its page when it is not the last.
    let mut parts: Vec<(String, Option<String>)> = vec![("Catalog".into(), Some("/".into()))];
    if let Some(catalog) = &catalog {
        parts.push((catalog.clone(), Some(catalog_href(catalog))));
        if let Some(schema) = &schema {
            parts.push((schema.clone(), Some(schema_href(catalog, schema))));
        }
    }
    if let Some(parent) = parent {
        parts.push((parent.label, Some(parent.href)));
    }
    if let Some(leaf) = leaf {
        parts.push((leaf, None));
    }
    if let Some(last) = parts.last_mut() {
        last.1 = None;
    }
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
            let icon = (i == 0).then(|| view! { <Notebook/> });
            let item = match href {
                Some(href) => view! { <li><a href=href>{icon}{label}</a></li> }.into_any(),
                None => view! {
                    <li>
                        <span class="uc-crumbs__page" aria-current="page">{icon}{label}</span>
                    </li>
                }
                .into_any(),
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
                            .map(|badge| view! { <span class="uc-badge uc-badge--upper">{badge}</span> })
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
