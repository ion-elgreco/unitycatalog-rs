//! The tabs of a detail page, as the console's: Overview, Details and
//! Permissions, underlined in the primary color.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Details,
    Permissions,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Overview, Tab::Details, Tab::Permissions];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Details => "Details",
            Tab::Permissions => "Permissions",
        }
    }
}

#[component]
pub fn Tabs(tab: RwSignal<Tab>) -> impl IntoView {
    view! {
        <div class="uc-tabs" role="tablist">
            {Tab::ALL
                .into_iter()
                .map(|this| {
                    view! {
                        <button
                            type="button"
                            role="tab"
                            aria-selected=move || (tab.get() == this).to_string()
                            class:is-active=move || tab.get() == this
                            on:click=move |_| tab.set(this)
                        >
                            {this.label()}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}
