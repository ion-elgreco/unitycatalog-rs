//! A list: a title, a search box with optional filters beside it, and a
//! table with antd's pagination. Equivalent to upstream
//! `components/layouts/ListLayout.tsx`.

use leptos::prelude::*;
use std::sync::Arc;

use crate::components::icons::{
    DoubleLeftOutlined, DoubleRightOutlined, DownOutlined, LeftOutlined, RightOutlined,
    SearchOutlined,
};
use crate::components::{Dropdown, LoadError, LoadingBlock, Stage};
use crate::load::Load;

/// One column in a list-layout table.
pub struct Column<T> {
    pub title: &'static str,
    pub width: &'static str,
    /// Centered, as antd's `align: 'center'`.
    pub center: bool,
    /// Renders the cell.
    pub render: Box<dyn Fn(&T) -> AnyView + Send + Sync + 'static>,
    /// Returns the value used for the search filter. If `None`, the row is
    /// not searchable on this column.
    pub search_key: Option<SearchKey<T>>,
}

/// A column's search value.
pub type SearchKey<T> = Box<dyn Fn(&T) -> String + Send + Sync + 'static>;

/// The rows of a page, at first: antd's `pageSize`.
const PAGE_SIZE: usize = 10;
/// The page sizes to choose from, above 50 rows, as antd's.
const PAGE_SIZES: [usize; 4] = [10, 20, 50, 100];
const SIZE_CHOICE_ABOVE: usize = 50;

#[component]
pub fn ListLayout<T>(
    #[prop(optional)] title: Option<AnyView>,
    #[prop(into)] data: Signal<Load<Vec<T>>>,
    columns: Vec<Column<T>>,
    /// Row click handler: the clicked record, as upstream's
    /// `onRowClick(record)`.
    #[prop(into, optional)]
    on_row_click: Option<Callback<T>>,
    #[prop(default = true)] show_search: bool,
    /// Beside the search box, as upstream's `filters`.
    #[prop(optional)]
    filters: Option<AnyView>,
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

    let clickable = on_row_click.is_some();
    let table = move || {
        let columns = columns.clone();
        let header = columns
            .iter()
            .map(|c| {
                view! {
                    <th class:uc-center=c.center style=format!("width:{}", c.width)>
                        {c.title}
                    </th>
                }
            })
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
                    <tr class="uc-table__placeholder">
                        <td colspan="99">
                            <Empty/>
                        </td>
                    </tr>
                }
                .into_any();
            }
            rows.into_iter()
                .map(|row| {
                    let cells = columns
                        .iter()
                        .map(|c| view! { <td class:uc-center=c.center>{(c.render)(&row)}</td> })
                        .collect_view();
                    view! {
                        <tr on:click=move |_| {
                            if let Some(on_row_click) = on_row_click {
                                on_row_click.run(row.clone());
                            }
                        }>{cells}</tr>
                    }
                })
                .collect_view()
                .into_any()
        };
        view! {
            <table class=if clickable { "uc-table uc-table--clickable" } else { "uc-table" }>
                <thead>
                    <tr>{header}</tr>
                </thead>
                <tbody>{body}</tbody>
            </table>
            <Pager matches current page page_size/>
        }
    };

    let bar = (show_search || filters.is_some()).then(|| {
        view! {
            <div class="uc-list__bar">
                {show_search
                    .then(|| {
                        view! {
                            <div class="uc-list__search">
                                <span class="uc-input-affix">
                                    <span class="uc-input-affix__prefix">
                                        <SearchOutlined/>
                                    </span>
                                    <input
                                        type="text"
                                        placeholder="Search"
                                        on:input=move |e| {
                                            filter.set(event_target_value(&e));
                                            page.set(0);
                                        }
                                    />
                                </span>
                            </div>
                        }
                    })}
                {filters.map(|filters| view! { <div class="uc-list__filters">{filters}</div> })}
            </div>
        }
    });

    view! {
        <div class="uc-flex-col uc-gap-md uc-grow">
            {title}
            {bar}
            {move || match stage.get() {
                Stage::Loading => view! { <LoadingBlock/> }.into_any(),
                Stage::Failed(message) => view! { <LoadError message/> }.into_any(),
                Stage::Ready => table().into_any(),
            }}
        </div>
    }
}

/// antd's empty table: its image and "No data".
#[component]
fn Empty() -> impl IntoView {
    view! {
        <div class="uc-empty">
            <svg class="uc-empty__image" width="64" height="41" viewBox="0 0 64 41" aria-hidden="true">
                <g transform="translate(0 1)" fill="none" fill-rule="evenodd">
                    <ellipse fill="#f5f5f5" cx="32" cy="33" rx="32" ry="7"/>
                    <g fill-rule="nonzero" stroke="#d9d9d9">
                        <path d="M55 12.76L44.854 1.258C44.367.474 43.656 0 42.907 0H21.093c-.749 0-1.46.474-1.947 1.257L9 12.761V22h46v-9.24z"/>
                        <path
                            d="M41.613 15.931c0-1.605.994-2.93 2.227-2.931H55v18.137C55 33.26 53.68 35 52.05 35h-40.1C10.32 35 9 33.259 9 31.137V13h11.16c1.233 0 2.227 1.323 2.227 2.928v.022c0 1.605 1.005 2.901 2.237 2.901h14.752c1.232 0 2.237-1.308 2.237-2.913v-.007z"
                            fill="#fafafa"
                        />
                    </g>
                </g>
            </svg>
            <div class="uc-empty__text">"No data"</div>
        </div>
    }
}

