//! The sidebar tree, as the console's `CatalogTree`: catalogs, their
//! schemas, and under each schema its tables, volumes, functions and
//! models, in groups with their counts. A node loads its children when it
//! first opens; they load again when a create or delete changes them. The
//! page shown is selected, and its nodes open. The arrow keys move between
//! the labels, and open and close the nodes.

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

use super::icons::{ChevronDown, ChevronRight};
use super::loaded::Stage;
use crate::api;
use crate::kind::{Kind, catalog_icon, schema_icon};
use crate::load::{Key, Load, use_load};
use crate::util::{Route, catalog_href, schema_href, use_route};

#[component]
pub fn CatalogTree() -> impl IntoView {
    let catalogs = use_load(Key::Catalogs, api::catalogs);
    let names = names(catalogs, |c| c.name.clone());
    let route = use_route();
    let tree = NodeRef::<html::Div>::new();
    view! {
        <div
            node_ref=tree
            class="uc-tree"
            role="tree"
            aria-label="Catalogs"
            on:keydown=move |e| keys(&e, tree)
        >
            <ul>
                <Names
                    names
                    empty="No catalogs."
                    node=Callback::new(move |name| view! { <CatalogNode name route/> }.into_any())
                />
            </ul>
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
fn CatalogNode(name: String, route: Memo<Route>) -> impl IntoView {
    let here = {
        let name = name.clone();
        Memo::new(move |_| route.with(|r| r.in_catalog(&name)))
    };
    let active = Signal::derive(move || here.get() && route.with(|r| r.schema.is_none()));
    let catalog = name.clone();
    view! {
        <Branch here active href=catalog_href(&name) icon=catalog_icon() label=name.clone()>
            <SchemaNodes catalog=catalog.clone() route/>
        </Branch>
    }
}

#[component]
fn SchemaNodes(catalog: String, route: Memo<Route>) -> impl IntoView {
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
            empty="No schemas."
            node=Callback::new(move |schema| {
                view! { <SchemaNode catalog=catalog.clone() schema route/> }.into_any()
            })
        />
    }
}

#[component]
fn SchemaNode(catalog: String, schema: String, route: Memo<Route>) -> impl IntoView {
    let here = {
        let catalog = catalog.clone();
        let schema = schema.clone();
        Memo::new(move |_| route.with(|r| r.in_schema(&catalog, &schema)))
    };
    let active = Signal::derive(move || here.get() && route.with(|r| r.object.is_none()));
    let href = schema_href(&catalog, &schema);
    let label = schema.clone();
    view! {
        <Branch here active href icon=schema_icon() label small=true>
            {Kind::ALL
                .into_iter()
                .map(|kind| {
                    view! { <GroupNode kind catalog=catalog.clone() schema=schema.clone() route/> }
                })
                .collect_view()}
        </Branch>
    }
}

/// A catalog or schema node: its label links to its page, and it opens
/// when the page shown is in it.
#[component]
fn Branch(
    /// Whether the page shown is in the node.
    here: Memo<bool>,
    /// Whether the page shown is the node's own.
    active: Signal<bool>,
    href: String,
    icon: AnyView,
    label: String,
    /// In the smaller font of the nodes under a catalog.
    #[prop(optional)]
    small: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let open = opens_with(here, false);
    let class = if small {
        "uc-tree__label uc-tree__label--sm"
    } else {
        "uc-tree__label"
    };
    view! {
        <li role="none">
            <div class="uc-tree__row" class:is-active=move || active.get()>
                <Chevron open/>
                <a
                    class=class
                    class:is-active=move || active.get()
                    href=href
                    role="treeitem"
                    aria-selected=move || aria(active.get())
                    aria-expanded=move || aria(open.get())
                    on:click=move |_| open.set(true)
                >
                    {icon}
                    <span class="uc-truncate">{label}</span>
                </a>
            </div>
            <Subtree open>{children()}</Subtree>
        </li>
    }
}

/// "Tables (5)", "Volumes", "Functions" or "Models" under a schema. Tables
/// open at once, as in the console; the count shows once the list has
/// loaded.
#[component]
fn GroupNode(kind: Kind, catalog: String, schema: String, route: Memo<Route>) -> impl IntoView {
    let here = {
        let catalog = catalog.clone();
        let schema = schema.clone();
        Memo::new(move |_| {
            route.with(|r| {
                r.in_schema(&catalog, &schema) && r.object.as_ref().is_some_and(|(k, _)| *k == kind)
            })
        })
    };
    let open = opens_with(here, kind == Kind::Tables);
    let count: RwSignal<Option<usize>> = RwSignal::new(None);
    let title = move || match count.get() {
        Some(n) => format!("{} ({n})", kind.label()),
        None => kind.label().to_string(),
    };
    view! {
        <li role="none">
            <div class="uc-tree__row">
                <Chevron open/>
                <button
                    type="button"
                    class="uc-tree__label uc-tree__label--sm uc-tree__label--group"
                    role="treeitem"
                    aria-expanded=move || aria(open.get())
                    on:click=move |_| open.update(|open| *open = !*open)
                >
                    {kind.icon()}
                    <span class="uc-truncate">{title}</span>
                </button>
            </div>
            <Subtree open>
                <EntryNodes kind catalog=catalog.clone() schema=schema.clone() route count/>
            </Subtree>
        </li>
    }
}

#[component]
fn EntryNodes(
    kind: Kind,
    catalog: String,
    schema: String,
    route: Memo<Route>,
    count: RwSignal<Option<usize>>,
) -> impl IntoView {
    let entries = use_load(Key::Entries(catalog.clone(), schema.clone(), kind), {
        let catalog = catalog.clone();
        let schema = schema.clone();
        move || {
            let catalog = catalog.clone();
            let schema = schema.clone();
            async move { kind.entries(&catalog, &schema).await }
        }
    });
    Effect::new(move |_| {
        let n = entries.with(|e| e.ready().map(Vec::len));
        if n.is_some() {
            count.set(n);
        }
    });
    // The object of this group whose page shows, if any.
    let selected = {
        let catalog = catalog.clone();
        let schema = schema.clone();
        Memo::new(move |_| {
            route.with(|r| match &r.object {
                Some((k, name)) if *k == kind && r.in_schema(&catalog, &schema) => {
                    Some(name.clone())
                }
                _ => None,
            })
        })
    };
    view! {
        <Names
            names=names(entries, |e| e.name.clone())
            empty="None"
            node=Callback::new(move |name: String| {
                let href = kind.href(&catalog, &schema, &name);
                let active = {
                    let name = name.clone();
                    Signal::derive(move || selected.with(|s| s.as_deref() == Some(name.as_str())))
                };
                view! {
                    <li role="none">
                        <div class="uc-tree__row" class:is-active=move || active.get()>
                            <a
                                class="uc-tree__label uc-tree__label--sm uc-tree__label--leaf"
                                class:is-active=move || active.get()
                                href=href
                                role="treeitem"
                                aria-selected=move || aria(active.get())
                            >
                                {kind.icon()}
                                <span class="uc-truncate">{name}</span>
                            </a>
                        </div>
                    </li>
                }
                    .into_any()
            })
        />
    }
}

/// Whether a node is open: at first when `at_first` or when the page shown
/// is in it (`here`), and again whenever the page shown comes into it.
fn opens_with(here: Memo<bool>, at_first: bool) -> RwSignal<bool> {
    let open = RwSignal::new(at_first || here.get_untracked());
    Effect::new(move |_| {
        if here.get() {
            open.set(true);
        }
    });
    open
}

/// An ARIA state, as its attribute's text.
fn aria(state: bool) -> &'static str {
    if state { "true" } else { "false" }
}

