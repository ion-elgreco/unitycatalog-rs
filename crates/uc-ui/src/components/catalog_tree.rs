//! The sidebar tree, as the console's `CatalogTree`: catalogs, their
//! schemas, and under each schema its tables, volumes, functions and
//! models, in groups with their counts. A node loads its children when it
//! first opens; they load again when a create or delete changes them. The
//! page shown is selected, and its nodes open. The arrow keys move between
//! the labels, and open and close the nodes.

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

use super::icons::{ChevronDown, ChevronRight, Database, Notebook};
use super::loaded::Stage;
use crate::api;
use crate::kind::{Kind, catalog_href, schema_href};
use crate::load::{Key, Load, use_load};

/// What the URL names: the catalog, the schema, an object of a kind, and
/// a model's version; or a page of the user's own, such as the tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Route {
    pub catalog: Option<String>,
    pub schema: Option<String>,
    pub object: Option<(Kind, String)>,
    pub version: Option<String>,
    /// The title of a page outside the catalogs.
    pub page: Option<&'static str>,
}

impl Route {
    fn parse(path: &str) -> Self {
        let mut parts = path.trim_start_matches('/').split('/').map(decode);
        match parts.next().as_deref() {
            Some("catalog") => {}
            Some("settings") => {
                let page = (parts.next().as_deref() == Some("tokens")).then_some("Access tokens");
                return Self {
                    page,
                    ..Self::default()
                };
            }
            _ => return Self::default(),
        }
        let catalog = parts.next().filter(|s| !s.is_empty());
        let schema = parts.next().filter(|s| !s.is_empty());
        let kind = parts.next().and_then(|segment| match segment.as_str() {
            "table" => Some(Kind::Tables),
            "volume" => Some(Kind::Volumes),
            "function" => Some(Kind::Functions),
            "model" => Some(Kind::Models),
            _ => None,
        });
        let name = parts.next().filter(|s| !s.is_empty());
        let version = (parts.next().as_deref() == Some("version"))
            .then(|| parts.next().filter(|s| !s.is_empty()))
            .flatten();
        Self {
            catalog,
            schema,
            object: kind.zip(name),
            version,
            page: None,
        }
    }

    fn in_catalog(&self, catalog: &str) -> bool {
        self.catalog.as_deref() == Some(catalog)
    }

    fn in_schema(&self, catalog: &str, schema: &str) -> bool {
        self.in_catalog(catalog) && self.schema.as_deref() == Some(schema)
    }
}

