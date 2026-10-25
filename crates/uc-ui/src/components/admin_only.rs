//! The pages that the server keeps for admins: the users, the groups and
//! the audit log. Others see why the page is empty.

use leptos::prelude::*;

use crate::auth::use_auth;

#[component]
pub fn AdminOnly(
    /// The page's title, over the note for the others.
    title: &'static str,
    /// The page takes a signed-in admin, even with authorization off: the
    /// audit log.
    #[prop(optional)]
    signed_in: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let auth = use_auth();
    let allowed = Memo::new(move |_| {
        auth.session.with(|session| match signed_in {
            true => session.user().is_some() && session.is_admin(),
            false => session.is_admin(),
        })
    });
    let note = match signed_in {
        true => "Only a signed-in admin sees this page.",
        false => "Only admins see this page.",
    };
    view! {
        <Show
            when=move || allowed.get()
            fallback=move || {
                view! {
                    <div class="uc-page">
                        <h1 class="uc-page__title">{title}</h1>
                        <p class="uc-muted">{note}</p>
                    </div>
                }
            }
        >
            {children()}
        </Show>
    }
}
