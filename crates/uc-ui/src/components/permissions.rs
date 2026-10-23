//! The Permissions tab's card, as upstream's console shows it: the
//! signed-in user's privileges on the object, every grant on it, and, for
//! its owner and the admins, the form that grants and the × that revokes.
//! The server shows other users their own grants only.

use std::collections::BTreeSet;

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{Privilege, PrivilegeAssignment};
use uc_common::{SecurableType, wire_name};

use super::card::Card;
use super::field::{Select, TextField};
use super::icons::{ShieldCheck, X};
use super::loaded::loaded_list;
use crate::api;
use crate::auth::{Session, use_auth, use_manages};
use crate::components::use_notify;
use crate::load::{Key, use_cache, use_load};

/// The privileges that a securable of `kind` can hold.
fn choices(kind: SecurableType) -> &'static [Privilege] {
    use Privilege::*;
    match kind {
        SecurableType::Catalog => &[
            UseCatalog,
            CreateSchema,
            UseSchema,
            CreateTable,
            Select,
            Modify,
            CreateFunction,
            Execute,
            CreateVolume,
            ReadVolume,
            CreateModel,
        ],
        SecurableType::Schema => &[
            UseSchema,
            CreateTable,
            Select,
            Modify,
            CreateFunction,
            Execute,
            CreateVolume,
            ReadVolume,
            CreateModel,
        ],
        SecurableType::Table => &[Select, Modify],
        SecurableType::Volume => &[ReadVolume],
        SecurableType::Function | SecurableType::RegisteredModel => &[Execute],
        _ => Privilege::ALL,
    }
}

