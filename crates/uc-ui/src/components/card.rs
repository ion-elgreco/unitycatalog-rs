//! The console's `Card`: a title row, with an icon, a subtitle and actions
//! at the right, over a body.

use leptos::prelude::*;

#[component]
pub fn Card(
    #[prop(into, optional)] title: Option<String>,
    #[prop(optional)] icon: Option<AnyView>,
    /// Beside the title, in gray: such as "table · main.default.orders".
    #[prop(into, optional)]
    subtitle: Option<String>,
    /// At the right of the title row: buttons.
    #[prop(optional)]
    actions: Option<AnyView>,
    children: Children,
) -> impl IntoView {
    let header = (title.is_some() || actions.is_some()).then(|| {
        view! {
            <div class="uc-card__header">
                <h3 class="uc-card__title">
                    {icon}
                    {title}
                    {subtitle.map(|text| view! { <span class="uc-card__subtitle">{text}</span> })}
                </h3>
                {actions.map(|actions| view! { <div class="uc-card__actions">{actions}</div> })}
            </div>
        }
    });
    view! {
        <section class="uc-card">
            {header}
            <div class="uc-card__body">{children()}</div>
        </section>
    }
}
