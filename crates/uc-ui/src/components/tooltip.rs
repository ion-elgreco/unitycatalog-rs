//! A tooltip, as antd's `Tooltip`: a dark bubble above its child while the
//! pointer is on it, after antd's 0.1 s.

use leptos::prelude::*;

#[component]
pub fn Tooltip(text: &'static str, children: Children) -> impl IntoView {
    view! {
        <span class="uc-tooltip">
            {children()}
            <span class="uc-tooltip__bubble" role="tooltip">
                {text}
            </span>
        </span>
    }
}
