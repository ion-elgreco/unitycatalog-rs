//! The tabs of a detail page, as the console's: Overview, Details and
//! Permissions, underlined in the primary color; volumes and tables also
//! have Files. A tab's panel is built when the tab first shows, then kept
//! and only hidden, so that what it loaded and holds, such as vended
//! credentials, stays.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Overview,
    Files,
    Details,
    Permissions,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Files => "Files",
            Tab::Details => "Details",
            Tab::Permissions => "Permissions",
        }
    }
}

#[component]
pub fn Tabs(
    #[prop(into)] overview: ViewFn,
    /// The Files tab, after Overview; none without it.
    #[prop(into, optional)]
    files: Option<ViewFn>,
    #[prop(into)] details: ViewFn,
    #[prop(into)] permissions: ViewFn,
) -> impl IntoView {
    let tab = RwSignal::new(Tab::Overview);
    let panels: Vec<(Tab, ViewFn)> = [
        Some((Tab::Overview, overview)),
        files.map(|files| (Tab::Files, files)),
        Some((Tab::Details, details)),
        Some((Tab::Permissions, permissions)),
    ]
    .into_iter()
    .flatten()
    .collect();
    view! {
        <div class="uc-tabs" role="tablist">
            {panels
                .iter()
                .map(|&(this, _)| {
                    view! {
                        <button
                            type="button"
                            role="tab"
                            aria-selected=move || if tab.get() == this { "true" } else { "false" }
                            class:is-active=move || tab.get() == this
                            on:click=move |_| tab.set(this)
                        >
                            {this.label()}
                        </button>
                    }
                })
                .collect_view()}
        </div>
        {panels
            .into_iter()
            .map(|(this, panel)| view! { <Panel tab this panel/> })
            .collect_view()}
    }
}

/// A tab's panel, from the first time its tab shows.
#[component]
fn Panel(tab: RwSignal<Tab>, this: Tab, panel: ViewFn) -> impl IntoView {
    let shown = Memo::new(move |before: Option<&bool>| before == Some(&true) || tab.get() == this);
    view! {
        <Show when=move || shown.get()>
            <div class="uc-tab-panel" role="tabpanel" class:uc-hidden=move || tab.get() != this>
                {panel.run()}
            </div>
        </Show>
    }
}
