//! A click menu, as the console's `DropdownMenu`: the account menu. A
//! click outside the menu, Escape, Tab and a change of page close it. The
//! open menu takes the focus, and the arrow keys move it between the
//! items.

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[component]
pub fn Dropdown(
    /// The class of the button that opens the menu.
    button_class: &'static str,
    #[prop(optional)] title: &'static str,
    /// What the button shows.
    button: AnyView,
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
        <div class="uc-menu">
            <button
                node_ref=trigger
                type="button"
                class=button_class
                title=title
                aria-label=title
                aria-haspopup="menu"
                aria-expanded=move || open.try_get().unwrap_or(false).to_string()
                on:click=move |_| open.update(|open| *open = !*open)
            >
                {button}
            </button>
            // `Show` keeps its condition in a memo, which can run once more
            // after the page that owns `open` is gone.
            <Show when=move || open.try_get().unwrap_or(false)>
                <Menu open trigger content=children.clone()/>
            </Show>
        </div>
    }
}

/// The open menu, below the button and to its right edge, over a backdrop
/// that takes the clicks outside it. It listens for Escape while it is
/// open.
#[component]
fn Menu(
    open: RwSignal<bool>,
    trigger: NodeRef<html::Button>,
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
    // The focus on the first item.
    Effect::new(move |_| {
        if let Some(first) = menu.get().and_then(|menu| items(&menu).into_iter().next()) {
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
    view! {
        <div class="uc-menu__backdrop" on:click=move |_| open.set(false)></div>
        <div
            node_ref=menu
            class="uc-menu__panel"
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
