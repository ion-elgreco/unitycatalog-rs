//! The Permissions tab's card, as upstream's console shows it: the
//! signed-in user's privileges on the object, every grant on it, and, for
//! its owner and the admins, "Grant me …", the form that grants and the ×
//! that revokes. The server shows other users their own grants only.

use std::collections::BTreeSet;

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::permissions::v1::PrivilegeAssignment;
use uc_common::SecurableType;

use super::card::Card;
use super::field::{Select, TextField};
use super::icons::{ShieldCheck, X};
use super::loaded::loaded_list;
use crate::api;
use crate::auth::{Session, use_auth, use_manages};
use crate::components::use_notify;
use crate::load::{Key, use_cache, use_load};

const CREATE_CATALOG: &str = "CREATE CATALOG";
const USE_CATALOG: &str = "USE CATALOG";
const CREATE_SCHEMA: &str = "CREATE SCHEMA";
const USE_SCHEMA: &str = "USE SCHEMA";
const CREATE_TABLE: &str = "CREATE TABLE";
const SELECT: &str = "SELECT";
const MODIFY: &str = "MODIFY";
const CREATE_FUNCTION: &str = "CREATE FUNCTION";
const EXECUTE: &str = "EXECUTE";
const CREATE_VOLUME: &str = "CREATE VOLUME";
const READ_VOLUME: &str = "READ VOLUME";
const CREATE_MODEL: &str = "CREATE MODEL";
const CREATE_EXTERNAL_LOCATION: &str = "CREATE EXTERNAL LOCATION";
const READ_FILES: &str = "READ FILES";
const WRITE_FILES: &str = "WRITE FILES";
const CREATE_EXTERNAL_TABLE: &str = "CREATE EXTERNAL TABLE";
const CREATE_EXTERNAL_VOLUME: &str = "CREATE EXTERNAL VOLUME";
const CREATE_MANAGED_STORAGE: &str = "CREATE MANAGED STORAGE";
const CREATE_STORAGE_CREDENTIAL: &str = "CREATE STORAGE CREDENTIAL";

/// The privileges that a securable of `kind` can hold, as the server names
/// them.
fn choices(kind: SecurableType) -> &'static [&'static str] {
    match kind {
        SecurableType::Catalog => &[
            USE_CATALOG,
            CREATE_SCHEMA,
            USE_SCHEMA,
            CREATE_TABLE,
            SELECT,
            MODIFY,
            CREATE_FUNCTION,
            EXECUTE,
            CREATE_VOLUME,
            READ_VOLUME,
            CREATE_MODEL,
        ],
        SecurableType::Schema => &[
            USE_SCHEMA,
            CREATE_TABLE,
            SELECT,
            MODIFY,
            CREATE_FUNCTION,
            EXECUTE,
            CREATE_VOLUME,
            READ_VOLUME,
            CREATE_MODEL,
        ],
        SecurableType::Table => &[SELECT, MODIFY],
        SecurableType::Volume => &[READ_VOLUME],
        SecurableType::Function | SecurableType::RegisteredModel => &[EXECUTE],
        SecurableType::Credential => &[CREATE_EXTERNAL_LOCATION],
        SecurableType::ExternalLocation => &[
            READ_FILES,
            WRITE_FILES,
            CREATE_EXTERNAL_TABLE,
            CREATE_EXTERNAL_VOLUME,
            CREATE_MANAGED_STORAGE,
        ],
        SecurableType::Metastore => &[
            CREATE_CATALOG,
            USE_CATALOG,
            CREATE_SCHEMA,
            USE_SCHEMA,
            CREATE_TABLE,
            SELECT,
            MODIFY,
            CREATE_FUNCTION,
            EXECUTE,
            CREATE_VOLUME,
            READ_VOLUME,
            CREATE_MODEL,
            CREATE_EXTERNAL_LOCATION,
            READ_FILES,
            WRITE_FILES,
            CREATE_EXTERNAL_TABLE,
            CREATE_EXTERNAL_VOLUME,
            CREATE_MANAGED_STORAGE,
            CREATE_STORAGE_CREDENTIAL,
        ],
    }
}