#[component]
pub fn PermissionsCard(
    kind: SecurableType,
    #[prop(into)] full_name: String,
    #[prop(into)] owner: Signal<Option<String>>,
) -> impl IntoView {
    let key = Key::Permissions(kind, full_name.clone());
    let grants = use_load(key.clone(), {
        let full_name = full_name.clone();
        move || {
            let full_name = full_name.clone();
            async move { api::permissions(kind, &full_name).await }
        }
    });
    let auth = use_auth();
    let cache = use_cache();
    let notify = use_notify();
    let me = Memo::new(move |_| auth.session.with(|s| s.principal().map(str::to_string)));
    let manages = use_manages(owner);
    let anonymous = move || auth.session.with(|s| matches!(s, Session::Anonymous));
    let is_owner = Memo::new(move |_| me.with(|me| me.is_some() && *me == owner.get()));
    // The user's own grants.
    let mine = Memo::new(move |_| {
        let me = me.get();
        grants.with(|g| {
            g.ready()
                .map(|list| {
                    list.iter()
                        .filter(|a| Some(&a.principal) == me.as_ref())
                        .flat_map(|a| a.privileges.iter().copied())
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default()
        })
    });

    let busy = RwSignal::new(false);
    // Grant or revoke, then show the grants the server answers with.
    let change = Callback::new({
        let full_name = full_name.clone();
        move |(principal, privilege, grant): (String, Privilege, bool)| {
            let full_name = full_name.clone();
            let key = key.clone();
            busy.set(true);
            spawn_local(async move {
                let answer =
                    api::change_permission(kind, &full_name, principal.clone(), privilege, grant)
                        .await;
                match answer {
                    Ok(list) => {
                        cache.set(key, list.privilege_assignments);
                        let what = wire_name(&privilege);
                        notify.success(match grant {
                            true => format!("Granted {what} to {principal}"),
                            false => format!("Revoked {what} from {principal}"),
                        });
                    }
                    Err(e) => notify.failed(
                        &e,
                        if grant {
                            "Failed to grant"
                        } else {
                            "Failed to revoke"
                        },
                    ),
                }
                busy.set(false);
            });
        }
    });

    let own = move || {
        if anonymous() {
            return view! {
                <p class="uc-empty">"Authorization is off: everyone may do everything."</p>
            }
            .into_any();
        }
        let mine = mine.get();
        if !is_owner.get() && mine.is_empty() {
            return view! { <p class="uc-empty">"None."</p> }.into_any();
        }
        view! {
            <div class="uc-inline uc-inline--wrap">
                <Show when=move || is_owner.get()>
                    <span class="uc-badge uc-badge--primary">"OWNER"</span>
                </Show>
                {mine
                    .into_iter()
                    .map(|p| view! { <span class="uc-badge uc-badge--success">{wire_name(&p)}</span> })
                    .collect_view()}
            </div>
        }
        .into_any()
    };

    let table = move || {
        loaded_list(grants.get(), "No privileges granted.", |list| {
            view! {
                <div class="uc-table-wrap">
                    <table class="uc-table">
                        <thead>
                            <tr>
                                <th>"Principal"</th>
                                <th>"Privileges"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {list
                                .into_iter()
                                .map(|assignment| view! { <GrantRow assignment manages change/> })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
            }
            .into_any()
        })
    };

    view! {
        <Card
            icon=view! { <ShieldCheck/> }.into_any()
            title="Permissions"
            subtitle=format!("{} · {full_name}", kind.as_str())
        >
            <div>
                <h4 class="uc-section-label">"Your privileges"</h4>
                {own}
            </div>
            <div class="uc-divider"></div>
            <div>
                <h4 class="uc-section-label">
                    {move || if manages.get() { "Manage grants" } else { "Grants" }}
                </h4>
                {table}
            </div>
            <Show when=move || manages.get()>
                <GrantForm kind busy change/>
            </Show>
        </Card>
    }
}

/// One principal's grants; with `manages`, an × after each privilege
/// revokes it.
#[component]
fn GrantRow(
    assignment: PrivilegeAssignment,
    manages: Memo<bool>,
    change: Callback<(String, Privilege, bool)>,
) -> impl IntoView {
    let principal = assignment.principal;
    let chips = assignment
        .privileges
        .into_iter()
        .map(|privilege| {
            let principal = principal.clone();
            let name = wire_name(&privilege);
            let label = format!("Revoke {name} from {principal}");
            view! {
                <span class="uc-badge">
                    {name}
                    <Show when=move || manages.get()>
                        {
                            let principal = principal.clone();
                            let label = label.clone();
                            view! {
                                <button
                                    type="button"
                                    class="uc-badge__x"
                                    title=label.clone()
                                    aria-label=label
                                    on:click=move |_| change.run((principal.clone(), privilege, false))
                                >
                                    <X/>
                                </button>
                            }
                        }
                    </Show>
                </span>
            }
        })
        .collect_view();
    view! {
        <tr>
            <td class="uc-strong">{principal.clone()}</td>
            <td>
                <div class="uc-inline uc-inline--wrap">{chips}</div>
            </td>
        </tr>
    }
}

/// The principal, the privilege and "Grant".
#[component]
fn GrantForm(
    kind: SecurableType,
    busy: RwSignal<bool>,
    change: Callback<(String, Privilege, bool)>,
) -> impl IntoView {
    let choices = choices(kind);
    let principal = RwSignal::new(String::new());
    let privilege = RwSignal::new(choices[0]);
    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let who = principal.get_untracked().trim().to_string();
        if who.is_empty() {
            return;
        }
        change.run((who, privilege.get_untracked(), true));
        principal.set(String::new());
    };
    view! {
        <form class="uc-form-row" on:submit=submit>
            <TextField
                id="uc-grant-principal"
                label="Principal (email)"
                value=principal
                placeholder="ada@example.com"
                autocomplete="off"
            />
            <Select
                id="uc-grant-privilege"
                label="Privilege"
                choices
                value=privilege
                name=wire_name
                fixed=true
            />
            <button
                type="submit"
                class="uc-btn"
                disabled=move || busy.get() || principal.with(|p| p.trim().is_empty())
            >
                "Grant"
            </button>
        </form>
    }
}
