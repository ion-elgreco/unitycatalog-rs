//! A list: a search box, a table and pages of 10 rows; above 50 rows, a
//! choice of the page size. The access tokens page uses it.

use leptos::prelude::*;
use std::sync::Arc;

use crate::components::icons::{ChevronLeft, ChevronRight, Search};
use crate::components::{LoadError, LoadingBlock, Stage};
use crate::load::Load;

/// One column of the table.
pub struct Column<T> {
    pub title: &'static str,
    pub width: &'static str,
    /// Renders the cell.
    pub render: Box<dyn Fn(&T) -> AnyView + Send + Sync + 'static>,
    /// The value the search box matches; none for a column it skips.
    pub search_key: Option<SearchKey<T>>,
}

/// A column's search value.
pub type SearchKey<T> = Box<dyn Fn(&T) -> String + Send + Sync + 'static>;

/// The rows of a page, at first.
const PAGE_SIZE: usize = 10;
/// The page sizes to choose from, above 50 rows.
const PAGE_SIZES: [usize; 4] = [10, 20, 50, 100];
const SIZE_CHOICE_ABOVE: usize = 50;

#[component]
pub fn ListLayout<T>(
    #[prop(into)] data: Signal<Load<Vec<T>>>,
    columns: Vec<Column<T>>,
    /// What the table says when it has no rows.
    #[prop(default = "No data.")]
    empty: &'static str,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
{
    let filter = RwSignal::new(String::new());
    let page = RwSignal::new(0usize);
    let page_size = RwSignal::new(PAGE_SIZE);
    let columns = Arc::new(columns);

    let stage = Memo::new(move |_| data.with(Stage::of));
    // The rows that the search matches, by their index in `data`.
    let matches = Memo::new({
        let columns = columns.clone();
        move |_| {
            let needle = filter.with(|filter| filter.to_lowercase());
            data.with(|data| {
                let rows = data.ready().map(Vec::as_slice).unwrap_or_default();
                (0..rows.len())
                    .filter(|&i| {
                        needle.is_empty()
                            || columns.iter().any(|c| {
                                c.search_key.as_ref().is_some_and(|key| {
                                    key(&rows[i]).to_lowercase().contains(&needle)
                                })
                            })
                    })
                    .collect::<Vec<_>>()
            })
        }
    });
    // The page shown: the last one when the list got shorter.
    let current = Memo::new(move |_| {
        let pages = matches.with(Vec::len).div_ceil(page_size.get()).max(1);
        page.get().min(pages - 1)
    });

    let table = move || {
        let columns = columns.clone();
        let header = columns
            .iter()
            .map(|c| view! { <th style=format!("width:{}", c.width)>{c.title}</th> })
            .collect_view();
        let body = move || {
            let size = page_size.get();
            let rows: Vec<T> = matches.with(|matches| {
                data.with(|data| {
                    let rows = data.ready().map(Vec::as_slice).unwrap_or_default();
                    matches
                        .iter()
                        .skip(current.get() * size)
                        .take(size)
                        .filter_map(|&i| rows.get(i).cloned())
                        .collect()
                })
            });
            if rows.is_empty() {
                return view! {
                    <tr>
                        <td colspan="99" class="uc-empty">{empty}</td>
                    </tr>
                }
                .into_any();
            }
            rows.into_iter()
                .map(|row| {
                    let cells = columns
                        .iter()
                        .map(|c| view! { <td>{(c.render)(&row)}</td> })
                        .collect_view();
                    view! { <tr>{cells}</tr> }
                })
                .collect_view()
                .into_any()
        };
        view! {
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>{header}</tr>
                    </thead>
                    <tbody>{body}</tbody>
                </table>
            </div>
            <Pager matches current page page_size/>
        }
    };

    view! {
        <div class="uc-stack">
            <div class="uc-search">
                <Search/>
                <input
                    class="uc-input"
                    type="search"
                    placeholder="Search"
                    aria-label="Search"
                    on:input=move |e| {
                        filter.set(event_target_value(&e));
                        page.set(0);
                    }
                />
            </div>
            {move || match stage.get() {
                Stage::Loading => view! { <LoadingBlock/> }.into_any(),
                Stage::Failed(message) => view! { <LoadError message/> }.into_any(),
                Stage::Ready => table().into_any(),
            }}
        </div>
    }
}

/// One item of the page list.
#[derive(Clone, Copy, PartialEq)]
enum Item {
    Page(usize),
    /// Pages left out.
    Gap,
}

/// The items of the page list for 1-based page `current` of `pages`: the
/// first, the last, and two on each side of the current one.
fn items(current: usize, pages: usize) -> Vec<Item> {
    const AROUND: usize = 2;
    if pages <= 3 + AROUND * 2 {
        return (1..=pages).map(Item::Page).collect();
    }
    let mut left = current.saturating_sub(AROUND).max(1);
    let mut right = (current + AROUND).min(pages);
    if current <= AROUND + 1 {
        right = 1 + AROUND * 2;
    }
    if pages - current <= AROUND {
        left = pages - AROUND * 2;
    }
    let mut items: Vec<Item> = (left..=right).map(Item::Page).collect();
    if left > 2 {
        items.insert(0, Item::Gap);
    }
    if right < pages - 1 {
        items.push(Item::Gap);
    }
    if left != 1 {
        items.insert(0, Item::Page(1));
    }
    if right != pages {
        items.push(Item::Page(pages));
    }
    items
}

/// "1-10 of 25", the pages, and above 50 rows the page size. None for a
/// single page.
#[component]
fn Pager(
    matches: Memo<Vec<usize>>,
    current: Memo<usize>,
    page: RwSignal<usize>,
    page_size: RwSignal<usize>,
) -> impl IntoView {
    move || {
        let total = matches.with(Vec::len);
        let size = page_size.get();
        let pages = total.div_ceil(size);
        if pages <= 1 {
            return ().into_any();
        }
        let p = current.get();
        let start = p * size + 1;
        let end = ((p + 1) * size).min(total);
        let page_items = items(p + 1, pages)
            .into_iter()
            .map(|item| match item {
                Item::Page(n) => view! {
                    <button
                        type="button"
                        class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--sm"
                        class:is-active={n == p + 1}
                        aria-current=move || (n == p + 1).then_some("page")
                        on:click=move |_| page.set(n - 1)
                    >
                        {n}
                    </button>
                }
                .into_any(),
                Item::Gap => view! { <span class="uc-muted">"…"</span> }.into_any(),
            })
            .collect_view();
        view! {
            <nav class="uc-pager" aria-label="Pages">
                <span class="uc-pager__total">{format!("{start}-{end} of {total}")}</span>
                {(total > SIZE_CHOICE_ABOVE)
                    .then(|| {
                        view! {
                            <select
                                class="uc-select"
                                style="width:auto"
                                aria-label="Page size"
                                on:change=move |e| {
                                    if let Ok(new_size) = event_target_value(&e).parse::<usize>() {
                                        // The page that shows the same first row.
                                        let first = page.get_untracked() * page_size.get_untracked();
                                        page_size.set(new_size);
                                        page.set((first / new_size).min(total.div_ceil(new_size) - 1));
                                    }
                                }
                            >
                                {PAGE_SIZES
                                    .into_iter()
                                    .map(|n| {
                                        view! {
                                            <option value=n.to_string() selected=n == size>
                                                {format!("{n} / page")}
                                            </option>
                                        }
                                    })
                                    .collect_view()}
                            </select>
                        }
                    })}
                <div class="uc-pager__pages">
                    <button
                        type="button"
                        class="uc-btn uc-btn--outline uc-btn--icon uc-btn--sm"
                        aria-label="Previous page"
                        disabled={p == 0}
                        on:click=move |_| page.set(p.saturating_sub(1))
                    >
                        <ChevronLeft/>
                    </button>
                    {page_items}
                    <button
                        type="button"
                        class="uc-btn uc-btn--outline uc-btn--icon uc-btn--sm"
                        aria-label="Next page"
                        disabled={p + 1 >= pages}
                        on:click=move |_| page.set(p + 1)
                    >
                        <ChevronRight/>
                    </button>
                </div>
            </nav>
        }
            .into_any()
    }
}

#[cfg(test)]
mod tests {
    use super::{Item, items};

    fn pages(list: &[Item]) -> String {
        list.iter()
            .map(|item| match item {
                Item::Page(n) => n.to_string(),
                Item::Gap => "…".into(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn pages_around_the_current_one() {
        assert_eq!(pages(&items(1, 3)), "1 2 3");
        assert_eq!(pages(&items(4, 7)), "1 2 3 4 5 6 7");
        assert_eq!(pages(&items(1, 10)), "1 2 3 4 5 … 10");
        assert_eq!(pages(&items(3, 10)), "1 2 3 4 5 … 10");
        assert_eq!(pages(&items(4, 10)), "1 2 3 4 5 6 … 10");
        assert_eq!(pages(&items(5, 10)), "1 … 3 4 5 6 7 … 10");
        assert_eq!(pages(&items(8, 10)), "1 … 6 7 8 9 10");
        assert_eq!(pages(&items(10, 10)), "1 … 6 7 8 9 10");
    }
}
