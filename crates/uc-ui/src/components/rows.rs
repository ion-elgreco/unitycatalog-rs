//! A row in a list of objects, as the console's lists show them: the icon
//! and the name, with the comment after a dash where the list has one, as
//! a link to the object's page.

use leptos::prelude::*;

#[component]
pub fn ObjectRow(
    icon: AnyView,
    name: String,
    href: String,
    #[prop(optional_no_strip)] comment: Option<String>,
) -> impl IntoView {
    view! {
        <a class="uc-row" href=href>
            {icon}
            <span class="uc-row__name">{name}</span>
            {comment
                .filter(|c| !c.is_empty())
                .map(|c| view! { <span class="uc-row__comment">{format!("— {c}")}</span> })}
        </a>
    }
}
