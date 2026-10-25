use leptos::prelude::*;

/// Code in a block: a function's definition, on the console's dark
/// background.
#[component]
pub fn CodeBlock(#[prop(into)] text: Signal<String>) -> impl IntoView {
    view! { <pre class="uc-code uc-code--dark">{move || text.get().replace("\\n", "\n")}</pre> }
}
