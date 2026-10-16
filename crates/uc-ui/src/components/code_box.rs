use leptos::prelude::*;

#[component]
pub fn CodeBox(#[prop(into)] definition: Signal<String>) -> impl IntoView {
    view! {
        <div>
            <h5 class="uc-title-5">"Definition"</h5>
            <code class="uc-codebox">
                {move || definition.get().replace("\\n", "\n")}
            </code>
        </div>
    }
}