/// A part of the URL, as the name it stands for: its `%XX` escapes
/// decoded.
fn decode(part: &str) -> String {
    let bytes = part.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%' && i + 2 < bytes.len())
            .then(|| u8::from_str_radix(&part[i + 1..i + 3], 16).ok())
            .flatten();
        match escaped {
            Some(byte) => {
                out.push(byte);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What the page shows, from the URL.
pub fn use_route() -> Memo<Route> {
    let pathname = use_location().pathname;
    Memo::new(move |_| pathname.with(|path| Route::parse(path)))
}

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
    name: fn(&T) -> Option<String>,
) -> Memo<Load<Vec<String>>> {
    Memo::new(move |_| {
        list.with(|list| match list {
            Load::Loading => Load::Loading,
            Load::Ready(items) => Load::Ready(items.iter().filter_map(name).collect()),
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
    let open = RwSignal::new(here.get_untracked());
    Effect::new(move |_| {
        if here.get() {
            open.set(true);
        }
    });
    let active = move || here.get() && route.with(|r| r.schema.is_none());
    let href = catalog_href(&name);
    let label = name.clone();
    view! {
        <li role="none">
            <div class="uc-tree__row" class:is-active=active>
                <Chevron open/>
                <a
                    class="uc-tree__label"
                    class:is-active=active
                    href=href
                    role="treeitem"
                    aria-selected=move || active().to_string()
                    aria-expanded=move || open.get().to_string()
                    on:click=move |_| open.set(true)
                >
                    <Notebook class="uc-icon--catalog"/>
                    <span class="uc-truncate">{label}</span>
                </a>
            </div>
            <Subtree open>
                <SchemaNodes catalog=name.clone() route/>
            </Subtree>
        </li>
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
    let open = RwSignal::new(here.get_untracked());
    Effect::new(move |_| {
        if here.get() {
            open.set(true);
        }
    });
    let active = move || here.get() && route.with(|r| r.object.is_none());
    let href = schema_href(&catalog, &schema);
    let label = schema.clone();
    view! {
        <li role="none">
            <div class="uc-tree__row" class:is-active=active>
                <Chevron open/>
                <a
                    class="uc-tree__label uc-tree__label--sm"
                    class:is-active=active
                    href=href
                    role="treeitem"
                    aria-selected=move || active().to_string()
                    aria-expanded=move || open.get().to_string()
                    on:click=move |_| open.set(true)
                >
                    <Database class="uc-icon--schema"/>
                    <span class="uc-truncate">{label}</span>
                </a>
            </div>
            <Subtree open>
                {Kind::ALL
                    .into_iter()
                    .map(|kind| {
                        view! { <GroupNode kind catalog=catalog.clone() schema=schema.clone() route/> }
                    })
                    .collect_view()}
            </Subtree>
        </li>
    }
}

/// "Tables (5)", "Volumes", "Functions" or "Models" under a schema. Tables
/// open at once, as in the console; the count shows once the list has
/// loaded.
#[component]
fn GroupNode(kind: Kind, catalog: String, schema: String, route: Memo<Route>) -> impl IntoView {
    let route_open = {
        let catalog = catalog.clone();
        let schema = schema.clone();
        Memo::new(move |_| {
            route.with(|r| {
                r.in_schema(&catalog, &schema) && r.object.as_ref().is_some_and(|(k, _)| *k == kind)
            })
        })
    };
    let open = RwSignal::new(kind == Kind::Tables || route_open.get_untracked());
    Effect::new(move |_| {
        if route_open.get() {
            open.set(true);
        }
    });
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
                    aria-expanded=move || open.get().to_string()
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
    let in_schema = {
        let catalog = catalog.clone();
        let schema = schema.clone();
        move |r: &Route| r.in_schema(&catalog, &schema)
    };
    view! {
        <Names
            names=names(entries, |e| Some(e.name.clone()))
            empty="None"
            node=Callback::new(move |name: String| {
                let href = kind.href(&catalog, &schema, &name);
                let in_schema = in_schema.clone();
                let active = {
                    let name = name.clone();
                    Memo::new(move |_| {
                        route
                            .with(|r| {
                                in_schema(r)
                                    && r.object.as_ref().is_some_and(|(k, n)| *k == kind && *n == name)
                            })
                    })
                };
                view! {
                    <li role="none">
                        <div class="uc-tree__row" class:is-active=move || active.get()>
                            <a
                                class="uc-tree__label uc-tree__label--sm uc-tree__label--leaf"
                                class:is-active=move || active.get()
                                href=href
                                role="treeitem"
                                aria-selected=move || active.get().to_string()
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
        Stage::Loading => note("Loading…".into(), false),
        Stage::Failed(message) => note(message, true),
        Stage::Ready => {
            let empty = empty.clone();
            view! {
                <Show when=is_empty>{note(empty.clone(), false)}</Show>
                <For each key=|name| name.clone() children=move |name| node.run(name)/>
            }
            .into_any()
        }
    }
}

fn note(text: String, error: bool) -> AnyView {
    let class = if error {
        "uc-tree__note uc-tree__note--error"
    } else {
        "uc-tree__note"
    };
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

#[cfg(test)]
mod tests {
    use super::Route;
    use crate::kind::Kind;

    #[test]
    fn routes_name_the_objects() {
        assert_eq!(Route::parse("/"), Route::default());
        assert_eq!(Route::parse("/settings/other"), Route::default());
        let route = Route::parse("/catalog/unity");
        assert_eq!(route.catalog.as_deref(), Some("unity"));
        assert_eq!(route.schema, None);
        let route = Route::parse("/catalog/unity/default/table/marksheet");
        assert_eq!(route.schema.as_deref(), Some("default"));
        assert_eq!(route.object, Some((Kind::Tables, "marksheet".into())));
        let route = Route::parse("/catalog/unity/default/model/iris/version/2");
        assert_eq!(route.object, Some((Kind::Models, "iris".into())));
        assert_eq!(route.version.as_deref(), Some("2"));
        let route = Route::parse("/catalog/unity/default/view/x");
        assert_eq!(route.object, None);
        let route = Route::parse("/settings/tokens");
        assert_eq!(route.page, Some("Access tokens"));
        assert_eq!(route.catalog, None);
        let route = Route::parse("/catalog/caf%C3%A9%20bar/a%2Fb");
        assert_eq!(route.catalog.as_deref(), Some("café bar"));
        assert_eq!(route.schema.as_deref(), Some("a/b"));
    }
}
