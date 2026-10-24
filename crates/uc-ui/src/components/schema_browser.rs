//! The sidebar tree, as upstream's `SchemaBrowser` (antd's `Tree` with
//! `showLine`): catalogs, their schemas, and under each schema its tables,
//! volumes, functions and models. A node loads its children when it first
//! opens; they load again when a create or delete changes them. The labels
//! link to the pages; the page shown is selected. The arrow keys move
//! between the labels, and open and close the nodes.

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

use super::icons::{DatabaseOutlined, DownOutlined, ProductOutlined};
use super::loaded::Stage;
use crate::api;
use crate::kind::Kind;
use crate::load::{Key, Load, use_load};

#[component]
pub fn SchemaBrowser() -> impl IntoView {
    let catalogs = use_load(Key::Catalogs, api::catalogs);
    let names = names(catalogs, |c| c.name.clone());
    let tree = NodeRef::<html::Div>::new();
    view! {
        <div class="uc-browser">
            <h4 class="uc-browser__title">"Browse"</h4>
            <div
                node_ref=tree
                class="uc-tree"
                role="tree"
                aria-label="Catalogs"
                on:keydown=move |e| keys(&e, tree)
            >
                <Names
                    names
                    empty="No catalogs"
                    node=Callback::new(|name| view! { <CatalogNode name/> }.into_any())
                />
            </div>
        </div>
    }
}

/// The names of a loaded list.
fn names<T: Send + Sync + 'static>(
    list: RwSignal<Load<Vec<T>>>,
    name: fn(&T) -> String,
) -> Memo<Load<Vec<String>>> {
    Memo::new(move |_| {
        list.with(|list| match list {
            Load::Loading => Load::Loading,
            Load::Ready(items) => Load::Ready(items.iter().map(name).collect()),
            Load::Failed(message) => Load::Failed(message.clone()),
        })
    })
}

#[component]
fn CatalogNode(name: String) -> impl IntoView {
    let open = RwSignal::new(false);
    let href = format!("/data/{name}");
    let label = name.clone();
    view! {
        <div class="uc-tree-item">
            <div class="uc-tree-row">
                <Switcher open/>
                <Label href open>
                    <ProductOutlined/>
                    " "
                    {label}
                </Label>
            </div>
            <Subtree open>
                <SchemaNodes catalog=name.clone()/>
            </Subtree>
        </div>
    }
}

#[component]
fn SchemaNodes(catalog: String) -> impl IntoView {
    let schemas = use_load(Key::Schemas(catalog.clone()), {
        let catalog = catalog.clone();
        move || {
            let catalog = catalog.clone();
            async move { api::schemas(&catalog).await }
        }
    });
    view! {
        <Names
            names=names(schemas, |s| s.name.clone())
            empty="No schemas"
            node=Callback::new(move |schema| {
                view! { <SchemaNode catalog=catalog.clone() schema/> }.into_any()
            })
        />
    }
}

#[component]
fn SchemaNode(catalog: String, schema: String) -> impl IntoView {
    let open = RwSignal::new(false);
    let href = format!("/data/{catalog}/{schema}");
    let label = schema.clone();
    view! {
        <div class="uc-tree-item">
            <div class="uc-tree-row">
                <Switcher open/>
                <Label href open>
                    <DatabaseOutlined/>
                    " "
                    {label}
                </Label>
            </div>
            <Subtree open>
                {Kind::ALL
                    .into_iter()
                    .map(|kind| view! { <KindNode catalog=catalog.clone() schema=schema.clone() kind/> })
                    .collect_view()}
            </Subtree>
        </div>
    }
}

/// "Tables", "Volumes", "Functions" or "Models" under a schema.
#[component]
fn KindNode(catalog: String, schema: String, kind: Kind) -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <div class="uc-tree-item">
            <div class="uc-tree-row">
                <Switcher open/>
                <Label href=kind.tab_href(&catalog, &schema) open>
                    {kind.label()}
                </Label>
            </div>
            <Subtree open>
                <EntryNodes catalog=catalog.clone() schema=schema.clone() kind/>
            </Subtree>
        </div>
    }
}

#[component]
fn EntryNodes(catalog: String, schema: String, kind: Kind) -> impl IntoView {
    let entries = use_load(Key::Entries(catalog.clone(), schema.clone(), kind), {
        let catalog = catalog.clone();
        let schema = schema.clone();
        move || {
            let catalog = catalog.clone();
            let schema = schema.clone();
            async move { kind.entries(&catalog, &schema).await }
        }
    });
    view! {
        <Names
            names=names(entries, |e| e.name.clone())
            empty=format!("No {}", kind.label().to_lowercase())
            node=Callback::new(move |name: String| {
                let href = kind.href(&catalog, &schema, &name);
                view! {
                    <div class="uc-tree-item uc-tree-item--leaf">
                        <div class="uc-tree-row">
                            <LeafLine/>
                            <Label href>
                                {kind.icon()}
                                " "
                                {name}
                            </Label>
                        </div>
                    </div>
                }
                    .into_any()
            })
        />
    }
}

/// A node's link; selected while its page shows, as antd selects the
/// clicked node.
#[component]
fn Label(
    href: String,
    /// Whether the node is open; none for a leaf.
    #[prop(optional)]
    open: Option<RwSignal<bool>>,
    children: Children,
) -> impl IntoView {
    let location = use_location();
    let selected = {
        let href = href.clone();
        move || {
            let search = location.search.get();
            let here = match search.is_empty() {
                true => location.pathname.get(),
                false => format!("{}?{search}", location.pathname.get()),
            };
            here == href
        }
    };
    view! {
        <a
            class="uc-tree-label"
            class:is-selected=selected.clone()
            href=href
            role="treeitem"
            aria-selected=move || selected().to_string()
            aria-expanded=move || open.map(|open| open.get().to_string())
        >
            {children()}
        </a>
    }
}

