//! The search page: the objects whose name or comment holds the words of
//! the URL's `?q=`, of the type `?type=` or of all types, in the server's
//! order; "Load more" for the next page. The header's search box opens it.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};
use uc_client::models::search::v1::SearchResult;

use crate::api;
use crate::components::icons::Search;
use crate::components::{Empty, EmptyState, LoadError, LoadingBlock, use_notify};
use crate::load::Load;
use crate::search::{MIN_CHARS, SearchType, page_href, result_href, search, searchable};
use crate::util::{format_timestamp, or_dash};

/// The results of one call.
const PAGE: i32 = 50;

#[component]
pub fn SearchResults() -> impl IntoView {
    let query = use_query_map();
    // The text and the type that the URL asks for.
    let asked = Memo::new(move |_| {
        query.with(|query| {
            let text = query.get("q").unwrap_or_default();
            let kind = query
                .get("type")
                .map_or(SearchType::All, |t| SearchType::from_param(&t));
            (text, kind)
        })
    });
    let text = RwSignal::new(asked.get_untracked().0);
    let results: RwSignal<Load<Vec<SearchResult>>> = RwSignal::new(Load::Loading);
    // The token of the next page, when there is one.
    let next: RwSignal<Option<String>> = RwSignal::new(None);
    let loading_more = RwSignal::new(false);
    // The last load: only its answer counts.
    let loads = StoredValue::new(0u64);
    let notify = use_notify();
    // The first page for the URL, or the page after `token`.
    let load = move |token: Option<String>| {
        loads.update_value(|n| *n += 1);
        let this = loads.get_value();
        let (asked_text, kind) = asked.get_untracked();
        let more = token.is_some();
        if !searchable(&asked_text) {
            next.set(None);
            results.set(Load::Ready(Vec::new()));
            return;
        }
        match more {
            true => loading_more.set(true),
            false => results.set(Load::Loading),
        }
        spawn_local(async move {
            let answer = search(&asked_text, kind, PAGE, token).await;
            if loads.try_get_value() != Some(this) {
                return;
            }
            match answer {
                Ok(page) => {
                    next.set(page.next_page_token.filter(|t| !t.is_empty()));
                    if more {
                        results.update(|results| {
                            if let Load::Ready(items) = results {
                                items.extend(page.results);
                            }
                        });
                    } else {
                        results.set(Load::Ready(page.results));
                    }
                }
                Err(e) if more => notify.failed(&e, "Failed to load more results"),
                Err(e) => results.set(Load::Failed(api::failure(&e, "Failed to search"))),
            }
            loading_more.set(false);
        });
    };
    // A new URL, such as from the header's box, sets the field and loads.
    Effect::new(move |_| {
        text.set(asked.get().0);
        load(None);
    });

    let navigate = use_navigate();
    let submit = {
        let navigate = navigate.clone();
        move |e: leptos::ev::SubmitEvent| {
            e.prevent_default();
            let kind = asked.get_untracked().1;
            navigate(&page_href(&text.get_untracked(), kind), Default::default());
        }
    };
    let pick = move |kind: SearchType| {
        let in_place = NavigateOptions {
            replace: true,
            scroll: false,
            ..Default::default()
        };
        navigate(&page_href(&asked.get_untracked().0, kind), in_place);
    };

    let empty = move || {
        let (asked_text, kind) = asked.get();
        if !searchable(&asked_text) {
            return Empty::new(
                format!("Type at least {MIN_CHARS} characters."),
                "The search finds catalogs, schemas, tables, volumes, functions and models by the words of their names and comments.",
            );
        }
        let text = match kind {
            SearchType::All => format!("Nothing found for \u{201c}{}\u{201d}.", asked_text.trim()),
            kind => format!(
                "No {} found for \u{201c}{}\u{201d}.",
                kind.label().to_lowercase(),
                asked_text.trim()
            ),
        };
        Empty::new(
            text,
            "An object matches when its name or comment holds all the words. You see only the objects that you may read.",
        )
    };
    let table = move || {
        match results.get() {
        Load::Loading => view! { <LoadingBlock/> }.into_any(),
        Load::Failed(message) => view! { <LoadError message/> }.into_any(),
        Load::Ready(list) if list.is_empty() => {
            view! { <EmptyState empty=empty()/> }.into_any()
        }
        Load::Ready(list) => view! {
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>
                            <th>"Name"</th>
                            <th>"Type"</th>
                            <th>"Owner"</th>
                            <th>"Updated"</th>
                            <th>"Comment"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {list.into_iter().map(|result| view! { <ResultRow result/> }).collect_view()}
                    </tbody>
                </table>
            </div>
        }
        .into_any(),
    }
    };
    view! {
        <div class="uc-page">
            <h1 class="uc-page__title">"Search"</h1>
            <form class="uc-inline uc-search-form" role="search" on:submit=submit>
                <div class="uc-search">
                    <Search/>
                    <input
                        class="uc-input"
                        type="search"
                        aria-label="Search for"
                        placeholder="Words of names and comments"
                        autocomplete="off"
                        spellcheck="false"
                        prop:value=move || text.get()
                        on:input=move |e| text.set(event_target_value(&e))
                    />
                </div>
                <button type="submit" class="uc-btn">"Search"</button>
            </form>
            <div class="uc-segmented" role="tablist" aria-label="Types">
                {SearchType::ALL
                    .into_iter()
                    .map(|kind| {
                        let pick = pick.clone();
                        let current = move || asked.get().1 == kind;
                        view! {
                            <button
                                type="button"
                                role="tab"
                                aria-selected=move || current().to_string()
                                class:is-active=current
                                on:click=move |_| pick(kind)
                            >
                                {kind.icon()}
                                {kind.label()}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            {table}
            {move || {
                next.get()
                    .map(|token| {
                        view! {
                            <button
                                type="button"
                                class="uc-btn uc-btn--outline uc-self-start"
                                disabled=move || loading_more.get()
                                on:click=move |_| load(Some(token.clone()))
                            >
                                {move || if loading_more.get() { "Loading…" } else { "Load more" }}
                            </button>
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn ResultRow(result: SearchResult) -> impl IntoView {
    let kind = SearchType::of(&result);
    let href = result_href(&result);
    view! {
        <tr>
            <td class="uc-strong">
                <a class="uc-btn uc-btn--link" href=href>
                    {kind.icon()}
                    {result.full_name}
                </a>
            </td>
            <td>
                <span class="uc-badge uc-badge--type">{kind.noun()}</span>
            </td>
            <td class="uc-muted">{or_dash(result.owner)}</td>
            <td class="uc-muted uc-nowrap">{format_timestamp(result.updated_at)}</td>
            <td class="uc-muted">{or_dash(result.comment)}</td>
        </tr>
    }
}
