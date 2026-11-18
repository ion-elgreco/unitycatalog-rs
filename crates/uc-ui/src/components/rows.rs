//! A row in a list of objects, as the console's lists show them: the icon
//! and the name, with the comment after a dash where the list has one, as
//! a link to the object's page, and "Browse only" for an object that the
//! user sees through `BROWSE` only.

use leptos::prelude::*;

use super::browse_only::BrowseOnlyBadge;

#[component]
pub fn ObjectRow(
    icon: AnyView,
    name: String,
    href: String,
    #[prop(optional_no_strip)] comment: Option<String>,
    #[prop(optional)] browse_only: bool,
) -> impl IntoView {
    view! {
        <a class="uc-row" href=href>
            {icon}
            <span class="uc-row__name">{name}</span>
            {browse_only.then(|| view! { <BrowseOnlyBadge/> })}
            {comment
                .filter(|c| !c.is_empty())
                .map(|c| view! { <span class="uc-row__comment">{format!("— {c}")}</span> })}
        </a>
    }
}