/// "Grant me …": the button's text, and the privileges that use an object
/// of `kind`, as the console offers them to its owner.
fn self_grant(kind: SecurableType) -> Option<(&'static str, &'static [&'static str])> {
    Some(match kind {
        SecurableType::Catalog => ("Grant me USE CATALOG", &[USE_CATALOG]),
        SecurableType::Schema => ("Grant me USE SCHEMA", &[USE_SCHEMA]),
        SecurableType::Table => ("Grant me read/write", &[SELECT, MODIFY]),
        SecurableType::Volume => ("Grant me READ VOLUME", &[READ_VOLUME]),
        SecurableType::Function | SecurableType::RegisteredModel => {
            ("Grant me EXECUTE", &[EXECUTE])
        }
        _ => return None,
    })
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
    // A memo, as `Cache::progress` says: the session outlives the page.
    let anonymous = Memo::new(move |_| auth.session.with(|s| matches!(s, Session::Anonymous)));
    let is_owner = Memo::new(move |_| me.with(|me| me.is_some() && *me == owner.get()));
    // The user's own grants.
    let mine = Memo::new(move |_| {
        let me = me.get();
        grants.with(|g| {
            g.ready()
                .map(|list| {
                    list.iter()
                        .filter(|a| Some(&a.principal) == me.as_ref())
                        .flat_map(|a| a.privileges.iter().cloned())
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default()
        })
    });

    let busy = RwSignal::new(false);
    // Grant or revoke, then show the grants the server answers with.
    let change = Callback::new({
        let full_name = full_name.clone();
        move |(principal, privileges, grant): (String, Vec<String>, bool)| {
            let full_name = full_name.clone();
            let key = key.clone();
            let what = privileges.join(", ");
            let (add, remove) = match grant {
                true => (privileges, Vec::new()),
                false => (Vec::new(), privileges),
            };
            busy.set(true);
            spawn_local(async move {
                let answer =
                    api::change_permissions(kind, &full_name, principal.clone(), add, remove).await;
                match answer {
                    Ok(list) => {
                        cache.set(key, list.privilege_assignments);
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
    let grant_me = self_grant(kind).map(|(label, privileges)| {
        view! {
            <Show when=move || manages.get() && me.with(Option::is_some)>
                <button
                    type="button"
                    class="uc-btn uc-btn--outline uc-btn--sm"
                    disabled=move || busy.get()
                    on:click=move |_| {
                        if let Some(me) = me.get_untracked() {
                            let privileges = privileges.iter().map(|p| p.to_string()).collect();
                            change.run((me, privileges, true));
                        }
                    }
                >
                    {label}
                </button>
            </Show>
        }
    });

    let own = move || {
        if anonymous.get() {
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
                    .map(|p| view! { <span class="uc-badge uc-badge--success">{p}</span> })
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
            actions=grant_me.into_any()
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
    change: Callback<(String, Vec<String>, bool)>,
) -> impl IntoView {
    let principal = assignment.principal;
    let chips = assignment
        .privileges
        .into_iter()
        .map(|privilege| {
            let principal = principal.clone();
            let label = format!("Revoke {privilege} from {principal}");
            view! {
                <span class="uc-badge">
                    {privilege.clone()}
                    <Show when=move || manages.get()>
                        {
                            let principal = principal.clone();
                            let privilege = privilege.clone();
                            let label = label.clone();
                            view! {
                                <button
                                    type="button"
                                    class="uc-badge__x"
                                    title=label.clone()
                                    aria-label=label
                                    on:click=move |_| {
                                        change.run((principal.clone(), vec![privilege.clone()], false))
                                    }
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
    change: Callback<(String, Vec<String>, bool)>,
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
        change.run((who, vec![privilege.get_untracked().to_string()], true));
        principal.set(String::new());
    };
    view! {
        <form class="uc-form-row" on:submit=submit>
            <TextField
                id="uc-grant-principal"
                label="principal (email)"
                value=principal
                placeholder="ada@example.com"
                autocomplete="off"
            />
            <Select
                id="uc-grant-privilege"
                label="privilege"
                choices
                value=privilege
                name=|p| p.to_string()
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
