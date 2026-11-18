//! The mark of an object that the user sees through the `BROWSE` privilege
//! only: its name, comment and other metadata, but not its data. Its page
//! has no Files tab, no temporary credentials and no buttons that change
//! it.

use leptos::prelude::*;

/// "Browse only", with what it means in its title.
#[component]
pub fn BrowseOnlyBadge() -> impl IntoView {
    view! {
        <span
            class="uc-badge uc-badge--outline"
            title="You see this object through the BROWSE privilege: its metadata, not its data."
        >
            "Browse only"
        </span>
    }
}
