use leptos::prelude::*;

/// antd's `Spin`: four dots that turn.
#[component]
pub fn LoadingBlock() -> impl IntoView {
    view! {
        <div class="uc-loading-block">
            <span class="uc-spin-dot" role="img" aria-label="Loading">
                <i></i>
                <i></i>
                <i></i>
                <i></i>
            </span>
        </div>
    }
}
