//! The path at the top of the page (`Crumbs`), which the shell draws from
//! the URL, and the head of a detail page, as the console's `EntityHeader`:
//! the icon, the name, its type badges, a button that loads the object
//! again, and the page's actions.

use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::icons::{ChevronRight, Notebook, RefreshCw};
use super::label::{Label, LabelText};
use crate::load::{Key, use_cache};
use crate::util::{Route, catalog_href, schema_href};

/// One turn of the Reload icon, as `.uc-spin` turns it.
const TURN: Duration = Duration::from_secs(1);

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
    /// Such as "MANAGED" and "DELTA", with its logo, for a table.
    #[prop(into, optional)]
    badges: Signal<Vec<Label>>,
    /// What the Reload button loads again: the object, and the lists the
    /// page shows. None hides the button.
    #[prop(into, optional)]
    reload: Option<Signal<Vec<Key>>>,
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
                            .map(|badge| {
                                view! {
                                    <span class="uc-badge uc-badge--type">
                                        <LabelText label=badge/>
                                    </span>
                                }
                            })
                            .collect_view()
                    }}
                    {reload.map(|keys| view! { <ReloadButton keys/> })}
                </div>
                {actions.map(|actions| view! { <div class="uc-entity__actions">{actions}</div> })}
            </div>
        </div>
    }
}

/// Loads `keys` again. The icon turns while they load, at least one turn
/// for a click, and turns red when the last load failed, with the reason
/// in its title. The page keeps what it shows meanwhile.
#[component]
fn ReloadButton(keys: Signal<Vec<Key>>) -> impl IntoView {
    let cache = use_cache();
    let progress = cache.progress(keys);
    let turning = RwSignal::new(false);
    let reload = move |_| {
        for key in keys.get_untracked() {
            cache.changed(key);
        }
        turning.set(true);
        spawn_local(async move {
            gloo_timers::future::sleep(TURN).await;
            // Only when the page is still there.
            let _ = turning.try_set(false);
        });
    };
    let busy = move || turning.get() || progress.with(|p| p.loading);
    let failed = move || progress.with(|p| p.failure.is_some());
    let title = move || {
        progress.with(|p| match &p.failure {
            Some(failure) => format!("Reload failed: {failure}"),
            None => "Reload".to_string(),
        })
    };
    view! {
        <button
            type="button"
            class="uc-btn uc-btn--outline uc-btn--icon uc-btn--sm uc-reload"
            class:is-busy=busy
            class:is-failed=failed
            aria-busy=move || if busy() { "true" } else { "false" }
            title=title
            aria-label=title
            on:click=reload
        >
            <RefreshCw/>
        </button>
    }
}
