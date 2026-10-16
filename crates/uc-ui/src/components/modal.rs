//! A modal dialog, as antd's `Modal`: a title, a close button, a body,
//! Cancel and OK. Escape, the close button, Cancel and a click on the
//! backdrop close it; Enter in a field submits it, as the forms in
//! upstream's modals do. As antd's, it takes the focus when it opens, keeps
//! Tab inside, and gives the focus back when it closes. A modal can open
//! over another: Escape closes the top one.

use std::cell::{Cell, RefCell};

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

use super::icons::{CloseOutlined, LoadingOutlined};

/// What takes the focus in a dialog, for Tab.
const FOCUSABLE: &str = "button:not([disabled]), input:not([disabled]), \
    textarea:not([disabled]), select:not([disabled]), a[href], [tabindex]:not([tabindex='-1'])";

thread_local! {
    /// The element that had the focus before each open dialog.
    static FOCUS_BEFORE: RefCell<Vec<Option<Element>>> = const { RefCell::new(Vec::new()) };
    /// How many dialogs are open, one over the other.
    static OPEN: Cell<usize> = const { Cell::new(0) };
}

#[component]
pub fn Modal(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] title: String,
    ok_text: &'static str,
    /// A red title and a red OK button, for deletes.
    #[prop(optional)]
    danger: bool,
    /// A call runs: OK spins and waits for it.
    #[prop(into, optional)]
    busy: Signal<bool>,
    /// OK is off.
    #[prop(into, optional)]
    disabled: Signal<bool>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(into)] on_ok: Callback<()>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        // `Show` keeps its condition in a memo, which can run once more
        // after the page that owns `open` is gone.
        <Show when=move || open.try_get().unwrap_or(false)>
            <Dialog
                title=title.clone()
                ok_text
                danger
                busy
                disabled
                on_cancel
                on_ok
                content=children.clone()
            />
        </Show>
    }
}

/// The open modal. It listens for Escape while it is open.
#[component]
fn Dialog(
    title: String,
    ok_text: &'static str,
    danger: bool,
    busy: Signal<bool>,
    disabled: Signal<bool>,
    on_cancel: Callback<()>,
    on_ok: Callback<()>,
    content: ChildrenFn,
) -> impl IntoView {
    // Its place in the stack of open dialogs: Escape is for the top one.
    let level = OPEN.get();
    OPEN.set(level + 1);
    let escape = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" && OPEN.get() == level + 1 {
            on_cancel.run(());
        }
    });
    on_cleanup(move || {
        escape.remove();
        OPEN.set(OPEN.get().saturating_sub(1));
    });
    let panel = NodeRef::<html::Form>::new();
    FOCUS_BEFORE.with_borrow_mut(|before| before.push(document().active_element()));
    Effect::new(move |_| {
        if let Some(panel) = panel.get() {
            let _ = panel.focus();
        }
    });
    on_cleanup(|| {
        let before = FOCUS_BEFORE.with_borrow_mut(Vec::pop).flatten();
        if let Some(element) = before.and_then(|e| e.dyn_into::<HtmlElement>().ok())
            && element.is_connected()
        {
            let _ = element.focus();
        }
    });
    // A press of the main button on the backdrop closes the dialog when it
    // ends, as a click on antd's mask does. A press in the dialog, such as
    // a text selection that ends on the backdrop, does not. The backdrop
    // has no click handler: Leptos 0.7 drops the handlers of a closed
    // dialog but leaves them on its elements, and the click on Cancel or ×
    // that closes the dialog goes on up to the backdrop.
    let pressed_on_backdrop = StoredValue::new(false);
    let submit = move |e: ev::SubmitEvent| {
        e.prevent_default();
        if !busy.get_untracked() && !disabled.get_untracked() {
            on_ok.run(());
        }
    };
    let (title_class, ok_class) = if danger {
        ("uc-title-4 uc-text-danger", "uc-btn uc-btn--danger")
    } else {
        ("uc-title-4", "uc-btn uc-btn--primary")
    };
    view! {
        <div
            class="uc-modal-backdrop"
            on:mousedown=move |e| {
                pressed_on_backdrop.set_value(e.button() == 0 && e.target() == e.current_target())
            }
            on:mouseup=move |_| {
                if pressed_on_backdrop.get_value() {
                    on_cancel.run(());
                }
            }
        >
            <form
                node_ref=panel
                class="uc-modal"
                role="dialog"
                aria-modal="true"
                tabindex="-1"
                on:submit=submit
                on:keydown=move |e| keep_tab_inside(&e, panel)
            >
                <button
                    type="button"
                    class="uc-modal__close"
                    aria-label="Close"
                    on:click=move |_| on_cancel.run(())
                >
                    <CloseOutlined/>
                </button>
                <div class="uc-modal__header">
                    <h4 class=title_class>{title}</h4>
                </div>
                <div class="uc-modal__body">{content()}</div>
                <div class="uc-modal__footer">
                    <button type="button" class="uc-btn" on:click=move |_| on_cancel.run(())>
                        "Cancel"
                    </button>
                    <button
                        type="submit"
                        class=ok_class
                        class:uc-btn--loading=move || busy.get()
                        disabled=move || disabled.get()
                    >
                        <Show when=move || busy.get()>
                            <span class="uc-spin-icon">
                                <LoadingOutlined/>
                            </span>
                        </Show>
                        <span>{ok_text}</span>
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Tab from the last control goes to the first, and Shift+Tab from the
/// first to the last, as antd's focus trap.
fn keep_tab_inside(e: &ev::KeyboardEvent, panel: NodeRef<html::Form>) {
    if e.key() != "Tab" {
        return;
    }
    let Some(panel) = panel.get_untracked() else {
        return;
    };
    let Ok(list) = panel.query_selector_all(FOCUSABLE) else {
        return;
    };
    let controls: Vec<HtmlElement> = (0..list.length())
        .filter_map(|i| list.item(i)?.dyn_into::<HtmlElement>().ok())
        .collect();
    let (Some(first), Some(last)) = (controls.first(), controls.last()) else {
        return;
    };
    let active = document().active_element();
    let on = |element: &HtmlElement| active.as_ref() == Some(element.unchecked_ref::<Element>());
    let on_panel = active.as_ref() == Some(panel.unchecked_ref::<Element>());
    let to = match e.shift_key() {
        true if on(first) || on_panel => last,
        false if on(last) => first,
        _ => return,
    };
    e.prevent_default();
    let _ = to.focus();
}
