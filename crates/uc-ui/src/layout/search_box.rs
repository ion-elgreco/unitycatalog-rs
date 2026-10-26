//! The search box in the header. While you type, the first results show
//! under it; the arrow keys move between them, Enter opens the one marked,
//! or the search page, as "See all results" does. `/` outside a field puts
//! the focus in the box; Escape closes the results, then empties the box.

use std::time::Duration;

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uc_client::models::search::v1::SearchResult;
use wasm_bindgen::JsCast;

use crate::api;
use crate::components::dropdown::close_on_navigate;
use crate::components::icons::Search;
use crate::load::Load;
use crate::search::{SearchType, page_href, result_href, result_parent, search, searchable};

/// The results that the box shows.
const SHOWN: i32 = 8;

/// The pause in the typing after which the box searches.
const PAUSE: Duration = Duration::from_millis(200);

/// Whether a key press in `target` types text: in a field, `/` is a
/// character.
fn in_field(target: Option<web_sys::EventTarget>) -> bool {
    target
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|element| {
            matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || element
                    .dyn_into::<web_sys::HtmlElement>()
                    .is_ok_and(|e| e.is_content_editable())
        })
}

#[component]
pub fn SearchBox() -> impl IntoView {
    let text = RwSignal::new(String::new());
    let open = RwSignal::new(false);
    let answer: RwSignal<Load<Vec<SearchResult>>> = RwSignal::new(Load::Loading);
    // The marked option: a result, or "See all results" after them.
    let active: RwSignal<Option<usize>> = RwSignal::new(None);
    let input = NodeRef::<html::Input>::new();
    // The last search: only its answer counts.
    let searches = StoredValue::new(0u64);
    // Kept, so that the closures that use it are `Copy`.
    let navigate = StoredValue::new_local(use_navigate());

    // A change of page closes the results.
    close_on_navigate(open);

    let slash = window_event_listener(ev::keydown, move |e| {
        if e.key() != "/" || e.ctrl_key() || e.meta_key() || e.alt_key() || in_field(e.target()) {
            return;
        }
        if let Some(input) = input.get_untracked() {
            e.prevent_default();
            let _ = input.focus();
        }
    });
    on_cleanup(move || slash.remove());

    let on_input = move |value: String| {
        searches.update_value(|n| *n += 1);
        let this = searches.get_value();
        active.set(None);
        text.set(value.clone());
        if !searchable(&value) {
            open.set(false);
            answer.set(Load::Loading);
            return;
        }
        open.set(true);
        set_timeout(
            move || {
                if searches.try_get_value() != Some(this) {
                    return;
                }
                spawn_local(async move {
                    let found = search(&value, SearchType::All, SHOWN, None).await;
                    if searches.try_get_value() != Some(this) {
                        return;
                    }
                    answer.set(match found {
                        Ok(found) => Load::Ready(found.results),
                        Err(e) => Load::Failed(api::failure(&e, "Failed to search")),
                    });
                });
            },
            PAUSE,
        );
    };

    let count = move || answer.with(|a| a.ready().map_or(0, Vec::len));
    // Close the results and empty the box, after a choice.
    let done = move || {
        searches.update_value(|n| *n += 1);
        text.set(String::new());
        answer.set(Load::Loading);
        open.set(false);
        active.set(None);
        if let Some(input) = input.get_untracked() {
            let _ = input.blur();
        }
    };
    let go = move |href: &str| {
        done();
        navigate.with_value(|navigate| navigate(href, Default::default()));
    };
    // A plain click opens the page here; with a modifier key, the browser
    // opens the link as it does others, such as in a new tab.
    let click = move |e: ev::MouseEvent, href: &str| {
        if e.button() != 0 || e.ctrl_key() || e.meta_key() || e.shift_key() || e.alt_key() {
            return;
        }
        e.prevent_default();
        go(href);
    };
    let keys = move |e: ev::KeyboardEvent| {
        // The results, then "See all results".
        let last = answer.with_untracked(|a| a.ready().map_or(0, Vec::len));
        match e.key().as_str() {
            "ArrowDown" if open.get_untracked() => {
                e.prevent_default();
                active.update(|a| *a = Some(a.map_or(0, |i| if i >= last { 0 } else { i + 1 })));
            }
            "ArrowUp" if open.get_untracked() => {
                e.prevent_default();
                active.update(|a| *a = Some(a.map_or(last, |i| if i == 0 { last } else { i - 1 })));
            }
            "Enter" => {
                e.prevent_default();
                let value = text.get_untracked();
                let marked = active.get_untracked().and_then(|i| {
                    answer.with_untracked(|a| a.ready().and_then(|r| r.get(i)).map(result_href))
                });
                let href = match marked {
                    Some(href) => href,
                    None if searchable(&value) => page_href(&value, SearchType::All),
                    None => return,
                };
                go(&href);
            }
            // Not the browser's own Escape of a search field, which empties
            // it at once.
            "Escape" => {
                e.prevent_default();
                if open.get_untracked() {
                    open.set(false);
                } else {
                    on_input(String::new());
                }
            }
            _ => {}
        }
    };

    let option_id = |i: usize| format!("uc-search-option-{i}");
    let all_href = move || page_href(&text.get(), SearchType::All);
    view! {
        <div class="uc-header__search" role="search">
            <div class="uc-search">
                <Search/>
                <input
                    node_ref=input
                    class="uc-input"
                    type="search"
                    placeholder="Search catalogs, tables, volumes…"
                    aria-label="Search"
                    autocomplete="off"
                    spellcheck="false"
                    role="combobox"
                    aria-controls="uc-search-results"
                    aria-expanded=move || open.get().to_string()
                    aria-activedescendant=move || active.get().map(option_id)
                    prop:value=move || text.get()
                    on:input=move |e| on_input(event_target_value(&e))
                    on:keydown=keys
                    on:focus=move |_| open.set(searchable(&text.get_untracked()))
                    on:blur=move |_| open.set(false)
                />
            </div>
            <Show when=move || open.get()>
                // A press in the list keeps the focus in the box, so that
                // the click on a link still counts.
                <div
                    id="uc-search-results"
                    class="uc-menu__panel uc-search__panel"
                    role="listbox"
                    aria-label="Search results"
                    on:mousedown=|e| e.prevent_default()
                >
                    {move || {
                        answer.with(|a| match a {
                            Load::Loading => {
                                view! { <p class="uc-search__note uc-muted">"Searching…"</p> }
                                    .into_any()
                            }
                            Load::Failed(message) => {
                                let message = message.clone();
                                view! { <p class="uc-search__note uc-error">{message}</p> }
                                    .into_any()
                            }
                            Load::Ready(list) if list.is_empty() => {
                                view! { <p class="uc-search__note uc-muted">"No results."</p> }
                                    .into_any()
                            }
                            Load::Ready(_) => ().into_any(),
                        })
                    }}
                    {move || {
                        answer
                            .with(|a| a.ready().cloned().unwrap_or_default())
                            .into_iter()
                            .enumerate()
                            .map(|(i, result)| {
                                let href = result_href(&result);
                                let kind = SearchType::of(&result);
                                let parent = result_parent(&result);
                                view! {
                                    <a
                                        id=option_id(i)
                                        class="uc-search__option"
                                        class:is-active=move || active.get() == Some(i)
                                        role="option"
                                        aria-selected=move || (active.get() == Some(i)).to_string()
                                        href=href.clone()
                                        on:click=move |e| click(e, &href)
                                    >
                                        {kind.icon()}
                                        <span class="uc-search__text">
                                            <span class="uc-truncate uc-strong">{result.name}</span>
                                            {parent
                                                .map(|parent| {
                                                    view! {
                                                        <span class="uc-truncate uc-muted">{parent}</span>
                                                    }
                                                })}
                                        </span>
                                        <span class="uc-badge uc-badge--type">{kind.noun()}</span>
                                    </a>
                                }
                            })
                            .collect_view()
                    }}
                    <a
                        id=move || option_id(count())
                        class="uc-search__option uc-search__all"
                        class:is-active=move || active.get() == Some(count())
                        role="option"
                        aria-selected=move || (active.get() == Some(count())).to_string()
                        href=all_href
                        on:click=move |e| click(e, &all_href())
                    >
                        "See all results"
                    </a>
                </div>
            </Show>
        </div>
    }
}
