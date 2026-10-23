//! The console's `Card`: a title row, with an icon, a subtitle and actions
//! at the right, over a body.

use leptos::prelude::*;
use leptos::text_prop::TextProp;

#[component]
pub fn Card(
    /// The title: a text, or a closure for one that changes, such as a
    /// count.
    #[prop(into, optional)]
    title: Option<TextProp>,
    #[prop(optional)] icon: Option<AnyView>,
    /// Beside the title, in gray: such as "table · main.default.orders".
    #[prop(into, optional)]
    subtitle: Option<String>,
    /// In the title row: buttons, at the right of a title, else at the
    /// left.
    #[prop(optional)]
    actions: Option<AnyView>,
    children: Children,
) -> impl IntoView {
    let heading = (title.is_some() || icon.is_some()).then(|| {
        view! {
            <h3 class="uc-card__title">
                {icon}
                {title.map(|title| move || title.get())}
                {subtitle.map(|text| view! { <span class="uc-card__subtitle">{text}</span> })}
            </h3>
        }
    });
    let header = (heading.is_some() || actions.is_some()).then(|| {
        view! {
            <div class="uc-card__header">
                {heading}
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
