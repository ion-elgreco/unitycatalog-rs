//! Click menus, as antd's `Dropdown` with `trigger={['click']}`: the
//! three-dot action menu of the detail pages and the avatar menu. A click
//! outside the menu, Escape, Tab and a change of page close it. The open
//! menu takes the focus, and the arrow keys move it between the items.

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use super::icons::{DeleteOutlined, DeploymentUnitOutlined, MoreOutlined};

/// Where the menu opens, as antd's `placement`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Below the button, from its left edge; moved left when the window
    /// is too narrow, as antd does.
    BottomLeft,
    /// Below the button, to its right edge.
    BottomRight,
}

#[component]
pub fn Dropdown(
    /// The class of the button that opens the menu.
    button_class: &'static str,
    #[prop(optional)] title: &'static str,
    /// What the button shows.
    button: AnyView,
    #[prop(default = Placement::BottomLeft)] placement: Placement,
    /// The menu's items; a click on one closes the menu.
    children: ChildrenFn,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let trigger = NodeRef::<html::Button>::new();
    // A change of page closes the menu. The same change can remove this
    // dropdown, and `open` with it: a read of a removed signal panics.
    let pathname = use_location().pathname;
    Effect::new(move |previous: Option<String>| {
        let path = pathname.get();
        if previous.is_some_and(|previous| previous != path)
            && open.try_get_untracked() == Some(true)
        {
            open.set(false);
        }
        path
    });
    view! {
        <div class="uc-dropdown" class:is-open=move || open.try_get().unwrap_or(false)>
            <button
                node_ref=trigger
                type="button"
                class=button_class
                title=title
                aria-haspopup="menu"
                aria-expanded=move || open.try_get().unwrap_or(false).to_string()
                on:click=move |_| open.update(|open| *open = !*open)
            >
                {button}
            </button>
            // `Show` keeps its condition in a memo, which can run once more
            // after the page that owns `open` is gone.
            <Show when=move || open.try_get().unwrap_or(false)>
                <Menu open trigger placement content=children.clone()/>
            </Show>
        </div>
    }
}

/// The open menu, over a backdrop that takes the clicks outside it. It
/// listens for Escape while it is open.
#[component]
fn Menu(
    open: RwSignal<bool>,
    trigger: NodeRef<html::Button>,
    placement: Placement,
    content: ChildrenFn,
) -> impl IntoView {
    let escape = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            open.set(false);
            if let Some(trigger) = trigger.get_untracked() {
                let _ = trigger.focus();
            }
        }
    });
    on_cleanup(move || escape.remove());
    let menu = NodeRef::<html::Div>::new();
    // Into the window, then the focus on the first item.
    Effect::new(move |_| {
        let Some(menu) = menu.get() else {
            return;
        };
        if placement == Placement::BottomLeft {
            let width = window()
                .inner_width()
                .ok()
                .and_then(|w| w.as_f64())
                .unwrap_or_default();
            let over = menu.get_bounding_client_rect().right() - (width - 8.0);
            if over > 0.0 {
                let _ = menu.set_attribute("style", &format!("left: -{over}px"));
            }
        }
        if let Some(first) = items(&menu).first() {
            let _ = first.focus();
        }
    });
    let keys = move |e: ev::KeyboardEvent| {
        let Some(menu) = menu.get_untracked() else {
            return;
        };
        let items = items(&menu);
        let active = document().active_element();
        let at = items
            .iter()
            .position(|item| active.as_ref() == Some(item.unchecked_ref::<web_sys::Element>()));
        let last = items.len().saturating_sub(1);
        let to = match e.key().as_str() {
            "ArrowDown" => at.map_or(0, |at| if at == last { 0 } else { at + 1 }),
            "ArrowUp" => at.map_or(last, |at| if at == 0 { last } else { at - 1 }),
            "Home" => 0,
            "End" => last,
            // Back to the button, which the next Tab leaves.
            "Tab" => {
                e.prevent_default();
                open.set(false);
                if let Some(trigger) = trigger.get_untracked() {
                    let _ = trigger.focus();
                }
                return;
            }
            _ => return,
        };
        e.prevent_default();
        if let Some(item) = items.get(to) {
            let _ = item.focus();
        }
    };
    let class = match placement {
        Placement::BottomLeft => "uc-dropdown__menu",
        Placement::BottomRight => "uc-dropdown__menu uc-dropdown__menu--right",
    };
    view! {
        <div class="uc-dropdown__backdrop" on:click=move |_| open.set(false)></div>
        <div
            node_ref=menu
            class=class
            role="menu"
            on:click=move |_| open.set(false)
            on:keydown=keys
        >
            {content()}
        </div>
    }
}

/// The menu's items, in order.
fn items(menu: &web_sys::HtmlDivElement) -> Vec<HtmlElement> {
    let Ok(list) = menu.query_selector_all("[role=menuitem]") else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.item(i)?.dyn_into::<HtmlElement>().ok())
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub enum MenuIcon {
    /// A red item.
    Delete,
    Model,
}

#[derive(Clone)]
pub struct MenuItem {
    pub label: &'static str,
    pub icon: MenuIcon,
    pub on_click: Callback<()>,
}

/// The three-dot menu of a detail page, as upstream's `*ActionsDropdown`:
/// the dots turn while it is open.
#[component]
pub fn ActionDropdown(items: Vec<MenuItem>) -> impl IntoView {
    view! {
        <Dropdown
            button_class="uc-btn uc-btn--text uc-btn--icon"
            title="Actions"
            button=view! {
                <span class="uc-more">
                    <MoreOutlined/>
                </span>
            }
                .into_any()
        >
            {items
                .iter()
                .map(|item| {
                    let on_click = item.on_click;
                    let (class, icon) = match item.icon {
                        MenuIcon::Delete => (
                            "uc-dropdown__item uc-dropdown__item--danger",
                            view! { <DeleteOutlined/> }.into_any(),
                        ),
                        MenuIcon::Model => {
                            ("uc-dropdown__item", view! { <DeploymentUnitOutlined/> }.into_any())
                        }
                    };
                    view! {
                        <button
                            type="button"
                            class=class
                            role="menuitem"
                            on:click=move |_| on_click.run(())
                        >
                            <span class="uc-dropdown__icon">{icon}</span>
                            <span>{item.label}</span>
                        </button>
                    }
                })
                .collect_view()}
        </Dropdown>
    }
}