/// One item of the page list.
#[derive(Clone, Copy, PartialEq)]
enum Item {
    Page(usize),
    /// Five pages back.
    JumpBack,
    /// Five pages on.
    JumpOn,
}

/// The items of the page list for 1-based page `current` of `pages`, as
/// antd's `Pagination` (`rc-pagination`, with 2 pages on each side).
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
    if current > AROUND * 2 && current != 1 + 2 {
        items.insert(0, Item::JumpBack);
    }
    if pages - current >= AROUND * 2 && current != pages - 2 {
        items.push(Item::JumpOn);
    }
    if left != 1 {
        items.insert(0, Item::Page(1));
    }
    if right != pages {
        items.push(Item::Page(pages));
    }
    items
}

/// antd's pagination: "1-10 of 25", the pages, and above 50 rows the page
/// size. None for a single page.
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
        let go = move |to: usize| page.set(to.min(pages) - 1);
        let page_items = items(p + 1, pages)
            .into_iter()
            .map(|item| match item {
                Item::Page(n) => view! {
                    <li class="uc-pagination__item" class:is-active={n == p + 1}>
                        <button type="button" title=n.to_string() on:click=move |_| go(n)>
                            {n}
                        </button>
                    </li>
                }
                .into_any(),
                Item::JumpBack => view! {
                    <li class="uc-pagination__jump">
                        <button
                            type="button"
                            title="Previous 5 Pages"
                            on:click=move |_| go((p + 1).saturating_sub(5).max(1))
                        >
                            <span class="uc-pagination__dots">"•••"</span>
                            <span class="uc-pagination__arrow">
                                <DoubleLeftOutlined/>
                            </span>
                        </button>
                    </li>
                }
                .into_any(),
                Item::JumpOn => view! {
                    <li class="uc-pagination__jump">
                        <button
                            type="button"
                            title="Next 5 Pages"
                            on:click=move |_| go(p + 1 + 5)
                        >
                            <span class="uc-pagination__dots">"•••"</span>
                            <span class="uc-pagination__arrow">
                                <DoubleRightOutlined/>
                            </span>
                        </button>
                    </li>
                }
                .into_any(),
            })
            .collect_view();
        view! {
            <ul class="uc-pagination">
                <li class="uc-pagination__total">{format!("{start}-{end} of {total}")}</li>
                <li class="uc-pagination__step" title="Previous Page">
                    <button type="button" disabled={p == 0} on:click=move |_| page.set(p.saturating_sub(1))>
                        <LeftOutlined/>
                    </button>
                </li>
                {page_items}
                <li class="uc-pagination__step" title="Next Page">
                    <button type="button" disabled={p + 1 >= pages} on:click=move |_| page.set(p + 1)>
                        <RightOutlined/>
                    </button>
                </li>
                {(total > SIZE_CHOICE_ABOVE).then(|| view! { <SizeChoice page page_size total/> })}
            </ul>
        }
            .into_any()
    }
}

/// "10 / page": antd's size changer.
#[component]
fn SizeChoice(page: RwSignal<usize>, page_size: RwSignal<usize>, total: usize) -> impl IntoView {
    view! {
        <li class="uc-pagination__size">
            <Dropdown
                button_class="uc-select"
                title="Page size"
                button=view! {
                    <span class="uc-select__value">{move || format!("{} / page", page_size.get())}</span>
                    <span class="uc-select__arrow">
                        <DownOutlined/>
                    </span>
                }
                    .into_any()
            >
                {PAGE_SIZES
                    .into_iter()
                    .map(|size| {
                        view! {
                            <button
                                type="button"
                                class="uc-dropdown__item uc-select__option"
                                class:is-selected=move || page_size.get() == size
                                role="menuitem"
                                on:click=move |e| {
                                    // The pager draws itself again with the new
                                    // size, and this menu with it. Leptos 0.7 drops
                                    // the menu's click handler but leaves it on the
                                    // menu: the click must not go up to it.
                                    e.stop_propagation();
                                    // The page that shows the same first row, as antd.
                                    let first = page.get_untracked() * page_size.get_untracked();
                                    page_size.set(size);
                                    page.set((first / size).min(total.div_ceil(size) - 1));
                                }
                            >
                                {format!("{size} / page")}
                            </button>
                        }
                    })
                    .collect_view()}
            </Dropdown>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::{Item, items};

    fn pages(list: &[Item]) -> String {
        list.iter()
            .map(|item| match item {
                Item::Page(n) => n.to_string(),
                Item::JumpBack => "<".into(),
                Item::JumpOn => ">".into(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn pages_as_antd() {
        assert_eq!(pages(&items(1, 3)), "1 2 3");
        assert_eq!(pages(&items(4, 7)), "1 2 3 4 5 6 7");
        assert_eq!(pages(&items(1, 10)), "1 2 3 4 5 > 10");
        assert_eq!(pages(&items(3, 10)), "1 2 3 4 5 > 10");
        assert_eq!(pages(&items(4, 10)), "1 2 3 4 5 6 > 10");
        assert_eq!(pages(&items(5, 10)), "1 < 3 4 5 6 7 > 10");
        assert_eq!(pages(&items(8, 10)), "1 < 6 7 8 9 10");
        assert_eq!(pages(&items(10, 10)), "1 < 6 7 8 9 10");
    }
}