/// The button that opens and closes a node. The arrow keys do the same,
/// so Tab skips it.
#[component]
fn Chevron(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <button
            type="button"
            class="uc-tree__chevron"
            tabindex="-1"
            aria-label=move || if open.get() { "Collapse" } else { "Expand" }
            on:click=move |_| open.update(|open| *open = !*open)
        >
            <Show when=move || open.get() fallback=|| view! { <ChevronRight/> }>
                <ChevronDown/>
            </Show>
        </button>
    }
}

/// A node's children: built when the node first opens, then kept and only
/// hidden while it is closed, so that their own open nodes stay open.
#[component]
fn Subtree(open: RwSignal<bool>, children: ChildrenFn) -> impl IntoView {
    let opened = Memo::new(move |before: Option<&bool>| before == Some(&true) || open.get());
    view! {
        <Show when=move || opened.get()>
            <ul class="uc-tree__children" role="group" class:uc-hidden=move || !open.get()>
                {children()}
            </ul>
        </Show>
    }
}

/// A node per name, kept by name when the list loads again; a note while
/// it loads, when it failed and when it is empty.
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
        Stage::Loading => note("Loading…".into(), "uc-tree__note"),
        Stage::Failed(message) => note(message, "uc-tree__note uc-tree__note--error"),
        Stage::Ready => {
            let empty = empty.clone();
            view! {
                <Show when=is_empty>{note(empty.clone(), "uc-tree__note")}</Show>
                <For each key=|name| name.clone() children=move |name| node.run(name)/>
            }
            .into_any()
        }
    }
}

fn note(text: String, class: &'static str) -> AnyView {
    view! { <li class=class role="none">{text}</li> }.into_any()
}

/// The tree's keys: up and down move between the labels that show; right
/// opens a node, or goes to its first child; left closes it, or goes to
/// its parent; Home and End go to the first and last label.
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
    // The node's chevron, unless it is a leaf.
    let chevron = label
        .previous_element_sibling()
        .and_then(|s| s.dyn_into::<HtmlElement>().ok())
        .filter(|s| s.class_list().contains("uc-tree__chevron"));
    let is_open = || label.get_attribute("aria-expanded").as_deref() == Some("true");
    let to = match e.key().as_str() {
        "ArrowDown" => at + 1,
        "ArrowUp" => match at.checked_sub(1) {
            Some(to) => to,
            None => return,
        },
        "Home" => 0,
        "End" => labels.len() - 1,
        "ArrowRight" => match &chevron {
            Some(chevron) if !is_open() => {
                e.prevent_default();
                chevron.click();
                return;
            }
            Some(_) => at + 1,
            None => return,
        },
        "ArrowLeft" => match &chevron {
            Some(chevron) if is_open() => {
                e.prevent_default();
                chevron.click();
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
    let Ok(list) = tree.query_selector_all(".uc-tree__label") else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.item(i)?.dyn_into::<HtmlElement>().ok())
        .filter(|label| label.offset_parent().is_some())
        .collect()
}

/// The label of a node's parent.
fn parent_label(label: &HtmlElement) -> Option<HtmlElement> {
    let children = label.closest(".uc-tree__children").ok()??;
    let row = children.previous_element_sibling()?;
    row.query_selector(".uc-tree__label")
        .ok()??
        .dyn_into::<HtmlElement>()
        .ok()
}
