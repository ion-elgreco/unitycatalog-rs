//! Protection against deletes on a detail page: whether the object is
//! protected, which its header shows and which turns Delete off, and the
//! button that sets or removes it, for those who can delete the object.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::protection::v1::Protection;
use uc_common::SecurableType;

use super::icons::{Lock, LockOpen};
use crate::api;
use crate::components::use_notify;
use crate::load::{Key, field, use_cache, use_load};

/// Whether the securable `full_name` of `kind` is protected: false until
/// it is loaded.
pub fn use_protection(kind: SecurableType, full_name: String) -> Signal<bool> {
    let name = StoredValue::new(full_name.clone());
    let protection = use_load(Key::Protection(kind, full_name), move || {
        let full_name = name.get_value();
        async move { api::protection(kind, &full_name).await }
    });
    field(protection, |p: &Protection| p.protected)
}

/// "Protect" or "Remove protection" in a page's actions, for those who can
/// delete the object (`when`).
#[component]
pub fn ProtectButton(
    kind: SecurableType,
    #[prop(into)] full_name: String,
    protected: Signal<bool>,
    when: Memo<bool>,
) -> impl IntoView {
    let cache = use_cache();
    let notify = use_notify();
    let busy = RwSignal::new(false);
    let name = StoredValue::new(full_name);
    let toggle = move |_| {
        let full_name = name.get_value();
        let protect = !protected.get_untracked();
        busy.set(true);
        spawn_local(async move {
            match api::set_protection(kind, &full_name, protect).await {
                Ok(protection) => {
                    cache.set(Key::Protection(kind, full_name), protection);
                    notify.success(match protect {
                        true => "Protection set",
                        false => "Protection removed",
                    });
                }
                Err(e) => notify.failed(&e, "Failed to change protection"),
            }
            // Only when the page is still there.
            let _ = busy.try_set(false);
        });
    };
    view! {
        <Show when=move || when.get()>
            <button
                type="button"
                class="uc-btn uc-btn--outline uc-btn--sm"
                disabled=move || busy.get()
                title=move || match protected.get() {
                    true => "Allow deletes again",
                    false => "Refuse deletes until the protection is removed",
                }
                on:click=toggle
            >
                {move || match protected.get() {
                    true => view! { <LockOpen/> "Remove protection" }.into_any(),
                    false => view! { <Lock/> "Protect" }.into_any(),
                }}
            </button>
        </Show>
    }
}