/// The button that opens and closes a node: antd's switcher with
/// `DownOutlined`, turned while the node is closed. The arrow keys do the
/// same, so Tab skips it.
#[component]
fn Switcher(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <button
            type="button"
            class="uc-tree-switcher"
            class:is-closed=move || !open.get()
            tabindex="-1"
            aria-label=move || if open.get() { "Close" } else { "Open" }
            on:click=move |_| open.update(|open| *open = !*open)
        >
            <DownOutlined/>
        </button>
    }
}

/// A leaf's line, in the place of the switcher.
#[component]
fn LeafLine() -> impl IntoView {
    view! {
        <span class="uc-tree-switcher uc-tree-switcher--leaf" aria-hidden="true">
            <span class="uc-tree-leaf-line"></span>
        </span>
    }
}

/// A node's children: built when the node first opens, then kept and only
/// hidden while it is closed, so that their own open nodes stay open.
#[component]
fn Subtree(open: RwSignal<bool>, children: ChildrenFn) -> impl IntoView {
    let opened = Memo::new(move |before: Option<&bool>| before == Some(&true) || open.get());
    view! {
        <Show when=move || opened.get()>
            <div class="uc-tree-children" role="group" class:uc-hidden=move || !open.get()>
                {children()}
            </div>
        </Show>
    }
}

/// A node per name, kept by name when the list loads again; a leaf with a
/// text while it loads, when it failed and when it is empty.
#[component]
fn Names(
    #[prop(into)] names: Signal<Load<Vec<String>>>,
    #[prop(into)] empty: String,
    node: Callback<String, AnyView>,
) -> impl IntoView {
    let stage = Memo::new(move |_| names.with(Stage::of));
    let is_empty = move || names.with(|names| names.ready().is_some_and(Vec::is_empty));
    let each = move || names.with(|names| names.ready().cloned().unwrap_or_default());
    move || match stage.get() {
        Stage::Loading => placeholder("Loading…".into()),
        Stage::Failed(message) => placeholder(format!("Error: {message}")),
        Stage::Ready => {
            let empty = empty.clone();
            view! {
                <Show when=is_empty>{placeholder(empty.clone())}</Show>
                <For each key=|name| name.clone() children=move |name| node.run(name)/>
            }
            .into_any()
        }
    }
}

fn placeholder(text: String) -> AnyView {
    view! {
        <div class="uc-tree-item uc-tree-item--leaf">
            <div class="uc-tree-row">
                <LeafLine/>
                <span class="uc-tree-placeholder">{text}</span>
            </div>
        </div>
    }
    .into_any()
}

/// The tree's keys, as antd's: up and down move between the labels that
/// show; right opens a node, or goes to its first child; left closes it,
/// or goes to its parent; Home and End go to the first and last label.
fn keys(e: &ev::KeyboardEvent, tree: NodeRef<html::Div>) {
    let Some(tree) = tree.get_untracked() else {
        return;
    };
    let labels = visible_labels(&tree);
    let active = document().active_element();
    let Some(at) = labels
        .iter()
        .position(|label| active.as_ref() == Some(label.unchecked_ref::<Element>()))
    else {
        return;
    };
    let label = &labels[at];
    // The node's switcher, unless it is a leaf.
    let switcher = label
        .previous_element_sibling()
        .and_then(|s| s.dyn_into::<HtmlElement>().ok())
        .filter(|s| s.tag_name() == "BUTTON");
    let is_open = || label.get_attribute("aria-expanded").as_deref() == Some("true");
    let to = match e.key().as_str() {
        "ArrowDown" => at + 1,
        "ArrowUp" => match at.checked_sub(1) {
            Some(to) => to,
            None => return,
        },
        "Home" => 0,
        "End" => labels.len() - 1,
        "ArrowRight" => match &switcher {
            Some(switcher) if !is_open() => {
                e.prevent_default();
                switcher.click();
                return;
            }
            Some(_) => at + 1,
            None => return,
        },
        "ArrowLeft" => match &switcher {
            Some(switcher) if is_open() => {
                e.prevent_default();
                switcher.click();
                return;
            }
            _ => {
                e.prevent_default();
                if let Some(parent) = parent_label(label) {
                    let _ = parent.focus();
                }
                return;
            }
        },
        _ => return,
    };
    e.prevent_default();
    if let Some(label) = labels.get(to) {
        let _ = label.focus();
    }
}

/// The labels of the nodes that show, in order.
fn visible_labels(tree: &web_sys::HtmlDivElement) -> Vec<HtmlElement> {
    let Ok(list) = tree.query_selector_all(".uc-tree-label") else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.item(i)?.dyn_into::<HtmlElement>().ok())
        .filter(|label| label.offset_parent().is_some())
        .collect()
}

/// The label of a node's parent.
fn parent_label(label: &HtmlElement) -> Option<HtmlElement> {
    let children = label.closest(".uc-tree-children").ok()??;
    let row = children.previous_element_sibling()?;
    row.query_selector(".uc-tree-label")
        .ok()??
        .dyn_into::<HtmlElement>()
        .ok()
}
